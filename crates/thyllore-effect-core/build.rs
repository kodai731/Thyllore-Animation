use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use thyllore_shader_manifest::slang_root;

// shaders/cpu/exports.slang is the C ABI of the shader math; slangc emits it as C++ and cc links
// it so the CPU side (tests, pick, Blender wheel) evaluates the same source as the GPU.
fn generate_cpp(slang_root: &Path, shader_root: &Path, out_dir: &Path) -> PathBuf {
    let generated = out_dir.join("shader_exports.cpp");
    let status = Command::new(slang_root.join("bin/slangc"))
        .arg(shader_root.join("cpu/exports.slang"))
        .arg("-I")
        .arg(shader_root.join("include"))
        .args(["-target", "cpp", "-o"])
        .arg(&generated)
        .status()
        .expect("slangc runs (SLANG_ROOT/bin/slangc, default ~/.local/slang)");
    assert!(status.success(), "slangc failed on cpu/exports.slang");
    generated
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let shader_root = manifest_dir.join("../../shaders");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let slang_root = slang_root();

    println!("cargo:rerun-if-env-changed=SLANG_ROOT");
    println!(
        "cargo:rerun-if-changed={}",
        shader_root.join("include").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shader_root.join("cpu").display()
    );

    let generated = generate_cpp(&slang_root, &shader_root, &out_dir);
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag("-ffp-contract=off")
        .include(slang_root.join("include"))
        .file(generated)
        .compile("thyllore_shader_exports");
}
