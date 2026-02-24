use std::collections::{HashMap, HashSet};

fn main() {
    let original_path = "assets/models/stickman/stickman_bin.fbx";
    let exported_path = "assets/exports/Armature-ArmatureAction.fbx";

    println!("=== FBX Structure Comparison ===\n");
    println!("Original: {}", original_path);
    println!("Exported: {}", exported_path);
    println!("\n");

    match load_and_analyze(original_path) {
        Ok(original) => {
            match load_and_analyze(exported_path) {
                Ok(exported) => {
                    compare_structures(&original, &exported);
                }
                Err(e) => {
                    eprintln!("Failed to load exported FBX: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to load original FBX: {}", e);
        }
    }
}

struct FbxStructure {
    models: HashMap<String, ModelInfo>,
    node_attributes: HashMap<String, String>,
    animation_curve_nodes: Vec<String>,
    connections: Vec<Connection>,
    mesh_nodes: HashSet<String>,
}

struct ModelInfo {
    uid: u64,
    name: String,
    type_str: String,
    parent_model: Option<String>,
    attributes: Vec<String>,
}

#[derive(Clone, Debug)]
struct Connection {
    source_uid: u64,
    dest_uid: u64,
    source_name: String,
    dest_name: String,
    connection_type: String,
}

fn load_and_analyze(path: &str) -> Result<FbxStructure, String> {
    let scene = ufbx::load_file(path, ufbx::LoadOpts::default())
        .map_err(|e| format!("ufbx load failed: {}", e.description))?;

    let mut structure = FbxStructure {
        models: HashMap::new(),
        node_attributes: HashMap::new(),
        animation_curve_nodes: Vec::new(),
        connections: Vec::new(),
        mesh_nodes: HashSet::new(),
    };

    println!("File: {}", path);
    println!("  Nodes: {}", scene.nodes.len());
    println!("  Meshes: {}", scene.meshes.len());
    println!("  AnimationStacks: {}", scene.anim_stacks.len());

    for node in &scene.nodes {
        let name = node.name.to_string();
        let type_str = if let Some(mesh) = &node.mesh {
            structure.mesh_nodes.insert(name.clone());
            format!("Model(Mesh) - mesh has {} vertices", mesh.num_vertices)
        } else if let Some(attr) = &node.attrib {
            format!("Model(NodeAttribute) - {}", attr.type_name.to_string())
        } else if node.children.len() > 0 {
            "Model(Transform/Armature)".to_string()
        } else {
            "Model(Unknown)".to_string()
        };

        println!("  Node: {} [{}]", name, type_str);

        if let Some(parent) = &node.parent {
            println!("    Parent: {}", parent.name);
        }

        if !node.children.is_empty() {
            println!("    Children: {:?}",
                node.children.iter().map(|c| c.name.to_string()).collect::<Vec<_>>());
        }

        if let Some(mesh) = &node.mesh {
            println!("    Mesh: {} vertices, {} triangles", mesh.num_vertices, mesh.num_triangles);
        }

        if let Some(attr) = &node.attrib {
            println!("    NodeAttribute: {}", attr.type_name);
        }

        structure.models.insert(
            name.clone(),
            ModelInfo {
                uid: node.element.uid,
                name: name.clone(),
                type_str: type_str.clone(),
                parent_model: node.parent.map(|p| p.name.to_string()),
                attributes: Vec::new(),
            },
        );
    }

    println!("\nAnimation Stacks:");
    for (stack_idx, stack) in scene.anim_stacks.iter().enumerate() {
        println!("  Stack[{}]: {}", stack_idx, stack.name);
        for (layer_idx, layer) in stack.layers.iter().enumerate() {
            println!("    Layer[{}]: {}", layer_idx, layer.name);
            println!("      AnimationCurveNodes: {}", layer.anim_curves.len());

            for curve_node in &layer.anim_curves {
                let target_name = curve_node
                    .curves
                    .first()
                    .and_then(|c| c.target_node.as_ref())
                    .map(|n| n.name.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                println!("        CurveNode -> Target: {}", target_name);

                for (curve_idx, curve) in curve_node.curves.iter().enumerate() {
                    if let Some(target_node) = &curve.target_node {
                        let prop_name = curve.prop_name.to_string();
                        println!("          Curve[{}]: {} on {}", curve_idx, prop_name, target_node.name);
                    }
                }
            }
        }
    }

    println!("\nMesh Nodes Summary:");
    for mesh_node in &structure.mesh_nodes {
        println!("  - {}", mesh_node);
    }

    Ok(structure)
}

fn compare_structures(original: &FbxStructure, exported: &FbxStructure) {
    println!("\n=== COMPARISON ===\n");

    println!("Mesh Nodes:");
    let orig_meshes: HashSet<_> = original.mesh_nodes.iter().cloned().collect();
    let exp_meshes: HashSet<_> = exported.mesh_nodes.iter().cloned().collect();

    println!("  Original only: {:?}",
        orig_meshes.difference(&exp_meshes).collect::<Vec<_>>());
    println!("  Exported only: {:?}",
        exp_meshes.difference(&orig_meshes).collect::<Vec<_>>());
    println!("  Common: {:?}",
        orig_meshes.intersection(&exp_meshes).collect::<Vec<_>>());

    println!("\nModel Structure:");
    println!("  Original models: {}", original.models.len());
    println!("  Exported models: {}", exported.models.len());

    for (name, orig_model) in &original.models {
        if let Some(exp_model) = exported.models.get(name) {
            if orig_model.type_str != exp_model.type_str {
                println!("  Difference in '{}': original={}, exported={}",
                    name, orig_model.type_str, exp_model.type_str);
            }
            if orig_model.parent_model != exp_model.parent_model {
                println!("  Parent change in '{}': original={:?}, exported={:?}",
                    name, orig_model.parent_model, exp_model.parent_model);
            }
        }
    }
}
