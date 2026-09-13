use crate::ecs::storage::Component;
use crate::ecs::world::World;
use thyllore_vulkan_core::raytracing::GpuPrimitive;

/// A component that contributes one ray-tracing instance (BLAS geometry, transform, hit record).
pub trait GpuPrimitiveSource {
    fn gpu_primitive(&self) -> GpuPrimitive<'static>;
}

pub type GpuPrimitiveCollectFn = fn(&World) -> Vec<GpuPrimitive<'static>>;

#[derive(Clone, Copy)]
pub struct GpuPrimitiveHook {
    pub type_name: fn() -> &'static str,
    pub collect: GpuPrimitiveCollectFn,
}

inventory::collect!(GpuPrimitiveHook);

/// Registers a component type whose entities become TLAS instances.
#[macro_export]
macro_rules! gpu_primitive_source {
    ($component:ty) => {
        inventory::submit! { $crate::hooks::gpu_primitive::GpuPrimitiveHook::of::<$component>() }
    };
}

impl GpuPrimitiveHook {
    pub const fn of<C: Component + GpuPrimitiveSource>() -> Self {
        Self {
            type_name: std::any::type_name::<C>,
            collect: collect_from::<C>,
        }
    }
}

/// Entity order is the TLAS instance order, so it must stay stable across
/// frames: the same collector runs for the build and for the transform refresh.
fn collect_from<C: Component + GpuPrimitiveSource>(world: &World) -> Vec<GpuPrimitive<'static>> {
    let mut sources: Vec<_> = world.iter_components::<C>().collect();
    sources.sort_by_key(|(entity, _)| *entity);

    sources
        .into_iter()
        .map(|(_, component)| component.gpu_primitive())
        .collect()
}

/// Every registered hook sorted by type name, so the instance order does not
/// depend on link order.
fn registered_hooks() -> Vec<GpuPrimitiveHook> {
    let mut hooks: Vec<GpuPrimitiveHook> = inventory::iter::<GpuPrimitiveHook>
        .into_iter()
        .copied()
        .collect();
    hooks.sort_by_key(|hook| (hook.type_name)());
    hooks
}

pub fn collect_all(world: &World) -> Vec<GpuPrimitive<'static>> {
    registered_hooks()
        .into_iter()
        .flat_map(|hook| (hook.collect)(world))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_hooks_have_unique_type_names() {
        let names: Vec<&str> = registered_hooks()
            .iter()
            .map(|hook| (hook.type_name)())
            .collect();
        let mut unique = names.clone();
        unique.dedup();

        assert_eq!(names, unique, "a gpu primitive source is registered twice");
        assert!(!names.is_empty(), "no gpu primitive source is registered");
    }
}
