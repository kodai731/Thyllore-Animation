use crate::animation::spring_bone::{ChainRamp, RampEnd, RampScope};
use crate::asset::AssetStorage;
use crate::ecs::component::SpringChainId;
use crate::ecs::component::SpringColliderGroupId;
use crate::ecs::component::SpringJointParam;
use crate::ecs::events::UiCommand;
use crate::ecs::world::{Entity, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

#[derive(Debug)]
pub struct ApplyChainRamp {
    pub entity: Entity,
    pub chain_ids: Vec<SpringChainId>,
    pub ramp: ChainRamp,
}

impl UiCommand for ApplyChainRamp {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_edit_systems::handle_spring_chain_ramp_apply;
        handle_spring_chain_ramp_apply(world, self.entity, &self.chain_ids, &self.ramp);
    }
}

#[derive(Debug)]
pub struct LoadChainRamp {
    pub entity: Entity,
    pub chain_id: SpringChainId,
}

impl UiCommand for LoadChainRamp {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_edit_systems::handle_spring_chain_ramp_load;
        handle_spring_chain_ramp_load(world, self.entity, self.chain_id);
    }
}

#[derive(Debug)]
pub struct SetChainRampForm {
    pub ramp: ChainRamp,
    pub scope: RampScope,
}

impl UiCommand for SetChainRampForm {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_edit_systems::handle_spring_chain_ramp_form;
        handle_spring_chain_ramp_form(world, self.ramp, self.scope);
    }
}

#[derive(Debug)]
pub struct ApplyChainColliderGroups {
    pub entity: Entity,
    pub chain_ids: Vec<SpringChainId>,
    pub group_ids: Vec<SpringColliderGroupId>,
}

impl UiCommand for ApplyChainColliderGroups {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_edit_systems::handle_spring_chain_collider_groups_apply;
        handle_spring_chain_collider_groups_apply(
            world,
            self.entity,
            &self.chain_ids,
            &self.group_ids,
        );
    }
}

#[derive(Debug)]
pub struct LoadChainColliderGroups {
    pub entity: Entity,
    pub chain_id: SpringChainId,
}

impl UiCommand for LoadChainColliderGroups {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_edit_systems::handle_spring_chain_collider_groups_load;
        handle_spring_chain_collider_groups_load(world, self.entity, self.chain_id);
    }
}

#[derive(Debug)]
pub struct SetChainColliderGroupForm {
    pub group_ids: Vec<SpringColliderGroupId>,
    pub scope: RampScope,
}

impl UiCommand for SetChainColliderGroupForm {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_edit_systems::handle_spring_chain_collider_groups_form;
        handle_spring_chain_collider_groups_form(world, self.group_ids, self.scope);
    }
}

#[derive(Debug)]
pub struct SetChainRampEnd {
    pub end: RampEnd,
    pub joint: SpringJointParam,
}

impl UiCommand for SetChainRampEnd {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        use crate::ecs::systems::spring_bone_edit_systems::handle_spring_chain_ramp_set_end;
        handle_spring_chain_ramp_set_end(world, self.end, self.joint);
    }
}
