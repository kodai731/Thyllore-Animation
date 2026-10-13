use crate::ecs::world::Entity;

#[derive(Clone, Debug)]
pub struct DischargeEvent {
    pub entity: Entity,
    pub burst_index: u32,
    pub start_time: f32,
    pub origin: [f32; 3],
    pub target: [f32; 3],
    pub sound_speed: f32,
}

impl DischargeEvent {
    pub fn delay_to(&self, listener: [f32; 3]) -> f32 {
        let dx = self.target[0] - listener[0];
        let dy = self.target[1] - listener[1];
        let dz = self.target[2] - listener[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        distance / self.sound_speed
    }
}

#[derive(Clone, Debug, Default)]
pub struct DischargeEvents {
    events: Vec<DischargeEvent>,
}

impl DischargeEvents {
    pub fn push(&mut self, event: DischargeEvent) {
        self.events.push(event);
    }

    pub fn drain(&mut self) -> Vec<DischargeEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DischargeEvent> {
        self.events.iter()
    }
}
