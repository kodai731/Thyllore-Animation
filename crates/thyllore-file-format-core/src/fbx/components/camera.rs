use cgmath::Matrix4;

#[derive(Clone, Debug, PartialEq)]
pub enum CameraProjection {
    Perspective {
        yfov: f32,
        aspect_ratio: Option<f32>,
        znear: f32,
        zfar: Option<f32>,
    },
    Orthographic {
        xmag: f32,
        ymag: f32,
        znear: f32,
        zfar: f32,
    },
}

#[derive(Clone, Debug)]
pub struct LoadedCamera {
    pub node_index: usize,
    pub name: String,
    pub world_transform: Matrix4<f32>,
    pub projection: CameraProjection,
}
