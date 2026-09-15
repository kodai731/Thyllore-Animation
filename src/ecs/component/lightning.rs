pub use thyllore_effect_core::{LightningEffect, LightningSource};

crate::scene_owner!(LightningEffect {
    icon: Lightning,
    placement: |e| (e.position, e.rotation),
});
