use crate::command::*;
use crate::core::device::*;
use crate::raytracing::GpuPrimitive;
use crate::resource::allocation::{create_buffer_allocation, BufferAllocation};
use crate::resource::gpu_resource::GpuResource;
use crate::resource::{HitShadingRecord, HitShadingTable};
use crate::vulkan::*;
use anyhow::Result;
use cgmath::Matrix4;
use thyllore_math_core::{AffineRows3x4, GpuMat4};
use vulkanalia::vk::KhrAccelerationStructureExtension;

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct RRBLAS {
    pub buffer_allocation: Option<BufferAllocation>,
    pub acceleration_structure: Option<vk::AccelerationStructureKHR>,
    #[gpu_resource(skip)]
    pub device_address: vk::DeviceAddress,
    pub update_scratch: Option<BufferAllocation>,
    #[gpu_resource(skip)]
    pub update_scratch_size: vk::DeviceSize,
    #[gpu_resource(skip)]
    pub transform: vk::TransformMatrixKHR,
}

#[must_use]
#[derive(Debug, Default, GpuResource)]
pub struct RRTLAS {
    pub buffer_allocation: Option<BufferAllocation>,
    pub acceleration_structure: Option<vk::AccelerationStructureKHR>,
    #[gpu_resource(skip)]
    pub device_address: vk::DeviceAddress,
    pub update_scratch: Option<BufferAllocation>,
    #[gpu_resource(skip)]
    pub update_scratch_size: vk::DeviceSize,
    pub instances_buf: Option<BufferAllocation>,
    #[gpu_resource(skip)]
    pub instances_size: vk::DeviceSize,
}

#[must_use]
#[derive(Debug, GpuResource)]
pub struct RRAccelerationStructure {
    pub blas_list: Vec<RRBLAS>,
    pub procedural_blas: Vec<RRBLAS>,
    pub tlas: RRTLAS,
    pub hit_shading_table: Option<HitShadingTable>,
}

unsafe fn get_buffer_address(rrdevice: &RRDevice, buffer: vk::Buffer) -> vk::DeviceAddress {
    rrdevice
        .device
        .get_buffer_device_address(&vk::BufferDeviceAddressInfo::builder().buffer(buffer))
}

unsafe fn create_scratch_buffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    size: vk::DeviceSize,
) -> Result<BufferAllocation> {
    create_buffer_allocation(
        instance,
        rrdevice,
        size,
        vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )
}

unsafe fn ensure_update_scratch(
    instance: &Instance,
    rrdevice: &RRDevice,
    scratch: &mut Option<BufferAllocation>,
    scratch_size: &mut vk::DeviceSize,
    required_size: vk::DeviceSize,
) -> Result<vk::DeviceAddress> {
    let allocation = match scratch {
        Some(existing) if required_size <= *scratch_size => existing,
        _ => {
            let new_scratch = create_scratch_buffer(instance, rrdevice, required_size)?;
            scratch.destroy_gpu(rrdevice);
            *scratch_size = required_size;
            scratch.insert(new_scratch)
        }
    };
    Ok(get_buffer_address(rrdevice, allocation.buffer))
}

unsafe fn execute_as_build(
    rrdevice: &RRDevice,
    rrcommand_pool: &RRCommandPool,
    build_info: &vk::AccelerationStructureBuildGeometryInfoKHRBuilder,
    primitive_count: u32,
) -> Result<()> {
    let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::builder()
        .primitive_count(primitive_count)
        .primitive_offset(0)
        .first_vertex(0)
        .transform_offset(0)
        .build();
    let build_range_infos = [build_range_info];

    let command_buffer = begin_single_time_commands(rrdevice, rrcommand_pool.command_pool)?;

    rrdevice.device.cmd_build_acceleration_structures_khr(
        command_buffer,
        std::slice::from_ref(build_info),
        &[&build_range_infos[0]],
    );

    end_single_time_commands(
        rrdevice,
        rrdevice.graphics_queue,
        rrcommand_pool.command_pool,
        command_buffer,
    )
}

unsafe fn create_acceleration_structure_with_buffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    size: vk::DeviceSize,
    as_type: vk::AccelerationStructureTypeKHR,
) -> Result<(vk::AccelerationStructureKHR, BufferAllocation)> {
    let as_buf = create_buffer_allocation(
        instance,
        rrdevice,
        size,
        vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let as_create_info = vk::AccelerationStructureCreateInfoKHR::builder()
        .buffer(as_buf.buffer)
        .size(size)
        .type_(as_type);

    let acceleration_structure = rrdevice
        .device
        .create_acceleration_structure_khr(&as_create_info, None)?;

    Ok((acceleration_structure, as_buf))
}

fn build_triangle_geometry(
    vertex_buffer_address: vk::DeviceAddress,
    vertex_count: u32,
    vertex_stride: u32,
    index_buffer_address: vk::DeviceAddress,
) -> vk::AccelerationStructureGeometryKHR {
    let triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
        .vertex_format(vk::Format::R32G32B32_SFLOAT)
        .vertex_data(vk::DeviceOrHostAddressConstKHR {
            device_address: vertex_buffer_address,
        })
        .vertex_stride(vertex_stride as vk::DeviceSize)
        .max_vertex(vertex_count - 1)
        .index_type(vk::IndexType::UINT32)
        .index_data(vk::DeviceOrHostAddressConstKHR {
            device_address: index_buffer_address,
        })
        .build();

    vk::AccelerationStructureGeometryKHR::builder()
        .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
        .geometry(vk::AccelerationStructureGeometryDataKHR { triangles })
        .flags(vk::GeometryFlagsKHR::OPAQUE)
        .build()
}

unsafe fn fill_instances_buffer(
    rrdevice: &RRDevice,
    buf: &BufferAllocation,
    instances_size: vk::DeviceSize,
    blas_list: &[RRBLAS],
    procedural_blas: &[RRBLAS],
) -> Result<()> {
    let ptr =
        rrdevice
            .device
            .map_memory(buf.memory, 0, instances_size, vk::MemoryMapFlags::empty())?
            as *mut vk::AccelerationStructureInstanceKHR;

    let mesh_count = blas_list.len();
    let total = mesh_count + procedural_blas.len();
    let mut instances: Vec<vk::AccelerationStructureInstanceKHR> = Vec::with_capacity(total);

    for (i, blas) in blas_list.iter().enumerate() {
        instances.push(vk::AccelerationStructureInstanceKHR {
            transform: blas.transform,
            instance_custom_index_and_mask: vk::Bitfield24_8::new(i as u32, 0xFF),
            instance_shader_binding_table_record_offset_and_flags: vk::Bitfield24_8::new(0, 0),
            acceleration_structure_reference: blas.device_address,
        });
    }

    for (j, blas) in procedural_blas.iter().enumerate() {
        instances.push(vk::AccelerationStructureInstanceKHR {
            transform: blas.transform,
            instance_custom_index_and_mask: vk::Bitfield24_8::new((mesh_count + j) as u32, 0xFF),
            instance_shader_binding_table_record_offset_and_flags: vk::Bitfield24_8::new(1, 0),
            acceleration_structure_reference: blas.device_address,
        });
    }

    std::ptr::copy_nonoverlapping(instances.as_ptr(), ptr, instances.len());
    rrdevice.device.unmap_memory(buf.memory);

    Ok(())
}

unsafe fn upload_instances_buffer(
    instance: &Instance,
    rrdevice: &RRDevice,
    blas_list: &[RRBLAS],
    procedural_blas: &[RRBLAS],
) -> Result<BufferAllocation> {
    let total = blas_list.len() + procedural_blas.len();
    let instances_size =
        (std::mem::size_of::<vk::AccelerationStructureInstanceKHR>() * total) as vk::DeviceSize;

    let buf = create_buffer_allocation(
        instance,
        rrdevice,
        instances_size,
        vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    fill_instances_buffer(rrdevice, &buf, instances_size, blas_list, procedural_blas)?;

    Ok(buf)
}

const AS_BUILD_FLAGS: vk::BuildAccelerationStructureFlagsKHR =
    vk::BuildAccelerationStructureFlagsKHR::from_bits_truncate(
        vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE.bits()
            | vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE.bits(),
    );

impl RRAccelerationStructure {
    pub fn new() -> Self {
        Self {
            blas_list: Vec::new(),
            procedural_blas: Vec::new(),
            tlas: RRTLAS::default(),
            hit_shading_table: None,
        }
    }

    pub unsafe fn create_blas(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        vertex_buffer: &vk::Buffer,
        vertex_count: u32,
        vertex_stride: u32,
        index_buffer: &vk::Buffer,
        index_count: u32,
    ) -> Result<RRBLAS> {
        let device = &rrdevice.device;

        let vertex_addr = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(*vertex_buffer),
        );
        let index_addr = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(*index_buffer),
        );

        let geometry =
            build_triangle_geometry(vertex_addr, vertex_count, vertex_stride, index_addr);
        let primitive_count = index_count / 3;

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(std::slice::from_ref(&geometry));

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        device.get_acceleration_structure_build_sizes_khr(
            vk::AccelerationStructureBuildTypeKHR::DEVICE,
            &build_info,
            &[primitive_count],
            &mut size_info,
        );

        let (acceleration_structure, as_buf) = create_acceleration_structure_with_buffer(
            instance,
            rrdevice,
            size_info.acceleration_structure_size,
            vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
        )?;

        let mut scratch = create_scratch_buffer(instance, rrdevice, size_info.build_scratch_size)?;

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(acceleration_structure)
            .geometries(std::slice::from_ref(&geometry))
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: get_buffer_address(rrdevice, scratch.buffer),
            });

        execute_as_build(rrdevice, rrcommand_pool, &build_info, primitive_count)?;
        scratch.destroy_gpu(rrdevice);

        let device_address = device.get_acceleration_structure_device_address_khr(
            &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                .acceleration_structure(acceleration_structure),
        );

        Ok(RRBLAS {
            buffer_allocation: Some(as_buf),
            acceleration_structure: Some(acceleration_structure),
            device_address,
            update_scratch: None,
            update_scratch_size: 0,
            transform: vk::TransformMatrixKHR {
                matrix: AffineRows3x4::IDENTITY.rows,
            },
        })
    }

    pub unsafe fn create_aabb_blas(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        aabb_buffer: &vk::Buffer,
        aabb_count: u32,
    ) -> Result<RRBLAS> {
        let device = &rrdevice.device;

        let aabb_addr = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(*aabb_buffer),
        );

        let aabbs_data = vk::AccelerationStructureGeometryAabbsDataKHR::builder()
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: aabb_addr,
            })
            .stride(24)
            .build();

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::AABBS)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                aabbs: vk::AccelerationStructureGeometryAabbsDataKHR::builder()
                    .data(vk::DeviceOrHostAddressConstKHR {
                        device_address: aabb_addr,
                    })
                    .stride(std::mem::size_of::<vk::AabbPositionsKHR>() as u64)
                    .build(),
            })
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .build();

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(std::slice::from_ref(&geometry));

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        device.get_acceleration_structure_build_sizes_khr(
            vk::AccelerationStructureBuildTypeKHR::DEVICE,
            &build_info,
            &[aabb_count],
            &mut size_info,
        );

        let (acceleration_structure, as_buf) = create_acceleration_structure_with_buffer(
            instance,
            rrdevice,
            size_info.acceleration_structure_size,
            vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
        )?;

        let mut scratch = create_scratch_buffer(instance, rrdevice, size_info.build_scratch_size)?;

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(acceleration_structure)
            .geometries(std::slice::from_ref(&geometry))
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: get_buffer_address(rrdevice, scratch.buffer),
            });

        execute_as_build(rrdevice, rrcommand_pool, &build_info, aabb_count)?;
        scratch.destroy_gpu(rrdevice);

        let device_address = device.get_acceleration_structure_device_address_khr(
            &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                .acceleration_structure(acceleration_structure),
        );

        Ok(RRBLAS {
            buffer_allocation: Some(as_buf),
            acceleration_structure: Some(acceleration_structure),
            device_address,
            update_scratch: None,
            update_scratch_size: 0,
            transform: vk::TransformMatrixKHR {
                matrix: AffineRows3x4::IDENTITY.rows,
            },
        })
    }

    pub unsafe fn create_procedural_blas(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        model: &Matrix4<f32>,
        aabb: vk::AabbPositionsKHR,
    ) -> Result<RRBLAS> {
        let size = std::mem::size_of::<vk::AabbPositionsKHR>() as vk::DeviceSize;
        let mut buf = create_buffer_allocation(
            instance,
            rrdevice,
            size,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let ptr = rrdevice
            .device
            .map_memory(buf.memory, 0, size, vk::MemoryMapFlags::empty())?
            as *mut vk::AabbPositionsKHR;
        ptr.write(aabb);
        rrdevice.device.unmap_memory(buf.memory);
        let mut blas = Self::create_aabb_blas(instance, rrdevice, rrcommand_pool, &buf.buffer, 1)?;
        buf.destroy_gpu(rrdevice);
        blas.transform = vk::TransformMatrixKHR {
            matrix: AffineRows3x4::from_mat4(*model).rows,
        };
        Ok(blas)
    }

    pub unsafe fn create_tlas(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        blas_list: &[RRBLAS],
        procedural_blas: &[RRBLAS],
    ) -> Result<RRTLAS> {
        let device = &rrdevice.device;

        let mut instances_buf = if blas_list.is_empty() && procedural_blas.is_empty() {
            // Empty case: allocate buffer with 1 zero-initialized instance (mask=0, accelerationStructureReference=0)
            // This is an "inactive instance" per spec — the TLAS has 1 primitive but no hits.
            let instances_size =
                std::mem::size_of::<vk::AccelerationStructureInstanceKHR>() as vk::DeviceSize;

            let buf = create_buffer_allocation(
                instance,
                rrdevice,
                instances_size,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;

            // Zero-initialize: write a single zeroed instance
            let ptr = rrdevice.device.map_memory(
                buf.memory,
                0,
                instances_size,
                vk::MemoryMapFlags::empty(),
            )? as *mut vk::AccelerationStructureInstanceKHR;
            std::ptr::write(ptr, vk::AccelerationStructureInstanceKHR::default());
            rrdevice.device.unmap_memory(buf.memory);

            buf
        } else {
            upload_instances_buffer(instance, rrdevice, blas_list, procedural_blas)?
        };

        let instances_data = vk::AccelerationStructureGeometryInstancesDataKHR::builder()
            .array_of_pointers(false)
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: get_buffer_address(rrdevice, instances_buf.buffer),
            });

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: *instances_data,
            })
            .flags(vk::GeometryFlagsKHR::OPAQUE);

        let primitive_count = if blas_list.is_empty() && procedural_blas.is_empty() {
            1
        } else {
            (blas_list.len() + procedural_blas.len()) as u32
        };

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(std::slice::from_ref(&geometry));

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        device.get_acceleration_structure_build_sizes_khr(
            vk::AccelerationStructureBuildTypeKHR::DEVICE,
            &build_info,
            &[primitive_count],
            &mut size_info,
        );

        let (acceleration_structure, as_buf) = create_acceleration_structure_with_buffer(
            instance,
            rrdevice,
            size_info.acceleration_structure_size,
            vk::AccelerationStructureTypeKHR::TOP_LEVEL,
        )?;

        let mut scratch = create_scratch_buffer(instance, rrdevice, size_info.build_scratch_size)?;

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(acceleration_structure)
            .geometries(std::slice::from_ref(&geometry))
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: get_buffer_address(rrdevice, scratch.buffer),
            });

        execute_as_build(rrdevice, rrcommand_pool, &build_info, primitive_count)?;

        scratch.destroy_gpu(rrdevice);
        instances_buf.destroy_gpu(rrdevice);

        let device_address = device.get_acceleration_structure_device_address_khr(
            &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                .acceleration_structure(acceleration_structure),
        );

        Ok(RRTLAS {
            buffer_allocation: Some(as_buf),
            acceleration_structure: Some(acceleration_structure),
            device_address,
            update_scratch: None,
            update_scratch_size: 0,
            instances_buf: None,
            instances_size: 0,
        })
    }

    pub unsafe fn update_blas(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        blas: &mut RRBLAS,
        vertex_buffer: &vk::Buffer,
        vertex_count: u32,
        vertex_stride: u32,
        index_buffer: &vk::Buffer,
        index_count: u32,
    ) -> Result<()> {
        let device = &rrdevice.device;

        let vertex_addr = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(*vertex_buffer),
        );
        let index_addr = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(*index_buffer),
        );

        let geometry =
            build_triangle_geometry(vertex_addr, vertex_count, vertex_stride, index_addr);
        let primitive_count = index_count / 3;

        let accel_structure = blas
            .acceleration_structure
            .ok_or_else(|| anyhow::anyhow!("BLAS acceleration structure not initialized"))?;

        let build_info_for_size = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::UPDATE)
            .geometries(std::slice::from_ref(&geometry));

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        device.get_acceleration_structure_build_sizes_khr(
            vk::AccelerationStructureBuildTypeKHR::DEVICE,
            &build_info_for_size,
            &[primitive_count],
            &mut size_info,
        );

        let scratch_address = ensure_update_scratch(
            instance,
            rrdevice,
            &mut blas.update_scratch,
            &mut blas.update_scratch_size,
            size_info.update_scratch_size,
        )?;

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::UPDATE)
            .src_acceleration_structure(accel_structure)
            .dst_acceleration_structure(accel_structure)
            .geometries(std::slice::from_ref(&geometry))
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_address,
            });

        execute_as_build(rrdevice, rrcommand_pool, &build_info, primitive_count)?;

        Ok(())
    }

    pub unsafe fn update_tlas(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        tlas: &mut RRTLAS,
        blas_list: &[RRBLAS],
        procedural_blas: &[RRBLAS],
    ) -> Result<()> {
        let device = &rrdevice.device;

        if blas_list.is_empty() && procedural_blas.is_empty() {
            return Ok(());
        }

        let total = blas_list.len() + procedural_blas.len();
        let instances_size =
            (std::mem::size_of::<vk::AccelerationStructureInstanceKHR>() * total) as vk::DeviceSize;

        let instances_buf = match &mut tlas.instances_buf {
            Some(existing) if instances_size <= tlas.instances_size => {
                fill_instances_buffer(
                    rrdevice,
                    existing,
                    instances_size,
                    blas_list,
                    procedural_blas,
                )?;
                existing
            }
            _ => {
                let new_buf =
                    upload_instances_buffer(instance, rrdevice, blas_list, procedural_blas)?;
                tlas.instances_buf.destroy_gpu(rrdevice);
                tlas.instances_size = instances_size;
                tlas.instances_buf.insert(new_buf)
            }
        };

        let instances_data = vk::AccelerationStructureGeometryInstancesDataKHR::builder()
            .array_of_pointers(false)
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: get_buffer_address(rrdevice, instances_buf.buffer),
            });

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: *instances_data,
            })
            .flags(vk::GeometryFlagsKHR::OPAQUE);

        let primitive_count = total as u32;

        let accel_structure = tlas
            .acceleration_structure
            .ok_or_else(|| anyhow::anyhow!("TLAS acceleration structure not initialized"))?;

        let build_info_for_size = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::UPDATE)
            .geometries(std::slice::from_ref(&geometry));

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        device.get_acceleration_structure_build_sizes_khr(
            vk::AccelerationStructureBuildTypeKHR::DEVICE,
            &build_info_for_size,
            &[primitive_count],
            &mut size_info,
        );

        let scratch_address = ensure_update_scratch(
            instance,
            rrdevice,
            &mut tlas.update_scratch,
            &mut tlas.update_scratch_size,
            size_info.update_scratch_size,
        )?;

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(AS_BUILD_FLAGS)
            .mode(vk::BuildAccelerationStructureModeKHR::UPDATE)
            .src_acceleration_structure(accel_structure)
            .dst_acceleration_structure(accel_structure)
            .geometries(std::slice::from_ref(&geometry))
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_address,
            });

        execute_as_build(rrdevice, rrcommand_pool, &build_info, primitive_count)?;

        Ok(())
    }

    pub unsafe fn update_all(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        vertex_buffers: &[(&vk::Buffer, u32, u32, &vk::Buffer, u32)],
    ) -> Result<()> {
        for (i, (vertex_buffer, vertex_count, vertex_stride, index_buffer, index_count)) in
            vertex_buffers.iter().enumerate()
        {
            if i < self.blas_list.len() {
                Self::update_blas(
                    instance,
                    rrdevice,
                    rrcommand_pool,
                    &mut self.blas_list[i],
                    vertex_buffer,
                    *vertex_count,
                    *vertex_stride,
                    index_buffer,
                    *index_count,
                )?;
            }
        }

        Self::update_tlas(
            instance,
            rrdevice,
            rrcommand_pool,
            &mut self.tlas,
            &self.blas_list,
            &self.procedural_blas,
        )?;

        self.fill_hit_shading_table(instance, rrdevice, vertex_buffers, &[])?;

        Ok(())
    }

    pub unsafe fn fill_hit_shading_table(
        &mut self,
        instance: &Instance,
        rrdevice: &RRDevice,
        vertex_buffers: &[(&vk::Buffer, u32, u32, &vk::Buffer, u32)],
        procedurals: &[GpuPrimitive],
    ) -> Result<()> {
        let mut records: Vec<HitShadingRecord> =
            Vec::with_capacity(vertex_buffers.len() + procedurals.len());

        for (vertex_buffer, _, _, index_buffer, _) in vertex_buffers.iter() {
            let vertex_address = rrdevice.device.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::builder().buffer(**vertex_buffer),
            );
            let index_address = rrdevice.device.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::builder().buffer(**index_buffer),
            );

            records.push(HitShadingRecord {
                vertex_address,
                index_address,
                effect_data_address: 0,
                reserved: 0,
                model: GpuMat4::IDENTITY,
                normal_matrix: GpuMat4::IDENTITY,
                base_color: [1.0, 1.0, 1.0, 1.0],
                params: [0.0; 4],
            });
        }

        for primitive in procedurals {
            records.push(HitShadingRecord {
                vertex_address: 0,
                index_address: 0,
                effect_data_address: primitive.effect_data_address,
                reserved: 0,
                model: GpuMat4::from_mat4(primitive.model),
                normal_matrix: GpuMat4::normal_matrix_of(primitive.model),
                base_color: primitive.base_color,
                params: primitive.params,
            });
        }

        if records.is_empty() {
            records.push(HitShadingRecord::default_record());
        }

        if !records.is_empty() {
            let table = match &mut self.hit_shading_table {
                Some(table) if table.capacity >= records.len() => table,
                _ => {
                    let new_table = HitShadingTable::new(instance, rrdevice, records.len())?;
                    self.hit_shading_table.destroy_gpu(rrdevice);
                    self.hit_shading_table.insert(new_table)
                }
            };
            table.upload(rrdevice, &records)?;
        }

        Ok(())
    }
}
