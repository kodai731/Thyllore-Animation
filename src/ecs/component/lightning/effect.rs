pub use thyllore_effect_core::{LightningEffect, LightningSource};

crate::scene_owner!(LightningEffect {
    icon: crate::ecs::component::EntityIcon::Effect('Z'),
    placement: |e| (e.position, e.rotation),
});
