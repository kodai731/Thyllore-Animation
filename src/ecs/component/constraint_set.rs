use crate::animation::{BoneId, ConstraintId, ConstraintType};

#[derive(Clone, Debug)]
pub struct ConstraintEntry {
    pub id: ConstraintId,
    pub constraint: ConstraintType,
    pub priority: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ConstraintSet {
    pub constraints: Vec<ConstraintEntry>,
    next_id: ConstraintId,
}

impl ConstraintSet {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_constraint(
        &mut self,
        constraint: ConstraintType,
        priority: u32,
    ) -> ConstraintId {
        let id = self.next_id;
        self.next_id += 1;

        let entry = ConstraintEntry {
            id,
            constraint,
            priority,
        };
        self.constraints.push(entry);
        self.constraints.sort_by_key(|e| e.priority);

        id
    }

    pub fn remove_constraint(&mut self, id: ConstraintId) -> bool {
        let before = self.constraints.len();
        self.constraints.retain(|e| e.id != id);
        self.constraints.len() < before
    }

    pub fn find_constraint(
        &self,
        id: ConstraintId,
    ) -> Option<&ConstraintEntry> {
        self.constraints.iter().find(|e| e.id == id)
    }

    pub fn find_constraint_mut(
        &mut self,
        id: ConstraintId,
    ) -> Option<&mut ConstraintEntry> {
        self.constraints.iter_mut().find(|e| e.id == id)
    }

    pub fn find_by_bone(&self, bone_id: BoneId) -> Vec<&ConstraintEntry> {
        self.constraints
            .iter()
            .filter(|e| e.constraint.constrained_bone_id() == bone_id)
            .collect()
    }

    pub fn enabled_constraints(&self) -> Vec<&ConstraintEntry> {
        self.constraints
            .iter()
            .filter(|e| e.constraint.is_enabled())
            .collect()
    }
}
