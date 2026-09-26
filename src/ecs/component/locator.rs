use cgmath::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

/// An empty entity whose only data is its placement; other features point at it by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Locator {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}

impl Default for Locator {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

impl Locator {
    pub fn at(position: Vector3<f32>) -> Self {
        Self {
            position: position.into(),
            ..Self::default()
        }
    }
}

impl SceneComponent for Locator {
    const TYPE_KEY: &'static str = "locator";
    const PERSISTED_FIELDS: &'static [&'static str] = &["position", "rotation"];
}

crate::scene_owner!(Locator {
    icon: crate::ecs::component::EntityIcon::Empty,
    placement: |locator| (
        Vector3::from(locator.position),
        Quaternion::new(
            locator.rotation[0],
            locator.rotation[1],
            locator.rotation[2],
            locator.rotation[3],
        ),
    ),
});
