use std::path::{Path, PathBuf};

pub fn collect_spirv_files(spirv_dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![spirv_dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)?.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "spv") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}
