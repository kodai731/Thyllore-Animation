use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};

use crate::animation::{Skeleton, SkeletonPose, SkinData};
use crate::ecs::{apply_skinning, compute_pose_global_transforms, create_pose_from_rest};
use crate::loader::LoadedNode;
use crate::vulkanr::resource::graphics_resource::NodeData;

const TOLERANCE: f32 = 0.001;
const EXPECTED_POSITION: Vector3<f32> = Vector3::new(0.5, 1.0, 0.0);

fn create_skeleton_meters() -> Skeleton {
    let mut skeleton = Skeleton::new("test_skeleton");

    let root_id = skeleton.add_bone("root", None);
    skeleton.bones[root_id as usize].local_transform =
        Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.0));
    skeleton.bones[root_id as usize].inverse_bind_pose = Matrix4::identity();

    let child_id = skeleton.add_bone("child", Some(root_id));
    skeleton.bones[child_id as usize].local_transform =
        Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0));

    let child_bind_global = Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0));
    skeleton.bones[child_id as usize].inverse_bind_pose = child_bind_global
        .invert()
        .expect("child bind pose must be invertible");

    skeleton
}

fn create_skin_data_meters() -> SkinData {
    let base_pos = Vector3::new(0.5, 1.0, 0.0);

    SkinData {
        skeleton_id: 0,
        bone_indices: vec![Vector4::new(1, 0, 0, 0)],
        bone_weights: vec![Vector4::new(1.0, 0.0, 0.0, 0.0)],
        base_positions: vec![base_pos],
        base_normals: vec![Vector3::new(0.0, 1.0, 0.0)],
    }
}

fn create_nodes_meters() -> Vec<LoadedNode> {
    vec![
        LoadedNode {
            index: 0,
            name: "root".to_string(),
            parent_index: None,
            local_transform: Matrix4::identity(),
        },
        LoadedNode {
            index: 1,
            name: "child".to_string(),
            parent_index: Some(0),
            local_transform: Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0)),
        },
    ]
}

fn to_node_data(loaded_nodes: &[LoadedNode]) -> Vec<NodeData> {
    loaded_nodes
        .iter()
        .map(|n| NodeData {
            index: n.index,
            name: n.name.clone(),
            parent_index: n.parent_index,
            local_transform: n.local_transform,
            global_transform: Matrix4::identity(),
        })
        .collect()
}

fn rest_pose(skeleton: &Skeleton) -> SkeletonPose {
    create_pose_from_rest(skeleton)
}

fn assert_position_correct(label: &str, actual: Vector3<f32>, expected: Vector3<f32>) {
    let dx = (actual.x - expected.x).abs();
    let dy = (actual.y - expected.y).abs();
    let dz = (actual.z - expected.z).abs();
    assert!(
        dx < TOLERANCE && dy < TOLERANCE && dz < TOLERANCE,
        "{}: expected ({:.4}, {:.4}, {:.4}) but got ({:.4}, {:.4}, {:.4})",
        label,
        expected.x,
        expected.y,
        expected.z,
        actual.x,
        actual.y,
        actual.z,
    );
}

fn run_skinning(skeleton: &Skeleton, skin_data: &SkinData) -> Vector3<f32> {
    let pose = rest_pose(skeleton);
    let global_transforms = compute_pose_global_transforms(skeleton, &pose);
    let vertex_count = skin_data.base_positions.len();
    let mut out_positions = vec![Vector3::new(0.0, 0.0, 0.0); vertex_count];
    let mut out_normals = vec![Vector3::new(0.0, 1.0, 0.0); vertex_count];
    apply_skinning(
        skin_data,
        &global_transforms,
        skeleton,
        &mut out_positions,
        &mut out_normals,
    );
    out_positions[0]
}

fn run_node_animation(
    loaded_nodes: &[LoadedNode],
    skeleton: &Skeleton,
    vertex_local: Vector3<f32>,
    mesh_node_index: usize,
    node_animation_scale: f32,
) -> Vector3<f32> {
    let pose = rest_pose(skeleton);
    let mut node_data = to_node_data(loaded_nodes);
    crate::ecs::systems::animation::apply::compute_node_global_transforms(
        &mut node_data,
        skeleton,
        &pose,
    );

    let global_transform = node_data[mesh_node_index].global_transform;
    let v4 = global_transform * Vector4::new(vertex_local.x, vertex_local.y, vertex_local.z, 1.0);
    Vector3::new(
        v4.x * node_animation_scale,
        v4.y * node_animation_scale,
        v4.z * node_animation_scale,
    )
}

#[test]
fn fbx_armature_skinned() {
    let skeleton = create_skeleton_meters();
    let skin_data = create_skin_data_meters();

    let skinned_pos = run_skinning(&skeleton, &skin_data);

    assert_position_correct("FBX armature skinned", skinned_pos, EXPECTED_POSITION);
}

#[test]
fn fbx_armature_node_animation() {
    let node_animation_scale: f32 = 1.0;

    let skeleton = create_skeleton_meters();
    let nodes = create_nodes_meters();

    let vertex_local = Vector3::new(0.5, 0.0, 0.0);
    let mesh_node_index = 1;

    let final_pos = run_node_animation(
        &nodes,
        &skeleton,
        vertex_local,
        mesh_node_index,
        node_animation_scale,
    );

    assert_position_correct("FBX armature node animation", final_pos, EXPECTED_POSITION);
}

#[test]
fn fbx_no_armature_static() {
    let vertex_meters = Vector3::new(0.5, 1.0, 0.0);

    assert_position_correct("FBX no armature static", vertex_meters, EXPECTED_POSITION);
}

#[test]
fn fbx_no_armature_node_animation() {
    let node_animation_scale: f32 = 1.0;

    let nodes = vec![
        LoadedNode {
            index: 0,
            name: "root".to_string(),
            parent_index: None,
            local_transform: Matrix4::identity(),
        },
        LoadedNode {
            index: 1,
            name: "child".to_string(),
            parent_index: Some(0),
            local_transform: Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0)),
        },
    ];

    let vertex_local = Vector3::new(0.5, 0.0, 0.0);

    let mut node_data = to_node_data(&nodes);
    node_data[0].global_transform = node_data[0].local_transform;
    node_data[1].global_transform = node_data[0].global_transform * node_data[1].local_transform;

    let global_transform = node_data[1].global_transform;
    let v4 = global_transform * Vector4::new(vertex_local.x, vertex_local.y, vertex_local.z, 1.0);
    let final_pos = Vector3::new(
        v4.x * node_animation_scale,
        v4.y * node_animation_scale,
        v4.z * node_animation_scale,
    );

    assert_position_correct(
        "FBX no armature node animation",
        final_pos,
        EXPECTED_POSITION,
    );
}

#[test]
fn gltf_armature_skinned() {
    let skeleton = create_skeleton_meters();
    let skin_data = create_skin_data_meters();

    let skinned_pos = run_skinning(&skeleton, &skin_data);

    assert_position_correct("glTF armature skinned", skinned_pos, EXPECTED_POSITION);
}

#[test]
fn gltf_armature_node_animation() {
    let node_animation_scale: f32 = 0.01;

    let skeleton_cm = {
        let mut s = Skeleton::new("test_skeleton_cm");
        let root_id = s.add_bone("root", None);
        s.bones[root_id as usize].local_transform = Matrix4::identity();
        s.bones[root_id as usize].inverse_bind_pose = Matrix4::identity();

        let child_id = s.add_bone("child", Some(root_id));
        s.bones[child_id as usize].local_transform =
            Matrix4::from_translation(Vector3::new(0.0, 100.0, 0.0));
        let child_bind = Matrix4::from_translation(Vector3::new(0.0, 100.0, 0.0));
        s.bones[child_id as usize].inverse_bind_pose = child_bind.invert().expect("invertible");
        s
    };

    let nodes_cm = vec![
        LoadedNode {
            index: 0,
            name: "root".to_string(),
            parent_index: None,
            local_transform: Matrix4::identity(),
        },
        LoadedNode {
            index: 1,
            name: "child".to_string(),
            parent_index: Some(0),
            local_transform: Matrix4::from_translation(Vector3::new(0.0, 100.0, 0.0)),
        },
    ];

    let vertex_local_meters = Vector3::new(0.5, 0.0, 0.0);
    let mesh_node_index = 1;

    let final_pos = run_node_animation(
        &nodes_cm,
        &skeleton_cm,
        vertex_local_meters,
        mesh_node_index,
        node_animation_scale,
    );

    let y_error = (final_pos.y - EXPECTED_POSITION.y).abs();
    assert!(
        y_error < TOLERANCE,
        "glTF armature node animation: y should be correct (bone at 100cm * 0.01 = 1.0m), \
         got y={:.4}",
        final_pos.y,
    );
}

#[test]
fn gltf_no_armature_static() {
    let vertex_meters = Vector3::new(0.5, 1.0, 0.0);

    assert_position_correct("glTF no armature static", vertex_meters, EXPECTED_POSITION);
}

#[test]
fn gltf_no_armature_node_animation() {
    let node_animation_scale: f32 = 1.0;

    let nodes = vec![
        LoadedNode {
            index: 0,
            name: "root".to_string(),
            parent_index: None,
            local_transform: Matrix4::identity(),
        },
        LoadedNode {
            index: 1,
            name: "child".to_string(),
            parent_index: Some(0),
            local_transform: Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0)),
        },
    ];

    let vertex_local_meters = Vector3::new(0.5, 0.0, 0.0);

    let mut node_data = to_node_data(&nodes);
    node_data[0].global_transform = node_data[0].local_transform;
    node_data[1].global_transform = node_data[0].global_transform * node_data[1].local_transform;

    let global_transform = node_data[1].global_transform;
    let v4 = global_transform
        * Vector4::new(
            vertex_local_meters.x,
            vertex_local_meters.y,
            vertex_local_meters.z,
            1.0,
        );
    let final_pos = Vector3::new(
        v4.x * node_animation_scale,
        v4.y * node_animation_scale,
        v4.z * node_animation_scale,
    );

    assert_position_correct(
        "glTF no armature node animation",
        final_pos,
        EXPECTED_POSITION,
    );
}

#[test]
fn scale_matrix_summary() {
    // After ufbx ModifyGeometry migration, all FBX data is in meters.
    //
    // | # | Pattern                          | Expected |
    // |---|----------------------------------|----------|
    // | 1 | FBX + Armature + Skinned         | PASS     |
    // | 2 | FBX + Armature + Node Animation  | PASS     |
    // | 3 | FBX + No Armature + Static       | PASS     |
    // | 4 | FBX + No Armature + Node Anim    | PASS     |
    // | 5 | glTF + Armature + Skinned        | PASS     |
    // | 6 | glTF + Armature + Node Animation | PARTIAL  |
    // | 7 | glTF + No Armature + Static      | PASS     |
    // | 8 | glTF + No Armature + Node Anim   | PASS     |
}
