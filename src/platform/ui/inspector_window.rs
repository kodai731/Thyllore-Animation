use crate::asset::AssetStorage;
use crate::ecs::resource::{HierarchyState, LayoutSnapshot};
use crate::ecs::systems::collect_inspector_data;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

use super::blend_shape_inspector::blend_shape_sections;
use super::bone::bone_section;
use super::constraint::constraint_section;
use super::form_window::{FormPlacement, FormSection, FormWindow};
use super::inspector_sections::{entity_sections, no_selection_section};
use super::inspector_target::{inspector_target, InspectorTarget};
use super::spring_bone::{bone_spring_section, mesh_spring_section, spring_bone_section};

fn inspector_form(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Option<FormWindow> {
    let layout = world.resource::<LayoutSnapshot>().clone();
    let placement = FormPlacement::Docked {
        position: [layout.inspector_x, 0.0],
        size: [layout.inspector_width, layout.main_height],
    };
    let hierarchy_state = world.resource::<HierarchyState>().clone();

    let sections = match inspector_target(&hierarchy_state) {
        Some(InspectorTarget::Entity(entity)) => {
            let data = collect_inspector_data(world, entity, assets, graphics);
            let mut sections: Vec<Box<dyn FormSection>> = entity_sections(world, &data);
            sections.push(constraint_section(world, assets, &hierarchy_state, None));
            sections.extend(mesh_spring_section(world, assets, graphics, entity));
            sections.push(spring_bone_section(world, assets, &hierarchy_state));
            sections.extend(blend_shape_sections(world, entity, assets, graphics));
            sections
        }
        Some(InspectorTarget::Bone(bone_id)) => vec![
            bone_section(assets, bone_id),
            bone_spring_section(world, assets, bone_id),
            constraint_section(world, assets, &hierarchy_state, Some(bone_id)),
        ],
        None => vec![no_selection_section()],
    };

    Some(FormWindow {
        title: "Inspector",
        placement,
        sections,
    })
}

crate::ui_window!("inspector", Side, 3, form = inspector_form);
