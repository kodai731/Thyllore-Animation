use cgmath::Vector3;

#[derive(Clone, Debug)]
pub struct HiddenMaskParams {
    pub cover_distance: f32,
    pub cone_angle_deg: f32,
    pub ray_count: usize,
    pub hidden_ratio: f32,
}

#[derive(Clone, Debug, Default)]
pub struct MaskMesh {
    pub positions: Vec<Vector3<f32>>,
    pub normals: Vec<Vector3<f32>>,
    pub triangles: Vec<[u32; 3]>,
}
