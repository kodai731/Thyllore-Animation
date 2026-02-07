use crate::animation::{ConstraintId, ConstraintType};

#[derive(Clone, Debug)]
pub struct ConstraintEntry {
    pub id: ConstraintId,
    pub constraint: ConstraintType,
    pub priority: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ConstraintSet {
    pub constraints: Vec<ConstraintEntry>,
    pub next_id: ConstraintId,
}

impl ConstraintSet {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            next_id: 1,
        }
    }
}
