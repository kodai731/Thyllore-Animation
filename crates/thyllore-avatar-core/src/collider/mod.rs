pub(crate) mod components;
pub(crate) mod systems;

pub use components::capsule::HeadCapsule;
pub use systems::body_fit::fit_body_capsules;
pub use systems::head_capsule::estimate_head_capsule;
pub use systems::torso_volume::{
    fitted_torso_fallback, torso_fallback_colliders, torso_roles, OwnRadius, RoleBone,
};
