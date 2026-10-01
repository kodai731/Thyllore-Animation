use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::material::components::texture_remap::MaterialTextureRemap;

pub fn material_texture_remap_path(model_path: &Path) -> PathBuf {
    let stem = model_path
        .file_stem()
        .expect("model path has a file stem")
        .to_string_lossy();
    find_model_dir(model_path).join(format!("{}.materials.ron", stem))
}

pub fn save_material_texture_remap(
    path: &Path,
    remap: &MaterialTextureRemap,
) -> anyhow::Result<()> {
    let content = ron::ser::to_string_pretty(remap, ron::ser::PrettyConfig::new())?;
    fs::write(path, content)?;
    Ok(())
}

pub fn load_material_texture_remap(path: &Path) -> anyhow::Result<MaterialTextureRemap> {
    let data = fs::read_to_string(path)?;
    Ok(ron::from_str(&data)?)
}

pub fn resolve_remapped_texture(
    remap: &MaterialTextureRemap,
    material_name: &str,
    model_path: &Path,
) -> Option<PathBuf> {
    remap
        .textures
        .get(material_name)
        .map(|relative_path| find_model_dir(model_path).join(relative_path))
}

pub fn make_model_relative_path(texture_path: &Path, model_path: &Path) -> anyhow::Result<String> {
    let texture_path = std::path::absolute(texture_path)?;
    let model_dir = std::path::absolute(find_model_dir(model_path))?;

    let texture_parts: Vec<Component> = texture_path.components().collect();
    let model_dir_parts: Vec<Component> = model_dir.components().collect();
    let shared_length = texture_parts
        .iter()
        .zip(&model_dir_parts)
        .take_while(|(texture_part, model_part)| texture_part == model_part)
        .count();

    let climb = model_dir_parts[shared_length..].iter().map(|_| "..".into());
    let descend = texture_parts[shared_length..]
        .iter()
        .map(|part| part.as_os_str().to_string_lossy().into_owned());
    Ok(climb.chain(descend).collect::<Vec<String>>().join("/"))
}

fn find_model_dir(model_path: &Path) -> &Path {
    model_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_load_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("avatar.materials.ron");
        let mut remap = MaterialTextureRemap::default();
        remap
            .textures
            .insert("skin".to_string(), "../Textures/skin.png".to_string());

        save_material_texture_remap(&path, &remap).unwrap();

        assert_eq!(load_material_texture_remap(&path).unwrap(), remap);
    }

    #[test]
    fn test_remap_path_sits_beside_model() {
        assert_eq!(
            material_texture_remap_path(Path::new("/models/avatar/FBX/avatar.fbx")),
            PathBuf::from("/models/avatar/FBX/avatar.materials.ron")
        );
        assert_eq!(
            material_texture_remap_path(Path::new("avatar.fbx")),
            PathBuf::from("./avatar.materials.ron")
        );
    }

    #[test]
    fn test_relative_path_climbs_to_sibling_folder() {
        let relative_path = make_model_relative_path(
            Path::new("/models/avatar/Textures/Color/skin.png"),
            Path::new("/models/avatar/FBX/avatar.fbx"),
        )
        .unwrap();

        assert_eq!(relative_path, "../Textures/Color/skin.png");
    }

    #[test]
    fn test_relative_path_below_model_dir() {
        let relative_path = make_model_relative_path(
            Path::new("/models/avatar/textures/skin.png"),
            Path::new("/models/avatar/avatar.fbx"),
        )
        .unwrap();

        assert_eq!(relative_path, "textures/skin.png");
    }

    #[test]
    fn test_resolve_joins_model_dir() {
        let mut remap = MaterialTextureRemap::default();
        remap
            .textures
            .insert("skin".to_string(), "../Textures/skin.png".to_string());
        let model_path = Path::new("/models/avatar/FBX/avatar.fbx");

        assert_eq!(
            resolve_remapped_texture(&remap, "skin", model_path),
            Some(PathBuf::from("/models/avatar/FBX/../Textures/skin.png"))
        );
        assert_eq!(resolve_remapped_texture(&remap, "hair", model_path), None);
    }
}
