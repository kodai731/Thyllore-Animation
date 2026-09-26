use cgmath::Matrix4;

use crate::animation::SkeletonId;
use crate::ecs::resource::gizmo::BoneGizmoData;
use crate::ecs::resource::AnimationType;
use crate::ecs::transform_propagation_system;
use crate::ecs::FrameContext;

pub fn run_transform_propagate_phase(
    ctx: &mut FrameContext,
    bone_transforms: Option<(SkeletonId, Vec<Matrix4<f32>>, AnimationType)>,
) {
    transform_propagation_system(ctx.world);

    if let Some((skel_id, transforms, anim_type)) = bone_transforms {
        if ctx.world.contains_resource::<BoneGizmoData>() {
            let entity_transform = find_skin_entity_transform(ctx.world);
            let final_transforms = apply_entity_transform(&transforms, &entity_transform);

            log!(
                "BoneGizmo: type={:?}, bones={}, head_pos=[{:.3},{:.3},{:.3}]",
                anim_type,
                final_transforms.len(),
                final_transforms.first().map_or(0.0, |t| t[3][0]),
                final_transforms.first().map_or(0.0, |t| t[3][1]),
                final_transforms.first().map_or(0.0, |t| t[3][2]),
            );

            let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
            bone_gizmo.cached_skeleton_id = Some(skel_id);
            bone_gizmo.cached_animation_type = anim_type;
            bone_gizmo.cached_global_transforms = final_transforms;
        }
    }
}

fn find_skin_entity_transform(world: &crate::ecs::World) -> Matrix4<f32> {
    use crate::ecs::world::{Animator, GlobalTransform};
    use cgmath::SquareMatrix;

    world
        .iter_components::<Animator>()
        .next()
        .and_then(|(entity, _)| {
            world
                .get_component::<GlobalTransform>(entity)
                .map(|gt| gt.0)
        })
        .unwrap_or_else(Matrix4::identity)
}

fn apply_entity_transform(
    bone_transforms: &[Matrix4<f32>],
    entity_transform: &Matrix4<f32>,
) -> Vec<Matrix4<f32>> {
    bone_transforms
        .iter()
        .map(|bt| entity_transform * bt)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::component::{motion_path_position, MotionPath};
    use crate::ecs::resource::FrameClock;
    use crate::ecs::sync_motion_paths;
    use crate::ecs::world::{Animator, GlobalTransform, Transform};
    use cgmath::{SquareMatrix, Vector3};

    fn create_world_with_animated_entity(translation: Vector3<f32>) -> crate::ecs::World {
        let mut world = crate::ecs::World::new();

        let mut transform = Transform::default();
        transform.translation = translation;
        let global_matrix = Matrix4::from_translation(translation);

        let parent = world
            .entity()
            .with_name("test_model")
            .with_transform(transform)
            .with_visible(true)
            .with_animator(Animator::new())
            .build();
        world.insert_component(parent, GlobalTransform(global_matrix));

        world
            .entity()
            .with_name("test_mesh")
            .with_global_transform()
            .with_visible(true)
            .with_parent(parent)
            .with_mesh(1, 0)
            .build();

        world
    }

    #[test]
    fn find_skin_entity_transform_returns_entity_transform_for_animated_mesh() {
        let offset = Vector3::new(5.0, 3.0, -2.0);
        let world = create_world_with_animated_entity(offset);

        let result = find_skin_entity_transform(&world);

        let expected = Matrix4::from_translation(offset);
        assert_ne!(
            result,
            Matrix4::identity(),
            "BUG: find_skin_entity_transform returns identity even though animated mesh entity exists with non-identity GlobalTransform"
        );
        assert_eq!(
            result, expected,
            "find_skin_entity_transform should return the animated mesh entity's GlobalTransform"
        );
    }

    #[test]
    fn apply_entity_transform_includes_entity_offset() {
        let offset = Vector3::new(10.0, 0.0, 0.0);
        let world = create_world_with_animated_entity(offset);

        let bone_transforms = vec![
            Matrix4::identity(),
            Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0)),
        ];

        let entity_transform = find_skin_entity_transform(&world);
        let result = apply_entity_transform(&bone_transforms, &entity_transform);

        let expected_bone0 = Matrix4::from_translation(offset);
        let expected_bone1 = Matrix4::from_translation(Vector3::new(10.0, 1.0, 0.0));

        assert_ne!(
            result[0],
            Matrix4::identity(),
            "BUG: bone[0] at origin should be offset by entity transform (10,0,0), but got identity"
        );
        assert_eq!(
            result[0], expected_bone0,
            "bone[0] should be at entity position"
        );
        assert_eq!(
            result[1], expected_bone1,
            "bone[1] should be offset by entity position"
        );
    }

    #[test]
    fn motion_path_position_reaches_global_transform_in_the_same_frame() {
        let mut world = crate::ecs::World::new();
        world.insert_resource(FrameClock::fixed(FrameClock::BATCH_DELTA_SECONDS));
        world.resource_mut::<FrameClock>().frame = 15;

        let path = MotionPath {
            center: Vector3::new(1.0, 2.0, 3.0),
            radius: 5.0,
            angular_speed: 1.0,
            phase_offset: 0.0,
            enabled: true,
        };
        let entity = world.spawn();
        world.insert_component(entity, path.clone());
        world.insert_component(entity, GlobalTransform(Matrix4::identity()));

        sync_motion_paths(&mut world);
        transform_propagation_system(&mut world);

        let frame_time = world.resource::<FrameClock>().fixed_time_seconds().unwrap();
        let expected_position = motion_path_position(&path, frame_time);
        let global = world.get_component::<GlobalTransform>(entity).unwrap().0;
        assert_eq!(global.w.truncate(), expected_position);
    }
}
