use std::sync::LazyLock;

use cgmath::Vector3;
use thyllore_scene_core::{RootPath, ScalarParam, SceneFields, UiParam};

use super::submesh_role::{SubmeshRole, SubmeshSelector};
use crate::humanoid::components::role::HumanoidRole;

pub use thyllore_anim_core::spring_bone::ChainRole;

#[derive(Debug, Clone, Copy)]
pub enum ChainGenOwner {
    Setup,
}

#[derive(Clone, Debug, PartialEq, SceneFields)]
#[params(tag = ChainGenOwner, owner = Setup)]
pub struct ChainGenSettings {
    /// Joints per chain
    #[persist(ui(min = 1.0, max = 16.0, format = "%.0f"))]
    pub joint_count: u32,
    /// Chains per submesh; 0 takes the count from the φ distribution
    #[persist(ui(min = 0.0, max = 32.0, format = "%.0f"))]
    pub column_count: u32,
    /// Centerline distance in model units under which two strands join one chain; 0 keeps every strand apart
    #[persist(ui(min = 0.0, max = 100.0, format = "%.3f"))]
    pub strand_merge_distance: f32,
    /// Arc length in model units over which the root bone keeps its weight; 0 uses the first joint length
    #[persist(ui(min = 0.0, max = 100.0, format = "%.3f"))]
    pub root_blend_width: f32,
    /// Distance band in model units over which a vertex blends with the neighbouring column; 0 keeps columns hard
    #[persist(ui(min = 0.0, max = 100.0, format = "%.3f"))]
    pub column_blend_width: f32,
    #[nested]
    pub azimuth: AzimuthStrandSettings,
    #[nested]
    pub card: CardStrandSettings,
}

/// Settings of the azimuth placement, which rings the hair around the skin's widest cross-section.
/// Columns are placed by the spherical azimuth φ about the world up axis (+Y), measured from the
/// reference axis of `tangent_frame_around(up)` (+X) toward its side axis (−Z).
#[derive(Clone, Debug, Default, PartialEq, SceneFields)]
#[params(tag = ChainGenOwner, owner = Setup)]
pub struct AzimuthStrandSettings {
    /// Shift of the hair root band from the skin's widest cross-section in model units (positive = toward tips, negative = toward the root bone)
    #[persist(renamed_from = ["root_band_offset"], ui(min = -100.0, max = 100.0, format = "%.3f"))]
    pub root_band_offset: f32,
}

/// Settings of the card placement, which follows each hair card's UV strand.
#[derive(Clone, Debug, Default, PartialEq, SceneFields)]
#[params(tag = ChainGenOwner, owner = Setup)]
pub struct CardStrandSettings {
    /// Length from each card's base kept as a rigid root in model units (0 = the full card is chain)
    #[persist(ui(min = 0.0, max = 100.0, format = "%.3f"))]
    pub root_rigid_length: f32,
}

impl Default for ChainGenSettings {
    fn default() -> Self {
        Self {
            joint_count: 4,
            column_count: 0,
            strand_merge_distance: 0.0,
            root_blend_width: 0.0,
            column_blend_width: 0.0,
            azimuth: AzimuthStrandSettings::default(),
            card: CardStrandSettings::default(),
        }
    }
}

pub static CHAIN_GEN_SCALAR_PARAMS: LazyLock<Vec<ScalarParam<ChainGenSettings>>> =
    LazyLock::new(|| {
        let mut out = Vec::new();
        ChainGenSettings::collect_scalars::<ChainGenSettings, RootPath<ChainGenSettings>>(
            "", &mut out,
        );
        out
    });

pub static CHAIN_GEN_UI_PARAMS: LazyLock<Vec<UiParam>> = LazyLock::new(|| {
    let mut out = Vec::new();
    ChainGenSettings::collect_ui("", "", &mut out);
    out
});

#[derive(Debug, Clone)]
pub struct ChainGenParams {
    pub role: ChainRole,
    pub target: SubmeshSelector,
    pub settings: ChainGenSettings,
}

pub fn chain_role_of_submesh(role: SubmeshRole) -> Option<ChainRole> {
    match role {
        SubmeshRole::Hair => Some(ChainRole::Hair),
        SubmeshRole::Garment => Some(ChainRole::Garment),
        SubmeshRole::Skin | SubmeshRole::Accessory => None,
    }
}

pub fn chain_root_role(role: ChainRole) -> HumanoidRole {
    match role {
        ChainRole::Hair => HumanoidRole::Head,
        ChainRole::Garment => HumanoidRole::Hips,
    }
}

/// The submesh part of a generated bone name: lower case, one `_` between runs of other characters.
pub fn submesh_label(target: &SubmeshSelector) -> String {
    let text = match target.material() {
        Some(material) => format!("{} {}", target.mesh(), material),
        None => target.mesh().to_string(),
    };
    let mut label = String::with_capacity(text.len());
    for ch in text.chars() {
        let lowered = ch.to_ascii_lowercase();
        if lowered.is_ascii_alphanumeric() {
            label.push(lowered);
        } else if !label.ends_with('_') && !label.is_empty() {
            label.push('_');
        }
    }
    label.trim_end_matches('_').to_string()
}

#[derive(Debug, Clone)]
pub struct ChainGenInput<'a> {
    pub positions: &'a [Vector3<f32>],
    pub triangles: &'a [[u32; 3]],
    pub root_position: Vector3<f32>,
    /// Skin vertices the root bone drives, in the same space as `positions`; empty falls back to `positions`
    pub attachment_positions: &'a [Vector3<f32>],
    /// Per-vertex texture coordinates; empty means no strand parameter is available
    pub tex_coords: &'a [[f32; 2]],
}

#[derive(Debug, Clone)]
pub struct GeneratedJoint {
    pub name: String,
    pub head: Vector3<f32>,
}

#[derive(Debug, Clone)]
pub struct GeneratedChain {
    pub column: usize,
    pub joints: Vec<GeneratedJoint>,
    pub tail: Vector3<f32>,
    pub vertices: Vec<u32>,
}

pub fn chain_name(params: &ChainGenParams, column: usize) -> String {
    format!(
        "{}_{}_{:02}",
        params.role.bone_prefix(),
        submesh_label(&params.target),
        column
    )
}

pub fn joint_name(params: &ChainGenParams, column: usize, joint: usize) -> String {
    format!("{}_{:02}", chain_name(params, column), joint)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfluenceTarget {
    Root,
    Joint { chain: usize, joint: usize },
}

#[derive(Debug, Clone)]
pub struct VertexInfluences {
    pub vertex: u32,
    pub influences: Vec<(InfluenceTarget, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum StrandMode {
    Card,
    Azimuth,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct GeneratedBoneRecord {
    pub name: String,
    pub parent: String,
    pub head: [f32; 3],
    /// The chain end point, present on the last joint of each chain
    pub tail: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ChainWeightRecord {
    pub mesh: String,
    pub vertex: u32,
    pub component: u32,
    pub bone: String,
    pub weight: f32,
    pub position: [f32; 3],
}

/// The hair root band as model-space heights along the up axis
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RootBandRecord {
    pub equator_height: f32,
    pub height: f32,
    pub center: [f32; 3],
}

/// The head capsule the hair chains collide with, as model-space points
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct HeadCapsuleRecord {
    pub bottom: [f32; 3],
    pub top: [f32; 3],
    pub radius: f32,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct StrandCenterlineRecord {
    pub component: u32,
    pub chain: usize,
    pub points: Vec<[f32; 3]>,
}

/// How the hair chains were placed and the card centerlines the card path used
#[derive(Debug, Clone)]
pub struct StrandPlacement {
    pub mode: StrandMode,
    pub centerlines: Vec<StrandCenterlineRecord>,
}

#[derive(Debug, Clone)]
pub struct ChainPlacement {
    pub chains: Vec<GeneratedChain>,
    pub strands: Option<StrandPlacement>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ChainGenReport {
    pub target: String,
    pub bones: Vec<GeneratedBoneRecord>,
    pub vertex_weights: Vec<ChainWeightRecord>,
    pub root_band: Option<RootBandRecord>,
    pub head_capsule: Option<HeadCapsuleRecord>,
    pub attachment_positions: Vec<[f32; 3]>,
    pub strand_mode: Option<StrandMode>,
    pub strand_centerlines: Vec<StrandCenterlineRecord>,
}

#[derive(Debug, Clone)]
pub struct ChainGenRequest {
    pub target: SubmeshSelector,
    pub root: HumanoidRole,
    pub settings: ChainGenSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submesh_label_lowers_and_joins_runs_with_one_underscore() {
        let selector: SubmeshSelector = "woman 1_mat2/Hair".parse().unwrap();
        assert_eq!(submesh_label(&selector), "woman_1_mat2_hair");
        let mesh_only: SubmeshSelector = "Hair_back".parse().unwrap();
        assert_eq!(submesh_label(&mesh_only), "hair_back");
    }

    #[test]
    fn joint_names_differ_between_submeshes_and_follow_the_chain_name() {
        let params = |target: &str| ChainGenParams {
            role: ChainRole::Hair,
            target: target.parse().unwrap(),
            settings: ChainGenSettings::default(),
        };
        assert_eq!(
            joint_name(&params("Hair_back"), 3, 1),
            "hair_hair_back_03_01"
        );
        assert_eq!(
            joint_name(&params("Hair_side"), 3, 1),
            "hair_hair_side_03_01"
        );
        assert_eq!(chain_name(&params("Hair_side"), 3), "hair_hair_side_03");
    }

    #[test]
    fn tables_list_every_setting_in_declaration_order() {
        let names: Vec<&str> = CHAIN_GEN_UI_PARAMS.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "joint_count",
                "column_count",
                "strand_merge_distance",
                "root_blend_width",
                "column_blend_width",
                "azimuth_root_band_offset",
                "card_root_rigid_length"
            ]
        );
        let scalar_names: Vec<&str> = CHAIN_GEN_SCALAR_PARAMS.iter().map(|p| p.name).collect();
        assert_eq!(names, scalar_names);
    }

    #[test]
    fn integer_settings_round_trip_through_the_scalar_table() {
        let mut settings = ChainGenSettings::default();
        let joint_count = CHAIN_GEN_SCALAR_PARAMS
            .iter()
            .find(|p| p.name == "joint_count")
            .unwrap();
        (joint_count.set)(&mut settings, 7.0);
        assert_eq!(settings.joint_count, 7);
        assert_eq!((joint_count.get)(&settings), 7.0);
    }
}
