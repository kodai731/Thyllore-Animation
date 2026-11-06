use crate::asset::AssetStorage;
use crate::ecs::component::{SpringBoneSetup, WithSpringBone};
use crate::ecs::resource::gizmo::BoneSelectionState;
use crate::ecs::resource::gizmo::{BoneDisplayStyle, BoneGizmoData};
use crate::ecs::resource::Camera;
use crate::ecs::resource::HierarchyState;
use crate::ecs::resource::ProjectionData;
use crate::ecs::systems::{
    chains_skinned_to_entity, compute_bone_draw_set, spring_chain_bone_indices, BoneDrawSet,
};
use crate::ecs::world::World;
use crate::ecs::FrameContext;
use crate::ecs::{
    build_bone_line_mesh, build_box_bone_meshes_with_selection,
    build_octahedral_bone_meshes_with_selection, build_sphere_bone_meshes_with_selection,
};
use crate::render::RenderBackend;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use anyhow::Result;
use cgmath::{InnerSpace, Matrix4, Vector3};
use thyllore_anim_core::spring_bone::SpringChainId;
use thyllore_math_core::ScreenSpaceScale;

pub(super) unsafe fn update_bone_gizmo_mesh(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.world.contains_resource::<BoneGizmoData>() {
        return Ok(());
    }

    let (
        visible,
        display_style,
        draw_mode,
        skeleton_id,
        transforms,
        offsets,
        distance_scaling_enabled,
        distance_scaling_factor,
        mesh_scale,
    ) = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        (
            bone_gizmo.visible,
            bone_gizmo.display_style,
            bone_gizmo.draw_mode,
            bone_gizmo.cached_skeleton_id,
            bone_gizmo.cached_global_transforms.clone(),
            bone_gizmo.bone_local_offsets.clone(),
            bone_gizmo.distance_scaling_enabled,
            bone_gizmo.distance_scaling_factor,
            bone_gizmo.mesh_scale,
        )
    };

    if !visible {
        ctx.world
            .resource_mut::<BoneGizmoData>()
            .drawn_bones
            .clear();
        return Ok(());
    }

    let Some(skel_id) = skeleton_id else {
        return Ok(());
    };

    let Some(skeleton) = ctx.assets.get_skeleton_by_skeleton_id(skel_id) else {
        return Ok(());
    };
    let skeleton = skeleton.clone();

    let visual_scale = compute_visual_scale(
        ctx,
        &transforms,
        distance_scaling_enabled,
        distance_scaling_factor,
    );

    let selection = bone_selection_snapshot(ctx.world);
    let chain_bones = find_spring_bone_setup(ctx.world)
        .map(|setup| spring_chain_bone_indices(&setup))
        .unwrap_or_default();
    let draw_set = compute_bone_draw_set(&skeleton, draw_mode, &selection, &chain_bones);
    ctx.world.resource_mut::<BoneGizmoData>().drawn_bones = draw_set.flags().to_vec();

    match display_style {
        BoneDisplayStyle::Stick => {
            let marker_scale = joint_marker_scale(ctx);
            update_stick_bone_mesh(
                ctx,
                &skeleton,
                &transforms,
                &offsets,
                mesh_scale,
                &draw_set,
                &marker_scale,
            )?;
        }
        BoneDisplayStyle::Octahedral => {
            update_octahedral_bone_mesh(
                ctx,
                &skeleton,
                &transforms,
                &offsets,
                visual_scale,
                mesh_scale,
                &draw_set,
            )?;
        }
        BoneDisplayStyle::Box => {
            update_box_bone_mesh(
                ctx,
                &skeleton,
                &transforms,
                &offsets,
                visual_scale,
                mesh_scale,
                &draw_set,
            )?;
        }
        BoneDisplayStyle::Sphere => {
            update_sphere_bone_mesh(
                ctx,
                &skeleton,
                &transforms,
                &offsets,
                visual_scale,
                mesh_scale,
                &draw_set,
            )?;
        }
    }

    Ok(())
}

pub(super) fn bone_selection_snapshot(world: &World) -> BoneSelectionState {
    world
        .get_resource::<BoneSelectionState>()
        .map(|selection| (*selection).clone())
        .unwrap_or_default()
}

/// A selected skinned mesh (hair, skirt) shows the chains that move it, so the chains are found
/// without selecting their root bone, and a viewport click that clears the bone selection keeps them.
pub(super) fn chains_of_selected_mesh(
    world: &World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
    setup: &SpringBoneSetup,
) -> Vec<SpringChainId> {
    let Some(entity) = world
        .get_resource::<HierarchyState>()
        .and_then(|state| state.selected_entity)
    else {
        return Vec::new();
    };
    chains_skinned_to_entity(world, assets, graphics, setup, entity)
}

/// The spring gizmo accompanies the bone gizmo: chains and colliders show only while a bone is drawn
/// or a skinned mesh is selected.
pub(super) fn any_bone_drawn(world: &World) -> bool {
    world
        .resource::<BoneGizmoData>()
        .drawn_bones
        .iter()
        .any(|&drawn| drawn)
}

pub(super) fn find_spring_bone_setup(world: &World) -> Option<SpringBoneSetup> {
    let (entity, _) = world.iter_components::<WithSpringBone>().next()?;
    world.get_component::<SpringBoneSetup>(entity).cloned()
}

fn joint_marker_scale(ctx: &FrameContext) -> ScreenSpaceScale {
    use crate::ecs::systems::camera_systems::compute_camera_position;
    let camera = ctx.world.resource::<Camera>();
    let viewport_height = ctx.world.resource::<ProjectionData>().screen_size.y;
    ScreenSpaceScale::new(
        compute_camera_position(&camera),
        cgmath::Rad::from(camera.fov_y).0,
        viewport_height,
    )
}

fn compute_visual_scale(
    ctx: &FrameContext,
    transforms: &[Matrix4<f32>],
    distance_scaling_enabled: bool,
    distance_scaling_factor: f32,
) -> f32 {
    if !distance_scaling_enabled || transforms.is_empty() {
        return 1.0;
    }

    let camera_pos = {
        use crate::ecs::systems::camera_systems::compute_camera_position;
        compute_camera_position(&ctx.world.resource::<Camera>())
    };

    let mut center = Vector3::new(0.0f32, 0.0, 0.0);
    for t in transforms.iter() {
        center.x += t[3][0];
        center.y += t[3][1];
        center.z += t[3][2];
    }
    let count = transforms.len() as f32;
    center /= count;

    let distance = (center - camera_pos).magnitude();
    (distance * distance_scaling_factor).max(0.1)
}

unsafe fn update_stick_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
    mesh_scale: f32,
    draw_set: &BoneDrawSet,
    marker_scale: &ScreenSpaceScale,
) -> Result<()> {
    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        build_bone_line_mesh(
            skeleton,
            transforms,
            offsets,
            mesh_scale,
            None,
            draw_set,
            marker_scale,
            &mut bone_gizmo.stick_mesh,
        );
    }

    let mut mesh_clone = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        bone_gizmo.stick_mesh.clone()
    };

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut mesh_clone, frame_slot)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.stick_mesh.vertex_buffer_handles = mesh_clone.vertex_buffer_handles;
        bone_gizmo.stick_mesh.index_buffer_handles = mesh_clone.index_buffer_handles;
    }

    Ok(())
}

unsafe fn update_octahedral_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
    visual_scale: f32,
    mesh_scale: f32,
    draw_set: &BoneDrawSet,
) -> Result<()> {
    let selection = bone_selection_snapshot(ctx.world);

    let (mut solid_mesh, mut wire_mesh) = {
        let bg = ctx.world.resource::<BoneGizmoData>();
        let solid_mesh = bg.solid_mesh.clone();
        let wire_mesh = bg.wire_mesh.clone();
        drop(bg);
        (solid_mesh, wire_mesh)
    };
    build_octahedral_bone_meshes_with_selection(
        skeleton,
        transforms,
        offsets,
        &selection,
        visual_scale,
        mesh_scale,
        None,
        draw_set,
        &mut solid_mesh,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut solid_mesh, frame_slot)?;
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.solid_mesh = solid_mesh;
        bone_gizmo.wire_mesh = wire_mesh;
    }

    Ok(())
}

unsafe fn update_box_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
    visual_scale: f32,
    mesh_scale: f32,
    draw_set: &BoneDrawSet,
) -> Result<()> {
    let selection = bone_selection_snapshot(ctx.world);

    let (mut solid_mesh, mut wire_mesh) = {
        let bg = ctx.world.resource::<BoneGizmoData>();
        let solid_mesh = bg.solid_mesh.clone();
        let wire_mesh = bg.wire_mesh.clone();
        drop(bg);
        (solid_mesh, wire_mesh)
    };
    build_box_bone_meshes_with_selection(
        skeleton,
        transforms,
        offsets,
        &selection,
        visual_scale,
        mesh_scale,
        None,
        draw_set,
        &mut solid_mesh,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut solid_mesh, frame_slot)?;
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.solid_mesh = solid_mesh;
        bone_gizmo.wire_mesh = wire_mesh;
    }

    Ok(())
}

unsafe fn update_sphere_bone_mesh(
    ctx: &mut FrameContext,
    skeleton: &crate::animation::Skeleton,
    transforms: &[Matrix4<f32>],
    offsets: &[[f32; 3]],
    visual_scale: f32,
    mesh_scale: f32,
    draw_set: &BoneDrawSet,
) -> Result<()> {
    let selection = bone_selection_snapshot(ctx.world);

    let (mut solid_mesh, mut wire_mesh) = {
        let bg = ctx.world.resource::<BoneGizmoData>();
        let solid_mesh = bg.solid_mesh.clone();
        let wire_mesh = bg.wire_mesh.clone();
        drop(bg);
        (solid_mesh, wire_mesh)
    };

    build_sphere_bone_meshes_with_selection(
        skeleton,
        transforms,
        offsets,
        &selection,
        visual_scale,
        mesh_scale,
        None,
        draw_set,
        &mut solid_mesh,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut solid_mesh, frame_slot)?;
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut bone_gizmo = ctx.world.resource_mut::<BoneGizmoData>();
        bone_gizmo.solid_mesh = solid_mesh;
        bone_gizmo.wire_mesh = wire_mesh;
    }

    Ok(())
}
