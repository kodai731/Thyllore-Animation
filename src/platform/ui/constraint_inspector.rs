use crate::animation::{
    AimConstraintData, BoneId, ConstraintType,
    IkConstraintData, ParentConstraintData,
    PositionConstraintData, RotationConstraintData,
    ScaleConstraintData,
};
use crate::asset::AssetStorage;
use crate::ecs::component::{ConstraintEntry, ConstraintSet};
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::world::{Animator, Entity, World};

const CONSTRAINT_TYPE_NAMES: &[&str] = &[
    "IK",
    "Aim",
    "Parent",
    "Position",
    "Rotation",
    "Scale",
];

pub fn build_constraint_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    world: &World,
    _entity: Entity,
    assets: &AssetStorage,
    add_type_index: &mut i32,
) {
    if !ui.collapsing_header(
        "Constraints",
        imgui::TreeNodeFlags::DEFAULT_OPEN,
    ) {
        return;
    }

    let target_entity =
        find_animated_entity(world);
    let Some(target_entity) = target_entity else {
        ui.text("No animated entity");
        return;
    };

    let bone_list = collect_bone_list(assets);

    build_add_constraint_row(
        ui,
        ui_events,
        target_entity,
        add_type_index,
    );

    let constraint_set =
        world.get_component::<ConstraintSet>(target_entity);
    let Some(set) = constraint_set else {
        ui.text("No constraints");
        return;
    };

    let entries: Vec<_> = set.constraints.clone();
    for entry in &entries {
        build_constraint_entry(
            ui,
            ui_events,
            target_entity,
            entry,
            &bone_list,
        );
    }
}

fn find_animated_entity(world: &World) -> Option<Entity> {
    world
        .component_entities::<Animator>()
        .into_iter()
        .next()
}

fn build_add_constraint_row(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    add_type_index: &mut i32,
) {
    ui.set_next_item_width(120.0);
    let current_label =
        CONSTRAINT_TYPE_NAMES[*add_type_index as usize];
    if let Some(token) =
        ui.begin_combo("##constraint_type", current_label)
    {
        for (i, name) in
            CONSTRAINT_TYPE_NAMES.iter().enumerate()
        {
            let selected = i as i32 == *add_type_index;
            if ui
                .selectable_config(name)
                .selected(selected)
                .build()
            {
                *add_type_index = i as i32;
            }
        }
        token.end();
    }

    ui.same_line();
    if ui.button("Add") {
        ui_events.send(UIEvent::ConstraintAdd {
            entity,
            constraint_type_index: *add_type_index as u8,
        });
    }

    ui.separator();
}

fn build_constraint_entry(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    entity: Entity,
    entry: &ConstraintEntry,
    bone_list: &[(BoneId, String)],
) {
    let type_name = constraint_type_name(&entry.constraint);
    let header_label = format!(
        "[{}] {} (id:{})###constraint_{}",
        entry.priority, type_name, entry.id, entry.id
    );

    let opened = ui.collapsing_header(
        &header_label,
        imgui::TreeNodeFlags::empty(),
    );

    if !opened {
        return;
    }

    let id_token = ui.push_id_int(entry.id as i32);

    let changed = match &entry.constraint {
        ConstraintType::Ik(data) => {
            build_ik_fields(ui, data, bone_list)
        }
        ConstraintType::Aim(data) => {
            build_aim_fields(ui, data, bone_list)
        }
        ConstraintType::Parent(data) => {
            build_parent_fields(ui, data, bone_list)
        }
        ConstraintType::Position(data) => {
            build_position_fields(ui, data, bone_list)
        }
        ConstraintType::Rotation(data) => {
            build_rotation_fields(ui, data, bone_list)
        }
        ConstraintType::Scale(data) => {
            build_scale_fields(ui, data, bone_list)
        }
    };

    if let Some(updated_constraint) = changed {
        ui_events.send(UIEvent::ConstraintUpdate {
            entity,
            constraint_id: entry.id,
            constraint: updated_constraint,
        });
    }

    if ui.button("Remove") {
        ui_events.send(UIEvent::ConstraintRemove {
            entity,
            constraint_id: entry.id,
        });
    }

    id_token.end();
}

fn build_common_fields(
    ui: &imgui::Ui,
    enabled: &mut bool,
    weight: &mut f32,
) -> bool {
    let mut changed = false;

    if ui.checkbox("Enabled", enabled) {
        changed = true;
    }

    ui.set_next_item_width(-1.0);
    if ui.slider_config("Weight", 0.0, 1.0).build(weight) {
        changed = true;
    }

    changed
}

fn build_ik_fields(
    ui: &imgui::Ui,
    data: &IkConstraintData,
    bone_list: &[(BoneId, String)],
) -> Option<ConstraintType> {
    let mut modified = data.clone();
    let mut changed = build_common_fields(
        ui,
        &mut modified.enabled,
        &mut modified.weight,
    );

    if let Some(bone) = build_bone_combo(
        ui,
        "Effector",
        modified.effector_bone,
        bone_list,
    ) {
        modified.effector_bone = bone;
        changed = true;
    }

    if let Some(bone) = build_bone_combo(
        ui,
        "Target",
        modified.target_bone,
        bone_list,
    ) {
        modified.target_bone = bone;
        changed = true;
    }

    let mut chain = modified.chain_length as i32;
    ui.set_next_item_width(-1.0);
    if ui
        .input_int("Chain Length", &mut chain)
        .step(1)
        .build()
    {
        modified.chain_length = chain.max(1) as u32;
        changed = true;
    }

    let pole = modified
        .pole_vector
        .unwrap_or(cgmath::Vector3::new(0.0, 0.0, 1.0));
    let mut pole_arr = [pole.x, pole.y, pole.z];
    ui.text("Pole Vector");
    ui.set_next_item_width(-1.0);
    if ui.input_float3("##pole_vector", &mut pole_arr).build() {
        modified.pole_vector = Some(cgmath::Vector3::new(
            pole_arr[0],
            pole_arr[1],
            pole_arr[2],
        ));
        changed = true;
    }

    ui.set_next_item_width(-1.0);
    if ui
        .slider_config("Twist", -180.0_f32, 180.0)
        .build(&mut modified.twist)
    {
        changed = true;
    }

    if changed {
        Some(ConstraintType::Ik(modified))
    } else {
        None
    }
}

fn build_aim_fields(
    ui: &imgui::Ui,
    data: &AimConstraintData,
    bone_list: &[(BoneId, String)],
) -> Option<ConstraintType> {
    let mut modified = data.clone();
    let mut changed = build_common_fields(
        ui,
        &mut modified.enabled,
        &mut modified.weight,
    );

    if let Some(bone) = build_bone_combo(
        ui,
        "Source",
        modified.source_bone,
        bone_list,
    ) {
        modified.source_bone = bone;
        changed = true;
    }

    if let Some(bone) = build_bone_combo(
        ui,
        "Target",
        modified.target_bone,
        bone_list,
    ) {
        modified.target_bone = bone;
        changed = true;
    }

    let mut aim = [
        modified.aim_axis.x,
        modified.aim_axis.y,
        modified.aim_axis.z,
    ];
    ui.text("Aim Axis");
    ui.set_next_item_width(-1.0);
    if ui.input_float3("##aim_axis", &mut aim).build() {
        modified.aim_axis =
            cgmath::Vector3::new(aim[0], aim[1], aim[2]);
        changed = true;
    }

    let mut up = [
        modified.up_axis.x,
        modified.up_axis.y,
        modified.up_axis.z,
    ];
    ui.text("Up Axis");
    ui.set_next_item_width(-1.0);
    if ui.input_float3("##up_axis", &mut up).build() {
        modified.up_axis =
            cgmath::Vector3::new(up[0], up[1], up[2]);
        changed = true;
    }

    if changed {
        Some(ConstraintType::Aim(modified))
    } else {
        None
    }
}

fn build_parent_fields(
    ui: &imgui::Ui,
    data: &ParentConstraintData,
    bone_list: &[(BoneId, String)],
) -> Option<ConstraintType> {
    let mut modified = data.clone();
    let mut changed = build_common_fields(
        ui,
        &mut modified.enabled,
        &mut modified.weight,
    );

    if let Some(bone) = build_bone_combo(
        ui,
        "Constrained",
        modified.constrained_bone,
        bone_list,
    ) {
        modified.constrained_bone = bone;
        changed = true;
    }

    if ui.checkbox(
        "Affect Translation",
        &mut modified.affect_translation,
    ) {
        changed = true;
    }

    if ui.checkbox(
        "Affect Rotation",
        &mut modified.affect_rotation,
    ) {
        changed = true;
    }

    ui.text("Sources:");
    let mut sources_changed = false;
    let mut new_sources = modified.sources.clone();

    for (i, (bone_id, weight)) in
        modified.sources.iter().enumerate()
    {
        let source_token = ui.push_id_int(i as i32);
        let mut current_weight = *weight;

        if let Some(bone) = build_bone_combo(
            ui,
            &format!("Src {}", i),
            *bone_id,
            bone_list,
        ) {
            new_sources[i].0 = bone;
            sources_changed = true;
        }

        ui.same_line();
        ui.set_next_item_width(60.0);
        if ui
            .slider_config("##src_weight", 0.0_f32, 1.0)
            .build(&mut current_weight)
        {
            new_sources[i].1 = current_weight;
            sources_changed = true;
        }

        ui.same_line();
        if ui.button("X") {
            new_sources.remove(i);
            sources_changed = true;
            source_token.end();
            break;
        }

        source_token.end();
    }

    if sources_changed {
        modified.sources = new_sources;
        changed = true;
    }

    if ui.button("Add Source") {
        modified.sources.push((0, 1.0));
        changed = true;
    }

    if changed {
        Some(ConstraintType::Parent(modified))
    } else {
        None
    }
}

fn build_position_fields(
    ui: &imgui::Ui,
    data: &PositionConstraintData,
    bone_list: &[(BoneId, String)],
) -> Option<ConstraintType> {
    let mut modified = data.clone();
    let mut changed = build_common_fields(
        ui,
        &mut modified.enabled,
        &mut modified.weight,
    );

    if let Some(bone) = build_bone_combo(
        ui,
        "Constrained",
        modified.constrained_bone,
        bone_list,
    ) {
        modified.constrained_bone = bone;
        changed = true;
    }

    if let Some(bone) = build_bone_combo(
        ui,
        "Target",
        modified.target_bone,
        bone_list,
    ) {
        modified.target_bone = bone;
        changed = true;
    }

    if build_offset_vector3(
        ui,
        "Offset",
        &mut modified.offset,
    ) {
        changed = true;
    }

    if build_axes_checkboxes(
        ui,
        "Axes",
        &mut modified.affect_axes,
    ) {
        changed = true;
    }

    if changed {
        Some(ConstraintType::Position(modified))
    } else {
        None
    }
}

fn build_rotation_fields(
    ui: &imgui::Ui,
    data: &RotationConstraintData,
    bone_list: &[(BoneId, String)],
) -> Option<ConstraintType> {
    let mut modified = data.clone();
    let mut changed = build_common_fields(
        ui,
        &mut modified.enabled,
        &mut modified.weight,
    );

    if let Some(bone) = build_bone_combo(
        ui,
        "Constrained",
        modified.constrained_bone,
        bone_list,
    ) {
        modified.constrained_bone = bone;
        changed = true;
    }

    if let Some(bone) = build_bone_combo(
        ui,
        "Target",
        modified.target_bone,
        bone_list,
    ) {
        modified.target_bone = bone;
        changed = true;
    }

    let euler = quaternion_to_euler(&modified.offset);
    let mut euler_arr = [euler.x, euler.y, euler.z];
    ui.text("Offset (Euler)");
    ui.set_next_item_width(-1.0);
    if ui
        .input_float3("##rotation_offset", &mut euler_arr)
        .build()
    {
        modified.offset = euler_to_quaternion(
            &cgmath::Vector3::new(
                euler_arr[0],
                euler_arr[1],
                euler_arr[2],
            ),
        );
        changed = true;
    }

    if build_axes_checkboxes(
        ui,
        "Axes",
        &mut modified.affect_axes,
    ) {
        changed = true;
    }

    if changed {
        Some(ConstraintType::Rotation(modified))
    } else {
        None
    }
}

fn build_scale_fields(
    ui: &imgui::Ui,
    data: &ScaleConstraintData,
    bone_list: &[(BoneId, String)],
) -> Option<ConstraintType> {
    let mut modified = data.clone();
    let mut changed = build_common_fields(
        ui,
        &mut modified.enabled,
        &mut modified.weight,
    );

    if let Some(bone) = build_bone_combo(
        ui,
        "Constrained",
        modified.constrained_bone,
        bone_list,
    ) {
        modified.constrained_bone = bone;
        changed = true;
    }

    if let Some(bone) = build_bone_combo(
        ui,
        "Target",
        modified.target_bone,
        bone_list,
    ) {
        modified.target_bone = bone;
        changed = true;
    }

    if build_offset_vector3(
        ui,
        "Offset",
        &mut modified.offset,
    ) {
        changed = true;
    }

    if build_axes_checkboxes(
        ui,
        "Axes",
        &mut modified.affect_axes,
    ) {
        changed = true;
    }

    if changed {
        Some(ConstraintType::Scale(modified))
    } else {
        None
    }
}

fn build_bone_combo(
    ui: &imgui::Ui,
    label: &str,
    current_bone: BoneId,
    bone_list: &[(BoneId, String)],
) -> Option<BoneId> {
    let current_name = bone_list
        .iter()
        .find(|(id, _)| *id == current_bone)
        .map(|(_, name)| name.as_str())
        .unwrap_or("(none)");

    let mut result = None;

    ui.set_next_item_width(-1.0);
    if let Some(combo_token) =
        ui.begin_combo(&format!("##{}", label), current_name)
    {
        for (bone_id, bone_name) in bone_list {
            let selected = *bone_id == current_bone;
            if ui
                .selectable_config(bone_name)
                .selected(selected)
                .build()
            {
                result = Some(*bone_id);
            }
        }
        combo_token.end();
    }

    ui.same_line();
    ui.text(label);

    result
}

fn build_offset_vector3(
    ui: &imgui::Ui,
    label: &str,
    offset: &mut cgmath::Vector3<f32>,
) -> bool {
    let mut arr = [offset.x, offset.y, offset.z];
    ui.text(label);
    ui.set_next_item_width(-1.0);
    let label_id = format!("##{}_offset", label);
    if ui.input_float3(&label_id, &mut arr).build() {
        offset.x = arr[0];
        offset.y = arr[1];
        offset.z = arr[2];
        return true;
    }
    false
}

fn build_axes_checkboxes(
    ui: &imgui::Ui,
    label: &str,
    axes: &mut [bool; 3],
) -> bool {
    let mut changed = false;

    ui.text(label);
    ui.same_line();
    if ui.checkbox("X##axes", &mut axes[0]) {
        changed = true;
    }
    ui.same_line();
    if ui.checkbox("Y##axes", &mut axes[1]) {
        changed = true;
    }
    ui.same_line();
    if ui.checkbox("Z##axes", &mut axes[2]) {
        changed = true;
    }

    changed
}

fn collect_bone_list(
    assets: &AssetStorage,
) -> Vec<(BoneId, String)> {
    let skeleton = match assets
        .skeletons
        .values()
        .next()
    {
        Some(skel_asset) => &skel_asset.skeleton,
        None => return Vec::new(),
    };

    skeleton
        .bones
        .iter()
        .map(|bone| (bone.id, bone.name.clone()))
        .collect()
}

fn constraint_type_name(ct: &ConstraintType) -> &'static str {
    match ct {
        ConstraintType::Ik(_) => "IK",
        ConstraintType::Aim(_) => "Aim",
        ConstraintType::Parent(_) => "Parent",
        ConstraintType::Position(_) => "Position",
        ConstraintType::Rotation(_) => "Rotation",
        ConstraintType::Scale(_) => "Scale",
    }
}

fn quaternion_to_euler(
    q: &cgmath::Quaternion<f32>,
) -> cgmath::Vector3<f32> {
    let sinr_cosp = 2.0 * (q.s * q.v.x + q.v.y * q.v.z);
    let cosr_cosp =
        1.0 - 2.0 * (q.v.x * q.v.x + q.v.y * q.v.y);
    let roll = sinr_cosp.atan2(cosr_cosp);

    let sinp = 2.0 * (q.s * q.v.y - q.v.z * q.v.x);
    let pitch = if sinp.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(sinp)
    } else {
        sinp.asin()
    };

    let siny_cosp = 2.0 * (q.s * q.v.z + q.v.x * q.v.y);
    let cosy_cosp =
        1.0 - 2.0 * (q.v.y * q.v.y + q.v.z * q.v.z);
    let yaw = siny_cosp.atan2(cosy_cosp);

    cgmath::Vector3::new(
        roll.to_degrees(),
        pitch.to_degrees(),
        yaw.to_degrees(),
    )
}

fn euler_to_quaternion(
    euler: &cgmath::Vector3<f32>,
) -> cgmath::Quaternion<f32> {
    use cgmath::Rotation3;

    let roll = euler.x.to_radians();
    let pitch = euler.y.to_radians();
    let yaw = euler.z.to_radians();

    let qx = cgmath::Quaternion::from_angle_x(cgmath::Rad(roll));
    let qy =
        cgmath::Quaternion::from_angle_y(cgmath::Rad(pitch));
    let qz = cgmath::Quaternion::from_angle_z(cgmath::Rad(yaw));

    qz * qy * qx
}
