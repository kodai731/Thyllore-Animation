use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

const OWN_MANIFEST: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));

/// The Slang release every build compiles with, pinned in `[package.metadata.slang]` of this
/// crate's Cargo.toml so the GPU SPIR-V and the CPU C++ always come from the same compiler.
pub fn slang_version() -> String {
    let manifest: toml::Value = toml::from_str(OWN_MANIFEST).expect("own Cargo.toml parses");
    manifest
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("slang"))
        .and_then(|slang| slang.get("version"))
        .and_then(|version| version.as_str())
        .expect("[package.metadata.slang] version is set in thyllore-shader-manifest/Cargo.toml")
        .to_string()
}

pub fn slang_root() -> PathBuf {
    if let Ok(root) = env::var("SLANG_ROOT") {
        return PathBuf::from(root);
    }
    let home = env::var("HOME").expect("HOME set");
    Path::new(&home).join(".local/slang")
}

pub fn slangc_path(slang_root: &Path) -> PathBuf {
    slang_root.join("bin/slangc")
}

/// Stops the build when the installed slangc is not the pinned release.
pub fn verify_slangc_version(slang_root: &Path) -> Result<(), String> {
    let slangc = slangc_path(slang_root);
    let output = Command::new(&slangc).arg("-v").output().map_err(|error| {
        format!(
            "cannot run {} ({error}); install the pinned release with scripts/setup_slang.sh",
            slangc.display()
        )
    })?;
    let installed = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let installed = if installed.is_empty() {
        String::from_utf8_lossy(&output.stderr).trim().to_string()
    } else {
        installed
    };

    let pinned = slang_version();
    if installed == pinned {
        return Ok(());
    }
    Err(format!(
        "slangc at {} is {installed} but [package.metadata.slang] pins {pinned}; run scripts/setup_slang.sh (or point SLANG_ROOT at a {pinned} install)",
        slangc.display()
    ))
}

const SPIRV_DEFINES: [&str; 2] = ["WATER_RAY_QUERY", "WIND_SHADOW_VOLUME"];
const FLAME_NOISE_ROTATION_ENV: &str = "THYLLORE_FLAME_NOISE_ROT_DEG";

/// `slangc <source> -I <shader_dir> -target spirv` with the engine's defines, shared by every
/// build script so the CPU side reflects the same SPIR-V the renderer loads.
pub fn spirv_command(slang_root: &Path, shader_dir: &Path, source_path: &Path) -> Command {
    let mut command = Command::new(slangc_path(slang_root));
    command.arg(source_path);
    command.arg("-I").arg(shader_dir);
    command.args(["-target", "spirv"]);
    for define in SPIRV_DEFINES {
        command.arg(format!("-D{define}"));
    }
    if let Ok(rotation) = env::var(FLAME_NOISE_ROTATION_ENV) {
        command.arg(format!("-DFLAME_NOISE_ROT_DEG_OVERRIDE={rotation}"));
    }
    command
}

pub fn spirv_rerun_if_env_changed() {
    println!("cargo:rerun-if-env-changed=SLANG_ROOT");
    println!("cargo:rerun-if-env-changed={FLAME_NOISE_ROTATION_ENV}");
}

/// `slangc <source> -I <shader_dir>/include -I <shader_dir> -target cpp`.
pub fn cpp_command(slang_root: &Path, shader_dir: &Path, source_path: &Path) -> Command {
    let mut command = Command::new(slangc_path(slang_root));
    command.arg(source_path);
    command.arg("-I").arg(shader_dir.join("include"));
    command.arg("-I").arg(shader_dir);
    command.args(["-target", "cpp"]);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_version_is_a_release_number() {
        let version = slang_version();
        let (year, minor) = version.split_once('.').expect("year.minor");
        assert!(
            year.parse::<u32>().is_ok() && minor.parse::<u32>().is_ok(),
            "{version}"
        );
    }
}
