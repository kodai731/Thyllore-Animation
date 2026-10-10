use cgmath::{InnerSpace, Matrix4, Vector3};
use thyllore_math_core::{Aabb, BoundingSphere};

use crate::asset::AssetStorage;
use crate::ecs::resource::gizmo::BoneGizmoData;
use crate::ecs::resource::{HierarchyDisplayMode, HierarchyState};
use crate::ecs::world::{Children, Entity, GlobalTransform, MeshRef, World};
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub const EMPTY_ENTITY_RADIUS: f32 = 0.5;
pub const BONE_RADIUS_FALLBACK: f32 = 0.1;

pub fn selection_bounding_sphere(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) -> Option<BoundingSphere> {
    let state = world.resource::<HierarchyState>();
    match state.display_mode {
        HierarchyDisplayMode::Bones => state
            .selected_bone_id
            .and_then(|bone_id| bone_bounding_sphere(world, assets, bone_id)),
        HierarchyDisplayMode::Entities => {
            let mut entities: Vec<Entity> = state.multi_selection.iter().copied().collect();
            entities.sort_unstable();
            BoundingSphere::union_all(
                entities
                    .into_iter()
                    .filter_map(|entity| entity_bounding_sphere(world, assets, graphics, entity)),
            )
        }
    }
}

pub fn entity_bounding_sphere(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    root: Entity,
) -> Option<BoundingSphere> {
    let mut mesh_spheres = Vec::new();
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get_component::<Children>(entity) {
            pending.extend(children.0.iter().copied());
        }
        if let Some(sphere) = mesh_entity_bounding_sphere(world, assets, graphics, entity) {
            mesh_spheres.push(sphere);
        }
    }

    BoundingSphere::union_all(mesh_spheres).or_else(|| {
        world
            .get_component::<GlobalTransform>(root)
            .map(|global| BoundingSphere {
                center: translation_of(&global.0),
                radius: EMPTY_ENTITY_RADIUS,
            })
    })
}

fn mesh_entity_bounding_sphere(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    entity: Entity,
) -> Option<BoundingSphere> {
    let mesh_ref = world.get_component::<MeshRef>(entity)?;
    let mesh_asset = assets.meshes.get(&mesh_ref.mesh_asset_id)?;
    let mesh = graphics.meshes.get(mesh_asset.graphics_mesh_index)?;
    let local_bounds = Aabb::from_points(
        mesh.vertex_data
            .vertices
            .iter()
            .map(|vertex| Vector3::new(vertex.pos.x, vertex.pos.y, vertex.pos.z)),
    )?;

    let world_matrix = world
        .get_component::<GlobalTransform>(entity)
        .map_or_else(|| Matrix4::from_scale(1.0), |global| global.0);
    Some(local_bounds.transformed(&world_matrix).bounding_sphere())
}

pub fn bone_bounding_sphere(
    world: &World,
    assets: &AssetStorage,
    bone_id: u32,
) -> Option<BoundingSphere> {
    let bone_gizmo = world.get_resource::<BoneGizmoData>()?;
    let skeleton = assets.get_skeleton_by_skeleton_id(bone_gizmo.cached_skeleton_id?)?;
    let globals = &bone_gizmo.cached_global_transforms;
    let bone = skeleton.get_bone(bone_id)?;
    let center = translation_of(globals.get(bone_id as usize)?);

    let neighbour_ids = bone
        .parent_id
        .into_iter()
        .chain(bone.children.iter().copied());
    let neighbour_distances: Vec<f32> = neighbour_ids
        .filter_map(|id| globals.get(id as usize))
        .map(|matrix| (translation_of(matrix) - center).magnitude())
        .collect();
    let radius = if neighbour_distances.is_empty() {
        BONE_RADIUS_FALLBACK
    } else {
        neighbour_distances.iter().sum::<f32>() / neighbour_distances.len() as f32
    };

    Some(BoundingSphere { center, radius })
}

fn translation_of(matrix: &Matrix4<f32>) -> Vector3<f32> {
    Vector3::new(matrix.w.x, matrix.w.y, matrix.w.z)
}
