use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn slang_root() -> PathBuf {
    if let Ok(root) = env::var("SLANG_ROOT") {
        return PathBuf::from(root);
    }
    let home = env::var("HOME").expect("HOME set");
    Path::new(&home).join(".local/slang")
}

fn generate_cpp(slang_root: &Path, shader_root: &Path, out_dir: &Path) -> PathBuf {
    let generated = out_dir.join("volume_shell.cpp");
    let status = Command::new(slang_root.join("bin/slangc"))
        .arg(shader_root.join("cpu/exports.slang"))
        .arg("-I")
        .arg(shader_root.join("include"))
        .args(["-target", "cpp", "-o"])
        .arg(&generated)
        .status()
        .expect("slangc runs");
    assert!(status.success(), "slangc failed on cpu/exports.slang");
    generated
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let shader_root = manifest_dir.join("../../shaders/slang");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let slang_root = slang_root();

    println!("cargo:rerun-if-env-changed=SLANG_ROOT");
    println!(
        "cargo:rerun-if-changed={}",
        shader_root.join("include/volume_shell.slang").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        shader_root.join("cpu/exports.slang").display()
    );

    let generated = generate_cpp(&slang_root, &shader_root, &out_dir);
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag("-ffp-contract=off")
        .include(slang_root.join("include"))
        .file(generated)
        .compile("thyllore_volume_shell_slang");
}
