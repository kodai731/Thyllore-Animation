use std::fmt::Debug;

use serde::de::DeserializeOwned;
use thyllore_scene_core::{find_scalar_param, ScalarParam, SceneComponent, UiParam};

fn assert_unique_names(names: &mut Vec<&str>) {
    names.sort_unstable();
    let len = names.len();
    names.dedup();
    assert_eq!(names.len(), len, "duplicate parameter names");
}

/// The invariants every effect's generated tables must hold.
pub fn assert_tables_are_consistent<C>(
    scalars: &[ScalarParam<C>],
    ui: &[UiParam],
    ownership_len: usize,
) where
    C: SceneComponent + Default + Debug + PartialEq + DeserializeOwned,
{
    assert_unique_names(&mut scalars.iter().map(|p| p.name).collect());
    assert_unique_names(&mut ui.iter().map(|p| p.name).collect());

    for param in ui {
        for accessor_name in param.scalar_accessor_names() {
            assert!(
                find_scalar_param(scalars, &accessor_name).is_some(),
                "{accessor_name} has no scalar accessor"
            );
        }
        assert!(param.min < param.max, "{}", param.name);
    }

    for (i, param) in scalars.iter().enumerate() {
        let mut effect = C::default();
        (param.set)(&mut effect, 3.0 + i as f32);
        let first = (param.get)(&effect);
        (param.set)(&mut effect, first);
        assert_eq!((param.get)(&effect), first, "{}", param.name);
    }

    let value = serde_json::to_value(C::default()).expect("serialize");
    let object = value.as_object().expect("object");
    let keys: Vec<&str> = object.keys().map(String::as_str).collect();
    let mut declared = C::PERSISTED_FIELDS.to_vec();
    declared.sort_unstable();
    assert_eq!(keys, declared);
    for param in ui.iter().filter(|param| !param.persisted) {
        assert!(
            !object.contains_key(param.name),
            "{} must stay runtime-only",
            param.name
        );
    }
    assert!(ownership_len >= ui.iter().filter(|param| param.persisted).count());

    let restored: C = serde_json::from_value(value).expect("deserialize");
    assert_eq!(restored, C::default());

    let ron_text = ron::to_string(&C::default()).expect("ron serialize");
    let ron_value: ron::Value = ron::from_str(&ron_text).expect("ron value");
    let through_value: C = ron_value.into_rust().expect("ron value into component");
    assert_eq!(through_value, C::default());
}

pub fn groups_in_display_order(ui: &[UiParam]) -> Vec<&'static str> {
    let mut groups: Vec<&str> = Vec::new();
    for param in ui {
        if !param.group.is_empty() && !groups.contains(&param.group) {
            groups.push(param.group);
        }
    }
    groups
}

pub fn primary_names(ui: &[UiParam]) -> Vec<&'static str> {
    ui.iter().filter(|p| p.primary).map(|p| p.name).collect()
}
