use imgui::Condition;

use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::resource::HierarchyState;
use crate::ecs::systems::collect_inspector_data;
use crate::ecs::world::World;

pub fn build_inspector_window(
    ui: &imgui::Ui,
    ui_events: &mut UIEventQueue,
    world: &World,
    state: &HierarchyState,
) {
    ui.window("Inspector")
        .size([300.0, 400.0], Condition::FirstUseEver)
        .build(|| {
            if let Some(entity) = state.selected_entity {
                let data = collect_inspector_data(world, entity);

                ui.text(&format!("[{}] {}", data.icon_char, data.name));
                ui.separator();

                build_transform_section(ui, ui_events, &data);

                build_visible_section(ui, ui_events, &data);
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
