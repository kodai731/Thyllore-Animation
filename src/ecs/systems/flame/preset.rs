use crate::ecs::component::{AppliedFlamePreset, FlameEffect};
use crate::ecs::systems::effect_edit::EffectPreset;

impl EffectPreset for FlameEffect {
    type Applied = AppliedFlamePreset;

    fn apply_preset(&mut self, name: &str) -> bool {
        thyllore_effect_core::apply_flame_preset(self, name)
    }

    fn applied(name: &str) -> Self::Applied {
        AppliedFlamePreset {
            name: name.to_string(),
        }
    }
}
