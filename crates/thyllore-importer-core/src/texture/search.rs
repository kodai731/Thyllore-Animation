use std::fs;
use std::path::{Path, PathBuf};

const TEXTURE_FOLDER_NAMES: [&str; 2] = ["Textures", "textures"];
const TEXTURE_FOLDER_ANCESTOR_LEVELS: usize = 3;
const PACKAGE_SEARCH_DEPTH: usize = 4;
const LOADABLE_EXTENSION: &str = "png";

/// Finds the file a model's texture reference names: by file name, as Unity's model importer does.
pub fn find_texture_file(reference: &str, model_path: &Path) -> Option<PathBuf> {
    let model_dir = model_path.parent().unwrap_or_else(|| Path::new("."));

    if let Some(written_path) = find_at_written_path(reference, model_dir) {
        return Some(written_path);
    }

    list_loadable_file_names(reference)
        .iter()
        .find_map(|file_name| {
            find_in_texture_folders(file_name, model_dir)
                .or_else(|| find_in_package(file_name, model_dir))
        })
}

fn find_at_written_path(reference: &str, model_dir: &Path) -> Option<PathBuf> {
    let written_path = PathBuf::from(reference.replace('\\', "/"));
    let candidate = if written_path.is_absolute() {
        written_path
    } else {
        model_dir.join(written_path)
    };
    candidate.is_file().then_some(candidate)
}

fn list_loadable_file_names(reference: &str) -> Vec<String> {
    let Some(file_name) = reference
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
    else {
        return Vec::new();
    };

    let mut file_names = vec![file_name.to_string()];
    let loadable_name = Path::new(file_name).with_extension(LOADABLE_EXTENSION);
    if let Some(loadable_name) = loadable_name.to_str().filter(|name| *name != file_name) {
        file_names.push(loadable_name.to_string());
    }
    file_names
}

fn find_in_texture_folders(file_name: &str, model_dir: &Path) -> Option<PathBuf> {
    let beside_model = model_dir.join(file_name);
    if beside_model.is_file() {
        return Some(beside_model);
    }

    model_dir
        .ancestors()
        .take(TEXTURE_FOLDER_ANCESTOR_LEVELS)
        .flat_map(|dir| {
            TEXTURE_FOLDER_NAMES
                .iter()
                .map(move |folder| dir.join(folder).join(file_name))
        })
        .find(|candidate| candidate.is_file())
}

fn find_in_package(file_name: &str, model_dir: &Path) -> Option<PathBuf> {
    let package_dir = model_dir.parent().unwrap_or(model_dir);

    let mut matches = Vec::new();
    collect_files_named(file_name, package_dir, 0, &mut matches);
    matches.sort();

    if matches.len() > 1 {
        log_warn!(
            "Texture {} matches {} files under {}, using {}",
            file_name,
            matches.len(),
            package_dir.display(),
            matches[0].1.display()
        );
    }
    matches.into_iter().next().map(|(_, path)| path)
}

fn collect_files_named(
    file_name: &str,
    dir: &Path,
    depth: usize,
    matches: &mut Vec<(usize, PathBuf)>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth < PACKAGE_SEARCH_DEPTH {
                collect_files_named(file_name, &path, depth + 1, matches);
            }
        } else if path.file_name().is_some_and(|name| name == file_name) {
            matches.push((depth, path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_file(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"texture").unwrap();
    }

    #[test]
    fn test_written_relative_path_wins() {
        let package = tempdir().unwrap();
        let model_path = package.path().join("model/avatar.fbx");
        write_file(&package.path().join("model/images/skin.png"));
        write_file(&package.path().join("model/Textures/skin.png"));

        let found = find_texture_file("images\\skin.png", &model_path);

        assert_eq!(found, Some(package.path().join("model/images/skin.png")));
    }

    #[test]
    fn test_authoring_directory_is_ignored_when_missing() {
        let package = tempdir().unwrap();
        let model_path = package.path().join("FBX/avatar.fbx");
        write_file(&package.path().join("Textures/skin.png"));

        let found = find_texture_file("C:\\work\\images\\skin.png", &model_path);

        assert_eq!(found, Some(package.path().join("Textures/skin.png")));
    }

    #[test]
    fn test_texture_folder_wins_over_other_folders() {
        let package = tempdir().unwrap();
        let model_path = package.path().join("FBX/avatar.fbx");
        write_file(&package.path().join("Backup/skin.png"));
        write_file(&package.path().join("FBX/textures/skin.png"));

        let found = find_texture_file("skin.png", &model_path);

        assert_eq!(found, Some(package.path().join("FBX/textures/skin.png")));
    }

    #[test]
    fn test_package_search_finds_any_folder() {
        let package = tempdir().unwrap();
        let model_path = package.path().join("FBX/avatar.fbx");
        write_file(&package.path().join("PNG/Color/skin.png"));

        let found = find_texture_file("skin.png", &model_path);

        assert_eq!(found, Some(package.path().join("PNG/Color/skin.png")));
    }

    #[test]
    fn test_package_search_prefers_shallowest_match() {
        let package = tempdir().unwrap();
        let model_path = package.path().join("FBX/avatar.fbx");
        write_file(&package.path().join("PNG/Color/skin.png"));
        write_file(&package.path().join("PNG/skin.png"));

        let found = find_texture_file("skin.png", &model_path);

        assert_eq!(found, Some(package.path().join("PNG/skin.png")));
    }

    #[test]
    fn test_unloadable_extension_falls_back_to_png() {
        let package = tempdir().unwrap();
        let model_path = package.path().join("FBX/avatar.fbx");
        write_file(&package.path().join("Textures/skin.png"));

        let found = find_texture_file("work/skin.psd", &model_path);

        assert_eq!(found, Some(package.path().join("Textures/skin.png")));
    }

    #[test]
    fn test_missing_texture_is_not_guessed() {
        let package = tempdir().unwrap();
        let model_path = package.path().join("FBX/avatar.fbx");
        write_file(&package.path().join("Textures/avatar.png"));

        assert_eq!(find_texture_file("skin.png", &model_path), None);
        assert_eq!(find_texture_file("", &model_path), None);
    }
}
