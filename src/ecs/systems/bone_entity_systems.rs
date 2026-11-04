use crate::animation::Skeleton;
use crate::asset::AssetStorage;
use crate::ecs::component::{BoneEntity, EditorDisplay, SkeletonBones};
use crate::ecs::world::{Entity, Name, Parent, World};
use crate::hooks::model_load::LoadedModel;

pub fn spawn_bone_entities(world: &mut World, owner: Entity, skeleton: &Skeleton) -> SkeletonBones {
    let mut entities: Vec<Entity> = Vec::with_capacity(skeleton.bones.len());

    for bone in &skeleton.bones {
        let entity = world.spawn();

        world.insert_component(entity, Name(bone.name.clone()));

        world.insert_component(
            entity,
            BoneEntity {
                skeleton_id: skeleton.id,
                bone_id: bone.id,
            },
        );

        world.insert_component(
            entity,
            EditorDisplay {
                hidden_in_hierarchy: true,
                ..Default::default()
            },
        );

        let parent = bone
            .parent_id
            .and_then(|parent_bone_id| entities.get(parent_bone_id as usize).copied())
            .unwrap_or(owner);
        world.insert_component(entity, Parent(parent));
        world.add_child(parent, entity);

        entities.push(entity);
    }

    SkeletonBones {
        skeleton_id: skeleton.id,
        entities,
    }
}

fn bone_entities_attach_loaded(world: &mut World, assets: &AssetStorage, loaded: &LoadedModel) {
    let Some(loaded_skeleton) = loaded.load_result.skeletons.first() else {
        return;
    };
    let Some(skeleton) = assets.get_skeleton_by_skeleton_id(loaded_skeleton.id) else {
        return;
    };

    let skeleton_bones = spawn_bone_entities(world, loaded.entity, skeleton);
    world.insert_component(loaded.entity, skeleton_bones);
}

crate::model_load_hook!("bone_entities", Rig, bone_entities_attach_loaded);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{Bone, BoneId};
    use cgmath::{Matrix4, SquareMatrix};

    fn make_skeleton(bone_count: usize) -> Skeleton {
        let mut skeleton = Skeleton::new("test");
        skeleton.id = 0;

        for i in 0..bone_count {
            let parent_id = if i == 0 {
                None
            } else {
                Some((i - 1) as BoneId)
            };
            let bone = Bone {
                id: i as BoneId,
                name: format!("bone_{}", i),
                parent_id,
                children: Vec::new(),
                local_transform: Matrix4::identity(),
                inverse_bind_pose: Matrix4::identity(),
                node_index: None,
            };
            skeleton.bones.push(bone);
        }

        skeleton.root_bone_ids = vec![0];
        skeleton
    }

    #[test]
    fn spawn_bone_entities_parents_each_bone_under_its_parent_bone() {
        let mut world = World::new();
        let owner = world.spawn();
        let skeleton = make_skeleton(3);

        let result = spawn_bone_entities(&mut world, owner, &skeleton);

        assert_eq!(result.entities.len(), 3);

        let root_entity = result.entities[0];
        let mid_entity = result.entities[1];
        let tip_entity = result.entities[2];

        let root_parent = world.get_component::<Parent>(root_entity);
        assert!(root_parent.is_some());
        assert_eq!(
            root_parent.unwrap().0,
            owner,
            "root bone's parent should be the owner"
        );

        let mid_parent = world.get_component::<Parent>(mid_entity);
        assert!(mid_parent.is_some());
        assert_eq!(
            mid_parent.unwrap().0,
            root_entity,
            "mid bone's parent should be root's entity"
        );

        let tip_parent = world.get_component::<Parent>(tip_entity);
        assert!(tip_parent.is_some());
        assert_eq!(
            tip_parent.unwrap().0,
            mid_entity,
            "tip bone's parent should be mid's entity"
        );
    }

    #[test]
    fn skeleton_bones_maps_every_bone_id_to_its_entity() {
        let mut world = World::new();
        let owner = world.spawn();
        let skeleton = make_skeleton(3);

        let result = spawn_bone_entities(&mut world, owner, &skeleton);

        for (i, bone) in skeleton.bones.iter().enumerate() {
            let entity = result.entity(bone.id);
            assert!(entity.is_some(), "bone {} should have an entity", bone.id);
            assert_eq!(
                entity.unwrap(),
                result.entities[i],
                "bone {}'s entity should match the table at index {}",
                bone.id,
                i
            );
        }
    }
}
