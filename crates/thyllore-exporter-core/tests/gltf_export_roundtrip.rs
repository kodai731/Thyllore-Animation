#![cfg(test)]

use std::fs;
use std::path::PathBuf;

use gltf::binary::Glb;
use gltf::json::{self, Index};
use thyllore_anim_core::editable::{curve_add_keyframe, PropertyType};
use thyllore_anim_core::Skeleton;

use thyllore_exporter_core::systems::gltf::export_gltf_animation_only;

fn create_test_skeleton() -> Skeleton {
    let mut skeleton = Skeleton::new("test");
    skeleton.add_bone("Hips", None);
    skeleton.add_bone("Spine", Some(0));
    skeleton.add_bone("Head", Some(1));
    skeleton
}

fn create_test_clip() -> thyllore_anim_core::editable::EditableAnimationClip {
    let mut clip =
        thyllore_anim_core::editable::EditableAnimationClip::new(1, "test_anim".to_string());
    let track = clip.add_track(0, "Hips".to_string());
    let curve = track.get_curve_mut(PropertyType::TranslationX);
    curve_add_keyframe(curve, 0.0, 0.0);
    curve_add_keyframe(curve, 1.0, 1.0);
    clip.duration = 1.0;
    clip
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

#[test]
fn test_export_gltf_animation_only_creates_file() {
    let skeleton = create_test_skeleton();
    let clip = create_test_clip();

    let output_path = temp_path("test_export_gltf_animation_only.glb");
    export_gltf_animation_only(&clip, &skeleton, &output_path, &[]).unwrap();

    assert!(
        output_path.exists(),
        "GLB file was not created at the specified path"
    );
    let bytes = fs::read(&output_path).unwrap();
    assert!(!bytes.is_empty(), "GLB file is empty");
    drop(bytes);
    fs::remove_file(output_path).ok();
}

#[test]
fn test_export_gltf_animation_only_node_count_matches_bone_count() {
    let skeleton = create_test_skeleton();
    let clip = create_test_clip();

    let output_path = temp_path("test_node_count.glb");
    export_gltf_animation_only(&clip, &skeleton, &output_path, &[]).unwrap();

    let bytes = fs::read(&output_path).unwrap();
    let glb = Glb::from_slice(&bytes).unwrap();
    let root: json::Root = json::Root::from_slice(&glb.json).unwrap();

    assert_eq!(
        root.nodes.len(),
        skeleton.bones.len(),
        "Number of nodes should equal number of bones in the skeleton"
    );
    fs::remove_file(output_path).ok();
}

#[test]
fn test_export_gltf_animation_only_has_animation_channels() {
    let skeleton = create_test_skeleton();
    let clip = create_test_clip();

    let output_path = temp_path("test_channels.glb");
    export_gltf_animation_only(&clip, &skeleton, &output_path, &[]).unwrap();

    let bytes = fs::read(&output_path).unwrap();
    let glb = Glb::from_slice(&bytes).unwrap();
    let root: json::Root = json::Root::from_slice(&glb.json).unwrap();

    assert!(!root.animations.is_empty(), "No animations found in GLB");
    let anim = &root.animations[0];
    assert!(!anim.channels.is_empty(), "No animation channels found");
    fs::remove_file(output_path).ok();
}

#[test]
fn test_export_gltf_animation_only_node_hierarchy_matches_skeleton() {
    // Create a skeleton with parent-child relationships:
    // Hips (0) -> Spine (1) -> Head (2)
    let skeleton = create_test_skeleton();
    let clip = create_test_clip();

    let output_path = temp_path("test_hierarchy.glb");
    export_gltf_animation_only(&clip, &skeleton, &output_path, &[]).unwrap();

    let bytes = fs::read(&output_path).unwrap();
    let glb = Glb::from_slice(&bytes).unwrap();
    let root: json::Root = json::Root::from_slice(&glb.json).unwrap();

    // Verify that the parent node's children indices match the skeleton's parent-child relationships
    // Hips (bone 0) should have Spine (bone 1) as child
    let hips_node = &root.nodes[0];
    assert!(
        hips_node.children.is_some(),
        "Hips node should have children"
    );
    let hips_children = hips_node.children.as_ref().unwrap();
    assert!(
        hips_children.contains(&Index::new(1)),
        "Hips should have Spine (index 1) as child"
    );

    // Spine (bone 1) should have Head (bone 2) as child
    let spine_node = &root.nodes[1];
    assert!(
        spine_node.children.is_some(),
        "Spine node should have children"
    );
    let spine_children = spine_node.children.as_ref().unwrap();
    assert!(
        spine_children.contains(&Index::new(2)),
        "Spine should have Head (index 2) as child"
    );

    // Head (bone 2) should have no children
    let head_node = &root.nodes[2];
    assert!(
        head_node.children.is_none() || head_node.children.as_ref().unwrap().is_empty(),
        "Head node should have no children"
    );

    fs::remove_file(output_path).ok();
}

#[test]
fn test_export_gltf_animation_only_scene_contains_root_bones() {
    // Create a skeleton with root bones
    let skeleton = create_test_skeleton();
    let clip = create_test_clip();

    let output_path = temp_path("test_scene.glb");
    export_gltf_animation_only(&clip, &skeleton, &output_path, &[]).unwrap();

    let bytes = fs::read(&output_path).unwrap();
    let glb = Glb::from_slice(&bytes).unwrap();
    let root: json::Root = json::Root::from_slice(&glb.json).unwrap();

    // Verify that a scene exists and contains nodes matching root bone indices
    assert!(root.scene.is_some(), "GLB should have a default scene");
    let scene_index = root.scene.unwrap().value();
    let scene = &root.scenes[scene_index as usize];

    // The skeleton has one root bone (Hips, id 0), so the scene should contain node 0
    assert!(
        scene.nodes.contains(&Index::new(0)),
        "Scene should contain root bone node (Hips, index 0)"
    );

    // Verify that the scene nodes match all root bone IDs from the skeleton
    for &root_bone_id in &skeleton.root_bone_ids {
        assert!(
            scene.nodes.contains(&Index::new(root_bone_id)),
            "Scene should contain root bone node with id {}",
            root_bone_id
        );
    }

    fs::remove_file(output_path).ok();
}

#[test]
fn test_export_gltf_animation_only_writes_vrm_humanoid() {
    let skeleton = create_test_skeleton();
    let clip = create_test_clip();
    let humanoid_bones = [("hips".to_string(), 0), ("spine".to_string(), 1)];

    let output_path = temp_path("test_vrm_humanoid.glb");
    export_gltf_animation_only(&clip, &skeleton, &output_path, &humanoid_bones).unwrap();

    let bytes = fs::read(&output_path).unwrap();
    let glb = Glb::from_slice(&bytes).unwrap();
    let json_value: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
    let root: json::Root = json::Root::from_slice(&glb.json).unwrap();
    let vrm = thyllore_importer_core::gltf::vrm_humanoid_extension::parse_vrm_humanoid(
        json_value.pointer("/extensions/VRMC_vrm"),
        None,
    )
    .unwrap();

    assert!(vrm.is_v1);
    assert_eq!(vrm.bones.len(), 2);
    for (vrm_name, bone_index) in &humanoid_bones {
        let bone_name = &skeleton.bones[*bone_index].name;
        let expected_node = root
            .nodes
            .iter()
            .position(|node| node.name.as_deref() == Some(bone_name.as_str()))
            .unwrap() as u32;
        assert!(vrm.bones.contains(&(vrm_name.clone(), expected_node)));
    }
    assert!(root.extensions_used.contains(&"VRMC_vrm".to_string()));
    fs::remove_file(output_path).ok();
}
