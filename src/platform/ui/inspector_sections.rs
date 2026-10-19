use crate::ecs::systems::phases::event_dispatch::hierarchy::HierarchyEvent;
use crate::ecs::systems::InspectorData;
use crate::ecs::world::{Entity, Visibility, World};

use super::form_window::{rows_section, FormSection};
use super::ui_rows::UiRow;

#[derive(Clone)]
struct HeaderView {
    pub icon: crate::ecs::component::EntityIcon,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
struct TransformView {
    pub entity: Entity,
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub scale: Option<[f32; 3]>,
}

#[derive(Clone)]
struct MeshView {
    pub name: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub has_skin: bool,
}

#[derive(Clone)]
struct MaterialView {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
}

#[derive(Clone)]
struct VisibleView {
    pub entity: Entity,
    pub visible: bool,
}

pub(super) fn no_selection_section() -> Box<dyn FormSection> {
    rows_section(
        vec![UiRow::Canvas(|ui, _, _: &()| ui.text("No entity selected"))],
        (),
        |_, _, _| {},
    )
}

pub(super) fn entity_sections(_world: &World, data: &InspectorData) -> Vec<Box<dyn FormSection>> {
    let mut sections = Vec::new();
    sections.push(header_section(data));
    if let Some(section) = transform_section(data) {
        sections.push(section);
    }
    if let Some(section) = mesh_section(data) {
        sections.push(section);
    }
    if let Some(section) = material_section(data) {
        sections.push(section);
    }
    if let Some(section) = visible_section(data) {
        sections.push(section);
    }
    sections
}

fn header_section(data: &InspectorData) -> Box<dyn FormSection> {
    rows_section(
        vec![UiRow::Canvas(
            |ui, _world, view: &HeaderView| match super::theme::entity_icon(view.icon) {
                Some(icon) => ui.text(&format!("{} {}", icon.glyph(), view.name)),
                None => ui.text(&view.name),
            },
        )],
        HeaderView {
            icon: data.icon,
            name: data.name.clone(),
        },
        |_, _, _| {},
    )
}

fn transform_section(data: &InspectorData) -> Option<Box<dyn FormSection>> {
    if data.translation.is_none() && data.rotation_euler.is_none() && data.scale.is_none() {
        return None;
    }

    let view = TransformView {
        entity: data.entity,
        position: data.translation.map(|t| [t.x, t.y, t.z]),
        rotation: data.rotation_euler.map(|r| [r.x, r.y, r.z]),
        scale: data.scale.map(|s| [s.x, s.y, s.z]),
    };

    let mut rows: Vec<UiRow<TransformView>> = Vec::new();

    if view.position.is_some() {
        rows.push(UiRow::Vector3 {
            label: "Position",
            speed: 0.1,
            format: "%.2f",
            get: |v: &TransformView| v.position.unwrap_or([0.0, 0.0, 0.0]),
            set: |v: &mut TransformView, val| {
                v.position = Some(val);
            },
        });
    }

    if view.rotation.is_some() {
        rows.push(UiRow::Vector3 {
            label: "Rotation",
            speed: 1.0,
            format: "%.1f",
            get: |v: &TransformView| v.rotation.unwrap_or([0.0, 0.0, 0.0]),
            set: |v: &mut TransformView, val| {
                v.rotation = Some(val);
            },
        });
    }

    if view.scale.is_some() {
        rows.push(UiRow::Vector3 {
            label: "Scale",
            speed: 0.01,
            format: "%.2f",
            get: |v: &TransformView| v.scale.unwrap_or([1.0, 1.0, 1.0]),
            set: |v: &mut TransformView, val| {
                v.scale = Some(val);
            },
        });
    }

    if rows.is_empty() {
        return None;
    }

    Some(rows_section(
        vec![UiRow::Group {
            title: "Transform".into(),
            initially_open: true,
            rows,
        }],
        view,
        commit_transform_view,
    ))
}

fn commit_transform_view(world: &World, shown: &TransformView, view: &TransformView) {
    if shown.position != view.position {
        let pos = view.position.unwrap_or([0.0, 0.0, 0.0]);
        world.send_command(HierarchyEvent::SetEntityTranslation(
            view.entity,
            cgmath::Vector3::new(pos[0], pos[1], pos[2]),
        ));
    }
    if shown.rotation != view.rotation {
        let rot = view.rotation.unwrap_or([0.0, 0.0, 0.0]);
        let euler = cgmath::Vector3::new(rot[0], rot[1], rot[2]);
        let quat = crate::math::euler_degrees_to_quaternion(&euler);
        world.send_command(HierarchyEvent::SetEntityRotation(view.entity, quat));
    }
    if shown.scale != view.scale {
        let scl = view.scale.unwrap_or([1.0, 1.0, 1.0]);
        world.send_command(HierarchyEvent::SetEntityScale(
            view.entity,
            cgmath::Vector3::new(scl[0], scl[1], scl[2]),
        ));
    }
}

fn mesh_section(data: &InspectorData) -> Option<Box<dyn FormSection>> {
    let mesh = data.mesh.as_ref()?;

    Some(rows_section(
        vec![UiRow::Group {
            title: "Mesh".into(),
            initially_open: true,
            rows: vec![
                UiRow::Canvas(|ui, _world, view: &MeshView| {
                    ui.text(&format!("Name        {}", view.name));
                }),
                UiRow::Canvas(|ui, _world, view: &MeshView| {
                    ui.text(&format!("Vertices    {}", format_number(view.vertex_count)));
                }),
                UiRow::Canvas(|ui, _world, view: &MeshView| {
                    ui.text(&format!(
                        "Triangles   {}",
                        format_number(view.triangle_count)
                    ));
                }),
                UiRow::Canvas(|ui, _world, view: &MeshView| {
                    ui.text(&format!(
                        "Skinned     {}",
                        if view.has_skin { "Yes" } else { "No" }
                    ));
                }),
            ],
        }],
        MeshView {
            name: mesh.name.clone(),
            vertex_count: mesh.vertex_count,
            triangle_count: mesh.triangle_count,
            has_skin: mesh.has_skin,
        },
        |_, _, _| {},
    ))
}

fn material_section(data: &InspectorData) -> Option<Box<dyn FormSection>> {
    let mat = data.material.as_ref()?;

    Some(rows_section(
        vec![UiRow::Group {
            title: "Material".into(),
            initially_open: true,
            rows: vec![
                UiRow::Canvas(|ui, _world, view: &MaterialView| {
                    ui.text(&format!("Name        {}", view.name));
                }),
                UiRow::Canvas(|ui, _world, view: &MaterialView| {
                    ui.text(&format!(
                        "Base Color  ({:.2}, {:.2}, {:.2}, {:.2})",
                        view.base_color[0],
                        view.base_color[1],
                        view.base_color[2],
                        view.base_color[3]
                    ));
                }),
                UiRow::Canvas(|ui, _world, view: &MaterialView| {
                    ui.text(&format!("Metallic    {:.2}", view.metallic));
                }),
                UiRow::Canvas(|ui, _world, view: &MaterialView| {
                    ui.text(&format!("Roughness   {:.2}", view.roughness));
                }),
            ],
        }],
        MaterialView {
            name: mat.name.clone(),
            base_color: [
                mat.base_color.x,
                mat.base_color.y,
                mat.base_color.z,
                mat.base_color.w,
            ],
            metallic: mat.metallic,
            roughness: mat.roughness,
        },
        |_, _, _| {},
    ))
}

fn visible_section(data: &InspectorData) -> Option<Box<dyn FormSection>> {
    let visible = data.visible?;

    Some(rows_section(
        vec![UiRow::Group {
            title: "Visible".into(),
            initially_open: true,
            rows: vec![UiRow::Switch {
                label: "Visible",
                get: |v: &VisibleView| v.visible,
                set: |v: &mut VisibleView, val| v.visible = val,
            }],
        }],
        VisibleView {
            entity: data.entity,
            visible,
        },
        commit_visible_view,
    ))
}

fn commit_visible_view(world: &World, shown: &VisibleView, view: &VisibleView) {
    if shown.visible != view.visible {
        world.send_command(HierarchyEvent::SetEntityVisible(
            view.entity,
            Visibility::from(view.visible),
        ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::events::UiCommandQueue;

    #[test]
    fn test_commit_transform_view_no_change() {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        let view = TransformView {
            entity: 0u64,
            position: Some([1.0, 2.0, 3.0]),
            rotation: Some([0.0, 0.0, 0.0]),
            scale: Some([1.0, 1.0, 1.0]),
        };
        let shown = view.clone();
        commit_transform_view(&world, &shown, &view);
        assert_eq!(world.resource_mut::<UiCommandQueue>().drain().count(), 0);
    }

    #[test]
    fn test_commit_transform_view_position_changed() {
        let mut world = World::new();
        world.insert_resource(UiCommandQueue::default());
        let shown = TransformView {
            entity: 0u64,
            position: Some([1.0, 2.0, 3.0]),
            rotation: Some([0.0, 0.0, 0.0]),
            scale: Some([1.0, 1.0, 1.0]),
        };
        let view = TransformView {
            entity: 0u64,
            position: Some([4.0, 5.0, 6.0]),
            rotation: Some([0.0, 0.0, 0.0]),
            scale: Some([1.0, 1.0, 1.0]),
        };
        commit_transform_view(&world, &shown, &view);
        assert_eq!(world.resource_mut::<UiCommandQueue>().drain().count(), 1);
    }

    #[test]
    fn test_format_number_small() {
        assert_eq!(format_number(42), "42");
    }

    #[test]
    fn test_format_number_thousands() {
        assert_eq!(format_number(1000), "1,000");
    }

    #[test]
    fn test_format_number_millions() {
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn test_format_number_zero() {
        assert_eq!(format_number(0), "0");
    }
}
