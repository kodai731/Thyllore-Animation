use cgmath::Vector3;

use super::format::{SidecarCollider, SidecarSpring};
use super::frame::BoneRestFrame;
use crate::humanoid::components::skeleton_input::BoneInput;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColliderSpace {
    BoneLocal,
    Model,
}

/// The spring with every collider's offset, tail and radius moved into `target` through its bone's rest
/// frame; names, order, chains and colliders on unknown bones pass through untouched.
pub fn map_sidecar_colliders(
    spring: &SidecarSpring,
    bones: &[BoneInput],
    frames: &[BoneRestFrame],
    target: ColliderSpace,
) -> SidecarSpring {
    let colliders = spring
        .colliders
        .iter()
        .map(|collider| match frame_of(collider, bones, frames) {
            Some(frame) => map_collider(collider, frame, target),
            None => collider.clone(),
        })
        .collect();
    SidecarSpring {
        colliders,
        ..spring.clone()
    }
}

fn frame_of<'a>(
    collider: &SidecarCollider,
    bones: &[BoneInput],
    frames: &'a [BoneRestFrame],
) -> Option<&'a BoneRestFrame> {
    let bone_index = bones.iter().position(|bone| bone.name == collider.bone)?;
    frames.get(bone_index)
}

fn map_collider(
    collider: &SidecarCollider,
    frame: &BoneRestFrame,
    target: ColliderSpace,
) -> SidecarCollider {
    let map_vector = |vector: [f32; 3]| -> [f32; 3] {
        match target {
            ColliderSpace::BoneLocal => frame.to_local(Vector3::from(vector)).into(),
            ColliderSpace::Model => frame.to_model(Vector3::from(vector)).into(),
        }
    };
    let radius = match target {
        ColliderSpace::BoneLocal => frame.length_to_local(collider.radius),
        ColliderSpace::Model => frame.length_to_model(collider.radius),
    };
    SidecarCollider {
        offset: map_vector(collider.offset),
        tail: collider.tail.map(map_vector),
        radius,
        ..collider.clone()
    }
}

#[cfg(test)]
mod tests {
    use cgmath::Matrix4;

    use super::*;
    use crate::vrchat::sidecar_spring::{SidecarColliderOrigin, SidecarColliderShape};

    fn bone(name: &str) -> BoneInput {
        BoneInput {
            name: name.to_string(),
            parent: None,
            rest_position: [0.0; 3],
        }
    }

    fn capsule(name: &str, bone: &str) -> SidecarCollider {
        SidecarCollider {
            name: name.to_string(),
            bone: bone.to_string(),
            offset: [0.0, 0.1, 0.0],
            shape: SidecarColliderShape::Capsule,
            radius: 0.05,
            tail: Some([0.0, 0.3, 0.0]),
            origin: SidecarColliderOrigin::Authored,
        }
    }

    #[test]
    fn names_order_and_unknown_bones_survive_a_round_trip() {
        let bones = [bone("Head")];
        let frames = [
            BoneRestFrame::from_global_transform(&Matrix4::from_scale(200.0), 0.01)
                .expect("regular frame"),
        ];
        let spring = SidecarSpring {
            colliders: vec![capsule("head_guard", "Head"), capsule("hat", "Hat")],
            ..Default::default()
        };

        let local = map_sidecar_colliders(&spring, &bones, &frames, ColliderSpace::BoneLocal);
        let model = map_sidecar_colliders(&local, &bones, &frames, ColliderSpace::Model);

        assert_eq!(local.colliders[0].name, "head_guard");
        assert!((local.colliders[0].radius - 0.025).abs() < 1e-6);
        assert_eq!(local.colliders[1], spring.colliders[1]);
        assert_eq!(model.colliders.len(), 2);
        assert_eq!(model.colliders[0].name, "head_guard");
        assert!((model.colliders[0].radius - 0.05).abs() < 1e-6);
        assert_eq!(model.colliders[1], spring.colliders[1]);
    }
}
