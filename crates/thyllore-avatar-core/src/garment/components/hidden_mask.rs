use cgmath::Vector3;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HiddenMaskParams {
    pub cover_distance: f32,
    pub cone_angle_deg: f32,
    pub ray_count: usize,
    pub hidden_ratio: f32,
}

impl Default for HiddenMaskParams {
    fn default() -> Self {
        Self {
            cover_distance: 0.1,
            cone_angle_deg: 30.0,
            ray_count: 5,
            hidden_ratio: 0.6,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MaskMesh {
    pub positions: Vec<Vector3<f32>>,
    pub normals: Vec<Vector3<f32>>,
    pub triangles: Vec<[u32; 3]>,
}
