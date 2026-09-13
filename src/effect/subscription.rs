use crate::ecs::systems::{
    FLAME_EFFECT_HOOK, FLAME_SCENE_COMPONENTS, WATER_EFFECT_HOOK, WATER_SCENE_COMPONENTS,
    WIND_EFFECT_HOOK, WIND_SCENE_COMPONENTS,
};
use crate::hooks::effect::EffectHooks;
use crate::hooks::scene::SceneComponentHooks;

pub fn subscribe_effects(hooks: &mut EffectHooks) {
    hooks.register(WATER_EFFECT_HOOK);
    hooks.register(FLAME_EFFECT_HOOK);
    hooks.register(WIND_EFFECT_HOOK);
}

pub fn subscribe_scene_components(hooks: &mut SceneComponentHooks) {
    hooks.register_all(WATER_SCENE_COMPONENTS);
    hooks.register_all(FLAME_SCENE_COMPONENTS);
    hooks.register_all(WIND_SCENE_COMPONENTS);
}
