use crate::ecs::component::{
    AppliedFlameStyle, AppliedWaterPreset, AppliedWindPreset, FlameEffect, WaterTorusEffect,
    WindTornadoEffect,
};
use crate::ecs::systems::{FLAME_EFFECT_HOOK, WATER_EFFECT_HOOK, WIND_EFFECT_HOOK};
use crate::hooks::effect::EffectHooks;
use crate::hooks::scene::{SceneComponentHook, SceneComponentHooks};

pub fn subscribe_effects(hooks: &mut EffectHooks) {
    hooks.register(WATER_EFFECT_HOOK);
    hooks.register(FLAME_EFFECT_HOOK);
    hooks.register(WIND_EFFECT_HOOK);
}

pub fn subscribe_scene_components(hooks: &mut SceneComponentHooks) {
    hooks.register(SceneComponentHook::owner::<WaterTorusEffect>());
    hooks.register(SceneComponentHook::attachment::<AppliedWaterPreset>());
    hooks.register(SceneComponentHook::owner::<FlameEffect>());
    hooks.register(SceneComponentHook::attachment::<AppliedFlameStyle>());
    hooks.register(SceneComponentHook::owner::<WindTornadoEffect>());
    hooks.register(SceneComponentHook::attachment::<AppliedWindPreset>());
}
