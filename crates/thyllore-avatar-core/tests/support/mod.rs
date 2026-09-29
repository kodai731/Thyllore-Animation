#[path = "rig_names.rs"]
pub mod rig_names;

#[path = "rig_convention.rs"]
pub mod rig_convention;

#[derive(Clone, Copy, Debug)]
pub struct CanonicalBone {
    pub role: &'static str,
    pub parent: Option<usize>,
    pub position: [f32; 3],
}

pub fn canonical_bones() -> Vec<CanonicalBone> {
    vec![
        CanonicalBone {
            role: "Hips",
            parent: None,
            position: [0.0, 1.0, 0.0],
        },
        CanonicalBone {
            role: "Spine",
            parent: Some(0),
            position: [0.0, 1.1, 0.0],
        },
        CanonicalBone {
            role: "Chest",
            parent: Some(1),
            position: [0.0, 1.3, 0.0],
        },
        CanonicalBone {
            role: "Neck",
            parent: Some(2),
            position: [0.0, 1.5, 0.0],
        },
        CanonicalBone {
            role: "Head",
            parent: Some(3),
            position: [0.0, 1.6, 0.0],
        },
        CanonicalBone {
            role: "RightShoulder",
            parent: Some(2),
            position: [0.05, 1.45, 0.0],
        },
        CanonicalBone {
            role: "RightUpperArm",
            parent: Some(5),
            position: [0.18, 1.45, 0.0],
        },
        CanonicalBone {
            role: "RightLowerArm",
            parent: Some(6),
            position: [0.46, 1.45, 0.0],
        },
        CanonicalBone {
            role: "RightHand",
            parent: Some(7),
            position: [0.72, 1.45, 0.0],
        },
        CanonicalBone {
            role: "LeftShoulder",
            parent: Some(2),
            position: [-0.05, 1.45, 0.0],
        },
        CanonicalBone {
            role: "LeftUpperArm",
            parent: Some(9),
            position: [-0.18, 1.45, 0.0],
        },
        CanonicalBone {
            role: "LeftLowerArm",
            parent: Some(10),
            position: [-0.46, 1.45, 0.0],
        },
        CanonicalBone {
            role: "LeftHand",
            parent: Some(11),
            position: [-0.72, 1.45, 0.0],
        },
        CanonicalBone {
            role: "RightUpperLeg",
            parent: Some(0),
            position: [0.1, 0.95, 0.0],
        },
        CanonicalBone {
            role: "RightLowerLeg",
            parent: Some(13),
            position: [0.1, 0.5, 0.0],
        },
        CanonicalBone {
            role: "RightFoot",
            parent: Some(14),
            position: [0.1, 0.08, 0.0],
        },
        CanonicalBone {
            role: "LeftUpperLeg",
            parent: Some(0),
            position: [-0.1, 0.95, 0.0],
        },
        CanonicalBone {
            role: "LeftLowerLeg",
            parent: Some(16),
            position: [-0.1, 0.5, 0.0],
        },
        CanonicalBone {
            role: "LeftFoot",
            parent: Some(17),
            position: [-0.1, 0.08, 0.0],
        },
    ]
}
