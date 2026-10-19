use crate::ecs::events::{send_dialog_request, DialogRequest};
use crate::ecs::resource::{AvatarSetupState, FbxModelCache};
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::platform::ui::ui_rows::UiRow;

#[derive(Clone)]
pub struct ExportView {
    pub spring_prefix: String,
}

impl ExportView {
    pub fn from_state(state: &AvatarSetupState) -> Self {
        Self {
            spring_prefix: state.spring_prefix.clone(),
        }
    }
}

pub fn rows() -> Vec<UiRow<ExportView>> {
    vec![
        UiRow::Action {
            label: "Export for Unity",
            enabled: |_world, _view| true,
            on_click: |_view| Box::new(AvatarSetupEvent::ExportUnityAvatar),
        },
        UiRow::Canvas(|ui, _world, _view| {
            ui.text_disabled("sidecar JSON, expression .anim files, current clip .anim");
        }),
        UiRow::Canvas(|ui, world, _view| {
            let has_fbx_model = world
                .get_resource::<FbxModelCache>()
                .map_or(false, |cache| cache.fbx_model().is_some());
            if has_fbx_model {
                if ui.button("Export FBX") {
                    send_dialog_request(world, DialogRequest::ExportModelFbx);
                }
                ui.text_disabled("mesh, skeleton and skin weights as loaded and edited, no clip");
            } else {
                ui.text_disabled("Export FBX (needs an FBX model)");
            }
        }),
        UiRow::Text {
            label: "Bone Name Prefix",
            get: |view| view.spring_prefix.as_str(),
            set: |view, text| view.spring_prefix = text,
        },
        UiRow::Action {
            label: "Add Spring Chains",
            enabled: |_world, view| !view.spring_prefix.trim().is_empty(),
            on_click: |view| {
                Box::new(AvatarSetupEvent::AddSpringChainsByPrefix {
                    prefix: view.spring_prefix.trim().to_string(),
                })
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::ui::ui_rows::visible_labels;

    #[test]
    fn test_row_labels() {
        let rows = rows();
        let labels = visible_labels(&rows);
        assert_eq!(
            labels,
            ["Export for Unity", "Bone Name Prefix", "Add Spring Chains"]
        );
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
