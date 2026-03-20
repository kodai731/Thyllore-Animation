use thyllore_animation::ecs::registry::ComponentRegistry;
use thyllore_animation::ecs::storage::{Components, SparseSet};
use thyllore_animation::ecs::world::{
    Entity, Name, Resources, Transform, Visibility, Visible, World,
};
use thyllore_animation::render::{BufferHandle, IndexBufferHandle, VertexBufferHandle};

mod release_build_tests {
    use thyllore_animation::ecs::resource::GridMeshData;
    use thyllore_animation::ecs::systems::create_grid_mesh;
    use thyllore_animation::ecs::world::World;
    use thyllore_animation::loader::ModelLoadResult;

    #[allow(deprecated)]
    #[test]
    fn test_grid_mesh_xz_toggle_in_release() {
        let (mesh, xz_only_index_count) = create_grid_mesh();

        let grid = GridMeshData {
            mesh,
            xz_only_index_count,
            show_y_axis_grid: false,
            ..Default::default()
        };

        assert_eq!(
            grid.xz_only_index_count, xz_only_index_count,
            "XZ-only count should match"
        );
        assert!(
            grid.xz_only_index_count < grid.mesh.indices.len() as u32,
            "XZ-only count should be less than total"
        );
    }

    #[test]
    fn test_gltf_model_load_stickman() {
        let path = "assets/models/stickman/stickman.glb";
        if !std::path::Path::new(path).exists() {
            println!("Skipping: {} not found", path);
            return;
        }

        let gltf_result = unsafe { thyllore_animation::loader::gltf::load_gltf_file(path) };
        assert!(gltf_result.is_ok(), "glTF load should succeed");

        let result = ModelLoadResult::from_gltf(gltf_result.unwrap());
        assert!(!result.meshes.is_empty(), "Should have at least one mesh");
        assert!(!result.nodes.is_empty(), "Should have nodes");
    }

    #[test]
    fn test_gltf_model_load_phoenix_bird() {
        let path = "assets/models/phoenix-bird/glb/phoenixBird.glb";
        if !std::path::Path::new(path).exists() {
            println!("Skipping: {} not found", path);
            return;
        }

        let gltf_result = unsafe { thyllore_animation::loader::gltf::load_gltf_file(path) };
        assert!(gltf_result.is_ok(), "glTF load should succeed");

        let result = ModelLoadResult::from_gltf(gltf_result.unwrap());
        assert!(!result.meshes.is_empty(), "Should have at least one mesh");
        assert!(!result.skeletons.is_empty(), "Should have a skeleton");
        assert!(result.has_skinned_meshes, "Should have skinned meshes");
    }

    #[test]
    fn test_ecs_world_resource_roundtrip_in_release() {
        let mut world = World::new();

        let (mesh, xz_only_index_count) = {
            #[allow(deprecated)]
            let (m, c) = create_grid_mesh();
            (m, c)
        };

        world.insert_resource(GridMeshData {
            mesh,
            xz_only_index_count,
            show_y_axis_grid: true,
            ..Default::default()
        });

        {
            let grid = world.get_resource::<GridMeshData>().unwrap();
            assert!(grid.show_y_axis_grid);
            assert!(grid.xz_only_index_count > 0);
        }

        {
            let mut grid = world.get_resource_mut::<GridMeshData>().unwrap();
            grid.show_y_axis_grid = false;
        }

        {
            let grid = world.get_resource::<GridMeshData>().unwrap();
            assert!(!grid.show_y_axis_grid);
        }
    }

    use thyllore_animation::vulkanr;

    unsafe fn create_headless_test_device(
    ) -> Option<(vulkanr::Entry, vulkanr::Instance, vulkanr::RRDevice)> {
        let loader = vulkanr::LibloadingLoader::new(vulkanr::LIBRARY).ok()?;
        let entry = vulkanr::Entry::new(loader).ok()?;
        let instance = vulkanr::create_headless_instance(&entry).ok()?;

        match vulkanr::RRDevice::new_headless(
            &entry,
            &instance,
            vulkanr::HEADLESS_DEVICE_EXTENSIONS,
            if cfg!(debug_assertions) {
                vulkanr::ValidationMode::Enabled
            } else {
                vulkanr::ValidationMode::Disabled
            },
            vulkanr::VALIDATION_LAYER,
            vulkanr::Version::new(1, 3, 216),
        ) {
            Ok(device) => Some((entry, instance, device)),
            Err(_) => {
                vulkanr::destroy_headless_instance(&instance);
                None
            }
        }
    }

    #[test]
    fn test_vulkan_headless_device_creation() {
        let result = unsafe { create_headless_test_device() };
        let Some((_entry, instance, device)) = result else {
            println!("Skipping: Vulkan headless device not available");
            return;
        };

        assert!(device.has_graphics_queue());
        assert!(device.min_uniform_buffer_offset_alignment > 0);

        unsafe { vulkanr::destroy_headless_device(&device, &instance) };
    }

    #[test]
    fn test_vulkan_headless_physical_device_properties() {
        let result = unsafe { create_headless_test_device() };
        let Some((_entry, instance, device)) = result else {
            println!("Skipping: Vulkan headless device not available");
            return;
        };

        let api_version = unsafe { device.query_physical_device_api_version(&instance) };
        assert!(api_version > 0);
        assert!(device.msaa_samples.bits() > 0);

        unsafe { vulkanr::destroy_headless_device(&device, &instance) };
    }

    #[test]
    fn test_vulkan_headless_queue_operations() {
        let result = unsafe { create_headless_test_device() };
        let Some((_entry, instance, device)) = result else {
            println!("Skipping: Vulkan headless device not available");
            return;
        };

        assert!(device.has_graphics_queue());
        assert!(!device.has_present_queue());

        unsafe {
            device.wait_graphics_queue_idle().unwrap();
            vulkanr::destroy_headless_device(&device, &instance);
        }
    }
}

mod sparse_set_tests {
    use super::*;

    #[test]
    fn test_sparse_set_new() {
        let set: SparseSet<i32> = SparseSet::new();
        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn test_sparse_set_insert_and_get() {
        let mut set = SparseSet::new();
        let entity: Entity = 1;

        set.insert(entity, 42);

        assert_eq!(set.get(entity), Some(&42));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_sparse_set_insert_multiple() {
        let mut set = SparseSet::new();

        set.insert(1, "first");
        set.insert(2, "second");
        set.insert(3, "third");

        assert_eq!(set.get(1), Some(&"first"));
        assert_eq!(set.get(2), Some(&"second"));
        assert_eq!(set.get(3), Some(&"third"));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_sparse_set_update_existing() {
        let mut set = SparseSet::new();
        let entity: Entity = 1;

        set.insert(entity, 10);
        set.insert(entity, 20);

        assert_eq!(set.get(entity), Some(&20));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_sparse_set_remove() {
        let mut set = SparseSet::new();

        set.insert(1, 100);
        set.insert(2, 200);

        set.remove(1);

        assert_eq!(set.get(1), None);
        assert_eq!(set.get(2), Some(&200));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_sparse_set_contains() {
        let mut set = SparseSet::new();

        set.insert(5, "value");

        assert!(set.contains(5));
        assert!(!set.contains(6));
    }

    #[test]
    fn test_sparse_set_get_nonexistent() {
        let set: SparseSet<i32> = SparseSet::new();

        assert_eq!(set.get(999), None);
    }

    #[test]
    fn test_sparse_set_large_entity_id() {
        let mut set = SparseSet::new();
        let large_entity: Entity = 10000;

        set.insert(large_entity, "large");

        assert_eq!(set.get(large_entity), Some(&"large"));
    }

    #[test]
    fn test_sparse_set_iteration() {
        let mut set = SparseSet::new();

        set.insert(1, 10);
        set.insert(3, 30);
        set.insert(5, 50);

        let collected: Vec<_> = set.iter().collect();
        assert_eq!(collected.len(), 3);

        let values: Vec<_> = collected.iter().map(|(_, v)| **v).collect();
        assert!(values.contains(&10));
        assert!(values.contains(&30));
        assert!(values.contains(&50));
    }

    #[test]
    fn test_sparse_set_clear() {
        let mut set = SparseSet::new();

        set.insert(1, 1);
        set.insert(2, 2);
        set.clear();

        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_sparse_set_entities() {
        let mut set = SparseSet::new();

        set.insert(10, "a");
        set.insert(20, "b");
        set.insert(30, "c");

        let entities = set.entities();
        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&10));
        assert!(entities.contains(&20));
        assert!(entities.contains(&30));
    }
}

mod resources_tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct TestResource {
        value: i32,
    }

    #[derive(Debug, PartialEq)]
    struct AnotherResource {
        name: String,
    }

    #[test]
    fn test_resources_insert_and_get() {
        let mut resources = Resources::new();

        resources.insert(TestResource { value: 42 });

        let res = resources.get::<TestResource>();
        assert!(res.is_some());
        assert_eq!(res.unwrap().value, 42);
    }

    #[test]
    fn test_resources_get_mut() {
        let mut resources = Resources::new();

        resources.insert(TestResource { value: 0 });

        {
            let mut res = resources.get_mut::<TestResource>().unwrap();
            res.value = 100;
        }

        let res = resources.get::<TestResource>().unwrap();
        assert_eq!(res.value, 100);
    }

    #[test]
    fn test_resources_multiple_types() {
        let mut resources = Resources::new();

        resources.insert(TestResource { value: 1 });
        resources.insert(AnotherResource {
            name: "test".to_string(),
        });

        assert_eq!(resources.get::<TestResource>().unwrap().value, 1);
        assert_eq!(
            resources.get::<AnotherResource>().unwrap().name,
            "test".to_string()
        );
    }

    #[test]
    fn test_resources_contains() {
        let mut resources = Resources::new();

        assert!(!resources.contains::<TestResource>());

        resources.insert(TestResource { value: 0 });

        assert!(resources.contains::<TestResource>());
    }

    #[test]
    fn test_resources_remove() {
        let mut resources = Resources::new();

        resources.insert(TestResource { value: 50 });
        let removed = resources.remove::<TestResource>();

        assert!(removed.is_some());
        assert_eq!(removed.unwrap().value, 50);
        assert!(!resources.contains::<TestResource>());
    }

    #[test]
    fn test_resources_get_nonexistent() {
        let resources = Resources::new();

        assert!(resources.get::<TestResource>().is_none());
    }

    #[test]
    fn test_resources_overwrite() {
        let mut resources = Resources::new();

        resources.insert(TestResource { value: 1 });
        resources.insert(TestResource { value: 2 });

        assert_eq!(resources.get::<TestResource>().unwrap().value, 2);
    }
}

mod component_registry_tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Debug)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ComponentRegistry::new();

        let info = registry.register::<Position>();

        assert!(info.size() >= 0);
        assert!(registry.is_registered::<Position>());
    }

    #[test]
    fn test_registry_multiple_types() {
        let mut registry = ComponentRegistry::new();

        registry.register::<Position>();
        registry.register::<Velocity>();

        assert!(registry.is_registered::<Position>());
        assert!(registry.is_registered::<Velocity>());
        assert_eq!(registry.component_count(), 2);
    }

    #[test]
    fn test_registry_not_registered() {
        let registry = ComponentRegistry::new();

        assert!(!registry.is_registered::<Position>());
    }

    #[test]
    fn test_registry_get_info() {
        let mut registry = ComponentRegistry::new();

        registry.register::<Position>();

        let info = registry.get_info::<Position>();
        assert!(info.is_some());
    }

    #[test]
    fn test_registry_double_register() {
        let mut registry = ComponentRegistry::new();

        registry.register::<Position>();
        registry.register::<Position>();

        assert_eq!(registry.component_count(), 1);
    }
}

mod world_tests {
    use super::*;
    use cgmath::Vector3;

    #[test]
    fn test_world_new() {
        let world = World::new();

        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_world_spawn() {
        let mut world = World::new();

        let e1 = world.spawn();
        let e2 = world.spawn();

        assert_ne!(e1, e2);
    }

    #[test]
    fn test_world_insert_and_get_component() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert_component(entity, Name("TestEntity".to_string()));

        let name = world.get_component::<Name>(entity);
        assert!(name.is_some());
        assert_eq!(name.unwrap().0, "TestEntity");
    }

    #[test]
    fn test_world_multiple_components() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert_component(entity, Name("Entity1".to_string()));
        world.insert_component(entity, Transform::default());
        world.insert_component(entity, Visible(Visibility::Shown));

        assert!(world.has_component::<Name>(entity));
        assert!(world.has_component::<Transform>(entity));
        assert!(world.has_component::<Visible>(entity));
    }

    #[test]
    fn test_world_get_component_mut() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert_component(
            entity,
            Transform {
                translation: Vector3::new(0.0, 0.0, 0.0),
                ..Transform::default()
            },
        );

        {
            let transform = world.get_component_mut::<Transform>(entity).unwrap();
            transform.translation = Vector3::new(10.0, 20.0, 30.0);
        }

        let transform = world.get_component::<Transform>(entity).unwrap();
        assert_eq!(transform.translation.x, 10.0);
        assert_eq!(transform.translation.y, 20.0);
        assert_eq!(transform.translation.z, 30.0);
    }

    #[test]
    fn test_world_remove_component() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert_component(entity, Name("ToRemove".to_string()));
        assert!(world.has_component::<Name>(entity));

        world.remove_component::<Name>(entity);
        assert!(!world.has_component::<Name>(entity));
    }

    #[test]
    fn test_world_component_entities() {
        let mut world = World::new();

        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        world.insert_component(e1, Name("A".to_string()));
        world.insert_component(e2, Name("B".to_string()));

        let entities = world.component_entities::<Name>();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
        assert!(!entities.contains(&e3));
    }

    #[test]
    fn test_world_resources() {
        #[derive(Debug)]
        struct GameTime(f32);

        let mut world = World::new();

        world.insert_resource(GameTime(0.0));

        assert!(world.contains_resource::<GameTime>());

        {
            let time = world.resource::<GameTime>();
            assert_eq!(time.0, 0.0);
        }

        {
            let mut time = world.resource_mut::<GameTime>();
            time.0 = 1.5;
        }

        let time = world.resource::<GameTime>();
        assert_eq!(time.0, 1.5);
    }

    #[test]
    fn test_world_iter_components() {
        let mut world = World::new();

        let e1 = world.spawn();
        let e2 = world.spawn();

        world.insert_component(e1, Name("First".to_string()));
        world.insert_component(e2, Name("Second".to_string()));

        let names: Vec<_> = world.iter_components::<Name>().collect();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_world_entity_count() {
        let mut world = World::new();

        world.entity().with_transform(Transform::default()).build();

        world.entity().with_transform(Transform::default()).build();

        assert_eq!(world.entity_count(), 2);
    }
}

mod buffer_handle_tests {
    use super::*;

    #[test]
    fn test_buffer_handle_new() {
        let handle = BufferHandle::new(5);

        assert_eq!(handle.index(), 5);
        assert!(handle.is_valid());
    }

    #[test]
    fn test_buffer_handle_invalid() {
        let handle = BufferHandle::INVALID;

        assert!(!handle.is_valid());
        assert_eq!(handle.0, u32::MAX);
    }

    #[test]
    fn test_buffer_handle_default() {
        let handle = BufferHandle::default();

        assert_eq!(handle.0, u32::MAX);
        assert!(!handle.is_valid());
    }

    #[test]
    fn test_buffer_handle_equality() {
        let h1 = BufferHandle::new(10);
        let h2 = BufferHandle::new(10);
        let h3 = BufferHandle::new(20);

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_vertex_buffer_handle() {
        let handle = VertexBufferHandle::new(3);

        assert_eq!(handle.index(), 3);
        assert!(handle.is_valid());
    }

    #[test]
    fn test_vertex_buffer_handle_invalid() {
        let handle = VertexBufferHandle::INVALID;

        assert!(!handle.is_valid());
    }

    #[test]
    fn test_index_buffer_handle() {
        let handle = IndexBufferHandle::new(7);

        assert_eq!(handle.index(), 7);
        assert!(handle.is_valid());
    }

    #[test]
    fn test_index_buffer_handle_invalid() {
        let handle = IndexBufferHandle::INVALID;

        assert!(!handle.is_valid());
    }

    #[test]
    fn test_handle_as_hash_key() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        let handle = VertexBufferHandle::new(1);

        map.insert(handle, "test_value");

        assert_eq!(map.get(&handle), Some(&"test_value"));
    }
}

mod components_tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Health(pub i32);

    #[derive(Clone, Debug, PartialEq)]
    struct Damage(pub i32);

    #[test]
    fn test_components_register_and_insert() {
        let mut components = Components::new();
        components.register::<Health>();

        let entity: Entity = 1;
        components.insert(entity, Health(100));

        assert!(components.contains::<Health>(entity));
        assert_eq!(components.get::<Health>(entity), Some(&Health(100)));
    }

    #[test]
    fn test_components_multiple_types() {
        let mut components = Components::new();
        components.register::<Health>();
        components.register::<Damage>();

        let entity: Entity = 1;
        components.insert(entity, Health(100));
        components.insert(entity, Damage(25));

        assert_eq!(components.get::<Health>(entity), Some(&Health(100)));
        assert_eq!(components.get::<Damage>(entity), Some(&Damage(25)));
    }

    #[test]
    fn test_components_remove() {
        let mut components = Components::new();
        components.register::<Health>();

        let entity: Entity = 1;
        components.insert(entity, Health(50));
        components.remove::<Health>(entity);

        assert!(!components.contains::<Health>(entity));
    }

    #[test]
    fn test_components_entities() {
        let mut components = Components::new();
        components.register::<Health>();

        components.insert(1, Health(100));
        components.insert(3, Health(200));
        components.insert(5, Health(300));

        let entities = components.entities::<Health>();
        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&1));
        assert!(entities.contains(&3));
        assert!(entities.contains(&5));
    }

    #[test]
    fn test_components_get_mut() {
        let mut components = Components::new();
        components.register::<Health>();

        let entity: Entity = 1;
        components.insert(entity, Health(100));

        if let Some(health) = components.get_mut::<Health>(entity) {
            health.0 -= 30;
        }

        assert_eq!(components.get::<Health>(entity), Some(&Health(70)));
    }
}

mod constraint_solver_tests {
    use cgmath::{assert_relative_eq, InnerSpace, Quaternion, Vector3};
    use thyllore_animation::animation::{
        BoneLocalPose, ConstraintType, IkConstraintData, PositionConstraintData,
        RotationConstraintData, ScaleConstraintData, Skeleton, SkeletonPose, PRIORITY_IK,
        PRIORITY_POSITION, PRIORITY_ROTATION, PRIORITY_SCALE,
    };
    use thyllore_animation::ecs::component::ConstraintSet;
    use thyllore_animation::ecs::systems::apply_constraints;
    use thyllore_animation::ecs::systems::constraint_set_add;

    fn create_chain_skeleton(bone_count: u32, bone_length: f32) -> Skeleton {
        let mut skeleton = Skeleton::new("test_chain");
        for i in 0..bone_count {
            let parent = if i == 0 { None } else { Some(i - 1) };
            let bone_id = skeleton.add_bone(&format!("bone_{}", i), parent);

            let bone = skeleton.get_bone_mut(bone_id).unwrap();
            bone.local_transform = cgmath::Matrix4::from_translation(Vector3::new(
                0.0,
                if i == 0 { 0.0 } else { bone_length },
                0.0,
            ));
        }
        skeleton
    }

    fn create_rest_pose(skeleton: &Skeleton) -> SkeletonPose {
        use thyllore_animation::animation::decompose_transform;
        let bone_poses = skeleton
            .bones
            .iter()
            .map(|bone| {
                let (t, r, s) = decompose_transform(&bone.local_transform);
                BoneLocalPose {
                    translation: t,
                    rotation: r,
                    scale: s,
                }
            })
            .collect();

        SkeletonPose {
            skeleton_id: skeleton.id,
            bone_poses,
        }
    }

    #[test]
    fn test_empty_constraints_noop() {
        let skeleton = create_chain_skeleton(3, 1.0);
        let mut pose = create_rest_pose(&skeleton);
        let original = pose.clone();
        let constraint_set = ConstraintSet::new();

        apply_constraints(&constraint_set, &skeleton, &mut pose);

        for (i, bp) in pose.bone_poses.iter().enumerate() {
            assert_relative_eq!(
                bp.translation.x,
                original.bone_poses[i].translation.x,
                epsilon = 0.001
            );
            assert_relative_eq!(
                bp.translation.y,
                original.bone_poses[i].translation.y,
                epsilon = 0.001
            );
            assert_relative_eq!(
                bp.translation.z,
                original.bone_poses[i].translation.z,
                epsilon = 0.001
            );
        }
    }

    #[test]
    fn test_disabled_constraint_skipped() {
        let skeleton = create_chain_skeleton(3, 1.0);
        let mut pose = create_rest_pose(&skeleton);
        let original = pose.clone();

        let mut constraint_set = ConstraintSet::new();
        constraint_set_add(
            &mut constraint_set,
            ConstraintType::Position(PositionConstraintData {
                constrained_bone: 1,
                target_bone: 2,
                enabled: false,
                weight: 1.0,
                ..Default::default()
            }),
            PRIORITY_POSITION,
        );

        apply_constraints(&constraint_set, &skeleton, &mut pose);

        assert_relative_eq!(
            pose.bone_poses[1].translation.y,
            original.bone_poses[1].translation.y,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_weight_zero_noop() {
        let skeleton = create_chain_skeleton(3, 1.0);
        let mut pose = create_rest_pose(&skeleton);
        let original = pose.clone();

        let mut constraint_set = ConstraintSet::new();
        constraint_set_add(
            &mut constraint_set,
            ConstraintType::Position(PositionConstraintData {
                constrained_bone: 1,
                target_bone: 2,
                enabled: true,
                weight: 0.0,
                ..Default::default()
            }),
            PRIORITY_POSITION,
        );

        apply_constraints(&constraint_set, &skeleton, &mut pose);

        assert_relative_eq!(
            pose.bone_poses[1].translation.y,
            original.bone_poses[1].translation.y,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_position_constraint_basic() {
        let skeleton = create_chain_skeleton(3, 1.0);
        let mut pose = create_rest_pose(&skeleton);

        pose.bone_poses[2].translation = Vector3::new(5.0, 5.0, 5.0);

        let mut constraint_set = ConstraintSet::new();
        constraint_set_add(
            &mut constraint_set,
            ConstraintType::Position(PositionConstraintData {
                constrained_bone: 1,
                target_bone: 2,
                offset: Vector3::new(0.0, 0.0, 0.0),
                affect_axes: [true, true, true],
                enabled: true,
                weight: 1.0,
            }),
            PRIORITY_POSITION,
        );

        let original_y = pose.bone_poses[1].translation.y;
        apply_constraints(&constraint_set, &skeleton, &mut pose);

        let moved =
            (pose.bone_poses[1].translation - Vector3::new(0.0, original_y, 0.0)).magnitude();
        assert!(moved > 0.01, "bone should have moved");
    }

    #[test]
    fn test_rotation_constraint_basic() {
        let skeleton = create_chain_skeleton(3, 1.0);
        let mut pose = create_rest_pose(&skeleton);

        let target_rot = Quaternion::new(0.7071, 0.0, 0.7071, 0.0);
        pose.bone_poses[2].rotation = target_rot;

        let mut constraint_set = ConstraintSet::new();
        constraint_set_add(
            &mut constraint_set,
            ConstraintType::Rotation(RotationConstraintData {
                constrained_bone: 1,
                target_bone: 2,
                offset: Quaternion::new(1.0, 0.0, 0.0, 0.0),
                affect_axes: [true, true, true],
                enabled: true,
                weight: 1.0,
            }),
            PRIORITY_ROTATION,
        );

        let original_rot = pose.bone_poses[1].rotation;
        apply_constraints(&constraint_set, &skeleton, &mut pose);

        let dot = pose.bone_poses[1].rotation.s * original_rot.s
            + pose.bone_poses[1].rotation.v.x * original_rot.v.x
            + pose.bone_poses[1].rotation.v.y * original_rot.v.y
            + pose.bone_poses[1].rotation.v.z * original_rot.v.z;
        assert!(dot.abs() < 0.999, "rotation should have changed");
    }

    #[test]
    fn test_scale_constraint_basic() {
        let skeleton = create_chain_skeleton(3, 1.0);
        let mut pose = create_rest_pose(&skeleton);

        pose.bone_poses[2].scale = Vector3::new(2.0, 2.0, 2.0);

        let mut constraint_set = ConstraintSet::new();
        constraint_set_add(
            &mut constraint_set,
            ConstraintType::Scale(ScaleConstraintData {
                constrained_bone: 1,
                target_bone: 2,
                offset: Vector3::new(1.0, 1.0, 1.0),
                affect_axes: [true, true, true],
                enabled: true,
                weight: 1.0,
            }),
            PRIORITY_SCALE,
        );

        apply_constraints(&constraint_set, &skeleton, &mut pose);

        assert!(
            (pose.bone_poses[1].scale.x - 1.0).abs() > 0.01,
            "scale should have changed from 1.0, got {}",
            pose.bone_poses[1].scale.x
        );
    }

    #[test]
    fn test_ik_two_bone_reachable() {
        let skeleton = create_chain_skeleton(4, 1.0);
        let mut pose = create_rest_pose(&skeleton);

        pose.bone_poses[3].translation = Vector3::new(1.0, 1.0, 0.0);

        let mut constraint_set = ConstraintSet::new();
        constraint_set_add(
            &mut constraint_set,
            ConstraintType::Ik(IkConstraintData {
                chain_length: 2,
                target_bone: 3,
                effector_bone: 2,
                pole_vector: Some(Vector3::new(0.0, 0.0, 1.0)),
                pole_target: None,
                twist: 0.0,
                enabled: true,
                weight: 1.0,
            }),
            PRIORITY_IK,
        );

        apply_constraints(&constraint_set, &skeleton, &mut pose);

        let rot_changed = {
            let rest = create_rest_pose(&skeleton);
            let dot = pose.bone_poses[0].rotation.s * rest.bone_poses[0].rotation.s
                + pose.bone_poses[0].rotation.v.x * rest.bone_poses[0].rotation.v.x
                + pose.bone_poses[0].rotation.v.y * rest.bone_poses[0].rotation.v.y
                + pose.bone_poses[0].rotation.v.z * rest.bone_poses[0].rotation.v.z;
            dot.abs() < 0.9999
        };
        assert!(rot_changed, "IK should have modified bone rotations");
    }

    #[test]
    fn test_ik_two_bone_unreachable() {
        let skeleton = create_chain_skeleton(4, 1.0);
        let mut pose = create_rest_pose(&skeleton);

        pose.bone_poses[3].translation = Vector3::new(0.0, 100.0, 0.0);

        let mut constraint_set = ConstraintSet::new();
        constraint_set_add(
            &mut constraint_set,
            ConstraintType::Ik(IkConstraintData {
                chain_length: 2,
                target_bone: 3,
                effector_bone: 2,
                pole_vector: Some(Vector3::new(0.0, 0.0, 1.0)),
                pole_target: None,
                twist: 0.0,
                enabled: true,
                weight: 1.0,
            }),
            PRIORITY_IK,
        );

        apply_constraints(&constraint_set, &skeleton, &mut pose);

        let rest = create_rest_pose(&skeleton);
        let dot = pose.bone_poses[0].rotation.s * rest.bone_poses[0].rotation.s
            + pose.bone_poses[0].rotation.v.x * rest.bone_poses[0].rotation.v.x
            + pose.bone_poses[0].rotation.v.y * rest.bone_poses[0].rotation.v.y
            + pose.bone_poses[0].rotation.v.z * rest.bone_poses[0].rotation.v.z;
        assert!(
            dot.abs() > 0.9,
            "unreachable target should still produce valid (near-rest) pose"
        );
    }
}

mod transform_tests {
    use super::*;
    use cgmath::{assert_relative_eq, Quaternion, Vector3};

    #[test]
    fn test_transform_default() {
        let transform = Transform::default();

        assert_eq!(transform.translation, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(transform.scale, Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_transform_to_matrix_identity() {
        let transform = Transform::default();
        let matrix = transform.to_matrix();

        for i in 0..4 {
            for j in 0..4 {
                if i == j {
                    assert_relative_eq!(matrix[i][j], 1.0, epsilon = 0.0001);
                } else {
                    assert_relative_eq!(matrix[i][j], 0.0, epsilon = 0.0001);
                }
            }
        }
    }

    #[test]
    fn test_transform_to_matrix_translation() {
        let transform = Transform {
            translation: Vector3::new(10.0, 20.0, 30.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        };
        let matrix = transform.to_matrix();

        assert_relative_eq!(matrix[3][0], 10.0, epsilon = 0.0001);
        assert_relative_eq!(matrix[3][1], 20.0, epsilon = 0.0001);
        assert_relative_eq!(matrix[3][2], 30.0, epsilon = 0.0001);
    }

    #[test]
    fn test_transform_to_matrix_scale() {
        let transform = Transform {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: Vector3::new(2.0, 3.0, 4.0),
        };
        let matrix = transform.to_matrix();

        assert_relative_eq!(matrix[0][0], 2.0, epsilon = 0.0001);
        assert_relative_eq!(matrix[1][1], 3.0, epsilon = 0.0001);
        assert_relative_eq!(matrix[2][2], 4.0, epsilon = 0.0001);
    }
}

mod message_log_tests {
    use thyllore_animation::ecs::resource::{MessageFilter, MessageLog};
    use thyllore_animation::ecs::systems::message_log_systems::{
        message_log_clear_buffer, message_log_filtered_messages,
    };
    use thyllore_animation::logger::message_buffer::{MessageBuffer, MessageLevel};

    fn make_log_with_messages() -> MessageLog {
        let mut log = MessageLog::default();
        log.messages = vec![
            thyllore_animation::logger::message_buffer::Message {
                level: MessageLevel::Info,
                text: "Model loaded".to_string(),
                timestamp: "12:00:00".to_string(),
            },
            thyllore_animation::logger::message_buffer::Message {
                level: MessageLevel::Warning,
                text: "Missing bone: Spine2".to_string(),
                timestamp: "12:00:01".to_string(),
            },
            thyllore_animation::logger::message_buffer::Message {
                level: MessageLevel::Error,
                text: "Failed to load texture".to_string(),
                timestamp: "12:00:02".to_string(),
            },
            thyllore_animation::logger::message_buffer::Message {
                level: MessageLevel::Info,
                text: "Screenshot saved".to_string(),
                timestamp: "12:00:03".to_string(),
            },
        ];
        log.info_count = 2;
        log.warning_count = 1;
        log.error_count = 1;
        log
    }

    #[test]
    fn test_message_buffer_push_populates_and_snapshot_returns_all() {
        let mut buf = MessageBuffer::new();
        buf.push(MessageLevel::Info, "info msg".to_string());
        buf.push(MessageLevel::Warning, "warn msg".to_string());
        buf.push(MessageLevel::Error, "error msg".to_string());

        let snap = buf.snapshot();
        assert_eq!(snap.len(), 3);
        assert_eq!(snap[0].level, MessageLevel::Info);
        assert_eq!(snap[1].level, MessageLevel::Warning);
        assert_eq!(snap[2].level, MessageLevel::Error);
    }

    #[test]
    fn test_message_log_filter_all_returns_every_message() {
        let mut log = make_log_with_messages();
        log.filter = MessageFilter::All;
        assert_eq!(message_log_filtered_messages(&log).len(), 4);
    }

    #[test]
    fn test_message_log_filter_warning_and_error_excludes_info() {
        let mut log = make_log_with_messages();
        log.filter = MessageFilter::WarningAndError;
        let filtered = message_log_filtered_messages(&log);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|m| m.level != MessageLevel::Info));
    }

    #[test]
    fn test_message_log_filter_error_only() {
        let mut log = make_log_with_messages();
        log.filter = MessageFilter::ErrorOnly;
        let filtered = message_log_filtered_messages(&log);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].level, MessageLevel::Error);
        assert_eq!(filtered[0].text, "Failed to load texture");
    }

    #[test]
    fn test_message_log_default_state() {
        let log = MessageLog::default();
        assert!(log.messages.is_empty());
        assert_eq!(log.filter, MessageFilter::All);
        assert!(log.auto_scroll);
        assert_eq!(log.info_count, 0);
        assert_eq!(log.warning_count, 0);
        assert_eq!(log.error_count, 0);
    }

    #[test]
    fn test_message_buffer_ring_buffer_evicts_oldest() {
        let mut buf = MessageBuffer::new();
        // Push more than capacity (256)
        for i in 0..300 {
            buf.push(MessageLevel::Info, format!("msg {}", i));
        }
        let snap = buf.snapshot();
        assert_eq!(snap.len(), 256);
        // Oldest messages (0-43) should have been evicted
        assert_eq!(snap[0].text, "msg 44");
        assert_eq!(snap[255].text, "msg 299");
    }

    #[test]
    fn test_message_buffer_count_by_level() {
        let mut buf = MessageBuffer::new();
        buf.push(MessageLevel::Info, "a".to_string());
        buf.push(MessageLevel::Info, "b".to_string());
        buf.push(MessageLevel::Warning, "c".to_string());
        buf.push(MessageLevel::Error, "d".to_string());
        buf.push(MessageLevel::Error, "e".to_string());
        buf.push(MessageLevel::Error, "f".to_string());

        assert_eq!(buf.count_by_level(MessageLevel::Info), 2);
        assert_eq!(buf.count_by_level(MessageLevel::Warning), 1);
        assert_eq!(buf.count_by_level(MessageLevel::Error), 3);
    }

    #[test]
    fn test_message_buffer_clear_empties_all() {
        let mut buf = MessageBuffer::new();
        buf.push(MessageLevel::Info, "test".to_string());
        buf.push(MessageLevel::Error, "err".to_string());
        buf.clear();

        assert_eq!(buf.snapshot().len(), 0);
        assert_eq!(buf.count_by_level(MessageLevel::Info), 0);
        assert_eq!(buf.count_by_level(MessageLevel::Error), 0);
    }

    #[test]
    fn test_message_log_clear_buffer_resets_counts() {
        let mut log = make_log_with_messages();
        assert_eq!(log.info_count, 2);
        assert_eq!(log.error_count, 1);

        message_log_clear_buffer(&mut log);

        assert!(log.messages.is_empty());
        assert_eq!(log.info_count, 0);
        assert_eq!(log.warning_count, 0);
        assert_eq!(log.error_count, 0);
    }

    #[test]
    fn test_message_timestamp_is_populated() {
        let mut buf = MessageBuffer::new();
        buf.push(MessageLevel::Info, "hello".to_string());
        let snap = buf.snapshot();
        assert!(!snap[0].timestamp.is_empty());
        // Timestamp should match HH:MM:SS format (8 chars)
        assert_eq!(snap[0].timestamp.len(), 8);
    }
}
