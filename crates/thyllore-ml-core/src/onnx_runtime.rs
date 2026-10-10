use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::Result;

pub const DYLIB_PATH_ENV_VAR: &str = "ORT_DYLIB_PATH";
const VENDOR_SUBDIR: &str = "vendor/onnxruntime";

pub fn release_archive_prefix(os: &str) -> Option<&'static str> {
    match os {
        "windows" => Some("onnxruntime-win-"),
        "linux" => Some("onnxruntime-linux-"),
        "macos" => Some("onnxruntime-osx-"),
        _ => None,
    }
}

pub fn dylib_file_name(os: &str) -> Option<&'static str> {
    match os {
        "windows" => Some("onnxruntime.dll"),
        "linux" => Some("libonnxruntime.so"),
        "macos" => Some("libonnxruntime.dylib"),
        _ => None,
    }
}

/// Picks the last release archive for `os`, in name order, among the directory names under `vendor/onnxruntime/`.
pub fn select_vendor_dylib(os: &str, archive_dir_names: &[String]) -> Option<PathBuf> {
    let prefix = release_archive_prefix(os)?;
    let file_name = dylib_file_name(os)?;
    let archive = archive_dir_names
        .iter()
        .filter(|name| name.starts_with(prefix))
        .max()?;

    Some(PathBuf::from(archive).join("lib").join(file_name))
}

pub fn ensure_onnx_runtime_loaded() -> Result<()> {
    static LOAD_RESULT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

    LOAD_RESULT
        .get_or_init(load_onnx_runtime)
        .clone()
        .map_err(anyhow::Error::msg)
}

fn load_onnx_runtime() -> std::result::Result<(), String> {
    let env_path_is_set = std::env::var(DYLIB_PATH_ENV_VAR).is_ok_and(|path| !path.is_empty());
    if env_path_is_set {
        return Ok(());
    }

    let Some(dylib_path) = find_vendor_dylib(&workspace_root()) else {
        return Ok(());
    };

    let builder = ort::init_from(&dylib_path).map_err(|error| {
        format!(
            "failed to load ONNX Runtime from {}: {error}",
            dylib_path.display()
        )
    })?;
    builder.commit();
    Ok(())
}

fn find_vendor_dylib(workspace_root: &Path) -> Option<PathBuf> {
    let vendor_dir = workspace_root.join(VENDOR_SUBDIR);
    let archive_dir_names: Vec<String> = std::fs::read_dir(&vendor_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    let dylib_path = vendor_dir.join(select_vendor_dylib(
        std::env::consts::OS,
        &archive_dir_names,
    )?);
    dylib_path.is_file().then_some(dylib_path)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn selects_the_archive_matching_each_os() {
        let archives = names(&[
            "onnxruntime-linux-x64-1.23.2",
            "onnxruntime-win-x64-1.23.2",
            "onnxruntime-osx-universal2-1.23.2",
        ]);

        assert_eq!(
            select_vendor_dylib("linux", &archives),
            Some(PathBuf::from(
                "onnxruntime-linux-x64-1.23.2/lib/libonnxruntime.so"
            ))
        );
        assert_eq!(
            select_vendor_dylib("windows", &archives),
            Some(PathBuf::from(
                "onnxruntime-win-x64-1.23.2/lib/onnxruntime.dll"
            ))
        );
        assert_eq!(
            select_vendor_dylib("macos", &archives),
            Some(PathBuf::from(
                "onnxruntime-osx-universal2-1.23.2/lib/libonnxruntime.dylib"
            ))
        );
    }

    #[test]
    fn prefers_the_last_release_by_name_when_several_are_vendored() {
        let archives = names(&[
            "onnxruntime-linux-x64-1.22.0",
            "onnxruntime-linux-x64-1.23.2",
        ]);

        assert_eq!(
            select_vendor_dylib("linux", &archives),
            Some(PathBuf::from(
                "onnxruntime-linux-x64-1.23.2/lib/libonnxruntime.so"
            ))
        );
    }

    #[test]
    fn returns_none_without_a_matching_archive_or_for_an_unknown_os() {
        let archives = names(&["onnxruntime-linux-x64-1.23.2"]);

        assert_eq!(select_vendor_dylib("windows", &archives), None);
        assert_eq!(select_vendor_dylib("freebsd", &archives), None);
    }
}
