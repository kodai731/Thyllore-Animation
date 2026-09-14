use crate::core::device::*;
use crate::descriptor::{PassShaders, ShaderStage};
use crate::resource::buffer::create_buffer;
use crate::resource::gpu_resource::GpuResource;
use crate::vulkan::*;
use std::fs::File;
use std::io::Read;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::vk::KhrRayTracingPipelineExtension;

#[derive(Clone, Debug)]
pub struct RRRayTracingPipeline {
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub sbt_buffer: vk::Buffer,
    pub sbt_memory: vk::DeviceMemory,
    pub raygen_region: vk::StridedDeviceAddressRegionKHR,
    pub miss_region: vk::StridedDeviceAddressRegionKHR,
    pub hit_region: vk::StridedDeviceAddressRegionKHR,
    pub callable_region: vk::StridedDeviceAddressRegionKHR,
}

/// One SBT hit record: a triangle group (closest hit only) or a procedural group
/// (intersection + closest hit). Records follow the closest hit order of the pass manifest, and an
/// intersection stage pairs with the closest hit stage listed right after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitGroup {
    Triangles {
        closest_hit: usize,
    },
    Procedural {
        intersection: usize,
        closest_hit: usize,
    },
}

impl HitGroup {
    fn closest_hit(self) -> usize {
        match self {
            HitGroup::Triangles { closest_hit } => closest_hit,
            HitGroup::Procedural { closest_hit, .. } => closest_hit,
        }
    }
}

/// Stage indices of a pass in manifest order, so the SBT can be laid out without naming a shader.
pub struct TraceStages {
    pub raygen: usize,
    pub miss: usize,
    pub hit_groups: Vec<HitGroup>,
}

impl TraceStages {
    pub fn from_pass(pass: &PassShaders) -> Result<Self> {
        let mut raygen = None;
        let mut miss = None;
        let mut pending_intersection = None;
        let mut hit_groups = Vec::new();

        for (index, file) in pass.stages.iter().enumerate() {
            match file.stage {
                ShaderStage::RayGeneration => raygen = Some(index),
                ShaderStage::Miss => miss = Some(index),
                ShaderStage::Intersection => {
                    anyhow::ensure!(
                        pending_intersection.is_none(),
                        "pass `{}`: intersection stage `{}` is not followed by a closest hit stage",
                        pass.name(),
                        file.path
                    );
                    pending_intersection = Some(index);
                }
                ShaderStage::ClosestHit => hit_groups.push(match pending_intersection.take() {
                    Some(intersection) => HitGroup::Procedural {
                        intersection,
                        closest_hit: index,
                    },
                    None => HitGroup::Triangles { closest_hit: index },
                }),
                ShaderStage::AnyHit
                | ShaderStage::Callable
                | ShaderStage::Vertex
                | ShaderStage::TessellationControl
                | ShaderStage::TessellationEvaluation
                | ShaderStage::Fragment
                | ShaderStage::Geometry
                | ShaderStage::Task
                | ShaderStage::Mesh
                | ShaderStage::Compute => anyhow::bail!(
                    "pass `{}`: stage `{}` is not supported by the ray tracing pipeline",
                    pass.name(),
                    file.path
                ),
            }
        }

        anyhow::ensure!(
            pending_intersection.is_none(),
            "pass `{}`: trailing intersection stage has no closest hit stage",
            pass.name()
        );
        anyhow::ensure!(
            matches!(hit_groups.first(), Some(HitGroup::Triangles { .. })),
            "pass `{}`: hit record 0 must be the triangle closest hit stage",
            pass.name()
        );
        Ok(Self {
            raygen: raygen.ok_or_else(|| {
                anyhow::anyhow!("pass `{}` has no RayGeneration stage", pass.name())
            })?,
            miss: miss
                .ok_or_else(|| anyhow::anyhow!("pass `{}` has no Miss stage", pass.name()))?,
            hit_groups,
        })
    }
}

/// Levels of traceRayEXT nesting: the ray generation stage plus one secondary trace from a hit shader.
const RAY_RECURSION_DEPTH: u32 = 2;

impl RRRayTracingPipeline {
    pub unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        pass: &PassShaders,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        push_constant_ranges: &[vk::PushConstantRange],
    ) -> Result<Self> {
        let device = &rrdevice.device;
        let trace_stages = TraceStages::from_pass(pass)?;

        let mut modules = Vec::with_capacity(pass.stages.len());
        for file in pass.stages {
            modules.push(load_shader_module(rrdevice, file.path)?);
        }
        let stages: Vec<vk::PipelineShaderStageCreateInfo> = pass
            .stages
            .iter()
            .zip(modules.iter())
            .map(|(file, module)| {
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(shader_stage_flags(file.stage))
                    .module(*module)
                    .name(b"main\0")
                    .build()
            })
            .collect();
        let groups = shader_groups(&trace_stages);

        let mut layout_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(descriptor_set_layouts);
        if !push_constant_ranges.is_empty() {
            layout_info = layout_info.push_constant_ranges(push_constant_ranges);
        }
        let pipeline_layout = device.create_pipeline_layout(&layout_info.build(), None)?;

        let rt_props = ray_tracing_properties(instance, rrdevice);
        anyhow::ensure!(
            rt_props.max_ray_recursion_depth >= RAY_RECURSION_DEPTH,
            "device supports ray recursion depth {} but the effect trace pipeline needs {}",
            rt_props.max_ray_recursion_depth,
            RAY_RECURSION_DEPTH
        );
        let rt_pipeline_info = vk::RayTracingPipelineCreateInfoKHR::builder()
            .stages(&stages)
            .groups(&groups)
            .max_pipeline_ray_recursion_depth(RAY_RECURSION_DEPTH)
            .layout(pipeline_layout)
            .build();
        let pipelines = device.create_ray_tracing_pipelines_khr(
            vk::DeferredOperationKHR::null(),
            vk::PipelineCache::null(),
            &[rt_pipeline_info],
            None,
        )?;
        let pipeline = pipelines.0[0];
        for module in modules {
            device.destroy_shader_module(module, None);
        }

        let sbt = ShaderBindingTable::new(
            instance,
            rrdevice,
            pipeline,
            &rt_props,
            trace_stages.hit_groups.len(),
        )?;
        Ok(Self {
            pipeline_layout,
            pipeline,
            sbt_buffer: sbt.buffer,
            sbt_memory: sbt.memory,
            raygen_region: sbt.raygen_region,
            miss_region: sbt.miss_region,
            hit_region: sbt.hit_region,
            callable_region: sbt.callable_region,
        })
    }

    pub unsafe fn destroy(&self, device: &vulkanalia::Device) {
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
        device.destroy_buffer(self.sbt_buffer, None);
        device.free_memory(self.sbt_memory, None);
    }
}

fn shader_stage_flags(stage: ShaderStage) -> vk::ShaderStageFlags {
    match stage {
        ShaderStage::RayGeneration => vk::ShaderStageFlags::RAYGEN_KHR,
        ShaderStage::Miss => vk::ShaderStageFlags::MISS_KHR,
        ShaderStage::Intersection => vk::ShaderStageFlags::INTERSECTION_KHR,
        ShaderStage::ClosestHit => vk::ShaderStageFlags::CLOSEST_HIT_KHR,
        ShaderStage::AnyHit => vk::ShaderStageFlags::ANY_HIT_KHR,
        ShaderStage::Callable => vk::ShaderStageFlags::CALLABLE_KHR,
        ShaderStage::Vertex => vk::ShaderStageFlags::VERTEX,
        ShaderStage::TessellationControl => vk::ShaderStageFlags::TESSELLATION_CONTROL,
        ShaderStage::TessellationEvaluation => vk::ShaderStageFlags::TESSELLATION_EVALUATION,
        ShaderStage::Fragment => vk::ShaderStageFlags::FRAGMENT,
        ShaderStage::Geometry => vk::ShaderStageFlags::GEOMETRY,
        ShaderStage::Task => vk::ShaderStageFlags::TASK_EXT,
        ShaderStage::Mesh => vk::ShaderStageFlags::MESH_EXT,
        ShaderStage::Compute => vk::ShaderStageFlags::COMPUTE,
    }
}

/// Group order is the SBT order: raygen, miss, then one hit group per hit record.
fn shader_groups(stages: &TraceStages) -> Vec<vk::RayTracingShaderGroupCreateInfoKHR> {
    let general = |index: usize| {
        vk::RayTracingShaderGroupCreateInfoKHR::builder()
            .type_(vk::RayTracingShaderGroupTypeKHR::GENERAL)
            .general_shader(index as u32)
            .closest_hit_shader(vk::SHADER_UNUSED_KHR)
            .any_hit_shader(vk::SHADER_UNUSED_KHR)
            .intersection_shader(vk::SHADER_UNUSED_KHR)
            .build()
    };
    let hit = |group: HitGroup| {
        let (group_type, intersection) = match group {
            HitGroup::Triangles { .. } => (
                vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP,
                vk::SHADER_UNUSED_KHR,
            ),
            HitGroup::Procedural { intersection, .. } => (
                vk::RayTracingShaderGroupTypeKHR::PROCEDURAL_HIT_GROUP,
                intersection as u32,
            ),
        };
        vk::RayTracingShaderGroupCreateInfoKHR::builder()
            .type_(group_type)
            .general_shader(vk::SHADER_UNUSED_KHR)
            .closest_hit_shader(group.closest_hit() as u32)
            .any_hit_shader(vk::SHADER_UNUSED_KHR)
            .intersection_shader(intersection)
            .build()
    };

    let mut groups = vec![general(stages.raygen), general(stages.miss)];
    groups.extend(stages.hit_groups.iter().copied().map(hit));
    groups
}

unsafe fn ray_tracing_properties(
    instance: &Instance,
    rrdevice: &RRDevice,
) -> vk::PhysicalDeviceRayTracingPipelinePropertiesKHR {
    let mut rt_props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
    let mut props2 = vk::PhysicalDeviceProperties2::builder().push_next(&mut rt_props);
    instance.get_physical_device_properties2(rrdevice.physical_device, &mut props2);
    rt_props
}

struct ShaderBindingTable {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    raygen_region: vk::StridedDeviceAddressRegionKHR,
    miss_region: vk::StridedDeviceAddressRegionKHR,
    hit_region: vk::StridedDeviceAddressRegionKHR,
    callable_region: vk::StridedDeviceAddressRegionKHR,
}

impl ShaderBindingTable {
    /// Layout: [raygen][miss][hit record 0 .. hit record n-1], each region base aligned.
    unsafe fn new(
        instance: &Instance,
        rrdevice: &RRDevice,
        pipeline: vk::Pipeline,
        rt_props: &vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
        hit_group_count: usize,
    ) -> Result<Self> {
        let device = &rrdevice.device;
        let handle_size = rt_props.shader_group_handle_size as u64;
        let handle_alignment = rt_props.shader_group_handle_alignment as u64;
        let base_alignment = rt_props.shader_group_base_alignment as u64;
        anyhow::ensure!(
            handle_size > 0 && handle_alignment > 0 && base_alignment > 0,
            "ray tracing pipeline properties not available"
        );
        let handle_stride = align_up(handle_size, handle_alignment);
        let region_size = align_up(handle_stride, base_alignment);
        let hit_region_size = align_up(hit_group_count as u64 * handle_stride, base_alignment);
        let sbt_size = 2 * region_size + hit_region_size;

        let group_count = 2 + hit_group_count;
        let mut handles = vec![0u8; group_count * handle_size as usize];
        device.get_ray_tracing_shader_group_handles_khr(
            pipeline,
            0,
            group_count as u32,
            &mut handles,
        )?;
        let handle = |group: usize| {
            &handles[group * handle_size as usize..(group + 1) * handle_size as usize]
        };

        let (buffer, memory) = create_buffer(
            instance,
            rrdevice,
            sbt_size,
            vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let mapped_ptr = device.map_memory(memory, 0, sbt_size, vk::MemoryMapFlags::empty())?;
        let slice = std::slice::from_raw_parts_mut(mapped_ptr as *mut u8, sbt_size as usize);
        let mut write = |offset: u64, group: usize| {
            slice[offset as usize..offset as usize + handle_size as usize]
                .copy_from_slice(handle(group));
        };
        write(0, 0);
        write(region_size, 1);
        for hit_record in 0..hit_group_count {
            write(
                2 * region_size + hit_record as u64 * handle_stride,
                2 + hit_record,
            );
        }
        device.unmap_memory(memory);

        let base_address = device
            .get_buffer_device_address(&vk::BufferDeviceAddressInfo::builder().buffer(buffer));
        let region = |offset: u64, size: u64, stride: u64| {
            vk::StridedDeviceAddressRegionKHR::builder()
                .device_address(base_address + offset)
                .size(size)
                .stride(stride)
                .build()
        };
        Ok(Self {
            buffer,
            memory,
            raygen_region: region(0, region_size, region_size),
            miss_region: region(region_size, region_size, handle_stride),
            hit_region: region(2 * region_size, hit_region_size, handle_stride),
            callable_region: region(2 * region_size + hit_region_size, 0, handle_stride),
        })
    }
}

unsafe fn load_shader_module(rrdevice: &RRDevice, path: &str) -> Result<vk::ShaderModule> {
    let mut file = File::open(path)?;
    let mut bytecode = Vec::new();
    file.read_to_end(&mut bytecode)?;
    create_shader_module(rrdevice, &bytecode)
}

unsafe fn create_shader_module(rrdevice: &RRDevice, bytecode: &[u8]) -> Result<vk::ShaderModule> {
    let bytecode =
        Bytecode::new(bytecode).map_err(|e| anyhow::anyhow!("Invalid shader bytecode: {:?}", e))?;
    let info = vk::ShaderModuleCreateInfo::builder()
        .code_size(bytecode.code_size())
        .code(bytecode.code());

    Ok(rrdevice.device.create_shader_module(&info, None)?)
}

pub fn align_up(v: u64, a: u64) -> u64 {
    (v + a - 1) & !(a - 1)
}

impl GpuResource for RRRayTracingPipeline {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        if self.pipeline == vk::Pipeline::null() {
            return;
        }
        self.destroy(&rrdevice.device);
        self.pipeline = vk::Pipeline::null();
        self.pipeline_layout = vk::PipelineLayout::null();
        self.sbt_buffer = vk::Buffer::null();
        self.sbt_memory = vk::DeviceMemory::null();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_up_basic() {
        assert_eq!(align_up(17, 16), 32);
    }

    #[test]
    fn test_align_up_already_aligned() {
        assert_eq!(align_up(32, 16), 32);
    }

    #[test]
    fn test_align_up_zero() {
        assert_eq!(align_up(0, 64), 0);
    }
}
