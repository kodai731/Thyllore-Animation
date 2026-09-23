use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::gizmo::BoneGizmoData;
use crate::ecs::resource::{
    AutoExposure, DepthOfField, GridMeshData, HierarchyState, MessageLog, OnionSkinningConfig,
    PhysicalCameraParameters, TransformGizmoState, WeightHeatmapState,
};
use crate::ecs::world::{Animator, World};
use crate::hooks::effect_spawn::EffectSpawnHooks;
use crate::hooks::effect_ui_event::EffectUiEventHooks;

pub fn dispatch_overlay_events(events: &[UIEvent], world: &mut World, assets: &mut AssetStorage) {
    for event in events {
        match event {
            UIEvent::SetBoneGizmoVisible(visible) => {
                if let Some(mut gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    gizmo.visible = *visible;
                }
            }
            UIEvent::SetWeightHeatmapEnabled(enabled) => {
                log!("UIEvent::SetWeightHeatmapEnabled({})", enabled);
                if let Some(mut heatmap) = world.get_resource_mut::<WeightHeatmapState>() {
                    heatmap.enabled = *enabled;
                } else {
                    log_warn!("WeightHeatmapState resource missing when toggling heatmap");
                }
            }
            UIEvent::SetTransformGizmoMode(mode) => {
                if let Some(mut state) = world.get_resource_mut::<TransformGizmoState>() {
                    state.mode = *mode;
                }
            }
            UIEvent::SetTransformGizmoSpace(space) => {
                if let Some(mut state) = world.get_resource_mut::<TransformGizmoState>() {
                    state.coordinate_space = *space;
                }
            }
            UIEvent::UpdateTransformGizmoState(new_state) => {
                if let Some(mut state) = world.get_resource_mut::<TransformGizmoState>() {
                    *state = *new_state.clone();
                }
            }
            UIEvent::UpdateDepthOfField(new_dof) => {
                if let Some(mut dof) = world.get_resource_mut::<DepthOfField>() {
                    *dof = new_dof.clone();
                }
            }
            UIEvent::UpdatePhysicalCamera(new_params) => {
                if let Some(mut params) = world.get_resource_mut::<PhysicalCameraParameters>() {
                    *params = new_params.clone();
                }
            }
            UIEvent::UpdateAutoExposure(new_ae) => {
                if let Some(mut ae) = world.get_resource_mut::<AutoExposure>() {
                    *ae = new_ae.clone();
                }
            }
            UIEvent::UpdateOnionSkinning(new_config) => {
                if new_config.enabled {
                    auto_select_animator_entity(world);
                }
                if let Some(mut config) = world.get_resource_mut::<OnionSkinningConfig>() {
                    *config = new_config.clone();
                }
            }
            UIEvent::SelectEffectInstance { key, index } => {
                select_effect_instance(world, key, *index);
            }
            UIEvent::SetGridShowYAxis(show) => {
                if let Some(mut grid) = world.get_resource_mut::<GridMeshData>() {
                    grid.show_y_axis_grid = *show;
                }
            }
            UIEvent::ClearMessageLog => {
                if let Some(mut log) = world.get_resource_mut::<MessageLog>() {
                    crate::ecs::systems::message_log_clear_buffer(&mut log);
                }
            }
            UIEvent::Effect { key, command } => {
                let hooks = EffectUiEventHooks::collect();
                if let Some(hook) = hooks.iter().find(|hook| hook.key == *key) {
                    (hook.apply)(world, assets, command.as_ref());
                }
            }
            _ => {}
        }
    }
}

fn auto_select_animator_entity(world: &mut World) {
    let already_selected = world
        .get_resource::<HierarchyState>()
        .and_then(|h| h.selected_entity)
        .is_some();
    if already_selected {
        return;
    }

    let first_animator = world.iter_components::<Animator>().next().map(|(e, _)| e);
    if let Some(entity) = first_animator {
        let mut hierarchy = world.resource_mut::<HierarchyState>();
        crate::ecs::systems::hierarchy_select(&mut hierarchy, entity);
    }
}

fn select_effect_instance(world: &mut World, key: &str, index: usize) {
    let Some(entities) = world
        .get_resource::<EffectSpawnHooks>()
        .and_then(|hooks| hooks.get(key).map(|hook| (hook.entities)(world)))
    else {
        return;
    };
    let Some(&target) = entities.get(index.min(entities.len().saturating_sub(1))) else {
        return;
    };
    if let Some(mut hierarchy) = world.get_resource_mut::<HierarchyState>() {
        hierarchy.selected_entity = Some(target);
    }
}
