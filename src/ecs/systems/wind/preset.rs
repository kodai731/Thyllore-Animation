use crate::ecs::component::{AppliedWindPreset, WindTornadoEffect};
use crate::ecs::systems::effect_edit::EffectPreset;

impl EffectPreset for WindTornadoEffect {
    type Applied = AppliedWindPreset;

    fn apply_preset(&mut self, name: &str) -> bool {
        thyllore_effect_core::apply_wind_preset(self, name)
    }

    fn applied(name: &str) -> Self::Applied {
        AppliedWindPreset {
            name: name.to_string(),
        }
    }
}
