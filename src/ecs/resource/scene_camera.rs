use crate::ecs::world::Entity;

#[derive(Clone, Debug, Default)]
pub struct ActiveSceneCamera {
    pub entity: Option<Entity>,
    pub view_enabled: bool,
}
