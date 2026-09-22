use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use thyllore_shader_manifest::{
    collect_shader_sources, collect_spirv_files, generate_pass_manifest_rust,
    generate_shader_bindings_rust, shader_entries, slang_root, spirv_output_name, EntryPoint,
    PassManifest, ShaderSource,
};
use thyllore_spirv_reflect::{reflect_shader_bytes, ShaderReflection};

const SPIRV_DIR: &str = "assets/shaders";

fn main() {
    let workspace_root = workspace_root();
    let shader_dir = workspace_root.join("shaders");
    let manifest_path = shader_dir.join("passes.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!("cargo:rerun-if-changed={}", shader_dir.display());
    println!("cargo:rerun-if-env-changed=THYLLORE_FLAME_NOISE_ROT_DEG");

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
    let mut cmd = slangc_command(shader_dir, source_path);
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

fn slangc_command(shader_dir: &Path, source_path: &Path) -> Command {
    let mut cmd = Command::new(slang_root().join("bin/slangc"));
    cmd.arg(source_path.to_str().unwrap());
    cmd.arg("-I").arg(shader_dir.to_str().unwrap());
    cmd.args(["-target", "spirv"]);
    cmd.arg("-DWATER_RAY_QUERY");
    cmd.arg("-DWIND_SHADOW_VOLUME");
    if let Ok(rotation) = std::env::var("THYLLORE_FLAME_NOISE_ROT_DEG") {
        cmd.arg(format!("-DFLAME_NOISE_ROT_DEG_OVERRIDE={}", rotation));
    }
    cmd
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
