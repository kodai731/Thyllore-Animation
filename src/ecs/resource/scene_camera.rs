use crate::ecs::resource::CameraPose;
use crate::ecs::world::Entity;

#[derive(Clone, Debug, Default)]
pub struct ActiveSceneCamera {
    pub entity: Option<Entity>,
    pub view_enabled: bool,
    pub saved_editor_pose: Option<CameraPose>,
    pub saved_fov_y: Option<f32>,
}
