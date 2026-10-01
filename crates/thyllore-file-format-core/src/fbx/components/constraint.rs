use thyllore_anim_core::ConstraintType;

#[derive(Clone, Debug)]
pub struct LoadedConstraint {
    pub constraint_type: ConstraintType,
    pub priority: u32,
}
