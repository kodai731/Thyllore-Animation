use std::collections::{HashMap, HashSet};

#[test]
#[ignore]
fn compare_fbx_structures() {
    let original_path = "assets/models/stickman/stickman_bin.fbx";
    let exported_path = "assets/exports/Armature-ArmatureAction.fbx";

    println!("\n=== FBX Structure Comparison ===\n");
    println!("Original: {}", original_path);
    println!("Exported: {}", exported_path);
    println!("\n");

    let original = load_and_analyze(original_path).expect("Failed to load original FBX");
    let exported = load_and_analyze(exported_path).expect("Failed to load exported FBX");

    compare_structures(&original, &exported);
}

struct FbxStructure {
    models: HashMap<String, ModelInfo>,
    animation_curve_nodes: Vec<String>,
    mesh_nodes: HashSet<String>,
}

#[derive(Clone, Debug)]
struct ModelInfo {
    uid: u32,
    name: String,
    type_str: String,
    parent_model: Option<String>,
}

fn load_and_analyze(path: &str) -> Result<FbxStructure, String> {
    let scene = ufbx::load_file(path, ufbx::LoadOpts::default())
        .map_err(|e| format!("ufbx load failed: {}", e.description))?;

    let mut structure = FbxStructure {
        models: HashMap::new(),
        animation_curve_nodes: Vec::new(),
        mesh_nodes: HashSet::new(),
    };

    println!("File: {}", path);
    println!("  Nodes: {}", scene.nodes.len());
    println!("  Meshes: {}", scene.meshes.len());
    println!("  AnimationStacks: {}", scene.anim_stacks.len());
    println!("\nNode Details:");

    for node in &scene.nodes {
        if node.is_root {
            continue;
        }

        let name = node.element.name.to_string();
        let type_str = if let Some(mesh) = &node.mesh {
            structure.mesh_nodes.insert(name.clone());
            format!("Model(Mesh) - {} vertices", mesh.num_vertices)
        } else if let Some(_attr) = &node.attrib {
            format!("Model(NodeAttribute)")
        } else if !node.children.is_empty() {
            "Model(Transform/Armature)".to_string()
        } else {
            "Model(Unknown)".to_string()
        };

        println!("  {}: {}", name, type_str);

        if let Some(parent) = &node.parent {
            if !parent.is_root {
                println!("    -> Parent: {}", parent.element.name);
            }
        }

        if let Some(mesh) = &node.mesh {
            println!("    -> Mesh: {} vertices", mesh.num_vertices);
        }

        let parent_model = node
            .parent
            .as_ref()
            .filter(|p| !p.is_root)
            .map(|p| p.element.name.to_string());

        structure.models.insert(
            name.clone(),
            ModelInfo {
                uid: node.element.element_id,
                name: name.clone(),
                type_str,
                parent_model,
            },
        );
    }

    println!("\nAnimation Details:");
    for (stack_idx, stack) in scene.anim_stacks.iter().enumerate() {
        println!("  AnimStack[{}]: {}", stack_idx, stack.element.name);

        let bake_opts = ufbx::BakeOpts::default();
        if let Ok(baked) = ufbx::bake_anim(&scene, &stack.anim, bake_opts) {
            println!("    Baked {} nodes", baked.nodes.len());

            for bake_node in &baked.nodes {
                let node_idx = bake_node.typed_id as usize;
                if node_idx < scene.nodes.len() {
                    let node = &scene.nodes[node_idx];
                    let has_anim = !bake_node.translation_keys.is_empty()
                        || !bake_node.rotation_keys.is_empty()
                        || !bake_node.scale_keys.is_empty();

                    if has_anim {
                        println!(
                            "      -> {}: T={} R={} S={}",
                            node.element.name,
                            bake_node.translation_keys.len(),
                            bake_node.rotation_keys.len(),
                            bake_node.scale_keys.len()
                        );
                    }
                }
            }
        }
    }

    println!("\nMesh Nodes Found:");
    if structure.mesh_nodes.is_empty() {
        println!("  (none)");
    } else {
        for mesh_node in &structure.mesh_nodes {
            println!("  - {}", mesh_node);
        }
    }

    Ok(structure)
}

fn compare_structures(original: &FbxStructure, exported: &FbxStructure) {
    println!("\n=== DETAILED COMPARISON ===\n");

    println!("Mesh Nodes Comparison:");
    let orig_meshes: HashSet<_> = original.mesh_nodes.iter().cloned().collect();
    let exp_meshes: HashSet<_> = exported.mesh_nodes.iter().cloned().collect();

    let orig_only: Vec<_> = orig_meshes.difference(&exp_meshes).collect();
    let exp_only: Vec<_> = exp_meshes.difference(&orig_meshes).collect();
    let common: Vec<_> = orig_meshes.intersection(&exp_meshes).collect();

    if !orig_only.is_empty() {
        println!("  Original only: {:?}", orig_only);
    }
    if !exp_only.is_empty() {
        println!("  Exported only: {:?}", exp_only);
    }
    println!("  Common mesh nodes: {} count", common.len());
    if !common.is_empty() {
        for mesh in common {
            println!("    - {}", mesh);
        }
    }

    println!("\nModel Structure Comparison:");
    println!("  Original models: {}", original.models.len());
    println!("  Exported models: {}", exported.models.len());

    let orig_names: HashSet<_> = original.models.keys().cloned().collect();
    let exp_names: HashSet<_> = exported.models.keys().cloned().collect();

    let only_in_orig: Vec<_> = orig_names.difference(&exp_names).collect();
    let only_in_exp: Vec<_> = exp_names.difference(&orig_names).collect();

    if !only_in_orig.is_empty() {
        println!("\n  Only in original:");
        for name in only_in_orig {
            if let Some(model) = original.models.get(name) {
                println!("    - {}: {}", name, model.type_str);
            }
        }
    }

    if !only_in_exp.is_empty() {
        println!("\n  Only in exported:");
        for name in only_in_exp {
            if let Some(model) = exported.models.get(name) {
                println!("    - {}: {}", name, model.type_str);
            }
        }
    }

    println!("\n  Type/Parent differences:");
    for (name, orig_model) in &original.models {
        if let Some(exp_model) = exported.models.get(name) {
            if orig_model.type_str != exp_model.type_str {
                println!("    Type change in '{}': ", name);
                println!("      original:  {}", orig_model.type_str);
                println!("      exported:  {}", exp_model.type_str);
            }
            if orig_model.parent_model != exp_model.parent_model {
                println!("    Parent change in '{}': ", name);
                println!("      original:  {:?}", orig_model.parent_model);
                println!("      exported:  {:?}", exp_model.parent_model);
            }
        }
    }

    println!("\n=== END COMPARISON ===\n");
}

#[test]
fn inspect_exported_fbx_node_types() {
    let path = "assets/exports/Armature-ArmatureAction.fbx";

    println!("\n=== Exported FBX Node Type Inspection ===\n");
    println!("File: {}\n", path);

    match ufbx::load_file(path, ufbx::LoadOpts::default()) {
        Ok(scene) => {
            println!("Total Nodes: {}", scene.nodes.len());
            println!("Total Meshes: {}", scene.meshes.len());
            println!("Total AnimStacks: {}\n", scene.anim_stacks.len());

            println!("Detailed check for bone-related nodes:");
            for node in &scene.nodes {
                if node.is_root {
                    continue;
                }
                let name = node.element.name.to_string();
                if name.contains("Bone") || name.contains("Armature") {
                    println!(
                        "  {}: mesh={}, attrib={}, bone={}, attrib_type={:?}",
                        name,
                        node.mesh.is_some(),
                        node.attrib.is_some(),
                        node.bone.is_some(),
                        node.attrib_type
                    );
                }
            }
        }
        Err(e) => {
            println!("Failed to load FBX: {}", e.description);
        }
    }
}

#[test]
fn inspect_stickman_fbx_node_types() {
    let path = "assets/models/stickman/stickman_bin.fbx";

    println!("\n=== Stickman FBX Node Type Inspection ===\n");
    println!("File: {}\n", path);

    match ufbx::load_file(path, ufbx::LoadOpts::default()) {
        Ok(scene) => {
            println!("Total Nodes: {}", scene.nodes.len());
            println!("Total Meshes: {}", scene.meshes.len());
            println!("Total AnimStacks: {}\n", scene.anim_stacks.len());

            println!("Node Details:");
            println!(
                "{:<30} | {:<40} | {:<30}",
                "Node Name", "Has Mesh", "Has Attrib"
            );
            println!("{}", "-".repeat(105));

            for node in &scene.nodes {
                if node.is_root {
                    continue;
                }

                let name = node.element.name.to_string();
                let has_mesh = node
                    .mesh
                    .as_ref()
                    .map(|m| format!("Yes ({}v)", m.num_vertices))
                    .unwrap_or_else(|| "No".to_string());
                let has_attrib = node
                    .attrib
                    .as_ref()
                    .map(|_| "Yes".to_string())
                    .unwrap_or_else(|| "No".to_string());

                println!("{:<30} | {:<40} | {:<30}", name, has_mesh, has_attrib);
            }

            println!("\nNode parent relationships:");
            for node in &scene.nodes {
                if node.is_root {
                    continue;
                }
                let name = node.element.name.to_string();
                if let Some(parent) = &node.parent {
                    if !parent.is_root {
                        println!("  {} <- {}", name, parent.element.name);
                    }
                }
            }

            println!("\nBone nodes (identified as having no mesh/attrib and being children of Armature-like nodes):");
            let mut bone_count = 0;
            for node in &scene.nodes {
                if node.is_root {
                    continue;
                }
                let name = node.element.name.to_string();
                let has_mesh = node.mesh.is_some();
                let has_attrib = node.attrib.is_some();
                let has_bone = node.bone.is_some();
                let attrib_type = format!("{:?}", node.attrib_type);

                if !has_mesh && !has_attrib {
                    println!(
                        "  {} (attrib_type: {}, has_bone: {})",
                        name, attrib_type, has_bone
                    );
                    bone_count += 1;
                }
            }
            println!("Total bone-like nodes: {}", bone_count);

            println!("\nAttribute type breakdown of all non-root nodes:");
            use std::collections::HashMap;
            let mut type_counts: HashMap<String, usize> = HashMap::new();
            for node in &scene.nodes {
                if node.is_root {
                    continue;
                }
                let attrib_type = format!("{:?}", node.attrib_type);
                *type_counts.entry(attrib_type).or_insert(0) += 1;
            }
            for (attrib_type, count) in type_counts.iter() {
                println!("  {}: {}", attrib_type, count);
            }

            println!("\nDetailed check for bone-related nodes:");
            for node in &scene.nodes {
                if node.is_root {
                    continue;
                }
                let name = node.element.name.to_string();
                if name.contains("Bone") || name.contains("Armature") {
                    println!(
                        "  {}: mesh={}, attrib={}, bone={}, attrib_type={:?}",
                        name,
                        node.mesh.is_some(),
                        node.attrib.is_some(),
                        node.bone.is_some(),
                        node.attrib_type
                    );
                }
            }
        }
        Err(e) => {
            println!("Failed to load FBX: {}", e.description);
        }
    }
}
