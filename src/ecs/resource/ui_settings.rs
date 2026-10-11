use std::fs;
use std::path::Path;

pub const UI_SETTINGS_PATH: &str = ".config/ui_settings.ron";
pub const MIN_UI_SCALE: f32 = 0.75;
pub const MAX_UI_SCALE: f32 = 2.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MotionPreference {
    #[default]
    Full,
    Reduced,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum UiDensity {
    Compact,
    #[default]
    Default,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UiSettings {
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default)]
    pub density: UiDensity,
    #[serde(default)]
    pub reduced_motion: MotionPreference,
}

fn default_scale() -> f32 {
    1.0
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            scale: 1.0,
            density: UiDensity::default(),
            reduced_motion: MotionPreference::default(),
        }
    }
}

pub fn clamp_ui_scale(value: f32) -> f32 {
    value.clamp(MIN_UI_SCALE, MAX_UI_SCALE)
}

#[derive(Clone, Copy, Debug)]
pub struct DensitySpacing {
    pub frame_padding: [f32; 2],
    pub item_spacing: [f32; 2],
}

pub fn density_spacing(density: UiDensity) -> DensitySpacing {
    match density {
        UiDensity::Compact => DensitySpacing {
            frame_padding: [6.0, 3.0],
            item_spacing: [6.0, 4.0],
        },
        UiDensity::Default => DensitySpacing {
            frame_padding: [10.0, 5.0],
            item_spacing: [8.0, 6.0],
        },
    }
}

pub fn load_ui_settings(path: &Path) -> UiSettings {
    match fs::read_to_string(path) {
        Ok(content) => match ron::from_str::<UiSettings>(&content) {
            Ok(settings) => UiSettings {
                scale: clamp_ui_scale(settings.scale),
                ..settings
            },
            Err(e) => {
                log_warn!("Failed to parse UI settings file: {}", e);
                UiSettings::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => UiSettings::default(),
        Err(e) => {
            log_warn!("Failed to read UI settings file: {}", e);
            UiSettings::default()
        }
    }
}

pub fn save_ui_settings(path: &Path, settings: &UiSettings) -> std::io::Result<()> {
    let parent = path.parent();
    if let Some(p) = parent {
        fs::create_dir_all(p)?;
    }
    let content = ron::ser::to_string_pretty(settings, ron::ser::PrettyConfig::new())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_ui_scale() {
        assert_eq!(clamp_ui_scale(0.5), 0.75);
        assert_eq!(clamp_ui_scale(0.75), 0.75);
        assert_eq!(clamp_ui_scale(1.0), 1.0);
        assert_eq!(clamp_ui_scale(1.5), 1.5);
        assert_eq!(clamp_ui_scale(2.0), 2.0);
        assert_eq!(clamp_ui_scale(3.0), 2.0);
    }

    #[test]
    fn test_density_spacing_values() {
        let compact = density_spacing(UiDensity::Compact);
        assert_eq!(compact.frame_padding, [6.0, 3.0]);
        assert_eq!(compact.item_spacing, [6.0, 4.0]);

        let default = density_spacing(UiDensity::Default);
        assert_eq!(default.frame_padding, [10.0, 5.0]);
        assert_eq!(default.item_spacing, [8.0, 6.0]);
    }

    #[test]
    fn test_ron_roundtrip() {
        let settings = UiSettings {
            scale: 1.25,
            density: UiDensity::Compact,
            reduced_motion: MotionPreference::Reduced,
        };
        let content = ron::ser::to_string_pretty(&settings, ron::ser::PrettyConfig::new())
            .expect("serialize");
        let loaded: UiSettings = ron::from_str(&content).expect("deserialize");
        assert_eq!(loaded.scale, 1.25);
        assert_eq!(loaded.density, UiDensity::Compact);
        assert_eq!(loaded.reduced_motion, MotionPreference::Reduced);
    }

    #[test]
    fn test_missing_fields_default() {
        let content = "(\n)";
        let loaded: UiSettings = ron::from_str(content).expect("deserialize");
        assert_eq!(loaded.scale, 1.0);
        assert_eq!(loaded.density, UiDensity::Default);
        assert_eq!(loaded.reduced_motion, MotionPreference::Full);
    }

    #[test]
    fn test_load_ui_settings_not_found() {
        let settings = load_ui_settings(Path::new("/nonexistent/ui_settings.ron"));
        assert_eq!(settings.scale, 1.0);
        assert_eq!(settings.density, UiDensity::Default);
    }

    #[test]
    fn test_load_ui_settings_invalid_content() {
        let tmp = std::env::temp_dir().join("ui_settings_invalid.ron");
        fs::write(&tmp, "not valid ron {{{").unwrap();
        let settings = load_ui_settings(&tmp);
        assert_eq!(settings.scale, 1.0);
        fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = std::env::temp_dir().join("ui_settings_roundtrip.ron");
        let settings = UiSettings {
            scale: 1.5,
            density: UiDensity::Compact,
            reduced_motion: MotionPreference::Reduced,
        };
        save_ui_settings(&tmp, &settings).unwrap();
        let loaded = load_ui_settings(&tmp);
        assert_eq!(loaded.scale, 1.5);
        assert_eq!(loaded.density, UiDensity::Compact);
        assert_eq!(loaded.reduced_motion, MotionPreference::Reduced);
        fs::remove_file(&tmp).ok();
    }
}
