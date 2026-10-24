use crate::ecs::resource::{MaterialTextureSaveState, MaterialTextureState};
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::world::World;
use crate::platform::ui::ui_rows::{ListRows, UiRow};

const NO_TEXTURE_LABEL: &str = "(from model)";
const UNRESOLVED_COLOR: [f32; 4] = [1.0, 0.6, 0.2, 1.0];
const FAILURE_COLOR: [f32; 4] = [1.0, 0.3, 0.3, 1.0];

#[derive(Clone)]
pub struct MaterialView {
    pub slots: Vec<(String, Option<String>)>,
    pub save_state: MaterialTextureSaveState,
}

impl MaterialView {
    pub fn from_state(state: &MaterialTextureState) -> Self {
        Self {
            slots: state
                .slots
                .iter()
                .map(|s| (s.material.clone(), s.texture.clone()))
                .collect(),
            save_state: state.save_state.clone(),
        }
    }
}

pub fn rows() -> Vec<UiRow<MaterialView>> {
    vec![
        UiRow::Action {
            label: "Save and Reload Model",
            enabled: |_world, _view| true,
            on_click: |_view| Box::new(AvatarSetupEvent::SaveMaterialTextures),
        },
        UiRow::Canvas(|ui, _world, view| match &view.save_state {
            MaterialTextureSaveState::Saved => {}
            MaterialTextureSaveState::Edited => {
                ui.same_line();
                ui.text_colored(UNRESOLVED_COLOR, "Unsaved changes");
            }
            MaterialTextureSaveState::SaveFailed { reason } => {
                let _wrap = ui.push_text_wrap_pos(0.0);
                ui.text_colored(FAILURE_COLOR, reason);
            }
        }),
        UiRow::List(Box::new(
            ListRows::<MaterialView, (String, Option<String>)> {
                title: "Materials",
                items: material_items,
                rows: material_item_rows,
                on_change: |_view, _index, _item| Box::new(AvatarSetupEvent::SaveMaterialTextures),
                initially_open: |_| true,
            },
        )),
    ]
}

fn material_items(_world: &World, view: &MaterialView) -> Vec<(String, (String, Option<String>))> {
    view.slots
        .iter()
        .enumerate()
        .map(|(index, (material, texture))| {
            let label = format!(
                "{}: {}",
                material,
                texture.as_deref().unwrap_or(NO_TEXTURE_LABEL)
            );
            (label, view.slots[index].clone())
        })
        .collect()
}

fn material_item_rows(_item: &(String, Option<String>)) -> Vec<UiRow<(String, Option<String>)>> {
    vec![
        UiRow::Action {
            label: "Browse Texture",
            enabled: |_world, _view| true,
            on_click: browse_texture,
        },
        UiRow::Action {
            label: "Clear Texture",
            enabled: |_world, _view| true,
            on_click: clear_texture,
        },
    ]
}

fn browse_texture(item: &(String, Option<String>)) -> Box<dyn crate::ecs::events::UiCommand> {
    Box::new(AvatarSetupEvent::PickMaterialTexture {
        material: item.0.clone(),
    })
}

fn clear_texture(item: &(String, Option<String>)) -> Box<dyn crate::ecs::events::UiCommand> {
    Box::new(AvatarSetupEvent::ClearMaterialTexture {
        material: item.0.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::visible_labels;

    #[test]
    fn test_row_labels() {
        let rows = rows();
        let labels = visible_labels(&rows);
        assert_eq!(labels, ["Save and Reload Model", "Materials"]);
    }

    #[test]
    fn test_no_duplicate_labels() {
        let rows = rows();
        let mut seen = std::collections::HashSet::new();
        for label in visible_labels(&rows) {
            assert!(seen.insert(label), "duplicate label: {}", label);
        }
    }
}
