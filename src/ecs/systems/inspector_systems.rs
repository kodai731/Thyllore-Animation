use cgmath::{Quaternion, Rotation3, Vector3, Vector4};

use crate::app::graphics_resource::GraphicsResources;
use crate::asset::AssetStorage;
use crate::ecs::component::EditorDisplay;
use crate::ecs::world::{Entity, MeshRef, Name, Transform, Visible, World};

#[derive(Clone, Debug)]
pub struct MeshInspectorData {
    pub name: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub has_skin: bool,
}

#[derive(Clone, Debug)]
pub struct MaterialInspectorData {
    pub name: String,
    pub base_color: Vector4<f32>,
    pub metallic: f32,
    pub roughness: f32,
}

#[derive(Clone, Debug)]
pub struct InspectorData {
    pub entity: Entity,
    pub name: String,
    pub translation: Option<Vector3<f32>>,
    pub rotation_euler: Option<Vector3<f32>>,
    pub scale: Option<Vector3<f32>>,
    pub visible: Option<bool>,
    pub icon_char: char,
    pub mesh: Option<MeshInspectorData>,
    pub material: Option<MaterialInspectorData>,
}

pub fn collect_inspector_data(
    world: &World,
    entity: Entity,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> InspectorData {
    let name = world
        .get_component::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| format!("Entity {}", entity));

    let transform = world.get_component::<Transform>(entity);
    let translation = transform.map(|t| t.translation);
    let rotation_euler = transform.map(|t| quaternion_to_euler(&t.rotation));
    let scale = transform.map(|t| t.scale);

    let visible = world.get_component::<Visible>(entity).map(|v| v.0);

    let icon_char = world
        .get_component::<EditorDisplay>(entity)
        .map(|ed| ed.icon.to_char())
        .unwrap_or(' ');

    let (mesh, material) = collect_mesh_and_material(world, entity, assets, graphics);

    InspectorData {
        entity,
        name,
        translation,
        rotation_euler,
        scale,
        visible,
        icon_char,
        mesh,
        material,
    }
}

fn collect_mesh_and_material(
    world: &World,
    entity: Entity,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> (Option<MeshInspectorData>, Option<MaterialInspectorData>) {
    let mesh_ref = match world.get_component::<MeshRef>(entity) {
        Some(r) => r,
        None => return (None, None),
    };

    let mesh_asset = match assets.get_mesh(mesh_ref.mesh_asset_id) {
        Some(a) => a,
        None => return (None, None),
    };

    let mesh_data = graphics.meshes.get(mesh_asset.graphics_mesh_index).map(|mb| {
        MeshInspectorData {
            name: mesh_asset.name.clone(),
            vertex_count: mb.vertex_data.vertices.len(),
            triangle_count: mb.vertex_data.indices.len() / 3,
            has_skin: mb.skin_data.is_some(),
        }
    });

    let material_data = mesh_asset
        .material_id
        .and_then(|mid| graphics.materials.get(mid))
        .map(|mat| MaterialInspectorData {
            name: mat.name.clone(),
            base_color: mat.properties.base_color,
            metallic: mat.properties.metallic,
            roughness: mat.properties.roughness,
        });

    (mesh_data, material_data)
}

pub fn update_entity_translation(world: &mut World, entity: Entity, translation: Vector3<f32>) {
    if let Some(transform) = world.get_component_mut::<Transform>(entity) {
        transform.translation = translation;
    }
}

pub fn update_entity_rotation(world: &mut World, entity: Entity, rotation: Quaternion<f32>) {
    if let Some(transform) = world.get_component_mut::<Transform>(entity) {
        transform.rotation = rotation;
    }
}

pub fn update_entity_rotation_euler(world: &mut World, entity: Entity, euler: Vector3<f32>) {
    let rotation = euler_to_quaternion(&euler);
    update_entity_rotation(world, entity, rotation);
}

pub fn update_entity_scale(world: &mut World, entity: Entity, scale: Vector3<f32>) {
    if let Some(transform) = world.get_component_mut::<Transform>(entity) {
        transform.scale = scale;
    }
}

pub fn update_entity_visible(world: &mut World, entity: Entity, visible: bool) {
    if let Some(vis) = world.get_component_mut::<Visible>(entity) {
        vis.0 = visible;
    }
}

pub fn rename_entity(world: &mut World, entity: Entity, new_name: String) {
    if let Some(name) = world.get_component_mut::<Name>(entity) {
        name.0 = new_name;
    }
}

fn quaternion_to_euler(q: &Quaternion<f32>) -> Vector3<f32> {
    let sinr_cosp = 2.0 * (q.s * q.v.x + q.v.y * q.v.z);
    let cosr_cosp = 1.0 - 2.0 * (q.v.x * q.v.x + q.v.y * q.v.y);
    let roll = sinr_cosp.atan2(cosr_cosp);

    let sinp = 2.0 * (q.s * q.v.y - q.v.z * q.v.x);
    let pitch = if sinp.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(sinp)
    } else {
        sinp.asin()
    };

    let siny_cosp = 2.0 * (q.s * q.v.z + q.v.x * q.v.y);
    let cosy_cosp = 1.0 - 2.0 * (q.v.y * q.v.y + q.v.z * q.v.z);
    let yaw = siny_cosp.atan2(cosy_cosp);

    Vector3::new(
        roll.to_degrees(),
        pitch.to_degrees(),
        yaw.to_degrees(),
    )
}

fn euler_to_quaternion(euler: &Vector3<f32>) -> Quaternion<f32> {
    let roll = euler.x.to_radians();
    let pitch = euler.y.to_radians();
    let yaw = euler.z.to_radians();

    let qx = Quaternion::from_angle_x(cgmath::Rad(roll));
    let qy = Quaternion::from_angle_y(cgmath::Rad(pitch));
    let qz = Quaternion::from_angle_z(cgmath::Rad(yaw));

    qz * qy * qx
}
