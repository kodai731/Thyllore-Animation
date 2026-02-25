use std::path::Path;

#[test]
fn test_gltf_export_preserves_structure() {
    let source_path = Path::new("assets/models/stickman/stickman.glb");
    if !source_path.exists() {
        println!("Skipping test: stickman.glb not found");
        return;
    }

    let raw_bytes = std::fs::read(source_path).unwrap();
    let glb = gltf::binary::Glb::from_slice(&raw_bytes).unwrap();
    let original_root: gltf::json::Root = gltf::json::Root::from_slice(&glb.json).unwrap();

    let original_node_count = original_root.nodes.len();
    let original_mesh_count = original_root.meshes.len();
    let original_skin_count = original_root.skins.len();
    let original_material_count = original_root.materials.len();

    println!(
        "Original: {} nodes, {} meshes, {} skins, {} materials",
        original_node_count, original_mesh_count, original_skin_count, original_material_count
    );

    let skeleton = build_skeleton_from_gltf(&original_root);
    let clip = build_test_clip(&skeleton);

    let output_path = std::env::temp_dir().join("test_gltf_export.glb");

    rust_rendering::exporter::gltf_exporter::export_gltf_animation(
        source_path,
        &clip,
        &skeleton,
        &output_path,
    )
    .unwrap();

    let exported_bytes = std::fs::read(&output_path).unwrap();
    let exported_glb = gltf::binary::Glb::from_slice(&exported_bytes).unwrap();
    let exported_root: gltf::json::Root =
        gltf::json::Root::from_slice(&exported_glb.json).unwrap();

    assert_eq!(
        exported_root.nodes.len(),
        original_node_count,
        "node count mismatch"
    );
    assert_eq!(
        exported_root.meshes.len(),
        original_mesh_count,
        "mesh count mismatch"
    );
    assert_eq!(
        exported_root.skins.len(),
        original_skin_count,
        "skin count mismatch"
    );
    assert_eq!(
        exported_root.materials.len(),
        original_material_count,
        "material count mismatch"
    );

    assert!(
        !exported_root.animations.is_empty(),
        "exported file should have animations"
    );

    let anim = &exported_root.animations[0];
    assert!(!anim.channels.is_empty(), "animation should have channels");
    assert!(!anim.samplers.is_empty(), "animation should have samplers");

    println!(
        "Exported: {} animations, {} channels",
        exported_root.animations.len(),
        anim.channels.len()
    );

    std::fs::remove_file(&output_path).ok();
}

fn build_skeleton_from_gltf(
    root: &gltf::json::Root,
) -> rust_rendering::animation::Skeleton {
    use rust_rendering::animation::Skeleton;

    let mut skeleton = Skeleton::new("test_skeleton");

    if root.skins.is_empty() {
        for (i, node) in root.nodes.iter().enumerate() {
            let fallback = format!("node_{}", i);
            let name = node.name.as_deref().unwrap_or(&fallback);
            skeleton.add_bone(name, None);
        }
        return skeleton;
    }

    let skin = &root.skins[0];
    let mut joint_to_bone: std::collections::HashMap<usize, u32> =
        std::collections::HashMap::new();

    for (bone_idx, joint_index) in skin.joints.iter().enumerate() {
        let node_idx = joint_index.value();
        let node = &root.nodes[node_idx];
        let fallback = format!("joint_{}", bone_idx);
        let name = node.name.as_deref().unwrap_or(&fallback);

        let parent_bone_id = find_parent_node(root, node_idx)
            .and_then(|parent_node_idx| joint_to_bone.get(&parent_node_idx).copied());

        let bone_id = skeleton.add_bone(name, parent_bone_id);
        joint_to_bone.insert(node_idx, bone_id);
    }

    skeleton
}

fn find_parent_node(root: &gltf::json::Root, target: usize) -> Option<usize> {
    for (i, node) in root.nodes.iter().enumerate() {
        if let Some(ref children) = node.children {
            for child in children {
                if child.value() == target {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn build_test_clip(
    skeleton: &rust_rendering::animation::Skeleton,
) -> rust_rendering::animation::editable::EditableAnimationClip {
    use rust_rendering::animation::editable::EditableAnimationClip;

    let mut clip = EditableAnimationClip::new(0, "test_animation".to_string());
    clip.duration = 1.0;

    for bone in &skeleton.bones {
        let track = clip.add_track(bone.id, bone.name.clone());
        track.translation_x.add_keyframe(0.0, 0.0);
        track.translation_x.add_keyframe(1.0, 1.0);
        track.translation_y.add_keyframe(0.0, 0.0);
        track.translation_y.add_keyframe(1.0, 0.0);
        track.translation_z.add_keyframe(0.0, 0.0);
        track.translation_z.add_keyframe(1.0, 0.0);
    }

    clip
}
