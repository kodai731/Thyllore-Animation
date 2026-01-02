use cgmath::{Matrix4, Vector3, Vector4, InnerSpace};

#[derive(Clone, Debug)]
pub struct BillboardTransform {
    pub position: Vector3<f32>,
    pub model_matrix: Matrix4<f32>,
}

impl BillboardTransform {
    pub fn new(position: Vector3<f32>) -> Self {
        Self {
            position,
            model_matrix: Matrix4::from_translation(position),
        }
    }

    pub fn update_look_at(&mut self, camera_position: Vector3<f32>, world_up: Vector3<f32>) {
        let forward = (camera_position - self.position).normalize();

        let right = world_up.cross(forward).normalize();
        let up = forward.cross(right);

        let rotation = Matrix4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            forward.extend(0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );

        let translation = Matrix4::from_translation(self.position);
        self.model_matrix = translation * rotation;
    }

    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }
}
