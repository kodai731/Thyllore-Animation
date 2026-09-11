use serde::{Deserialize, Serialize};

pub const SCENE_FORMAT_VERSION: u32 = 6;

pub use thyllore_anim_core::editable::{AnimationClipFile, ANIMATION_FORMAT_VERSION};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneFile {
    pub version: u32,
    #[serde(default)]
    pub metadata: SceneMetadata,
    pub model: ModelReference,
    #[serde(default)]
    pub animation_clips: Vec<AnimationClipRef>,
    #[serde(default)]
    pub current_clip: Option<String>,
    #[serde(default)]
    pub camera: CameraState,
    #[serde(default)]
    pub timeline: TimelineConfig,
    #[serde(default)]
    pub editor: EditorState,
    #[serde(default)]
    pub panel_layout: Option<PanelLayoutState>,
    #[serde(default)]
    pub debug_primitives: Vec<DebugPrimitiveSceneData>,
    #[serde(default)]
    pub entities: Vec<crate::scene::components::SceneEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugPrimitiveSceneData {
    pub kind: String,
    pub position: [f32; 3],
}

pub fn debug_primitive_kind_to_str(kind: crate::ecs::events::DebugPrimitiveKind) -> &'static str {
    match kind {
        crate::ecs::events::DebugPrimitiveKind::Cube => "cube",
        crate::ecs::events::DebugPrimitiveKind::Sphere => "sphere",
        crate::ecs::events::DebugPrimitiveKind::Floor => "floor",
    }
}

pub fn debug_primitive_kind_from_str(s: &str) -> Option<crate::ecs::events::DebugPrimitiveKind> {
    match s {
        "cube" => Some(crate::ecs::events::DebugPrimitiveKind::Cube),
        "sphere" => Some(crate::ecs::events::DebugPrimitiveKind::Sphere),
        "floor" => Some(crate::ecs::events::DebugPrimitiveKind::Floor),
        _ => None,
    }
}

impl SceneFile {
    pub fn new(name: &str, model_path: &str) -> Self {
        Self {
            version: SCENE_FORMAT_VERSION,
            metadata: SceneMetadata::new(name),
            model: ModelReference::new(model_path),
            animation_clips: Vec::new(),
            current_clip: None,
            camera: CameraState::default(),
            timeline: TimelineConfig::default(),
            editor: EditorState::default(),
            panel_layout: None,
            debug_primitives: Vec::new(),
            entities: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneMetadata {
    pub name: String,
    pub created_at: String,
    pub modified_at: String,
}

impl SceneMetadata {
    pub fn new(name: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            name: name.to_string(),
            created_at: now.clone(),
            modified_at: now,
        }
    }

    pub fn update_modified(&mut self) {
        self.modified_at = chrono::Utc::now().to_rfc3339();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReference {
    pub path: String,
    pub transform: crate::ecs::world::Transform,
}

impl ModelReference {
    /// Written in place of a file path when the mesh was generated in-app.
    pub const GENERATED_MESH: &'static str = "Generated Mesh";

    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            transform: crate::ecs::world::Transform::default(),
        }
    }

    pub fn is_generated_mesh(&self) -> bool {
        self.path == Self::GENERATED_MESH
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClipRef {
    pub path: String,
}

impl AnimationClipRef {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraState {
    pub pivot: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_y: f32,

    #[serde(default)]
    pub position: Option<[f32; 3]>,
    #[serde(default)]
    pub direction: Option<[f32; 3]>,
    #[serde(default)]
    pub up: Option<[f32; 3]>,

    #[serde(default)]
    pub physical_camera: Option<PhysicalCameraState>,
    #[serde(default)]
    pub exposure: Option<ExposureState>,
    #[serde(default)]
    pub depth_of_field: Option<DepthOfFieldState>,
    #[serde(default)]
    pub tone_mapping: Option<ToneMappingState>,
    #[serde(default)]
    pub lens_effects: Option<LensEffectsState>,
    #[serde(default)]
    pub bloom: Option<BloomState>,
    #[serde(default)]
    pub auto_exposure: Option<AutoExposureState>,
}

impl Default for CameraState {
    fn default() -> Self {
        use std::f32::consts::PI;
        Self {
            pivot: [0.0, 0.0, 0.0],
            yaw: PI / 4.0,
            pitch: (5.0_f32 / 75.0_f32.sqrt()).asin(),
            distance: 75.0_f32.sqrt(),
            fov_y: 45.0,
            position: None,
            direction: None,
            up: None,
            physical_camera: None,
            exposure: None,
            depth_of_field: None,
            tone_mapping: None,
            lens_effects: None,
            bloom: None,
            auto_exposure: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineConfig {
    pub current_time: f32,
    pub playing: bool,
    pub looping: bool,
    pub speed: f32,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            current_time: 0.0,
            playing: false,
            looping: true,
            speed: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorState {
    pub selected_bone_id: Option<u32>,
    pub curve_editor_open: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected_bone_id: None,
            curve_editor_open: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelLayoutState {
    pub hierarchy_width: f32,
    pub inspector_width: f32,
    pub timeline_height: f32,
    pub debug_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalCameraState {
    pub focal_length_mm: f32,
    pub sensor_height_mm: f32,
    pub aperture_f_stops: f32,
    pub shutter_speed_s: f32,
    pub sensitivity_iso: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureState {
    pub ev100: f32,
    pub exposure_value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthOfFieldState {
    pub enabled: bool,
    pub focus_distance: f32,
    pub max_blur_radius: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneMappingState {
    pub enabled: bool,
    pub operator: String,
    pub gamma: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LensEffectsState {
    pub vignette_enabled: bool,
    pub vignette_intensity: f32,
    pub chromatic_aberration_enabled: bool,
    pub chromatic_aberration_intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloomState {
    pub enabled: bool,
    pub intensity: f32,
    pub threshold: f32,
    pub knee: f32,
    pub mip_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoExposureState {
    pub enabled: bool,
    pub min_ev: f32,
    pub max_ev: f32,
    pub adaptation_speed_up: f32,
    pub adaptation_speed_down: f32,
    pub low_percent: f32,
    pub high_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectChannelData {
    pub param: String,
    pub keys: Vec<EffectKeyData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectKeyData {
    pub time: f32,
    pub value: f32,
    #[serde(default = "default_effect_interpolation")]
    pub interpolation: String,
    pub in_tangent: Option<[f32; 2]>,
    pub out_tangent: Option<[f32; 2]>,
    pub weight_mode: Option<String>,
}

fn default_effect_interpolation() -> String {
    "Linear".to_string()
}

/// Convert an `Interpolation` variant to its string representation.
pub fn interpolation_to_string(interp: thyllore_anim_core::Interpolation) -> String {
    match interp {
        thyllore_anim_core::Interpolation::Step => "Step".to_string(),
        thyllore_anim_core::Interpolation::Linear => "Linear".to_string(),
        thyllore_anim_core::Interpolation::CubicSpline => "CubicSpline".to_string(),
    }
}

/// Convert an editable `InterpolationType` variant to its string representation.
fn editable_interpolation_to_string(
    interp: thyllore_anim_core::editable::InterpolationType,
) -> String {
    match interp {
        thyllore_anim_core::editable::InterpolationType::Stepped => "Step".to_string(),
        thyllore_anim_core::editable::InterpolationType::Linear => "Linear".to_string(),
        thyllore_anim_core::editable::InterpolationType::Bezier => "Bezier".to_string(),
    }
}

/// Convert a string back to an `Interpolation` variant. Unknown strings default to Linear.
pub fn interpolation_from_string(s: &str) -> thyllore_anim_core::Interpolation {
    match s {
        "Step" => thyllore_anim_core::Interpolation::Step,
        "Linear" => thyllore_anim_core::Interpolation::Linear,
        "CubicSpline" => thyllore_anim_core::Interpolation::CubicSpline,
        _ => thyllore_anim_core::Interpolation::Linear,
    }
}

pub(super) fn effect_clip_min_duration(
    world: &crate::ecs::world::World,
    entity: crate::ecs::world::Entity,
) -> f32 {
    crate::ecs::systems::find_entity_clip_id(world, entity)
        .and_then(|clip_id| {
            world
                .get_resource::<crate::ecs::resource::ClipLibrary>()
                .and_then(|lib| lib.get(clip_id).map(|clip| clip.min_duration))
        })
        .unwrap_or(0.0)
}

pub(super) fn build_effect_channels_from_clip(
    world: &crate::ecs::world::World,
    entity: crate::ecs::world::Entity,
) -> Vec<EffectChannelData> {
    let Some(clip_id) = crate::ecs::systems::find_entity_clip_id(world, entity) else {
        return Vec::new();
    };
    let Some(lib) = world.get_resource::<crate::ecs::resource::ClipLibrary>() else {
        return Vec::new();
    };
    let Some(clip) = lib.get(clip_id) else {
        return Vec::new();
    };

    clip.scalar_curves
        .iter()
        .filter_map(|curve| {
            let (_, channel) =
                crate::ecs::component::scalar_channel_for_property(curve.property_type)?;
            Some(EffectChannelData {
                param: channel.scene_name.to_string(),
                keys: curve
                    .keyframes
                    .iter()
                    .map(|k| EffectKeyData {
                        time: k.time,
                        value: k.value,
                        interpolation: editable_interpolation_to_string(k.interpolation),
                        in_tangent: Some([k.in_tangent.time_offset, k.in_tangent.value_offset]),
                        out_tangent: Some([k.out_tangent.time_offset, k.out_tangent.value_offset]),
                        weight_mode: Some(match k.weight_mode {
                            thyllore_anim_core::editable::TangentWeightMode::NonWeighted => {
                                "NonWeighted".to_string()
                            }
                            thyllore_anim_core::editable::TangentWeightMode::Weighted => {
                                "Weighted".to_string()
                            }
                        }),
                    })
                    .collect(),
            })
        })
        .collect()
}

pub fn build_debug_primitives_scene_data(
    world: &crate::ecs::world::World,
) -> Vec<DebugPrimitiveSceneData> {
    let mut primitives: Vec<_> = world
        .iter_components::<crate::ecs::component::DebugPrimitiveTag>()
        .filter_map(|(entity, tag)| {
            let transform = world.get_component::<crate::ecs::world::Transform>(entity)?;
            Some((
                entity,
                DebugPrimitiveSceneData {
                    kind: debug_primitive_kind_to_str(tag.kind).to_string(),
                    position: [
                        transform.translation.x,
                        transform.translation.y,
                        transform.translation.z,
                    ],
                },
            ))
        })
        .collect();

    primitives.sort_by_key(|(entity, _)| *entity);
    primitives.into_iter().map(|(_, data)| data).collect()
}

/// Rebuild an effect clip (scalar curves) from scene channels and schedule it on the entity.
/// Loading is idempotent: any previously scheduled clip instance is replaced. An empty clip
/// is still scheduled so the Timeline shows a lane whose length can be edited before any key exists.
pub(super) fn rebuild_effect_clip(
    world: &mut crate::ecs::world::World,
    assets: &mut crate::asset::AssetStorage,
    entity: crate::ecs::world::Entity,
    domain_name: &str,
    channels: &[EffectChannelData],
    clip_min_duration: f32,
) {
    let mut editable =
        thyllore_anim_core::editable::EditableAnimationClip::new(0, domain_name.to_string());
    for ch in channels {
        let Some((_, channel)) = crate::ecs::component::scalar_channel_for_scene_name(&ch.param)
        else {
            continue;
        };
        let curve = editable.get_or_add_scalar_curve(channel.property_type());
        for k in ch.keys.iter() {
            let interp = match k.interpolation.as_str() {
                "Bezier" => thyllore_anim_core::editable::InterpolationType::Bezier,
                "Step" => thyllore_anim_core::editable::InterpolationType::Stepped,
                _ => thyllore_anim_core::editable::InterpolationType::Linear,
            };
            let id = thyllore_anim_core::editable::curve_add_keyframe(curve, k.time, k.value);
            let key = curve.get_keyframe_mut(id).expect("key just inserted");
            key.interpolation = interp;
            if let Some([t, v]) = k.in_tangent {
                key.in_tangent.time_offset = t;
                key.in_tangent.value_offset = v;
            }
            if let Some([t, v]) = k.out_tangent {
                key.out_tangent.time_offset = t;
                key.out_tangent.value_offset = v;
            }
            if let Some(wm) = &k.weight_mode {
                key.weight_mode = match wm.as_str() {
                    "Weighted" => thyllore_anim_core::editable::TangentWeightMode::Weighted,
                    _ => thyllore_anim_core::editable::TangentWeightMode::NonWeighted,
                };
            }
        }
    }
    editable.min_duration = clip_min_duration.max(0.0);
    thyllore_anim_core::editable::clip_recalculate_duration(&mut editable);

    if let Some(previous_clip_id) =
        crate::ecs::systems::scalar_clip_systems::find_entity_clip_id(world, entity)
    {
        world
            .resource_mut::<crate::ecs::resource::ClipLibrary>()
            .remove(previous_clip_id);
    }
    world.remove_component::<crate::ecs::component::ClipSchedule>(entity);
    let duration = editable.duration;
    let clip_id = {
        let mut clip_library = world.resource_mut::<crate::ecs::resource::ClipLibrary>();
        crate::ecs::systems::clip_library_systems::clip_library_register_and_activate(
            &mut clip_library,
            assets,
            editable,
        )
    };
    let mut schedule = crate::ecs::component::ClipSchedule::new();
    let instance_id = schedule.next_instance_id;
    schedule.next_instance_id += 1;
    schedule
        .instances
        .push(thyllore_anim_core::editable::ClipInstance::new(
            instance_id,
            clip_id,
            duration,
        ));
    world.insert_component(entity, schedule);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_channel_scene_name_roundtrip() {
        for domain in crate::ecs::component::scalar_channel_domains() {
            for channel in domain.channels {
                let (_, found) =
                    crate::ecs::component::scalar_channel_for_scene_name(channel.scene_name)
                        .expect("scene name resolves");
                assert_eq!(
                    found.code, channel.code,
                    "Roundtrip failed for {}",
                    channel.scene_name
                );
            }
        }
    }

    #[test]
    fn test_interpolation_string_roundtrip() {
        let variants: Vec<thyllore_anim_core::Interpolation> = vec![
            thyllore_anim_core::Interpolation::Step,
            thyllore_anim_core::Interpolation::Linear,
            thyllore_anim_core::Interpolation::CubicSpline,
        ];
        for v in variants {
            let s = interpolation_to_string(v.clone());
            let roundtrip = interpolation_from_string(&s);
            assert_eq!(roundtrip, v, "Roundtrip failed for {}", s);
        }
        // Unknown string maps to Linear
        assert_eq!(
            interpolation_from_string("unknown"),
            thyllore_anim_core::Interpolation::Linear
        );
    }
}
