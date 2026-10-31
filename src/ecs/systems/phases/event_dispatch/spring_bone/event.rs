use super::bake::dispatch_spring_bone_bake_events;
use crate::animation::spring_bone::{JointForce, SpringForceModel};
use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::ColliderShape;
use crate::ecs::component::SpringBoneSetup;
use crate::ecs::component::SpringChain;
use crate::ecs::component::SpringChainId;
use crate::ecs::component::SpringColliderDef;
use crate::ecs::component::SpringColliderGroup;
use crate::ecs::component::SpringColliderGroupId;
use crate::ecs::component::SpringColliderId;
use crate::ecs::component::SpringJointParam;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::gizmo::{SpringBoneGizmoData, SpringChainGizmoMode};
use crate::ecs::world::{Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum SpringBoneEvent {
    Bake,
    DiscardBake,
    Rebake,
    ChainAdd {
        entity: Entity,
        root_bone_id: BoneId,
        chain_length: u32,
    },
    ChainRemove {
        entity: Entity,
        chain_id: SpringChainId,
    },
    ChainUpdate {
        entity: Entity,
        chain_id: SpringChainId,
        chain: SpringChain,
    },
    JointUpdate {
        entity: Entity,
        chain_id: SpringChainId,
        joint_index: usize,
        joint: SpringJointParam,
    },
    ColliderAdd {
        entity: Entity,
        bone_id: BoneId,
        shape: ColliderShape,
    },
    ColliderRemove {
        entity: Entity,
        collider_id: SpringColliderId,
    },
    ColliderUpdate {
        entity: Entity,
        collider_id: SpringColliderId,
        collider: SpringColliderDef,
    },
    ColliderGroupAdd {
        entity: Entity,
        name: String,
    },
    ColliderGroupRemove {
        entity: Entity,
        group_id: SpringColliderGroupId,
    },
    ColliderGroupUpdate {
        entity: Entity,
        group_id: SpringColliderGroupId,
        group: SpringColliderGroup,
    },
    ToggleColliderGizmo(bool),
    SetChainGizmoMode(SpringChainGizmoMode),
}

impl UiCommand for SpringBoneEvent {
    fn apply(self: Box<Self>, world: &mut World, assets: &mut AssetStorage, _: &GraphicsResources) {
        let events = [*self];
        dispatch_spring_bone_bake_events(&events, world, assets);
        dispatch_spring_bone_edit_events(&events, world, assets);
    }
}

fn dispatch_spring_bone_edit_events(
    events: &[SpringBoneEvent],
    world: &mut World,
    assets: &AssetStorage,
) {
    use crate::ecs::systems::spring_bone_edit_systems::*;

    let skeleton = assets
        .skeletons
        .values()
        .next()
        .map(|sa| sa.skeleton.clone());

    for event in events {
        match event {
            SpringBoneEvent::ChainAdd {
                entity,
                root_bone_id,
                chain_length,
            } => {
                if let Some(ref skel) = skeleton {
                    handle_spring_chain_add(world, *entity, *root_bone_id, *chain_length, skel);
                }
            }

            SpringBoneEvent::ChainRemove { entity, chain_id } => {
                handle_spring_chain_remove(world, *entity, *chain_id);
            }

            SpringBoneEvent::ChainUpdate {
                entity,
                chain_id,
                chain,
            } => {
                handle_spring_chain_update(world, *entity, *chain_id, chain.clone());
            }

            SpringBoneEvent::JointUpdate {
                entity,
                chain_id,
                joint_index,
                joint,
            } => {
                handle_spring_joint_update(world, *entity, *chain_id, *joint_index, joint.clone());
            }

            SpringBoneEvent::ColliderAdd {
                entity,
                bone_id,
                shape,
            } => {
                handle_spring_collider_add(world, *entity, *bone_id, shape.clone());
            }

            SpringBoneEvent::ColliderRemove {
                entity,
                collider_id,
            } => {
                handle_spring_collider_remove(world, *entity, *collider_id);
            }

            SpringBoneEvent::ColliderUpdate {
                entity,
                collider_id,
                collider,
            } => {
                handle_spring_collider_update(world, *entity, *collider_id, collider.clone());
            }

            SpringBoneEvent::ColliderGroupAdd { entity, name } => {
                handle_spring_collider_group_add(world, *entity, name.clone());
            }

            SpringBoneEvent::ColliderGroupRemove { entity, group_id } => {
                handle_spring_collider_group_remove(world, *entity, *group_id);
            }

            SpringBoneEvent::ColliderGroupUpdate {
                entity,
                group_id,
                group,
            } => {
                handle_spring_collider_group_update(world, *entity, *group_id, group.clone());
            }

            SpringBoneEvent::ToggleColliderGizmo(visible) => {
                if let Some(mut gizmo) = world.get_resource_mut::<SpringBoneGizmoData>() {
                    gizmo.colliders_visible = *visible;
                }
            }

            SpringBoneEvent::SetChainGizmoMode(mode) => {
                if let Some(mut gizmo) = world.get_resource_mut::<SpringBoneGizmoData>() {
                    gizmo.chain_mode = *mode;
                }
            }

            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct SetSpringForceModel {
    pub force_model: SpringForceModel,
}

impl UiCommand for SetSpringForceModel {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::avatar_setup_systems::find_or_create_spring_bone_setup_entity;
        use crate::ecs::systems::spring_bone_edit_systems::reset_spring_bone_state;

        let Some(entity) = find_or_create_spring_bone_setup_entity(world) else {
            msg_warn!("Spring force model: no animated model entity");
            return;
        };
        if let Some(mut setup) = world.get_component_mut::<SpringBoneSetup>(entity) {
            setup.new_joint_force_model = self.force_model;
            for joint in setup.chains.iter_mut().flat_map(|chain| &mut chain.joints) {
                if joint.force.model() != self.force_model {
                    joint.force = JointForce::default_for(self.force_model);
                }
            }
        }
        reset_spring_bone_state(world);
    }
}
