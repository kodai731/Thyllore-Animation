use crate::animation::BoneId;
use crate::asset::AssetStorage;
use crate::ecs::component::{
    ColliderShape, SpringBoneSetup, SpringChain, SpringColliderDef, SpringColliderGroup,
    SpringJointParam, WithSpringBone,
};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::HierarchyState;
use crate::ecs::world::{Entity, World};

use super::constraint_inspector::{
    build_bone_combo, build_bone_combo_with_select, build_offset_vector3, collect_bone_list,
};

pub fn build_spring_bone_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    world: &World,
    _entity: Entity,
    assets: &AssetStorage,
    hierarchy_state: &HierarchyState,
) {
    if !ui.collapsing_header("Spring Bones", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        return;
    }

    let target_entity = find_spring_bone_entity(world);
    let Some(target_entity) = target_entity else {
        ui.text("No spring bone entity");
        return;
    };

    let bone_list = collect_bone_list(assets);

    let setup = match world.get_component::<SpringBoneSetup>(target_entity) {
        Some(s) => s.clone(),
        None => {
            ui.text("No SpringBoneSetup");
            return;
        }
    };

    build_chain_list(
        ui,
        ui_events,
        target_entity,
        &setup,
        &bone_list,
        hierarchy_state,
    );

    ui.separator();
    build_add_chain_row(ui, ui_events, target_entity, &bone_list, hierarchy_state);

    ui.separator();
    build_collider_list(
        ui,
        ui_events,
        target_entity,
        &setup,
        &bone_list,
        hierarchy_state,
    );

    ui.separator();
    build_add_collider_row(ui, ui_events, target_entity, &bone_list);

    ui.separator();
    build_collider_group_list(ui, ui_events, target_entity, &setup);

    ui.separator();
    build_add_collider_group_row(ui, ui_events, target_entity);

    ui.separator();
    build_gizmo_toggle(ui, ui_events, world);
}

fn find_spring_bone_entity(world: &World) -> Option<Entity> {
    world
        .iter_components::<WithSpringBone>()
        .next()
        .map(|(e, _)| e)
}

fn build_chain_list(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    setup: &SpringBoneSetup,
    bone_list: &[(BoneId, String)],
    hierarchy_state: &HierarchyState,
) {
    for chain in &setup.chains {
        let header = format!(
            "Chain: {} (id:{})###spring_chain_{}",
            chain.name, chain.id, chain.id
        );
        if !ui.collapsing_header(&header, imgui::TreeNodeFlags::empty()) {
            continue;
        }

        let id_token = ui.push_id_int(chain.id as i32);
        build_chain_detail(ui, ui_events, entity, chain, bone_list, hierarchy_state);
        id_token.end();
    }
}

fn build_chain_detail(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    chain: &SpringChain,
    bone_list: &[(BoneId, String)],
    hierarchy_state: &HierarchyState,
) {
    let mut modified = chain.clone();
    let mut chain_changed = false;

    if ui.checkbox("Enabled##chain", &mut modified.enabled) {
        chain_changed = true;
    }

    for (joint_idx, joint) in chain.joints.iter().enumerate() {
        let joint_header = format!(
            "Joint {} (bone:{})###joint_{}",
            joint_idx, joint.bone_id, joint_idx
        );
        if !ui.collapsing_header(&joint_header, imgui::TreeNodeFlags::empty()) {
            continue;
        }

        let joint_token = ui.push_id_int(joint_idx as i32 + 1000);
        if let Some(updated) = build_joint_fields(ui, ui_events, joint, bone_list, hierarchy_state)
        {
            ui_events.send(UIEvent::SpringJointUpdate {
                entity,
                chain_id: chain.id,
                joint_index: joint_idx,
                joint: updated,
            });
        }
        joint_token.end();
    }

    if chain_changed {
        ui_events.send(UIEvent::SpringChainUpdate {
            entity,
            chain_id: chain.id,
            chain: modified,
        });
    }

    if ui.button("Remove Chain") {
        ui_events.send(UIEvent::SpringChainRemove {
            entity,
            chain_id: chain.id,
        });
    }
}

fn build_joint_fields(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    joint: &SpringJointParam,
    bone_list: &[(BoneId, String)],
    hierarchy_state: &HierarchyState,
) -> Option<SpringJointParam> {
    let mut modified = joint.clone();
    let mut changed = false;

    if let Some(bone) = build_bone_combo_with_select(
        ui,
        ui_events,
        "Bone",
        modified.bone_id,
        bone_list,
        hierarchy_state,
    ) {
        modified.bone_id = bone;
        changed = true;
    }

    ui.set_next_item_width(-1.0);
    if ui
        .slider_config("Stiffness", 0.0_f32, 4.0)
        .build(&mut modified.stiffness)
    {
        changed = true;
    }

    ui.set_next_item_width(-1.0);
    if ui
        .slider_config("Drag Force", 0.0_f32, 1.0)
        .build(&mut modified.drag_force)
    {
        changed = true;
    }

    ui.set_next_item_width(-1.0);
    if ui
        .slider_config("Gravity Power", 0.0_f32, 2.0)
        .build(&mut modified.gravity_power)
    {
        changed = true;
    }

    if build_offset_vector3(ui, "Gravity Dir", &mut modified.gravity_dir) {
        changed = true;
    }

    ui.set_next_item_width(-1.0);
    if ui
        .slider_config("Hit Radius", 0.0_f32, 0.5)
        .build(&mut modified.hit_radius)
    {
        changed = true;
    }

    if changed {
        Some(modified)
    } else {
        None
    }
}

fn build_add_chain_row(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    bone_list: &[(BoneId, String)],
    hierarchy_state: &HierarchyState,
) {
    ui.text("Add Chain");

    static mut ADD_CHAIN_ROOT_BONE: BoneId = 0;
    static mut ADD_CHAIN_LENGTH: i32 = 3;

    let (root_bone, chain_length) = unsafe { (&mut ADD_CHAIN_ROOT_BONE, &mut ADD_CHAIN_LENGTH) };

    if let Some(bone) = build_bone_combo_with_select(
        ui,
        ui_events,
        "Root Bone##add_chain",
        *root_bone,
        bone_list,
        hierarchy_state,
    ) {
        *root_bone = bone;
    }

    ui.set_next_item_width(100.0);
    ui.input_int("Chain Length##add", chain_length)
        .step(1)
        .build();
    *chain_length = (*chain_length).max(1);

    ui.same_line();
    if ui.button("Add Chain") {
        ui_events.send(UIEvent::SpringChainAdd {
            entity,
            root_bone_id: *root_bone,
            chain_length: *chain_length as u32,
        });
    }
}

fn build_collider_list(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    setup: &SpringBoneSetup,
    bone_list: &[(BoneId, String)],
    hierarchy_state: &HierarchyState,
) {
    ui.text("Colliders");

    for collider in &setup.colliders {
        let shape_name = match &collider.shape {
            ColliderShape::Sphere { .. } => "Sphere",
            ColliderShape::Capsule { .. } => "Capsule",
        };
        let header = format!(
            "{} (id:{}, bone:{})###collider_{}",
            shape_name, collider.id, collider.bone_id, collider.id
        );
        if !ui.collapsing_header(&header, imgui::TreeNodeFlags::empty()) {
            continue;
        }

        let id_token = ui.push_id_int(collider.id as i32 + 2000);
        if let Some(updated) =
            build_collider_fields(ui, ui_events, collider, bone_list, hierarchy_state)
        {
            ui_events.send(UIEvent::SpringColliderUpdate {
                entity,
                collider_id: collider.id,
                collider: updated,
            });
        }

        if ui.button("Remove Collider") {
            ui_events.send(UIEvent::SpringColliderRemove {
                entity,
                collider_id: collider.id,
            });
        }
        id_token.end();
    }
}

fn build_collider_fields(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    collider: &SpringColliderDef,
    bone_list: &[(BoneId, String)],
    hierarchy_state: &HierarchyState,
) -> Option<SpringColliderDef> {
    let mut modified = collider.clone();
    let mut changed = false;

    if let Some(bone) = build_bone_combo_with_select(
        ui,
        ui_events,
        "Bone##collider",
        modified.bone_id,
        bone_list,
        hierarchy_state,
    ) {
        modified.bone_id = bone;
        changed = true;
    }

    if build_offset_vector3(ui, "Offset##collider", &mut modified.offset) {
        changed = true;
    }

    let shape_names = ["Sphere", "Capsule"];
    let mut current_shape_idx: i32 = match &modified.shape {
        ColliderShape::Sphere { .. } => 0,
        ColliderShape::Capsule { .. } => 1,
    };

    ui.set_next_item_width(-1.0);
    let current_label = shape_names[current_shape_idx as usize];
    if let Some(token) = ui.begin_combo("Shape", current_label) {
        for (i, name) in shape_names.iter().enumerate() {
            let selected = i as i32 == current_shape_idx;
            if ui.selectable_config(name).selected(selected).build() {
                if i as i32 != current_shape_idx {
                    current_shape_idx = i as i32;
                    modified.shape = match i {
                        0 => ColliderShape::Sphere { radius: 0.1 },
                        _ => ColliderShape::Capsule {
                            radius: 0.1,
                            tail: cgmath::Vector3::new(0.0, 0.1, 0.0),
                        },
                    };
                    changed = true;
                }
            }
        }
        token.end();
    }

    match &mut modified.shape {
        ColliderShape::Sphere { radius } => {
            ui.set_next_item_width(-1.0);
            if ui.slider_config("Radius", 0.001_f32, 1.0).build(radius) {
                changed = true;
            }
        }
        ColliderShape::Capsule { radius, tail } => {
            ui.set_next_item_width(-1.0);
            if ui
                .slider_config("Radius##cap", 0.001_f32, 1.0)
                .build(radius)
            {
                changed = true;
            }
            if build_offset_vector3(ui, "Tail", tail) {
                changed = true;
            }
        }
    }

    if changed {
        Some(modified)
    } else {
        None
    }
}

fn build_add_collider_row(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    bone_list: &[(BoneId, String)],
) {
    static mut ADD_COLLIDER_BONE: BoneId = 0;
    static mut ADD_COLLIDER_SHAPE_IDX: i32 = 0;

    let (bone, shape_idx) = unsafe { (&mut ADD_COLLIDER_BONE, &mut ADD_COLLIDER_SHAPE_IDX) };

    let current_name = bone_list
        .iter()
        .find(|(id, _)| *id == *bone)
        .map(|(_, name)| name.as_str())
        .unwrap_or("(none)");

    ui.text("Add Collider");
    ui.set_next_item_width(-1.0);
    if let Some(token) = ui.begin_combo("Bone##add_collider", current_name) {
        for (bone_id, bone_name) in bone_list {
            let selected = *bone_id == *bone;
            if ui.selectable_config(bone_name).selected(selected).build() {
                *bone = *bone_id;
            }
        }
        token.end();
    }

    let shape_names = ["Sphere", "Capsule"];
    ui.set_next_item_width(100.0);
    let label = shape_names[*shape_idx as usize];
    if let Some(token) = ui.begin_combo("Shape##add_collider", label) {
        for (i, name) in shape_names.iter().enumerate() {
            let selected = i as i32 == *shape_idx;
            if ui.selectable_config(name).selected(selected).build() {
                *shape_idx = i as i32;
            }
        }
        token.end();
    }

    ui.same_line();
    if ui.button("Add Collider") {
        let shape = match *shape_idx {
            0 => ColliderShape::Sphere { radius: 0.1 },
            _ => ColliderShape::Capsule {
                radius: 0.1,
                tail: cgmath::Vector3::new(0.0, 0.1, 0.0),
            },
        };
        ui_events.send(UIEvent::SpringColliderAdd {
            entity,
            bone_id: *bone,
            shape,
        });
    }
}

fn build_collider_group_list(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    setup: &SpringBoneSetup,
) {
    ui.text("Collider Groups");

    for group in &setup.collider_groups {
        let header = format!(
            "Group: {} (id:{})###group_{}",
            group.name, group.id, group.id
        );
        if !ui.collapsing_header(&header, imgui::TreeNodeFlags::empty()) {
            continue;
        }

        let id_token = ui.push_id_int(group.id as i32 + 3000);

        let mut modified = group.clone();
        let mut changed = false;

        ui.set_next_item_width(-1.0);
        if ui.input_text("Name##group", &mut modified.name).build() {
            changed = true;
        }

        ui.text(&format!("Colliders: {:?}", modified.collider_ids));

        let available: Vec<_> = setup
            .colliders
            .iter()
            .filter(|c| !modified.collider_ids.contains(&c.id))
            .map(|c| c.id)
            .collect();

        if !available.is_empty() {
            let preview = format!("Add collider ({})", available.len());
            if let Some(token) = ui.begin_combo("##add_to_group", &preview) {
                for cid in &available {
                    if ui.selectable_config(&format!("Collider {}", cid)).build() {
                        modified.collider_ids.push(*cid);
                        changed = true;
                    }
                }
                token.end();
            }
        }

        let mut remove_idx = None;
        for (i, cid) in modified.collider_ids.iter().enumerate() {
            ui.same_line();
            if ui.small_button(&format!("x##grp_rm_{}", cid)) {
                remove_idx = Some(i);
            }
        }
        if let Some(idx) = remove_idx {
            modified.collider_ids.remove(idx);
            changed = true;
        }

        if changed {
            ui_events.send(UIEvent::SpringColliderGroupUpdate {
                entity,
                group_id: group.id,
                group: modified,
            });
        }

        if ui.button("Remove Group") {
            ui_events.send(UIEvent::SpringColliderGroupRemove {
                entity,
                group_id: group.id,
            });
        }

        id_token.end();
    }
}

fn build_add_collider_group_row(ui: &imgui::Ui, ui_events: &mut UIEventQueue, entity: Entity) {
    static mut GROUP_NAME_BUF: Option<String> = None;

    let name_buf = unsafe {
        if GROUP_NAME_BUF.is_none() {
            GROUP_NAME_BUF = Some("NewGroup".to_string());
        }
        GROUP_NAME_BUF.as_mut().unwrap()
    };

    ui.set_next_item_width(150.0);
    ui.input_text("##new_group_name", name_buf).build();

    ui.same_line();
    if ui.button("Add Group") {
        ui_events.send(UIEvent::SpringColliderGroupAdd {
            entity,
            name: name_buf.to_string(),
        });
    }
}

fn build_gizmo_toggle(ui: &imgui::Ui, ui_events: &mut UIEventQueue, world: &World) {
    let current = world
        .get_resource::<crate::debugview::gizmo::SpringBoneGizmoData>()
        .map(|g| g.visible)
        .unwrap_or(false);

    let mut visible = current;
    if ui.checkbox("Show Collider Gizmos", &mut visible) {
        ui_events.send(UIEvent::SpringBoneToggleGizmo(visible));
    }
}
