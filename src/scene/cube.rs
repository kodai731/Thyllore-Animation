use crate::vulkanr::data::Vertex;
use crate::math::*;

#[derive(Clone, Debug, Default)]
pub struct CubeModel {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl CubeModel {
    pub fn new(size: f32) -> Self {
        let half = size / 2.0;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let faces = [
            ([0.0, 0.0, 1.0], [
                [-half, -half,  half],
                [-half,  half,  half],
                [ half,  half,  half],
                [ half, -half,  half],
            ]),
            ([0.0, 0.0, -1.0], [
                [ half, -half, -half],
                [ half,  half, -half],
                [-half,  half, -half],
                [-half, -half, -half],
            ]),
            ([1.0, 0.0, 0.0], [
                [ half, -half,  half],
                [ half,  half,  half],
                [ half,  half, -half],
                [ half, -half, -half],
            ]),
            ([-1.0, 0.0, 0.0], [
                [-half, -half, -half],
                [-half,  half, -half],
                [-half,  half,  half],
                [-half, -half,  half],
            ]),
            ([0.0, 1.0, 0.0], [
                [-half,  half,  half],
                [-half,  half, -half],
                [ half,  half, -half],
                [ half,  half,  half],
            ]),
            ([0.0, -1.0, 0.0], [
                [-half, -half, -half],
                [-half, -half,  half],
                [ half, -half,  half],
                [ half, -half, -half],
            ]),
        ];

        let tex_coords = [
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
        ];

        let face_colors = [
            Vec4::new(1.0, 0.3, 0.3, 1.0),
            Vec4::new(0.3, 1.0, 0.3, 1.0),
            Vec4::new(0.3, 0.3, 1.0, 1.0),
            Vec4::new(1.0, 1.0, 0.3, 1.0),
            Vec4::new(0.3, 1.0, 1.0, 1.0),
            Vec4::new(1.0, 0.3, 1.0, 1.0),
        ];

        for (face_idx, (normal, positions)) in faces.iter().enumerate() {
            let base_index = vertices.len() as u32;
            let color = face_colors[face_idx];

            for (i, pos) in positions.iter().enumerate() {
                vertices.push(Vertex {
                    pos: Vec3::new(pos[0], pos[1], pos[2]),
                    color,
                    tex_coord: Vec2::new(tex_coords[i][0], tex_coords[i][1]),
                    normal: Vec3::new(normal[0], normal[1], normal[2]),
                });
            }

            indices.push(base_index);
            indices.push(base_index + 1);
            indices.push(base_index + 2);
            indices.push(base_index);
            indices.push(base_index + 2);
            indices.push(base_index + 3);
        }

        Self { vertices, indices }
    }

    pub fn new_at_position(size: f32, position: [f32; 3]) -> Self {
        let mut cube = Self::new(size);
        for vertex in &mut cube.vertices {
            vertex.pos.x += position[0];
            vertex.pos.y += position[1];
            vertex.pos.z += position[2];
        }
        cube
    }
}
