use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use thyllore_shader_manifest::{
    collect_shader_sources, collect_spirv_files, find_declared_block, generate_pass_manifest_rust,
    generate_shader_bindings_rust, generate_spirv_block_rust, shader_entries, slang_root,
    spirv_command, spirv_output_name, spirv_rerun_if_env_changed, verify_slangc_version,
    EntryPoint, PassManifest, ShaderSource,
};
use thyllore_spirv_reflect::{reflect_shader_bytes, ShaderReflection};

const SPIRV_DIR: &str = "assets/shaders";

/// Push constant blocks whose Rust struct is generated from the SPIR-V that declares them.
const GPU_BLOCKS: [(&str, &str); 1] = [("TracePush", "trace_push.rs")];

fn main() {
    let workspace_root = workspace_root();
    let shader_dir = workspace_root.join("shaders");
    let manifest_path = shader_dir.join("passes.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!("cargo:rerun-if-changed={}", shader_dir.display());
    spirv_rerun_if_env_changed();
    if let Err(message) = verify_slangc_version(&slang_root()) {
        eprintln!("{message}");
        std::process::exit(1);
    }

    let sources = collect_shader_sources(&shader_dir).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    });
    let manifest = read_manifest(&manifest_path, &sources);
    let reflections = compile_shaders(&shader_dir, &workspace_root.join(SPIRV_DIR), &sources);

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    write_generated(
        &out_dir.join("pass_manifest.rs"),
        generate_pass_manifest_rust(&manifest, SPIRV_DIR),
    );
    let bindings =
        generate_shader_bindings_rust(&manifest, |spirv_name| reflections.get(spirv_name).cloned())
            .unwrap_or_else(|error| {
                eprintln!("shader binding generation failed: {error}");
                std::process::exit(1);
            });
    write_generated(&out_dir.join("shader_bindings.rs"), bindings);
    for (block_name, file_name) in GPU_BLOCKS {
        write_generated(
            &out_dir.join(file_name),
            gpu_block_source(&reflections, block_name),
        );
    }
}

fn gpu_block_source(reflections: &BTreeMap<String, ShaderReflection>, block_name: &str) -> String {
    let mut declaring = reflections.iter().filter_map(|(spirv_name, reflection)| {
        find_declared_block(reflection, block_name).map(|block| (spirv_name, block))
    });
    let Some((first_name, block)) = declaring.next() else {
        eprintln!("no SPIR-V under {SPIRV_DIR} declares block `{block_name}`");
        std::process::exit(1);
    };
    if let Some((other_name, other)) = declaring.find(|(_, other)| *other != block) {
        eprintln!("block `{block_name}` differs between {first_name} and {other_name}: {block:?} vs {other:?}");
        std::process::exit(1);
    }

    generate_spirv_block_rust(&block, first_name, &["cgmath::Matrix4"]).unwrap_or_else(|error| {
        eprintln!("generate `{block_name}`: {error}");
        std::process::exit(1)
    })
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/<name> lives two levels below the workspace root")
        .to_path_buf()
}

fn write_generated(out_path: &Path, content: String) {
    std::fs::write(out_path, content).unwrap_or_else(|error| {
        eprintln!("failed to write {}: {error}", out_path.display());
        std::process::exit(1);
    });
}

fn read_manifest(manifest_path: &Path, sources: &BTreeMap<String, ShaderSource>) -> PassManifest {
    let text = std::fs::read_to_string(manifest_path).unwrap_or_else(|error| {
        eprintln!("failed to read {}: {error}", manifest_path.display());
        std::process::exit(1);
    });
    PassManifest::parse(&text, &shader_entries(sources)).unwrap_or_else(|error| {
        eprintln!("{}: {error}", manifest_path.display());
        std::process::exit(1);
    })
}

/// Compiles every entry point of every source to its own SPIR-V; keyed by the SPIR-V path
/// relative to `spirv_dir`.
fn compile_shaders(
    shader_dir: &Path,
    spirv_dir: &Path,
    sources: &BTreeMap<String, ShaderSource>,
) -> BTreeMap<String, ShaderReflection> {
    let mut reflections = BTreeMap::new();
    let mut expected_outputs = Vec::new();
    for (file_name, source) in sources {
        for entry_point in &source.entry_points {
            let Some(out_name) = spirv_output_name(file_name, entry_point.stage) else {
                continue;
            };
            let out_path = spirv_dir.join(&out_name);
            create_output_directory(&out_path);
            expected_outputs.push(out_path.clone());

            compile_shader(shader_dir, &source.path, entry_point, &out_path);
            reflections.insert(out_name, reflect_from_spirv(&out_path));
        }
    }

    remove_stale_spirv(spirv_dir, &expected_outputs);
    reflections
}

fn create_output_directory(out_path: &Path) {
    let Some(parent) = out_path.parent() else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(parent) {
        eprintln!("failed to create {}: {error}", parent.display());
        std::process::exit(1);
    }
}

fn compile_shader(
    shader_dir: &Path,
    source_path: &Path,
    entry_point: &EntryPoint,
    out_path: &Path,
) {
    let file_name = source_path.file_name().unwrap().to_str().unwrap();
    let mut cmd = spirv_command(&slang_root(), shader_dir, source_path);
    cmd.args(["-entry", &entry_point.name]);
    cmd.args(["-stage", entry_point.stage.attribute()]);
    cmd.arg("-o").arg(out_path.to_str().unwrap());
    match cmd.output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            eprintln!(
                "シェーダーコンパイルエラー ({}):\n{}",
                file_name,
                String::from_utf8_lossy(&output.stderr)
            );
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("シェーダーコンパイラの実行に失敗しました: {}", error);
            eprintln!(
                "SLANG_ROOT/bin/slangc (既定 ~/.local/slang) が存在することを確認してください。"
            );
            std::process::exit(1);
        }
    }
}

fn reflect_from_spirv(spirv_path: &Path) -> ShaderReflection {
    let file_name = spirv_path.file_name().unwrap().to_str().unwrap();
    let spirv = std::fs::read(spirv_path).unwrap_or_else(|error| {
        eprintln!("SPIR-V read failed ({}): {}", spirv_path.display(), error);
        std::process::exit(1);
    });
    reflect_shader_bytes(&spirv).unwrap_or_else(|error| {
        eprintln!("SPIR-V reflection failed ({}): {}", file_name, error);
        std::process::exit(1);
    })
}

fn remove_stale_spirv(spirv_dir: &Path, expected_outputs: &[PathBuf]) {
    let Ok(compiled) = collect_spirv_files(spirv_dir) else {
        return;
    };
    for path in compiled {
        if expected_outputs.contains(&path) {
            continue;
        }
        if let Err(error) = std::fs::remove_file(&path) {
            eprintln!(
                "古い SPIR-V の削除に失敗しました ({}): {}",
                path.display(),
                error
            );
        }
    }
}
