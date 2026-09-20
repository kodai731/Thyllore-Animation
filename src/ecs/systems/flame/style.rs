use super::resolve_selected_flame;
use crate::ecs::component::{AppliedFlameStyle, FlameBaked, FlameEffect};
use crate::ecs::world::World;
use thyllore_effect_core::StyleGroups;

/// Load a FlameStyle file and apply it to the selected flame's parameter
/// component. The parsing and the pure apply live in effect-core / the shared
/// batch helper; this system owns the component I/O so UI and batch share one
/// behavior.
pub fn apply_flame_style_to_selected(world: &mut World, path: &str, groups: StyleGroups) {
    let Some(target) = resolve_selected_flame(world) else {
        return;
    };
    let Some(mut effect) = world
        .get_component::<FlameEffect>(target)
        .map(|e| e.clone())
    else {
        return;
    };
    let baked = world
        .get_component::<FlameBaked>(target)
        .cloned()
        .unwrap_or_default();

    let Some(style) = apply_flame_style_from_path(&mut effect, path, groups) else {
        return;
    };
    thyllore_effect_core::refresh_flame_coefficients(&mut effect, &baked);

    world.insert_component(target, effect);
    world.insert_component(
        target,
        AppliedFlameStyle {
            name: style.name,
            version: style.version,
        },
    );
}

/// Save the selected flame's current look as a named style file under the
/// styles asset directory, returning the written path.
pub fn save_flame_style_of_selected(world: &World, name: &str) -> Option<String> {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if sanitized.is_empty() {
        return None;
    }

    let target = resolve_selected_flame(world)?;
    let effect = world.get_component::<FlameEffect>(target)?;
    let path = format!("{}/{}.style.ron", crate::paths::FLAMES_STYLE_DIR, sanitized);
    dump_flame_style_to_path(effect, &path);
    Some(path)
}

pub fn load_flame_style_from_path(path: &str) -> Option<thyllore_effect_core::FlameStyle> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("warning: failed to read flame style '{}': {}", path, e);
            return None;
        }
    };
    match ron::from_str(&content) {
        Ok(style) => Some(style),
        Err(e) => {
            eprintln!("warning: failed to parse flame style '{}': {}", path, e);
            None
        }
    }
}

pub fn apply_flame_style_from_path(
    effect: &mut FlameEffect,
    path: &str,
    groups: StyleGroups,
) -> Option<thyllore_effect_core::FlameStyle> {
    let style = load_flame_style_from_path(path)?;
    thyllore_effect_core::apply_flame_style(effect, &style, groups);
    Some(style)
}

pub fn dump_flame_style_to_path(effect: &FlameEffect, path: &str) {
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("style")
        .trim_end_matches(".style.ron")
        .trim_end_matches(".ron")
        .to_string();
    let style = thyllore_effect_core::flame_style_from_effect(effect, &name);
    let content = match ron::ser::to_string_pretty(&style, ron::ser::PrettyConfig::default()) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("warning: failed to serialize flame style: {}", e);
            return;
        }
    };
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(path, content) {
        eprintln!("warning: failed to write flame style '{}': {}", path, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_ron_roundtrip_applies() {
        let ron_text = r#"FlameStyle(
            version: 1,
            name: "pillar-ref",
            motion: (twist_gain: Some(6.0), meander_amp_over_r0: Some(0.5)),
            optics: (tau0: Some(4.0)),
        )"#;
        let style: thyllore_effect_core::FlameStyle = ron::from_str(ron_text).unwrap();
        let mut effect = FlameEffect::default();
        effect.radius = 2.0;
        let applied =
            thyllore_effect_core::apply_flame_style(&mut effect, &style, StyleGroups::default());
        assert_eq!(effect.twist.gain, 6.0);
        assert_eq!(effect.meander.amp, 1.0);
        assert_eq!(effect.optical_depth, 4.0);
        assert_eq!(applied.len(), 3);
    }

    #[test]
    fn style_dump_load_roundtrip() {
        let effect = FlameEffect::default();
        let path = std::env::temp_dir().join("thyllore_style_test.style.ron");
        let path_str = path.to_str().unwrap();
        dump_flame_style_to_path(&effect, path_str);
        let style = load_flame_style_from_path(path_str).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            style,
            thyllore_effect_core::flame_style_from_effect(&effect, "thyllore_style_test")
        );
    }

    #[test]
    fn shipped_style_assets_parse() {
        for entry in std::fs::read_dir(crate::paths::FLAMES_STYLE_DIR).unwrap() {
            let path = entry.unwrap().path();
            if path.to_string_lossy().ends_with(".style.ron") {
                let content = std::fs::read_to_string(&path).unwrap();
                ron::from_str::<thyllore_effect_core::FlameStyle>(&content)
                    .unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
            }
        }
    }
}
