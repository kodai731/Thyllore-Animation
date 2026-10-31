use crate::ecs::resource::{
    AutoExposure, BoneGizmoData, DepthOfField, Exposure, OnionSkinningConfig,
    PhysicalCameraParameters, SceneBrightness, TransformGizmoState, WeightHeatmapState,
};
use crate::ecs::systems::phases::event_dispatch::overlay::OverlayEvent;
use crate::ecs::World;

#[derive(Clone, PartialEq)]
pub(crate) struct SceneSettingsView {
    pub brightness: Option<SceneBrightness>,
    pub bones_visible: Option<bool>,
    pub heatmap_enabled: Option<bool>,
    pub gizmo: Option<TransformGizmoState>,
    pub dof: Option<DepthOfField>,
    pub camera: Option<PhysicalCameraParameters>,
    pub auto_exposure: Option<AutoExposure>,
    pub exposure_readout: Option<(f32, f32)>,
    pub onion: Option<OnionSkinningConfig>,
}

pub(crate) fn read_scene_settings(world: &World) -> SceneSettingsView {
    let brightness = world.get_resource::<SceneBrightness>().map(|b| *b);
    let bones_visible = world.get_resource::<BoneGizmoData>().map(|b| b.visible);
    let heatmap_enabled = world
        .get_resource::<WeightHeatmapState>()
        .map(|h| h.enabled);
    let gizmo = world
        .get_resource::<TransformGizmoState>()
        .map(|g| g.clone());
    let dof = world.get_resource::<DepthOfField>().map(|d| d.clone());
    let camera = world
        .get_resource::<PhysicalCameraParameters>()
        .map(|c| c.clone());
    let auto_exposure = world.get_resource::<AutoExposure>().map(|a| a.clone());
    let exposure_readout = world
        .get_resource::<Exposure>()
        .map(|e| (e.exposure_value, e.ev100));
    let onion = world
        .get_resource::<OnionSkinningConfig>()
        .map(|o| o.clone());

    SceneSettingsView {
        brightness,
        bones_visible,
        heatmap_enabled,
        gizmo,
        dof,
        camera,
        auto_exposure,
        exposure_readout,
        onion,
    }
}

pub(crate) fn send_scene_settings_changes(
    world: &World,
    shown: &SceneSettingsView,
    view: &SceneSettingsView,
) {
    if shown.brightness != view.brightness {
        if let Some(b) = view.brightness {
            world.send_command(OverlayEvent::UpdateSceneBrightness(b));
        }
    }
    if shown.bones_visible != view.bones_visible {
        if let Some(v) = view.bones_visible {
            world.send_command(OverlayEvent::SetBoneGizmoVisible(v));
        }
    }
    if shown.heatmap_enabled != view.heatmap_enabled {
        if let Some(e) = view.heatmap_enabled {
            world.send_command(OverlayEvent::SetWeightHeatmapEnabled(e));
        }
    }
    if shown.gizmo != view.gizmo {
        if let Some(g) = &view.gizmo {
            world.send_command(OverlayEvent::UpdateTransformGizmoState(Box::new(g.clone())));
        }
    }
    if shown.dof != view.dof {
        if let Some(d) = &view.dof {
            world.send_command(OverlayEvent::UpdateDepthOfField(d.clone()));
        }
    }
    if shown.camera != view.camera {
        if let Some(c) = &view.camera {
            world.send_command(OverlayEvent::UpdatePhysicalCamera(c.clone()));
        }
    }
    if shown.auto_exposure != view.auto_exposure {
        if let Some(ae) = &view.auto_exposure {
            world.send_command(OverlayEvent::UpdateAutoExposure(ae.clone()));
        }
    }
    if shown.onion != view.onion {
        if let Some(o) = &view.onion {
            world.send_command(OverlayEvent::UpdateOnionSkinning(o.clone()));
        }
    }
}
