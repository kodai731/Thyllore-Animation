use crate::ecs::component::{AppliedWaterPreset, WaterTorusEffect};
use crate::ecs::systems::effect_edit::EffectPreset;

impl EffectPreset for WaterTorusEffect {
    type Applied = AppliedWaterPreset;

    fn apply_preset(&mut self, name: &str) -> bool {
        thyllore_effect_core::apply_water_preset(self, name)
    }

    fn applied(name: &str) -> Self::Applied {
        AppliedWaterPreset {
            name: name.to_string(),
        }
    }
}
