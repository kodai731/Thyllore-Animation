use imgui::Condition;

use crate::asset::AssetStorage;
use crate::ecs::resource::{ConstraintEditorState, HierarchyState};
use crate::ecs::systems::collect_inspector_data;
use crate::ecs::systems::phases::event_dispatch::avatar_setup::AvatarSetupEvent;
use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::world::{Visibility, World};
use crate::math::euler_degrees_to_quaternion;
use crate::platform::ui::theme::section_header;
use crate::platform::ui::theme::SectionDefault;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::blend_shape_inspector::build_blend_shape_section;
use super::constraint_inspector::build_constraint_section;
use super::spring_bone_inspector::build_spring_bone_section;
use crate::ecs::resource::LayoutSnapshot;

fn draw_inspector_window(
    ui: &imgui::Ui,
    world: &World,
    state: &HierarchyState,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    layout: &LayoutSnapshot,
) {
    ui.window("Inspector")
        .position([layout.inspector_x, 0.0], Condition::Always)
        .size(
            [layout.inspector_width, layout.main_height],
            Condition::Always,
        )
        .resizable(false)
        .movable(false)
        .collapsible(false)
        .bring_to_front_on_focus(false)
        .build(|| {
            if let Some(entity) = state.selected_entity {
                let data = collect_inspector_data(world, entity, assets, graphics);

                ui.text(&format!("[{}] {}", data.icon_char, data.name));
                ui.separator();

                build_transform_section(ui, world, &data);

                build_mesh_section(ui, world, &data);

                build_material_section(ui, world, &data);

                build_visible_section(ui, world, &data);

                let (mut add_type_index, mut bake_fps) = world
                    .get_resource::<ConstraintEditorState>()
                    .map(|s| (s.add_type_index, s.bake_fps))
                    .unwrap_or((3, 30.0));

                build_constraint_section(
                    ui,
                    world,
                    entity,
                    assets,
                    state,
                    &mut add_type_index,
                    &mut bake_fps,
                );

                if let Some(mut editor_state) = world.get_resource_mut::<ConstraintEditorState>() {
                    editor_state.add_type_index = add_type_index;
                    editor_state.bake_fps = bake_fps;
                }

                build_spring_bone_section(ui, world, entity, assets, state);

                build_blend_shape_section(ui, world, entity, assets, graphics);

                ui.separator();
                if ui.button("Avatar Setup...") {
                    world.send_command(AvatarSetupEvent::OpenAvatarSetup);
                }
            } else {
                ui.text("No entity selected");
            }
        });
}

fn build_transform_section(
    ui: &imgui::Ui,
    world: &World,
    data: &crate::ecs::systems::InspectorData,
) {
    if data.translation.is_none() && data.rotation_euler.is_none() && data.scale.is_none() {
        return;
    }

    if section_header(ui, world, "Transform", SectionDefault::Open) {
        if let Some(translation) = data.translation {
            let mut pos = [translation.x, translation.y, translation.z];
            ui.text("Position");
            if ui.input_float3("##position", &mut pos).build() {
                world.send_command(HierarchyEvent::SetEntityTranslation(
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
                let quat = euler_degrees_to_quaternion(&euler);
                world.send_command(HierarchyEvent::SetEntityRotation(data.entity, quat));
            }
        }

        if let Some(scale) = data.scale {
            let mut scl = [scale.x, scale.y, scale.z];
            ui.text("Scale");
            if ui.input_float3("##scale", &mut scl).build() {
                world.send_command(HierarchyEvent::SetEntityScale(
                    data.entity,
                    cgmath::Vector3::new(scl[0], scl[1], scl[2]),
                ));
            }
        }
    }
}

fn build_mesh_section(ui: &imgui::Ui, world: &World, data: &crate::ecs::systems::InspectorData) {
    let Some(ref mesh) = data.mesh else {
        return;
    };

    if section_header(ui, world, "Mesh", SectionDefault::Open) {
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

fn build_material_section(
    ui: &imgui::Ui,
    world: &World,
    data: &crate::ecs::systems::InspectorData,
) {
    let Some(ref mat) = data.material else {
        return;
    };

    if section_header(ui, world, "Material", SectionDefault::Open) {
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

fn build_visible_section(ui: &imgui::Ui, world: &World, data: &crate::ecs::systems::InspectorData) {
    if let Some(visible) = data.visible {
        if section_header(ui, world, "Visible", SectionDefault::Open) {
            let mut vis = visible;
            if crate::platform::ui::theme::toggle_switch(ui, world, "Visible##checkbox", &mut vis) {
                world.send_command(HierarchyEvent::SetEntityVisible(
                    data.entity,
                    Visibility::from(vis),
                ));
            }
        }
    }
}

fn build_inspector_window(
    ui: &imgui::Ui,
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) {
    let hierarchy_state = world.resource::<HierarchyState>();
    let layout = world.resource::<LayoutSnapshot>();
    draw_inspector_window(ui, world, &hierarchy_state, assets, graphics, &layout);
}

crate::ui_window!("inspector", Side, 3, build_inspector_window);
