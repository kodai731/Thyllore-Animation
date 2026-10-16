use cgmath::Vector3;

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

#[derive(Debug, Clone)]
pub struct ChainGenParams {
    pub role: ChainRole,
    pub joint_count: usize,
    pub column_count: usize,
    pub root_blend_width: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct ChainGenInput<'a> {
    pub positions: &'a [Vector3<f32>],
    pub triangles: &'a [[u32; 3]],
    pub root_position: Vector3<f32>,
    pub up: Vector3<f32>,
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
