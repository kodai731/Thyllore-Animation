pub use thyllore_effect_core::lightning::{LightningEffect, LightningSource};

crate::scene_owner!(LightningEffect {
    icon: crate::ecs::component::EntityIcon::Effect(crate::ecs::component::Icon::Zap),
    placement: |e| (e.position, e.rotation),
});
