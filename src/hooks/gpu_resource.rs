use anyhow::Result;

use crate::ecs::world::{Resource, World};
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::resource::GpuResource;

pub type GpuResourceDestroyFn = unsafe fn(&World, &RRDevice);

/// A world resource that owns Vulkan objects and is destroyed with the device.
#[derive(Clone, Copy)]
pub struct GpuResourceHook {
    pub type_name: fn() -> &'static str,
    pub destroy: GpuResourceDestroyFn,
}

inventory::collect!(GpuResourceHook);

/// Registers a world resource whose `GpuResource` impl runs at app teardown.
#[macro_export]
macro_rules! gpu_resource {
    ($resource:ty) => {
        inventory::submit! { $crate::hooks::gpu_resource::GpuResourceHook::of::<$resource>() }
    };
}

impl GpuResourceHook {
    pub const fn of<R: Resource + GpuResource>() -> Self {
        Self {
            type_name: std::any::type_name::<R>,
            destroy: destroy_resource::<R>,
        }
    }
}

unsafe fn destroy_resource<R: Resource + GpuResource>(world: &World, rrdevice: &RRDevice) {
    if let Some(mut resource) = world.get_resource_mut::<R>() {
        log!("Destroying {}", resource.resource_name());
        resource.destroy_gpu(rrdevice);
    }
}

/// Every GPU resource hook submitted at link time, sorted by type name.
pub struct GpuResourceHooks {
    entries: Vec<GpuResourceHook>,
}

impl GpuResourceHooks {
    pub fn collect() -> Result<Self> {
        let mut entries: Vec<GpuResourceHook> = inventory::iter::<GpuResourceHook>
            .into_iter()
            .copied()
            .collect();
        entries.sort_by_key(|hook| (hook.type_name)());
        for pair in entries.windows(2) {
            anyhow::ensure!(
                (pair[0].type_name)() != (pair[1].type_name)(),
                "gpu resource {} registered twice",
                (pair[0].type_name)()
            );
        }
        Ok(Self { entries })
    }

    pub unsafe fn destroy_all(&self, world: &World, rrdevice: &RRDevice) {
        for hook in &self.entries {
            (hook.destroy)(world, rrdevice);
        }
    }

    pub fn type_names(&self) -> Vec<&'static str> {
        self.entries.iter().map(|hook| (hook.type_name)()).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const GPU_OWNER_TYPES: [&str; 11] = [
        "vk::Image",
        "vk::Buffer",
        "vk::RenderPass",
        "vk::Framebuffer",
        "vk::DeviceMemory",
        "RRImage",
        "RRBuffer",
        "RRUniformBuffer",
        "RRPipeline",
        "UniformBuffer",
        "VolumeImage",
    ];

    fn resource_sources() -> Vec<(String, String)> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ecs/resource");
        let mut sources = Vec::new();
        collect_resource_sources(&dir, &mut sources);
        sources.sort();
        sources
    }

    fn collect_resource_sources(dir: &Path, sources: &mut Vec<(String, String)>) {
        let entries = std::fs::read_dir(dir).expect("src/ecs/resource exists");
        for path in entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
        {
            if path.is_dir() {
                collect_resource_sources(&path, sources);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let source = std::fs::read_to_string(&path).expect("resource file is readable");
                sources.push((path.display().to_string(), source));
            }
        }
    }

    fn field_type_tokens(line: &str) -> Vec<&str> {
        let trimmed = line.trim_start();
        let Some(field_type) = trimmed.strip_prefix("pub ").and_then(|f| f.split_once(':')) else {
            return Vec::new();
        };
        field_type
            .1
            .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
            .filter(|token| !token.is_empty())
            .collect()
    }

    fn declares_gpu_owner_field(source: &str) -> bool {
        source.lines().any(|line| {
            field_type_tokens(line)
                .iter()
                .any(|token| GPU_OWNER_TYPES.contains(token))
        })
    }

    #[test]
    fn every_resource_file_owning_gpu_objects_registers_a_gpu_resource_hook() {
        let missing: Vec<String> = resource_sources()
            .into_iter()
            .filter(|(_, source)| declares_gpu_owner_field(source))
            .filter(|(_, source)| !source.contains("gpu_resource!("))
            .map(|(path, _)| path)
            .collect();

        assert!(
            missing.is_empty(),
            "resource files own GPU objects but never call gpu_resource!: {missing:?}"
        );
    }

    #[test]
    fn gpu_owner_detection_reads_field_types_not_borrowed_handles() {
        assert!(declares_gpu_owner_field(
            "pub struct A {\n    pub image: vk::Image,\n}"
        ));
        assert!(declares_gpu_owner_field(
            "pub struct A {\n    pub ubo: Option<UniformBuffer<X>>,\n}"
        ));
        assert!(declares_gpu_owner_field(
            "pub struct A {\n    pub texture: Option<RRImage>,\n}"
        ));
        assert!(!declares_gpu_owner_field(
            "pub struct A {\n    pub view: vk::ImageView,\n}"
        ));
        assert!(!declares_gpu_owner_field(
            "pub struct A {\n    pub id: PipelineId,\n}"
        ));
    }

    #[test]
    fn registered_hooks_have_unique_type_names() {
        let hooks = GpuResourceHooks::collect().expect("no duplicate registrations");
        let names = hooks.type_names();

        assert!(
            names.iter().any(|name| name.ends_with("BillboardData")),
            "{names:?}"
        );
    }
}
