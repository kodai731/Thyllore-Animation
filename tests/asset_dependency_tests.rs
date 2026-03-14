use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn collect_rs_files(dir: &Path, files: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Some(s) = path.to_str() {
                files.push(s.replace('\\', "/"));
            }
        }
    }
}

fn extract_asset_paths(source: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let pattern = "\"assets/";

    let mut search_start = 0;
    while let Some(start) = source[search_start..].find(pattern) {
        let abs_start = search_start + start + 1;
        if let Some(end) = source[abs_start..].find('"') {
            let asset_path = &source[abs_start..abs_start + end];
            paths.push(asset_path.to_string());
        }
        search_start = abs_start + 1;
    }

    paths
}

fn is_output_only_path(path: &str) -> bool {
    path.starts_with("assets/exports")
}

fn is_directory_reference(path: &str) -> bool {
    path.ends_with('/')
}

fn is_in_test_block(source: &str, path_pos: usize) -> bool {
    let before = &source[..path_pos];

    let last_cfg_test = before.rfind("#[cfg(test)]");
    let last_test_attr = before.rfind("#[test]");

    if let Some(cfg_pos) = last_cfg_test {
        let between = &before[cfg_pos..];
        let close_brace_count = between.matches('}').count();
        let open_brace_count = between.matches('{').count();
        if open_brace_count > close_brace_count {
            return true;
        }
    }

    if let Some(test_pos) = last_test_attr {
        let between = &before[test_pos..];
        let fn_count = between.matches("fn ").count();
        if fn_count <= 1 {
            return true;
        }
    }

    false
}

#[test]
fn test_all_asset_file_references_exist() {
    let mut rs_files = Vec::new();
    collect_rs_files(Path::new("src"), &mut rs_files);

    let mut missing_assets: Vec<(String, String)> = Vec::new();
    let mut checked_assets: BTreeSet<String> = BTreeSet::new();
    let mut skipped_test_only: BTreeSet<String> = BTreeSet::new();
    let mut skipped_output_only: BTreeSet<String> = BTreeSet::new();

    for rs_file in &rs_files {
        let source = match fs::read_to_string(rs_file) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let asset_paths = extract_asset_paths(&source);

        for asset_path in &asset_paths {
            if is_output_only_path(asset_path) {
                skipped_output_only.insert(asset_path.clone());
                continue;
            }

            if is_directory_reference(asset_path) {
                continue;
            }

            if let Some(pos) = source.find(asset_path) {
                if is_in_test_block(&source, pos) {
                    skipped_test_only.insert(asset_path.clone());
                    continue;
                }
            }

            if checked_assets.contains(asset_path) {
                continue;
            }
            checked_assets.insert(asset_path.clone());

            if !Path::new(asset_path).exists() {
                missing_assets.push((asset_path.clone(), rs_file.clone()));
            }
        }
    }

    eprintln!("--- Asset Dependency Scan Results ---");
    eprintln!("Checked: {} unique asset paths", checked_assets.len());
    eprintln!("Skipped (output-only): {} paths", skipped_output_only.len());
    eprintln!("Skipped (test-only): {} paths", skipped_test_only.len());

    if !missing_assets.is_empty() {
        let mut msg = format!("\n{} asset file(s) missing:\n", missing_assets.len());
        for (asset, source_file) in &missing_assets {
            msg.push_str(&format!("  {} (referenced in {})\n", asset, source_file));
        }
        panic!("{}", msg);
    }
}

#[test]
fn test_asset_directories_exist() {
    let required_dirs = ["assets/shaders", "assets/models"];

    for dir in &required_dirs {
        assert!(
            Path::new(dir).is_dir(),
            "Required asset directory missing: {}",
            dir
        );
    }
}

#[test]
fn test_no_hardcoded_asset_paths_outside_constants() {
    let allowed_files = [
        "src/paths.rs",
        "src/app/raytracing.rs",
        "src/app/init/instance.rs",
        "src/vulkanr/pipeline/builder.rs",
        "src/vulkanr/resource/image.rs",
        "src/scene/scene_io.rs",
        "src/ecs/systems/inference_actor_systems.rs",
        "src/ecs/systems/phases/dispatch_scene.rs",
    ];

    let mut rs_files = Vec::new();
    collect_rs_files(Path::new("src"), &mut rs_files);

    let mut unexpected: Vec<(String, String)> = Vec::new();

    for rs_file in &rs_files {
        let normalized = rs_file.replace('\\', "/");

        let is_allowed = allowed_files.iter().any(|a| normalized.ends_with(a));
        if is_allowed {
            continue;
        }

        let source = match fs::read_to_string(rs_file) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for (line_num, line) in source.lines().enumerate() {
            if line.contains("\"assets/") {
                if is_in_test_block(
                    &source,
                    source.lines().take(line_num).map(|l| l.len() + 1).sum(),
                ) {
                    continue;
                }
                unexpected.push((
                    format!("{}:{}", rs_file, line_num + 1),
                    line.trim().to_string(),
                ));
            }
        }
    }

    if !unexpected.is_empty() {
        let mut msg = String::from(
            "\nUnexpected asset path references found outside known files.\n\
             Add the file to allowed_files in this test, or move the path to src/paths.rs:\n",
        );
        for (location, line) in &unexpected {
            msg.push_str(&format!("  {} : {}\n", location, line));
        }
        panic!("{}", msg);
    }
}

#[cfg(test)]
mod extract_tests {
    use super::*;

    #[test]
    fn test_extract_asset_paths_basic() {
        let source = r#"let path = "assets/shaders/vert.spv";"#;
        let paths = extract_asset_paths(source);
        assert_eq!(paths, vec!["assets/shaders/vert.spv"]);
    }

    #[test]
    fn test_extract_asset_paths_multiple() {
        let source = r#"
            let a = "assets/shaders/a.spv";
            let b = "assets/models/b.glb";
        "#;
        let paths = extract_asset_paths(source);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "assets/shaders/a.spv");
        assert_eq!(paths[1], "assets/models/b.glb");
    }

    #[test]
    fn test_extract_no_false_positive() {
        let source = r#"let x = "not_assets/foo.txt";"#;
        let paths = extract_asset_paths(source);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_is_output_only() {
        assert!(is_output_only_path("assets/exports/test.fbx"));
        assert!(!is_output_only_path("assets/shaders/vert.spv"));
    }
}
