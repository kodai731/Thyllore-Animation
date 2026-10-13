use std::collections::HashMap;

use crate::ecs::world::Entity;

#[derive(Clone, Debug, Default)]
pub struct LightningSurroundClock {
    pub previous_times: HashMap<Entity, f32>,
}
