use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::app::graphics_resource::{GraphicsResources, NodeData};
use crate::ecs::systems::{
    apply_skinning, compute_pose_global_transforms, create_pose_from_rest, sample_clip_to_pose,
};
use crate::loader::ModelLoadResult;

#[allow(dead_code)]
#[derive(Deserialize)]
struct BlenderBounds {
    frames: Vec<BlenderFrameData>,
    fps: f32,
    frame_start: i32,
    frame_end: i32,
}

#[derive(Deserialize)]
struct BlenderFrameData {
    frame: i32,
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
}

fn read_blender_path() -> Option<String> {
    let paths_file = Path::new(".claude/local/paths.md");
    let content = std::fs::read_to_string(paths_file).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("- BlenderPath = ") {
            let path = rest.trim().to_string();
            if Path::new(&path).exists() {
                return Some(path);
            }
        }
    }
    None
}

fn canonicalize_no_prefix(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => {
            let s = p.to_string_lossy();
            if let Some(stripped) = s.strip_prefix(r"\\?\") {
                PathBuf::from(stripped)
            } else {
                p
            }
        }
        Err(_) => path.to_path_buf(),
    }
}

fn run_blender_bounds(blender_path: &str, model_path: &str) -> BlenderBounds {
    let script_path = canonicalize_no_prefix(Path::new("scripts/blender_bounds_check.py"));
    let model_abs = canonicalize_no_prefix(Path::new(model_path));
    let output_dir = std::env::temp_dir();
    let output_file = output_dir.join("blender_bounds_output.json");

    let output = Command::new(blender_path)
        .args([
            "--background",
            "--python",
            &script_path.to_string_lossy(),
            "--",
            &model_abs.to_string_lossy(),
            &output_file.to_string_lossy(),
        ])
        .output()
        .expect("Failed to run Blender");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Blender script failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }

    let json_str =
        std::fs::read_to_string(&output_file).expect("Failed to read Blender output JSON");
    std::fs::remove_file(&output_file).ok();

    serde_json::from_str(&json_str).expect("Failed to parse Blender bounds JSON")
}

fn compute_skinned_bounds(
    load_result: &ModelLoadResult,
    time: f32,
) -> (Vector3<f32>, Vector3<f32>) {
    let skeleton = load_result
        .skeletons
        .first()
        .expect("No skeleton for skinned bounds");
    let clip = load_result.clips.first().expect("No animation clip");

    let mut pose = create_pose_from_rest(skeleton);
    sample_clip_to_pose(clip, time, skeleton, &mut pose, false);
    let global_transforms = compute_pose_global_transforms(skeleton, &pose);

    let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);
    let mut has_vertex = false;

    for mesh in &load_result.meshes {
        let skin_data = match &mesh.skin_data {
            Some(sd) => sd,
            None => continue,
        };

        let vertex_count = skin_data.base_positions.len();
        let mut out_positions = vec![Vector3::new(0.0, 0.0, 0.0); vertex_count];
        let mut out_normals = vec![Vector3::new(0.0, 0.0, 0.0); vertex_count];

        apply_skinning(
            skin_data,
            &global_transforms,
            skeleton,
            &mut out_positions,
            &mut out_normals,
        );

        for p in &out_positions {
            has_vertex = true;
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(p.z);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(p.z);
        }
    }

    if !has_vertex {
        return (Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0));
    }

    (min, max)
}

fn compute_node_animation_bounds(
    load_result: &ModelLoadResult,
    time: f32,
) -> (Vector3<f32>, Vector3<f32>) {
    let skeleton = load_result
        .skeletons
        .first()
        .expect("No skeleton for node animation bounds");
    let clip = load_result.clips.first().expect("No animation clip");

    let mut pose = create_pose_from_rest(skeleton);
    sample_clip_to_pose(clip, time, skeleton, &mut pose, false);

    let mut nodes: Vec<NodeData> = load_result
        .nodes
        .iter()
        .map(|n| NodeData {
            index: n.index,
            name: n.name.clone(),
            parent_index: n.parent_index,
            local_transform: n.local_transform,
            global_transform: Matrix4::identity(),
        })
        .collect();

    GraphicsResources::compute_node_global_transforms(&mut nodes, skeleton, &pose);

    let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);
    let mut has_vertex = false;

    for mesh in &load_result.meshes {
        let node_idx = match mesh.node_index {
            Some(idx) => idx,
            None => continue,
        };

        let node = match nodes.iter().find(|n| n.index == node_idx) {
            Some(n) => n,
            None => continue,
        };

        let transform = node.global_transform;

        for v in &mesh.local_vertices {
            has_vertex = true;
            let pos = Vector4::new(v.pos.x, v.pos.y, v.pos.z, 1.0);
            let world_pos = transform * pos;
            let p = Vector3::new(world_pos.x, world_pos.y, world_pos.z);
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(p.z);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(p.z);
        }
    }

    if !has_vertex {
        return (Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0));
    }

    (min, max)
}

fn diagonal_length(min: &Vector3<f32>, max: &Vector3<f32>) -> f32 {
    let diff = max - min;
    (diff.x * diff.x + diff.y * diff.y + diff.z * diff.z).sqrt()
}

fn blender_to_yup(blender_min: &[f32; 3], blender_max: &[f32; 3]) -> (Vector3<f32>, Vector3<f32>) {
    let corners = [
        Vector3::new(blender_min[0], blender_min[2], -blender_min[1]),
        Vector3::new(blender_min[0], blender_min[2], -blender_max[1]),
        Vector3::new(blender_min[0], blender_max[2], -blender_min[1]),
        Vector3::new(blender_min[0], blender_max[2], -blender_max[1]),
        Vector3::new(blender_max[0], blender_min[2], -blender_min[1]),
        Vector3::new(blender_max[0], blender_min[2], -blender_max[1]),
        Vector3::new(blender_max[0], blender_max[2], -blender_min[1]),
        Vector3::new(blender_max[0], blender_max[2], -blender_max[1]),
    ];

    let mut new_min = corners[0];
    let mut new_max = corners[0];
    for c in &corners[1..] {
        new_min.x = new_min.x.min(c.x);
        new_min.y = new_min.y.min(c.y);
        new_min.z = new_min.z.min(c.z);
        new_max.x = new_max.x.max(c.x);
        new_max.y = new_max.y.max(c.y);
        new_max.z = new_max.z.max(c.z);
    }

    (new_min, new_max)
}

fn assert_bounds_similar(
    label: &str,
    rust_min: &Vector3<f32>,
    rust_max: &Vector3<f32>,
    blender_min: &[f32; 3],
    blender_max: &[f32; 3],
    tolerance_ratio: f32,
) {
    let (blender_min_v, blender_max_v) = blender_to_yup(blender_min, blender_max);

    let blender_diag = diagonal_length(&blender_min_v, &blender_max_v);
    let tolerance = blender_diag * tolerance_ratio;

    let min_diff = diagonal_length(rust_min, &blender_min_v);
    let max_diff = diagonal_length(rust_max, &blender_max_v);

    eprintln!(
        "[{}] Rust  min: ({:.4}, {:.4}, {:.4})",
        label, rust_min.x, rust_min.y, rust_min.z
    );
    eprintln!(
        "[{}] Rust  max: ({:.4}, {:.4}, {:.4})",
        label, rust_max.x, rust_max.y, rust_max.z
    );
    eprintln!(
        "[{}] Blender min (Y-up): ({:.4}, {:.4}, {:.4})",
        label, blender_min_v.x, blender_min_v.y, blender_min_v.z
    );
    eprintln!(
        "[{}] Blender max (Y-up): ({:.4}, {:.4}, {:.4})",
        label, blender_max_v.x, blender_max_v.y, blender_max_v.z
    );
    eprintln!(
        "[{}] Diagonal: {:.4}, tolerance: {:.4}, min_diff: {:.4}, max_diff: {:.4}",
        label, blender_diag, tolerance, min_diff, max_diff
    );

    assert!(
        min_diff <= tolerance,
        "[{}] Bounds min diverged: diff={:.4}, tolerance={:.4}",
        label,
        min_diff,
        tolerance
    );
    assert!(
        max_diff <= tolerance,
        "[{}] Bounds max diverged: diff={:.4}, tolerance={:.4}",
        label,
        max_diff,
        tolerance
    );
}

fn frame_to_time(frame: i32, fps: f32) -> f32 {
    frame as f32 / fps
}

fn run_bounds_test(
    model_path: &str,
    compute_bounds_fn: fn(&ModelLoadResult, f32) -> (Vector3<f32>, Vector3<f32>),
    load_result: &ModelLoadResult,
    blender_bounds: &BlenderBounds,
) {
    let rest_frame = &blender_bounds.frames[0];
    let mid_frame = &blender_bounds.frames[1];

    let rest_time = frame_to_time(rest_frame.frame, blender_bounds.fps);
    let (rust_min, rust_max) = compute_bounds_fn(load_result, rest_time);
    assert_bounds_similar(
        &format!("{} frame {}", model_path, rest_frame.frame),
        &rust_min,
        &rust_max,
        &rest_frame.bounds_min,
        &rest_frame.bounds_max,
        0.2,
    );

    let mid_time = frame_to_time(mid_frame.frame, blender_bounds.fps);
    let (rust_min, rust_max) = compute_bounds_fn(load_result, mid_time);
    assert_bounds_similar(
        &format!("{} frame {}", model_path, mid_frame.frame),
        &rust_min,
        &rust_max,
        &mid_frame.bounds_min,
        &mid_frame.bounds_max,
        0.2,
    );
}

#[test]
fn test_fbx_stickman_bounds_match_blender() {
    let blender_path = match read_blender_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: BlenderPath not configured in .claude/local/paths.md");
            return;
        }
    };

    let model_path = "assets/models/stickman/stickman_bin.fbx";
    if !Path::new(model_path).exists() {
        eprintln!("Skipping: {} not found", model_path);
        return;
    }

    let (fbx_result, _) = crate::loader::fbx::loader::load_fbx_to_graphics_resources(model_path)
        .expect("Failed to load FBX");
    let load_result = ModelLoadResult::from_fbx(fbx_result);

    let blender_bounds = run_blender_bounds(&blender_path, model_path);

    run_bounds_test(
        model_path,
        compute_node_animation_bounds,
        &load_result,
        &blender_bounds,
    );
}

#[test]
fn test_fbx_fly_bounds_match_blender() {
    let blender_path = match read_blender_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: BlenderPath not configured in .claude/local/paths.md");
            return;
        }
    };

    let model_path = "assets/models/phoenix-bird/source/fly.fbx";
    if !Path::new(model_path).exists() {
        eprintln!("Skipping: {} not found", model_path);
        return;
    }

    let (fbx_result, _) = crate::loader::fbx::loader::load_fbx_to_graphics_resources(model_path)
        .expect("Failed to load FBX");
    let load_result = ModelLoadResult::from_fbx(fbx_result);

    let blender_bounds = run_blender_bounds(&blender_path, model_path);

    run_bounds_test(
        model_path,
        compute_skinned_bounds,
        &load_result,
        &blender_bounds,
    );
}

#[test]
fn test_gltf_stickman_bounds_match_blender() {
    let blender_path = match read_blender_path() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: BlenderPath not configured in .claude/local/paths.md");
            return;
        }
    };

    let model_path = "assets/models/stickman/stickman.glb";
    if !Path::new(model_path).exists() {
        eprintln!("Skipping: {} not found", model_path);
        return;
    }

    let gltf_result = unsafe { crate::loader::gltf::load_gltf_file(model_path) };
    let load_result = ModelLoadResult::from_gltf(gltf_result);

    let blender_bounds = run_blender_bounds(&blender_path, model_path);

    let has_skin = load_result.meshes.iter().any(|m| m.skin_data.is_some());
    let compute_fn = if has_skin {
        compute_skinned_bounds
    } else {
        compute_node_animation_bounds
    };

    run_bounds_test(model_path, compute_fn, &load_result, &blender_bounds);
}
