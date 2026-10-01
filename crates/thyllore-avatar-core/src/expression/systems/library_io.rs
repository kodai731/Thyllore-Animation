use std::fs;
use std::path::{Path, PathBuf};

use crate::expression::components::preset::ExpressionLibrary;

pub fn save_library(path: &Path, library: &ExpressionLibrary) -> anyhow::Result<()> {
    let content = ron::ser::to_string_pretty(library, ron::ser::PrettyConfig::new())?;
    fs::write(path, content)?;
    Ok(())
}

pub fn load_library(path: &Path) -> anyhow::Result<ExpressionLibrary> {
    let data = fs::read_to_string(path)?;
    let library: ExpressionLibrary = ron::from_str(&data)?;
    Ok(library)
}

pub fn expression_library_path(model_path: &Path) -> PathBuf {
    let stem = model_path
        .file_stem()
        .expect("model path has a file stem")
        .to_string_lossy();
    let mut library_path = model_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    library_path.push(format!("{}.expressions.ron", stem));
    library_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::systems::preset::{apply_preset, capture_preset};
    use tempfile::tempdir;

    #[test]
    fn test_save_load_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.expressions.ron");

        let names = vec![
            "eye_angry".to_string(),
            "mouth_smile".to_string(),
            "eyebrow_raise".to_string(),
        ];

        let preset1 = capture_preset("happy", &names, &[0.0, 1.0, 0.5]);
        let preset2 = capture_preset("angry", &names, &[1.0, 0.0, 0.0]);

        let library = ExpressionLibrary {
            presets: vec![preset1, preset2],
        };

        save_library(&path, &library).unwrap();
        let loaded = load_library(&path).unwrap();

        assert_eq!(loaded.presets.len(), 2);
        assert_eq!(loaded.presets[0].name, "happy");
        assert_eq!(loaded.presets[1].name, "angry");

        let applied1 = apply_preset(&loaded.presets[0], &names, &[]);
        assert_eq!(applied1.weights, vec![0.0, 1.0, 0.5]);

        let applied2 = apply_preset(&loaded.presets[1], &names, &[]);
        assert_eq!(applied2.weights, vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_expression_library_path() {
        let model_path = Path::new("/assets/models/character.fbx");
        let library_path = expression_library_path(model_path);
        assert_eq!(
            library_path,
            PathBuf::from("/assets/models/character.expressions.ron")
        );
    }

    #[test]
    fn test_expression_library_path_no_extension() {
        let model_path = Path::new("/assets/models/character");
        let library_path = expression_library_path(model_path);
        assert_eq!(
            library_path,
            PathBuf::from("/assets/models/character.expressions.ron")
        );
    }

    #[test]
    fn test_load_empty_library() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.expressions.ron");

        let library = ExpressionLibrary::default();
        save_library(&path, &library).unwrap();
        let loaded = load_library(&path).unwrap();

        assert!(loaded.presets.is_empty());
    }
}
