use cgmath::{InnerSpace, Vector3};

use crate::animation::{Bone, BoneId, Skeleton};
use crate::asset::AssetStorage;
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::world::World;

use super::super::form_window::{message_section, rows_section, FormSection};
use super::super::ui_rows::UiRow;

#[derive(Clone, Debug, PartialEq)]
pub struct BoneView {
    pub bone_id: BoneId,
    pub name: String,
    pub parent_name: String,
    pub offset: [f32; 3],
    pub child_name: String,
    pub child_offset: Vector3<f32>,
}

pub fn bone_section(assets: &AssetStorage, bone_id: BoneId) -> Box<dyn FormSection> {
    let Some(view) = assets
        .skeletons
        .values()
        .next()
        .and_then(|skeleton_asset| read_bone_view(&skeleton_asset.skeleton, bone_id))
    else {
        return message_section("Bone", true, format!("Bone {bone_id} not found"));
    };

    rows_section(
        vec![UiRow::Group {
            title: "Bone".into(),
            initially_open: true,
            rows: bone_rows(),
        }],
        view,
        commit_bone_view,
    )
}

fn read_bone_view(skeleton: &Skeleton, bone_id: BoneId) -> Option<BoneView> {
    let bone = skeleton.get_bone(bone_id)?;
    let parent_name = bone
        .parent_id
        .and_then(|parent| skeleton.get_bone(parent))
        .map_or("(root)", |parent| parent.name.as_str())
        .to_string();

    Some(BoneView {
        bone_id,
        name: bone.name.clone(),
        parent_name,
        offset: bone.local_transform.w.truncate().into(),
        child_name: default_child_name(skeleton, bone),
        child_offset: default_child_offset(bone),
    })
}

fn default_child_offset(bone: &Bone) -> Vector3<f32> {
    let own_offset = bone.local_transform.w.truncate();
    if own_offset.magnitude2() > 0.0 {
        own_offset
    } else {
        Vector3::new(0.0, 0.1, 0.0)
    }
}

fn default_child_name(skeleton: &Skeleton, bone: &Bone) -> String {
    (1..)
        .map(|index| format!("{}_{:02}", bone.name, index))
        .find(|candidate| !skeleton.bone_name_to_id.contains_key(candidate))
        .expect("the name sequence is unbounded")
}

pub fn bone_rows() -> Vec<UiRow<BoneView>> {
    vec![
        UiRow::Canvas(|ui, _, view: &BoneView| {
            ui.text(format!("Bone: {}", view.name));
            ui.text_disabled(format!("Parent: {}", view.parent_name));
        }),
        UiRow::Vector3 {
            label: "Offset From Parent",
            speed: 0.01,
            format: "%.3f",
            get: |view: &BoneView| view.offset,
            set: |view: &mut BoneView, offset| view.offset = offset,
        },
        UiRow::Action {
            label: "Add Child Bone",
            enabled: |_, _| true,
            on_click: |view: &BoneView| {
                Box::new(HierarchyEvent::AddChildBone {
                    parent: view.bone_id,
                    name: view.child_name.clone(),
                    local_offset: view.child_offset,
                })
            },
        },
        UiRow::Action {
            label: "Delete Bone",
            enabled: |_, _| true,
            on_click: |view: &BoneView| Box::new(HierarchyEvent::DeleteBone(view.bone_id)),
        },
        UiRow::Tooltip(
            "Children keep their rest pose and move to the parent; skin weights go to the parent",
        ),
    ]
}

fn commit_bone_view(world: &World, shown: &BoneView, view: &BoneView) {
    if view.offset != shown.offset {
        world.send_command(HierarchyEvent::SetBoneOffset(
            view.bone_id,
            view.offset.into(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn view() -> BoneView {
        BoneView {
            bone_id: 3,
            name: "hair_01".to_string(),
            parent_name: "head".to_string(),
            offset: [0.0, 0.1, 0.0],
            child_name: "hair_01_01".to_string(),
            child_offset: Vector3::new(0.0, 0.1, 0.0),
        }
    }

    #[test]
    fn bone_rows_label_order() {
        let rows = bone_rows();
        assert_eq!(
            visible_labels(&rows),
            ["Offset From Parent", "Add Child Bone", "Delete Bone"]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
    }

    #[test]
    fn commit_sends_offset_only_when_changed() {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        let shown = view();

        commit_bone_view(&world, &shown, &shown.clone());
        assert!(world.resource::<UiCommandQueue>().is_empty());

        let mut edited = shown.clone();
        edited.offset[1] += 0.05;
        commit_bone_view(&world, &shown, &edited);
        let names: Vec<String> = world
            .resource_mut::<UiCommandQueue>()
            .drain()
            .map(|command| format!("{command:?}"))
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].starts_with("SetBoneOffset"), "{names:?}");
    }
}
