use crate::vulkanr::command::*;
use crate::vulkanr::core::device::*;
use crate::vulkanr::vulkan::*;
use anyhow::Result;
use vulkanalia::vk::KhrAccelerationStructureExtension;

/// Bottom-Level Acceleration Structure (BLAS)
/// ジオメトリ（頂点・インデックス）データから構築
#[derive(Clone, Debug, Default)]
pub struct RRBLAS {
    pub acceleration_structure: Option<vk::AccelerationStructureKHR>,
    pub buffer: Option<vk::Buffer>,
    pub buffer_memory: Option<vk::DeviceMemory>,
    pub device_address: vk::DeviceAddress,
}

/// Top-Level Acceleration Structure (TLAS)
/// BLASインスタンスの配置情報から構築
#[derive(Clone, Debug, Default)]
pub struct RRTLAS {
    pub acceleration_structure: Option<vk::AccelerationStructureKHR>,
    pub buffer: Option<vk::Buffer>,
    pub buffer_memory: Option<vk::DeviceMemory>,
    pub device_address: vk::DeviceAddress,
}

/// Acceleration Structure管理
#[derive(Clone, Debug)]
pub struct RRAccelerationStructure {
    pub blas_list: Vec<RRBLAS>,
    pub tlas: RRTLAS,
}

impl RRAccelerationStructure {
    pub fn new() -> Self {
        Self {
            blas_list: Vec::new(),
            tlas: RRTLAS::default(),
        }
    }

    /// BLASを構築（三角形ジオメトリ）
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

        // 1. 頂点・インデックスバッファのデバイスアドレス取得
        let vertex_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(*vertex_buffer),
        );
        let index_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(*index_buffer),
        );

        // 2. ジオメトリデータの記述
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
            });

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                triangles: *triangles,
            })
            .flags(vk::GeometryFlagsKHR::OPAQUE);

        // 3. ビルド情報
        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(
                vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE
                    | vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE,
            )
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(std::slice::from_ref(&geometry));

        let primitive_count = index_count / 3;

        // 4. ビルドサイズの取得
        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        device.get_acceleration_structure_build_sizes_khr(
            vk::AccelerationStructureBuildTypeKHR::DEVICE,
            &build_info,
            &[primitive_count],
            &mut size_info,
        );

        // 5. Acceleration Structure用バッファ作成
        let as_buffer_info = vk::BufferCreateInfo::builder()
            .size(size_info.acceleration_structure_size)
            .usage(
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );

        let as_buffer = device.create_buffer(&as_buffer_info, None)?;
        let as_memory_requirements = device.get_buffer_memory_requirements(as_buffer);

        let as_memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            as_memory_requirements,
        )?;

        let mut memory_allocate_flags_info =
            vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

        let as_memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(as_memory_requirements.size)
            .memory_type_index(as_memory_type_index)
            .push_next(&mut memory_allocate_flags_info);

        let as_buffer_memory = device.allocate_memory(&as_memory_info, None)?;
        device.bind_buffer_memory(as_buffer, as_buffer_memory, 0)?;

        // 6. Acceleration Structure作成
        let as_create_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(as_buffer)
            .size(size_info.acceleration_structure_size)
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL);

        let acceleration_structure =
            device.create_acceleration_structure_khr(&as_create_info, None)?;

        // 7. スクラッチバッファ作成
        let scratch_buffer_info = vk::BufferCreateInfo::builder()
            .size(size_info.build_scratch_size)
            .usage(
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );

        let scratch_buffer = device.create_buffer(&scratch_buffer_info, None)?;
        let scratch_memory_requirements = device.get_buffer_memory_requirements(scratch_buffer);

        let scratch_memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            scratch_memory_requirements,
        )?;

        let mut scratch_memory_allocate_flags_info =
            vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

        let scratch_memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(scratch_memory_requirements.size)
            .memory_type_index(scratch_memory_type_index)
            .push_next(&mut scratch_memory_allocate_flags_info);

        let scratch_buffer_memory = device.allocate_memory(&scratch_memory_info, None)?;
        device.bind_buffer_memory(scratch_buffer, scratch_buffer_memory, 0)?;

        let scratch_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(scratch_buffer),
        );

        // 8. ビルドコマンド記録・実行
        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(
                vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE
                    | vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE,
            )
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(acceleration_structure)
            .geometries(std::slice::from_ref(&geometry))
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_buffer_address,
            });

        let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(primitive_count)
            .primitive_offset(0)
            .first_vertex(0)
            .transform_offset(0)
            .build();

        let build_range_infos = [build_range_info];

        // コマンドバッファでビルド実行
        let command_buffer = crate::vulkanr::command::begin_single_time_commands(
            rrdevice,
            rrcommand_pool.command_pool,
        )?;

        device.cmd_build_acceleration_structures_khr(
            command_buffer,
            std::slice::from_ref(&build_info),
            &[&build_range_infos[0]],
        );

        crate::vulkanr::command::end_single_time_commands(
            rrdevice,
            rrdevice.graphics_queue,
            rrcommand_pool.command_pool,
            command_buffer,
        )?;

        // スクラッチバッファは不要になったので破棄
        device.destroy_buffer(scratch_buffer, None);
        device.free_memory(scratch_buffer_memory, None);

        // デバイスアドレス取得
        let device_address = device.get_acceleration_structure_device_address_khr(
            &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                .acceleration_structure(acceleration_structure),
        );

        Ok(RRBLAS {
            acceleration_structure: Some(acceleration_structure),
            buffer: Some(as_buffer),
            buffer_memory: Some(as_buffer_memory),
            device_address,
        })
    }

    /// TLASを構築
    pub unsafe fn create_tlas(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        blas_list: &[RRBLAS],
    ) -> Result<RRTLAS> {
        let device = &rrdevice.device;

        if blas_list.is_empty() {
            return Ok(RRTLAS::default());
        }

        // 1. インスタンスデータの作成（単位行列で配置）
        let instances: Vec<vk::AccelerationStructureInstanceKHR> = blas_list
            .iter()
            .enumerate()
            .map(|(i, blas)| {
                vk::AccelerationStructureInstanceKHR {
                    transform: vk::TransformMatrixKHR {
                        matrix: [
                            [1.0, 0.0, 0.0, 0.0], // Row 0
                            [0.0, 1.0, 0.0, 0.0], // Row 1
                            [0.0, 0.0, 1.0, 0.0], // Row 2
                        ],
                    },
                    instance_custom_index_and_mask: vk::Bitfield24_8::new(i as u32, 0xFF),
                    instance_shader_binding_table_record_offset_and_flags: vk::Bitfield24_8::new(
                        0, 0,
                    ),
                    acceleration_structure_reference: blas.device_address,
                }
            })
            .collect();

        // 2. インスタンスバッファ作成
        let instances_size = (std::mem::size_of::<vk::AccelerationStructureInstanceKHR>()
            * instances.len()) as vk::DeviceSize;

        let instances_buffer_info = vk::BufferCreateInfo::builder().size(instances_size).usage(
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let instances_buffer = device.create_buffer(&instances_buffer_info, None)?;
        let instances_memory_requirements = device.get_buffer_memory_requirements(instances_buffer);

        let instances_memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            instances_memory_requirements,
        )?;

        let mut instances_memory_allocate_flags_info =
            vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

        let instances_memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(instances_memory_requirements.size)
            .memory_type_index(instances_memory_type_index)
            .push_next(&mut instances_memory_allocate_flags_info);

        let instances_buffer_memory = device.allocate_memory(&instances_memory_info, None)?;
        device.bind_buffer_memory(instances_buffer, instances_buffer_memory, 0)?;

        // インスタンスデータをコピー
        let instances_ptr = device.map_memory(
            instances_buffer_memory,
            0,
            instances_size,
            vk::MemoryMapFlags::empty(),
        )? as *mut vk::AccelerationStructureInstanceKHR;

        std::ptr::copy_nonoverlapping(instances.as_ptr(), instances_ptr, instances.len());

        device.unmap_memory(instances_buffer_memory);

        let instances_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(instances_buffer),
        );

        // 3. ジオメトリデータ（インスタンス）の記述
        let instances_data = vk::AccelerationStructureGeometryInstancesDataKHR::builder()
            .array_of_pointers(false)
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: instances_buffer_address,
            });

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: *instances_data,
            })
            .flags(vk::GeometryFlagsKHR::OPAQUE);

        // 4. ビルド情報
        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(
                vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE
                    | vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE,
            )
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(std::slice::from_ref(&geometry));

        let primitive_count = instances.len() as u32;

        // 5. ビルドサイズの取得
        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        device.get_acceleration_structure_build_sizes_khr(
            vk::AccelerationStructureBuildTypeKHR::DEVICE,
            &build_info,
            &[primitive_count],
            &mut size_info,
        );

        // 6. Acceleration Structure用バッファ作成
        let as_buffer_info = vk::BufferCreateInfo::builder()
            .size(size_info.acceleration_structure_size)
            .usage(
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );

        let as_buffer = device.create_buffer(&as_buffer_info, None)?;
        let as_memory_requirements = device.get_buffer_memory_requirements(as_buffer);

        let as_memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            as_memory_requirements,
        )?;

        let mut memory_allocate_flags_info =
            vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

        let as_memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(as_memory_requirements.size)
            .memory_type_index(as_memory_type_index)
            .push_next(&mut memory_allocate_flags_info);

        let as_buffer_memory = device.allocate_memory(&as_memory_info, None)?;
        device.bind_buffer_memory(as_buffer, as_buffer_memory, 0)?;

        // 7. Acceleration Structure作成
        let as_create_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(as_buffer)
            .size(size_info.acceleration_structure_size)
            .type_(vk::AccelerationStructureTypeKHR::TOP_LEVEL);

        let acceleration_structure =
            device.create_acceleration_structure_khr(&as_create_info, None)?;

        // 8. スクラッチバッファ作成
        let scratch_buffer_info = vk::BufferCreateInfo::builder()
            .size(size_info.build_scratch_size)
            .usage(
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );

        let scratch_buffer = device.create_buffer(&scratch_buffer_info, None)?;
        let scratch_memory_requirements = device.get_buffer_memory_requirements(scratch_buffer);

        let scratch_memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            scratch_memory_requirements,
        )?;

        let mut scratch_memory_allocate_flags_info =
            vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

        let scratch_memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(scratch_memory_requirements.size)
            .memory_type_index(scratch_memory_type_index)
            .push_next(&mut scratch_memory_allocate_flags_info);

        let scratch_buffer_memory = device.allocate_memory(&scratch_memory_info, None)?;
        device.bind_buffer_memory(scratch_buffer, scratch_buffer_memory, 0)?;

        let scratch_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(scratch_buffer),
        );

        // 9. ビルドコマンド記録・実行
        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(
                vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE
                    | vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE,
            )
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .dst_acceleration_structure(acceleration_structure)
            .geometries(std::slice::from_ref(&geometry))
            .scratch_data(vk::DeviceOrHostAddressKHR {
                device_address: scratch_buffer_address,
            });

        let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(primitive_count)
            .primitive_offset(0)
            .first_vertex(0)
            .transform_offset(0)
            .build();

        let build_range_infos = [build_range_info];

        // コマンドバッファでビルド実行
        let command_buffer = crate::vulkanr::command::begin_single_time_commands(
            rrdevice,
            rrcommand_pool.command_pool,
        )?;

        device.cmd_build_acceleration_structures_khr(
            command_buffer,
            std::slice::from_ref(&build_info),
            &[&build_range_infos[0]],
        );

        crate::vulkanr::command::end_single_time_commands(
            rrdevice,
            rrdevice.graphics_queue,
            rrcommand_pool.command_pool,
            command_buffer,
        )?;

        // スクラッチバッファとインスタンスバッファは不要になったので破棄
        device.destroy_buffer(scratch_buffer, None);
        device.free_memory(scratch_buffer_memory, None);
        device.destroy_buffer(instances_buffer, None);
        device.free_memory(instances_buffer_memory, None);

        // デバイスアドレス取得
        let device_address = device.get_acceleration_structure_device_address_khr(
            &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                .acceleration_structure(acceleration_structure),
        );

        Ok(RRTLAS {
            acceleration_structure: Some(acceleration_structure),
            buffer: Some(as_buffer),
            buffer_memory: Some(as_buffer_memory),
            device_address,
        })
    }

    /// BLASを更新（既存のジオメトリ変更時）
    pub unsafe fn update_blas(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        blas: &RRBLAS,
        vertex_buffer: &vk::Buffer,
        vertex_count: u32,
        vertex_stride: u32,
        index_buffer: &vk::Buffer,
        index_count: u32,
    ) -> Result<()> {
        let device = &rrdevice.device;

        let vertex_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(*vertex_buffer),
        );
        let index_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(*index_buffer),
        );

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
            });

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                triangles: *triangles,
            })
            .flags(vk::GeometryFlagsKHR::OPAQUE);

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .flags(
                vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE
                    | vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE,
            )
            .mode(vk::BuildAccelerationStructureModeKHR::UPDATE)
            .geometries(std::slice::from_ref(&geometry));

        let primitive_count = index_count / 3;

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        device.get_acceleration_structure_build_sizes_khr(
            vk::AccelerationStructureBuildTypeKHR::DEVICE,
            &build_info,
            &[primitive_count],
            &mut size_info,
        );

        let scratch_buffer_info = vk::BufferCreateInfo::builder()
            .size(size_info.update_scratch_size)
            .usage(
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );

        let scratch_buffer = device.create_buffer(&scratch_buffer_info, None)?;
        let scratch_memory_requirements = device.get_buffer_memory_requirements(scratch_buffer);

        let scratch_memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            scratch_memory_requirements,
        )?;

        let mut scratch_memory_allocate_flags_info =
            vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

        let scratch_memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(scratch_memory_requirements.size)
            .memory_type_index(scratch_memory_type_index)
            .push_next(&mut scratch_memory_allocate_flags_info);

        let scratch_buffer_memory = device.allocate_memory(&scratch_memory_info, None)?;
        device.bind_buffer_memory(scratch_buffer, scratch_buffer_memory, 0)?;

        let scratch_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(scratch_buffer),
        );

        let build_info =
            vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                .type_(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                .flags(
                    vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE
                        | vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE,
                )
                .mode(vk::BuildAccelerationStructureModeKHR::UPDATE)
                .src_acceleration_structure(blas.acceleration_structure.ok_or_else(|| {
                    anyhow::anyhow!("BLAS acceleration structure not initialized")
                })?)
                .dst_acceleration_structure(blas.acceleration_structure.ok_or_else(|| {
                    anyhow::anyhow!("BLAS acceleration structure not initialized")
                })?)
                .geometries(std::slice::from_ref(&geometry))
                .scratch_data(vk::DeviceOrHostAddressKHR {
                    device_address: scratch_buffer_address,
                });

        let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(primitive_count)
            .primitive_offset(0)
            .first_vertex(0)
            .transform_offset(0)
            .build();

        let build_range_infos = [build_range_info];

        let command_buffer = crate::vulkanr::command::begin_single_time_commands(
            rrdevice,
            rrcommand_pool.command_pool,
        )?;

        device.cmd_build_acceleration_structures_khr(
            command_buffer,
            std::slice::from_ref(&build_info),
            &[&build_range_infos[0]],
        );

        crate::vulkanr::command::end_single_time_commands(
            rrdevice,
            rrdevice.graphics_queue,
            rrcommand_pool.command_pool,
            command_buffer,
        )?;

        device.destroy_buffer(scratch_buffer, None);
        device.free_memory(scratch_buffer_memory, None);

        Ok(())
    }

    /// TLASを更新（BLASの位置やインスタンス変更時）
    pub unsafe fn update_tlas(
        instance: &Instance,
        rrdevice: &RRDevice,
        rrcommand_pool: &RRCommandPool,
        tlas: &RRTLAS,
        blas_list: &[RRBLAS],
    ) -> Result<()> {
        let device = &rrdevice.device;

        if blas_list.is_empty() {
            return Ok(());
        }

        let instances: Vec<vk::AccelerationStructureInstanceKHR> = blas_list
            .iter()
            .enumerate()
            .map(|(i, blas)| vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR {
                    matrix: [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                    ],
                },
                instance_custom_index_and_mask: vk::Bitfield24_8::new(i as u32, 0xFF),
                instance_shader_binding_table_record_offset_and_flags: vk::Bitfield24_8::new(0, 0),
                acceleration_structure_reference: blas.device_address,
            })
            .collect();

        let instances_size = (std::mem::size_of::<vk::AccelerationStructureInstanceKHR>()
            * instances.len()) as vk::DeviceSize;

        let instances_buffer_info = vk::BufferCreateInfo::builder().size(instances_size).usage(
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        );

        let instances_buffer = device.create_buffer(&instances_buffer_info, None)?;
        let instances_memory_requirements = device.get_buffer_memory_requirements(instances_buffer);

        let instances_memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            instances_memory_requirements,
        )?;

        let mut instances_memory_allocate_flags_info =
            vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

        let instances_memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(instances_memory_requirements.size)
            .memory_type_index(instances_memory_type_index)
            .push_next(&mut instances_memory_allocate_flags_info);

        let instances_buffer_memory = device.allocate_memory(&instances_memory_info, None)?;
        device.bind_buffer_memory(instances_buffer, instances_buffer_memory, 0)?;

        let instances_ptr = device.map_memory(
            instances_buffer_memory,
            0,
            instances_size,
            vk::MemoryMapFlags::empty(),
        )? as *mut vk::AccelerationStructureInstanceKHR;

        std::ptr::copy_nonoverlapping(instances.as_ptr(), instances_ptr, instances.len());

        device.unmap_memory(instances_buffer_memory);

        let instances_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(instances_buffer),
        );

        let instances_data = vk::AccelerationStructureGeometryInstancesDataKHR::builder()
            .array_of_pointers(false)
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: instances_buffer_address,
            });

        let geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: *instances_data,
            })
            .flags(vk::GeometryFlagsKHR::OPAQUE);

        let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .type_(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(
                vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE
                    | vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE,
            )
            .mode(vk::BuildAccelerationStructureModeKHR::UPDATE)
            .geometries(std::slice::from_ref(&geometry));

        let primitive_count = instances.len() as u32;

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        device.get_acceleration_structure_build_sizes_khr(
            vk::AccelerationStructureBuildTypeKHR::DEVICE,
            &build_info,
            &[primitive_count],
            &mut size_info,
        );

        let scratch_buffer_info = vk::BufferCreateInfo::builder()
            .size(size_info.update_scratch_size)
            .usage(
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            );

        let scratch_buffer = device.create_buffer(&scratch_buffer_info, None)?;
        let scratch_memory_requirements = device.get_buffer_memory_requirements(scratch_buffer);

        let scratch_memory_type_index = get_memory_type_index(
            instance,
            rrdevice.physical_device,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            scratch_memory_requirements,
        )?;

        let mut scratch_memory_allocate_flags_info =
            vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);

        let scratch_memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(scratch_memory_requirements.size)
            .memory_type_index(scratch_memory_type_index)
            .push_next(&mut scratch_memory_allocate_flags_info);

        let scratch_buffer_memory = device.allocate_memory(&scratch_memory_info, None)?;
        device.bind_buffer_memory(scratch_buffer, scratch_buffer_memory, 0)?;

        let scratch_buffer_address = device.get_buffer_device_address(
            &vk::BufferDeviceAddressInfo::builder().buffer(scratch_buffer),
        );

        let build_info =
            vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                .type_(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
                .flags(
                    vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE
                        | vk::BuildAccelerationStructureFlagsKHR::ALLOW_UPDATE,
                )
                .mode(vk::BuildAccelerationStructureModeKHR::UPDATE)
                .src_acceleration_structure(tlas.acceleration_structure.ok_or_else(|| {
                    anyhow::anyhow!("TLAS acceleration structure not initialized")
                })?)
                .dst_acceleration_structure(tlas.acceleration_structure.ok_or_else(|| {
                    anyhow::anyhow!("TLAS acceleration structure not initialized")
                })?)
                .geometries(std::slice::from_ref(&geometry))
                .scratch_data(vk::DeviceOrHostAddressKHR {
                    device_address: scratch_buffer_address,
                });

        let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::builder()
            .primitive_count(primitive_count)
            .primitive_offset(0)
            .first_vertex(0)
            .transform_offset(0)
            .build();

        let build_range_infos = [build_range_info];

        let command_buffer = crate::vulkanr::command::begin_single_time_commands(
            rrdevice,
            rrcommand_pool.command_pool,
        )?;

        device.cmd_build_acceleration_structures_khr(
            command_buffer,
            std::slice::from_ref(&build_info),
            &[&build_range_infos[0]],
        );

        crate::vulkanr::command::end_single_time_commands(
            rrdevice,
            rrdevice.graphics_queue,
            rrcommand_pool.command_pool,
            command_buffer,
        )?;

        device.destroy_buffer(scratch_buffer, None);
        device.free_memory(scratch_buffer_memory, None);
        device.destroy_buffer(instances_buffer, None);
        device.free_memory(instances_buffer_memory, None);

        Ok(())
    }

    /// リソースの解放
    pub unsafe fn destroy(&mut self, device: &vulkanalia::Device) {
        // TLAS破棄
        if let Some(tlas_as) = self.tlas.acceleration_structure {
            device.destroy_acceleration_structure_khr(tlas_as, None);
        }
        if let Some(buffer) = self.tlas.buffer {
            device.destroy_buffer(buffer, None);
        }
        if let Some(memory) = self.tlas.buffer_memory {
            device.free_memory(memory, None);
        }

        // BLAS破棄
        for blas in &mut self.blas_list {
            if let Some(blas_as) = blas.acceleration_structure {
                device.destroy_acceleration_structure_khr(blas_as, None);
            }
            if let Some(buffer) = blas.buffer {
                device.destroy_buffer(buffer, None);
            }
            if let Some(memory) = blas.buffer_memory {
                device.free_memory(memory, None);
            }
        }
        self.blas_list.clear();
    }

    pub unsafe fn update_all(
        &self,
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
                    &self.blas_list[i],
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
            &self.tlas,
            &self.blas_list,
        )?;

        Ok(())
    }
}
