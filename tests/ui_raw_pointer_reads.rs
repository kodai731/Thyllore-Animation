//! UI handlers read the mouse through `src/platform/ui/pointer.rs`, which resolves hover, active widgets and
//! `UiPointerOwner`; a direct read elsewhere bypasses that and lets a stacked window's drag fall through.

use std::fs;
use std::path::{Path, PathBuf};

const UI_DIR: &str = "src/platform/ui";
const POINTER_ACCESSOR: &str = "src/platform/ui/pointer.rs";
const RAW_POINTER_TOKENS: &[&str] = &[
    ".is_mouse_",
    ".mouse_pos",
    ".mouse_down",
    ".mouse_wheel",
    ".mouse_delta",
    ".mouse_clicked",
    ".mouse_released",
    ".mouse_double_clicked",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read ui dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn ui_windows_read_the_pointer_only_through_the_accessor() {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_rust_files(&root.join(UI_DIR), &mut files);

    let mut violations = Vec::new();
    for file in files {
        let relative = file.strip_prefix(&root).expect("under workspace root");
        if relative == Path::new(POINTER_ACCESSOR) {
            continue;
        }
        let source = fs::read_to_string(&file).expect("read ui source");
        for (line_index, line) in source.lines().enumerate() {
            if let Some(token) = RAW_POINTER_TOKENS
                .iter()
                .find(|token| line.contains(*token))
            {
                violations.push(format!(
                    "{}:{}: `{token}` — use platform::ui::pointer instead",
                    relative.display(),
                    line_index + 1
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "raw pointer reads in the UI layer:\n{}",
        violations.join("\n")
    );
}
