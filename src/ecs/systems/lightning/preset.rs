use crate::ecs::component::{AppliedLightningPreset, LightningEffect};
use crate::ecs::systems::effect_edit::EffectPreset;

impl EffectPreset for LightningEffect {
    type Applied = AppliedLightningPreset;

    fn apply_preset(&mut self, name: &str) -> bool {
        thyllore_effect_core::apply_lightning_preset(self, name)
    }

    fn applied(name: &str) -> Self::Applied {
        AppliedLightningPreset {
            name: name.to_string(),
        }
    }
}
