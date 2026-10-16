use std::sync::LazyLock;

use cgmath::Vector3;
use thyllore_scene_core::{RootPath, ScalarParam, SceneFields, UiParam};

use super::submesh_role::SubmeshKey;

#[derive(Debug, Clone, Copy)]
pub enum ChainRole {
    Hair,
    Skirt,
}

impl ChainRole {
    pub fn bone_prefix(self) -> &'static str {
        match self {
            ChainRole::Hair => "hair",
            ChainRole::Skirt => "skirt",
        }
    }
}

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
    /// Chains per submesh; 0 takes the count from the azimuth distribution
    #[persist(ui(min = 0.0, max = 32.0, format = "%.0f"))]
    pub column_count: u32,
    /// Arc length in model units over which the root bone keeps its weight; 0 uses the first joint length
    #[persist(ui(min = 0.0, max = 100.0, format = "%.3f"))]
    pub root_blend_width: f32,
    /// Distance band in model units over which a vertex blends with the neighbouring column; 0 keeps columns hard
    #[persist(ui(min = 0.0, max = 100.0, format = "%.3f"))]
    pub column_blend_width: f32,
}

impl Default for ChainGenSettings {
    fn default() -> Self {
        Self {
            joint_count: 4,
            column_count: 0,
            root_blend_width: 0.0,
            column_blend_width: 0.0,
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
    pub settings: ChainGenSettings,
}

#[derive(Debug, Clone)]
pub struct ChainGenInput<'a> {
    pub positions: &'a [Vector3<f32>],
    pub triangles: &'a [[u32; 3]],
    pub root_position: Vector3<f32>,
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

pub fn joint_name(role: ChainRole, column: usize, joint: usize) -> String {
    format!("{}_{:02}_{:02}", role.bone_prefix(), column, joint)
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

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct GeneratedBoneRecord {
    pub name: String,
    pub parent: String,
    pub head: [f32; 3],
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ChainWeightRecord {
    pub mesh: String,
    pub vertex: u32,
    pub bone: String,
    pub weight: f32,
    pub position: [f32; 3],
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ChainGenReport {
    pub submesh: String,
    pub bones: Vec<GeneratedBoneRecord>,
    pub vertex_weights: Vec<ChainWeightRecord>,
}

#[derive(Debug, Clone)]
pub struct ChainGenRequest {
    pub submesh: SubmeshKey,
    pub root_bone: String,
    pub settings: ChainGenSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_list_every_setting_in_declaration_order() {
        let names: Vec<&str> = CHAIN_GEN_UI_PARAMS.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "joint_count",
                "column_count",
                "root_blend_width",
                "column_blend_width"
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
