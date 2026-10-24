use std::rc::Rc;

use thyllore_anim_core::spring_bone::SpringChainId;

use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::{SpringBoneSetup, WithSpringBone};
use crate::ecs::systems::chains_skinned_to_entity;
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::world::{Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::super::bone_choice::collect_bone_list;
use super::super::form_window::{rows_section, FormSection};
use super::super::ui_rows::{ListRows, UiRow};

#[derive(Clone, Debug, PartialEq)]
pub struct SkinnedChain {
    pub chain_id: SpringChainId,
    pub name: String,
    pub root_bone: BoneId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshSpringView {
    pub chains: Vec<SkinnedChain>,
    /// The one bone every skinned chain hangs from, where a ramp reaches all of them at once
    pub chain_parent: Option<BoneId>,
    pub bones: Rc<[(BoneId, String)]>,
}

impl MeshSpringView {
    pub fn bone_name(&self, bone_id: BoneId) -> String {
        self.bones
            .iter()
            .find(|(id, _)| *id == bone_id)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| format!("bone {bone_id}"))
    }
}

/// The chains that move the selected mesh, with a jump to the bone whose Spring section edits them.
pub fn mesh_spring_section(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    entity: Entity,
) -> Option<Box<dyn FormSection>> {
    let spring_entity = world
        .iter_components::<WithSpringBone>()
        .next()
        .map(|(entity, _)| entity)?;
    let setup = world.get_component::<SpringBoneSetup>(spring_entity)?;
    let chain_ids = chains_skinned_to_entity(world, assets, graphics, &setup, entity);
    if chain_ids.is_empty() {
        return None;
    }

    let chains: Vec<SkinnedChain> = setup
        .chains
        .iter()
        .filter(|chain| chain_ids.contains(&chain.id))
        .filter_map(|chain| {
            Some(SkinnedChain {
                chain_id: chain.id,
                name: chain.name.clone(),
                root_bone: chain.joints.first()?.bone_id,
            })
        })
        .collect();
    let skeleton = assets.skeletons.values().next().map(|s| &s.skeleton);
    let view = MeshSpringView {
        chain_parent: common_parent(skeleton, chains.iter().map(|chain| chain.root_bone)),
        chains,
        bones: collect_bone_list(assets).into(),
    };

    Some(rows_section(
        vec![UiRow::Group {
            title: "Spring Chains".into(),
            initially_open: true,
            rows: mesh_spring_rows(),
        }],
        view,
        |_, _, _| {},
    ))
}

fn common_parent(
    skeleton: Option<&crate::animation::Skeleton>,
    roots: impl Iterator<Item = BoneId>,
) -> Option<BoneId> {
    let skeleton = skeleton?;
    let mut parent = None;
    for root in roots {
        let root_parent = skeleton.get_bone(root)?.parent_id?;
        match parent {
            None => parent = Some(root_parent),
            Some(seen) if seen == root_parent => {}
            Some(_) => return None,
        }
    }
    parent
}

pub fn mesh_spring_rows() -> Vec<UiRow<MeshSpringView>> {
    vec![
        UiRow::Canvas(|ui, _, view: &MeshSpringView| {
            let parent = view
                .chain_parent
                .map(|bone| view.bone_name(bone))
                .unwrap_or_else(|| "mixed".to_string());
            ui.text(&format!("{} chains under {}", view.chains.len(), parent));
        }),
        UiRow::Action {
            label: "Select Bone Above Chains",
            enabled: |_, view: &MeshSpringView| view.chain_parent.is_some(),
            on_click: |view: &MeshSpringView| {
                let bone = view.chain_parent.unwrap_or_default();
                Box::new(HierarchyEvent::RevealBone(bone))
            },
        },
        UiRow::Tooltip(
            "Opens the bone whose Spring section ramps every chain of this mesh at once",
        ),
        UiRow::List(Box::new(ListRows::<MeshSpringView, SkinnedChain> {
            title: "Chains",
            items: |_, view: &MeshSpringView| {
                view.chains
                    .iter()
                    .map(|chain| {
                        let header =
                            format!("{} (root: {})", chain.name, view.bone_name(chain.root_bone));
                        (header, chain.clone())
                    })
                    .collect()
            },
            rows: |_| skinned_chain_rows(),
            on_change: |_, _, _| unreachable!(),
            initially_open: |_| false,
        })),
    ]
}

fn skinned_chain_rows() -> Vec<UiRow<SkinnedChain>> {
    vec![UiRow::Action {
        label: "Select Root Bone",
        enabled: |_, _: &SkinnedChain| true,
        on_click: |chain: &SkinnedChain| Box::new(HierarchyEvent::RevealBone(chain.root_bone)),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{Bone, Skeleton};
    use crate::platform::ui::ui_rows::{find_duplicate_labels, visible_labels};

    fn skeleton(parents: &[Option<BoneId>]) -> Skeleton {
        let mut skeleton = Skeleton::default();
        for (id, parent_id) in parents.iter().enumerate() {
            skeleton.bones.push(Bone {
                id: id as BoneId,
                name: format!("b{id}"),
                parent_id: *parent_id,
                ..Bone::default()
            });
        }
        skeleton
    }

    #[test]
    fn mesh_spring_rows_label_order_and_no_duplicates() {
        let rows = mesh_spring_rows();
        assert_eq!(
            visible_labels(&rows),
            ["Select Bone Above Chains", "Chains"]
        );
        assert!(find_duplicate_labels(&rows).is_empty());
        assert_eq!(visible_labels(&skinned_chain_rows()), ["Select Root Bone"]);
    }

    #[test]
    fn common_parent_is_the_bone_shared_by_every_root() {
        let skeleton = skeleton(&[None, Some(0), Some(0), Some(1)]);
        assert_eq!(common_parent(Some(&skeleton), [1, 2].into_iter()), Some(0));
        assert_eq!(common_parent(Some(&skeleton), [1, 3].into_iter()), None);
        assert_eq!(common_parent(Some(&skeleton), [0].into_iter()), None);
        assert_eq!(common_parent(None, [1].into_iter()), None);
    }
}
