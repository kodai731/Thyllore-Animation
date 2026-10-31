use super::bone_gizmo_mesh::{
    any_bone_drawn, bone_selection_snapshot, chains_of_selected_mesh, find_spring_bone_setup,
};
use crate::ecs::resource::gizmo::TransformGizmoData;
use crate::ecs::resource::gizmo::{
    BoneGizmoData, ConstraintGizmoData, SpringBoneGizmoData, SpringChainGizmoMode,
};
use crate::ecs::systems::{ColliderGizmo, SpringGizmoSelection};
use crate::ecs::FrameContext;
use crate::ecs::{build_constraint_gizmo_mesh, build_spring_bone_gizmo_mesh};
use crate::render::RenderBackend;
use anyhow::Result;
use thyllore_anim_core::spring_bone::{chains_to_draw, chains_with_radius};

pub(super) unsafe fn update_transform_gizmo_mesh(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.world.contains_resource::<TransformGizmoData>() {
        return Ok(());
    }

    let visible = {
        let tg = ctx.world.resource::<TransformGizmoData>();
        tg.visible
    };

    if !visible {
        return Ok(());
    }

    let (mode, active_handle) = {
        let tg = ctx.world.resource::<TransformGizmoData>();
        let state = ctx
            .world
            .resource::<crate::ecs::resource::TransformGizmoState>();
        (state.mode, tg.active_handle)
    };

    let camera_dir = ctx.camera_direction();

    let (mut line_mesh_clone, mut solid_mesh_clone) = {
        let tg = ctx.world.resource::<TransformGizmoData>();
        let line_mesh_clone = tg.line_mesh.clone();
        let solid_mesh_clone = tg.solid_mesh.clone();
        drop(tg);
        (line_mesh_clone, solid_mesh_clone)
    };

    match mode {
        crate::ecs::resource::TransformGizmoMode::Translate => {
            crate::ecs::systems::transform_gizmo_systems::build_translate_gizmo_meshes(
                active_handle,
                &mut line_mesh_clone,
                &mut solid_mesh_clone,
            );
        }
        crate::ecs::resource::TransformGizmoMode::Rotate => {
            crate::ecs::systems::transform_gizmo_systems::build_rotate_gizmo_meshes(
                active_handle,
                camera_dir,
                &mut line_mesh_clone,
                &mut solid_mesh_clone,
            );
        }
        crate::ecs::resource::TransformGizmoMode::Scale => {
            crate::ecs::systems::transform_gizmo_systems::build_scale_gizmo_meshes(
                active_handle,
                &mut line_mesh_clone,
                &mut solid_mesh_clone,
            );
        }
    }

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut line_mesh_clone, frame_slot)?;
        backend.update_or_create_line_buffers(&mut solid_mesh_clone, frame_slot)?;
    }

    {
        let mut tg = ctx.world.resource_mut::<TransformGizmoData>();
        tg.line_mesh = line_mesh_clone;
        tg.solid_mesh = solid_mesh_clone;
    }

    Ok(())
}

pub(super) unsafe fn update_constraint_gizmo_mesh(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.world.contains_resource::<ConstraintGizmoData>() {
        return Ok(());
    }
    if !ctx.world.contains_resource::<BoneGizmoData>() {
        return Ok(());
    }

    let visible = {
        let cg = ctx.world.resource::<ConstraintGizmoData>();
        cg.visible
    };
    if !visible {
        return Ok(());
    }

    let (skeleton_id, transforms, offsets, constraint_mesh_scale) = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        (
            bone_gizmo.cached_skeleton_id,
            bone_gizmo.cached_global_transforms.clone(),
            bone_gizmo.bone_local_offsets.clone(),
            bone_gizmo.mesh_scale,
        )
    };

    let Some(skel_id) = skeleton_id else {
        return Ok(());
    };
    let Some(skeleton) = ctx.assets.get_skeleton_by_skeleton_id(skel_id) else {
        return Ok(());
    };
    let skeleton = skeleton.clone();

    let constraint_set = ctx
        .world
        .iter_constrained_entities()
        .next()
        .map(|(_, cs)| cs.clone());

    let Some(constraint_set) = constraint_set else {
        return Ok(());
    };

    let mut wire_mesh = ctx
        .world
        .resource::<ConstraintGizmoData>()
        .wire_mesh
        .clone();

    build_constraint_gizmo_mesh(
        &constraint_set,
        &skeleton,
        &transforms,
        &offsets,
        constraint_mesh_scale,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut cg = ctx.world.resource_mut::<ConstraintGizmoData>();
        cg.wire_mesh = wire_mesh;
    }

    Ok(())
}

pub(super) unsafe fn update_spring_bone_gizmo_mesh(ctx: &mut FrameContext) -> Result<()> {
    if !ctx.world.contains_resource::<SpringBoneGizmoData>() {
        return Ok(());
    }
    if !ctx.world.contains_resource::<BoneGizmoData>() {
        return Ok(());
    }

    let (colliders_visible, chain_mode) = {
        let sg = ctx.world.resource::<SpringBoneGizmoData>();
        (sg.colliders_visible, sg.chain_mode)
    };
    if !colliders_visible && chain_mode == SpringChainGizmoMode::None {
        return Ok(());
    }

    let (skeleton_id, transforms, offsets, mesh_scale) = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        (
            bone_gizmo.cached_skeleton_id,
            bone_gizmo.cached_global_transforms.clone(),
            bone_gizmo.bone_local_offsets.clone(),
            bone_gizmo.mesh_scale,
        )
    };

    let Some(setup) = find_spring_bone_setup(ctx.world) else {
        return Ok(());
    };
    let skeleton = skeleton_id.and_then(|id| ctx.assets.get_skeleton_by_skeleton_id(id));
    let Some(skeleton) = skeleton else {
        return Ok(());
    };

    let selection = bone_selection_snapshot(ctx.world);
    let bones_drawn = any_bone_drawn(ctx.world);
    let mut chain_ids = if bones_drawn {
        chains_to_draw(
            &setup,
            skeleton,
            &selection.selected_bone_indices,
            chain_mode,
        )
    } else {
        Vec::new()
    };
    if chain_mode != SpringChainGizmoMode::None {
        chain_ids.extend(chains_of_selected_mesh(
            ctx.world,
            ctx.assets,
            ctx.graphics,
            &setup,
        ));
        chain_ids.sort_unstable();
        chain_ids.dedup();
    }
    let colliders = if colliders_visible && (bones_drawn || !chain_ids.is_empty()) {
        ColliderGizmo::Shown
    } else {
        ColliderGizmo::Hidden
    };
    let shown = SpringGizmoSelection {
        chain_ids,
        radius_chain_ids: chains_with_radius(&setup, selection.active_bone_index, chain_mode),
        colliders,
    };

    let mut wire_mesh = ctx
        .world
        .resource::<SpringBoneGizmoData>()
        .wire_mesh
        .clone();

    build_spring_bone_gizmo_mesh(
        &setup,
        &shown,
        &transforms,
        &offsets,
        mesh_scale,
        &mut wire_mesh,
    );

    {
        let frame_slot = ctx.frame_slot;
        let mut backend = ctx.create_backend();
        backend.update_or_create_line_buffers(&mut wire_mesh, frame_slot)?;
    }

    {
        let mut sg = ctx.world.resource_mut::<SpringBoneGizmoData>();
        sg.wire_mesh = wire_mesh;
    }

    Ok(())
}
