use crate::ecs::world::Entity;
use crate::ecs::World;

use crate::platform::ui::ui_rows::ChoiceOption;

/// Build instance name list and selected index from effect entities.
/// Each entity's `Name` component is used if present, otherwise `"{fallback_prefix} {i+1}"`.
pub fn effect_instances(
    world: &World,
    entities: &[Entity],
    selected: usize,
    fallback_prefix: &str,
) -> (Vec<String>, usize) {
    let instances: Vec<String> = entities
        .iter()
        .enumerate()
        .map(|(i, &entity)| {
            world
                .get_component::<crate::ecs::world::Name>(entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| format!("{} {}", fallback_prefix, i + 1))
        })
        .collect();
    (instances, selected)
}

pub fn instance_options(names: &[String], selected: usize) -> Vec<ChoiceOption> {
    names
        .iter()
        .enumerate()
        .map(|(i, name)| ChoiceOption {
            label: name.clone(),
            selected: i == selected,
        })
        .collect()
}

pub fn preset_options(names: &[&str], applied: Option<&str>) -> Vec<ChoiceOption> {
    names
        .iter()
        .map(|&name| ChoiceOption {
            label: name.to_string(),
            selected: Some(name) == applied,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_options_marks_selected_by_index() {
        let names = vec!["Wind 1".to_string(), "Wind 2".to_string()];
        let options = instance_options(&names, 1);
        assert_eq!(options.len(), 2);
        assert!(!options[0].selected);
        assert!(options[1].selected);
        assert_eq!(options[0].label, "Wind 1");
        assert_eq!(options[1].label, "Wind 2");
    }

    #[test]
    fn instance_options_empty() {
        let options: Vec<ChoiceOption> = instance_options(&[], 0);
        assert!(options.is_empty());
    }

    #[test]
    fn preset_options_marks_applied_by_name() {
        let names: &[&str] = &["storm", "funnel", "column"];
        let options = preset_options(names, Some("funnel"));
        assert_eq!(options.len(), 3);
        assert!(!options[0].selected);
        assert!(options[1].selected);
        assert!(!options[2].selected);
    }

    #[test]
    fn preset_options_none_applied() {
        let names: &[&str] = &["storm", "funnel"];
        let options = preset_options(names, None);
        assert_eq!(options.len(), 2);
        assert!(!options[0].selected);
        assert!(!options[1].selected);
    }

    #[test]
    fn preset_options_empty() {
        let options: Vec<ChoiceOption> = preset_options(&[], None);
        assert!(options.is_empty());
    }

    #[test]
    fn effect_instances_fallback_names() {
        let world = crate::ecs::World::new();
        let (instances, selected) = effect_instances(&world, &[], 0, "Wind");
        assert!(instances.is_empty());
        assert_eq!(selected, 0);
    }
}
