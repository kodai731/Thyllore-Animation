use imgui::Condition;

use crate::app::graphics_resource::GraphicsResources;
use crate::asset::AssetStorage;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::{
    ConstraintEditorState, HierarchyState,
};
use crate::ecs::systems::collect_inspector_data;
use crate::ecs::world::World;

use super::constraint_inspector::build_constraint_section;

pub fn build_inspector_window(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    world: &World,
    state: &HierarchyState,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) {
    let display_size = ui.io().display_size;
    let inspector_width = 300.0;
    let debug_height = 250.0;
    let timeline_height = 300.0;
    let main_height = display_size[1] - debug_height - timeline_height;
    let inspector_x = display_size[0] - inspector_width;

    ui.window("Inspector")
        .position([inspector_x, 0.0], Condition::Always)
        .size([inspector_width, main_height], Condition::Always)
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .build(|| {
            if let Some(entity) = state.selected_entity {
                let data = collect_inspector_data(world, entity, assets, graphics);

                ui.text(&format!("[{}] {}", data.icon_char, data.name));
                ui.separator();

                build_transform_section(ui, ui_events, &data);

                build_mesh_section(ui, &data);

                build_material_section(ui, &data);

                build_visible_section(ui, ui_events, &data);

                let mut add_type_index = world
                    .get_resource::<ConstraintEditorState>()
                    .map(|s| s.add_type_index)
                    .unwrap_or(3);

                build_constraint_section(
                    ui,
                    ui_events,
                    world,
                    entity,
                    assets,
                    state,
                    &mut add_type_index,
                );

                if let Some(mut editor_state) =
                    world.get_resource_mut::<ConstraintEditorState>()
                {
                    editor_state.add_type_index = add_type_index;
                }
            } else {
                ui.text("No entity selected");
            }
        });
}

fn build_transform_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    data: &crate::ecs::systems::InspectorData,
) {
    if data.translation.is_none() && data.rotation_euler.is_none() && data.scale.is_none() {
        return;
    }

    if ui.collapsing_header("Transform", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        if let Some(translation) = data.translation {
            let mut pos = [translation.x, translation.y, translation.z];
            ui.text("Position");
            if ui.input_float3("##position", &mut pos).build() {
                ui_events.send(UIEvent::SetEntityTranslation(
                    data.entity,
                    cgmath::Vector3::new(pos[0], pos[1], pos[2]),
                ));
            }
        }

        if let Some(rotation) = data.rotation_euler {
            let mut rot = [rotation.x, rotation.y, rotation.z];
            ui.text("Rotation");
            if ui.input_float3("##rotation", &mut rot).build() {
                let euler = cgmath::Vector3::new(rot[0], rot[1], rot[2]);
                let quat = euler_to_quaternion(&euler);
                ui_events.send(UIEvent::SetEntityRotation(data.entity, quat));
            }
        }

        if let Some(scale) = data.scale {
            let mut scl = [scale.x, scale.y, scale.z];
            ui.text("Scale");
            if ui.input_float3("##scale", &mut scl).build() {
                ui_events.send(UIEvent::SetEntityScale(
                    data.entity,
                    cgmath::Vector3::new(scl[0], scl[1], scl[2]),
                ));
            }
        }
    }
}

fn build_mesh_section(ui: &imgui::Ui, data: &crate::ecs::systems::InspectorData) {
    let Some(ref mesh) = data.mesh else {
        return;
    };

    if ui.collapsing_header("Mesh", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        ui.text(&format!("Name        {}", mesh.name));
        ui.text(&format!("Vertices    {}", format_number(mesh.vertex_count)));
        ui.text(&format!(
            "Triangles   {}",
            format_number(mesh.triangle_count)
        ));
        ui.text(&format!(
            "Skinned     {}",
            if mesh.has_skin { "Yes" } else { "No" }
        ));
    }
}

fn build_material_section(ui: &imgui::Ui, data: &crate::ecs::systems::InspectorData) {
    let Some(ref mat) = data.material else {
        return;
    };

    if ui.collapsing_header("Material", imgui::TreeNodeFlags::DEFAULT_OPEN) {
        ui.text(&format!("Name        {}", mat.name));
        ui.text(&format!(
            "Base Color  ({:.2}, {:.2}, {:.2}, {:.2})",
            mat.base_color.x, mat.base_color.y, mat.base_color.z, mat.base_color.w
        ));
        ui.text(&format!("Metallic    {:.2}", mat.metallic));
        ui.text(&format!("Roughness   {:.2}", mat.roughness));
    }
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len + len / 3);

    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(b as char);
    }

    result
}

fn build_visible_section(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    data: &crate::ecs::systems::InspectorData,
) {
    if let Some(visible) = data.visible {
        if ui.collapsing_header("Visible", imgui::TreeNodeFlags::DEFAULT_OPEN) {
            let mut vis = visible;
            if ui.checkbox("Visible##checkbox", &mut vis) {
                ui_events.send(UIEvent::SetEntityVisible(data.entity, vis));
            }
        }
    }
}

fn euler_to_quaternion(euler: &cgmath::Vector3<f32>) -> cgmath::Quaternion<f32> {
    use cgmath::Rotation3;

    let roll = euler.x.to_radians();
    let pitch = euler.y.to_radians();
    let yaw = euler.z.to_radians();

    let qx = cgmath::Quaternion::from_angle_x(cgmath::Rad(roll));
    let qy = cgmath::Quaternion::from_angle_y(cgmath::Rad(pitch));
    let qz = cgmath::Quaternion::from_angle_z(cgmath::Rad(yaw));

    qz * qy * qx
}
