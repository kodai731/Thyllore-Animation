use anyhow::Result;
use cgmath::Vector3;

use crate::animation::BoneId;
use crate::app::data::LightMoveTarget;
use crate::debugview::gizmo::{BoneDisplayStyle, BoneGizmoData};
use crate::ecs::context::EcsContext;
use crate::ecs::resource::HierarchyDisplayMode;
use crate::ecs::systems::select_bone_by_ray;
use crate::ecs::GizmoAxis;
use crate::ecs::{
    gizmo_try_select, gizmo_update_position_with_constraint,
    update_light_auto_target,
};
use crate::math::screen_to_world_ray;

pub fn run_input_phase(ctx: &mut EcsContext) -> Result<()> {
    process_light_auto_target(ctx);

    let is_first_left_click = ctx.gui_data.is_left_clicked
        && !ctx.gui_data.is_right_clicked
        && !ctx.gui_data.is_wheel_clicked
        && ctx.gui_data.clicked_mouse_pos.is_none()
        && ctx.gui_data.viewport_hovered;

    ctx.gui_data.update();

    process_gizmo_interaction(ctx, is_first_left_click)?;

    if is_first_left_click {
        process_bone_selection(ctx)?;
    }

    let viewport_hovered = ctx.gui_data.viewport_hovered;
    if !ctx.light_gizmo().selectable.is_selected && viewport_hovered
    {
        let is_right_clicked = ctx.gui_data.is_right_clicked;
        let is_wheel_clicked = ctx.gui_data.is_wheel_clicked;
        let mouse_wheel = ctx.gui_data.mouse_wheel;
        let mouse_diff = ctx.gui_data.mouse_diff;
        let local_mouse_pos = [
            ctx.gui_data.mouse_pos[0]
                - ctx.gui_data.viewport_position[0],
            ctx.gui_data.mouse_pos[1]
                - ctx.gui_data.viewport_position[1],
        ];
        let screen_size = [
            ctx.swapchain_extent.0 as f32,
            ctx.swapchain_extent.1 as f32,
        ];
        let mut camera = ctx.camera_mut();
        crate::ecs::camera_input_system_inner(
            &mut *camera,
            is_right_clicked,
            is_wheel_clicked,
            mouse_wheel,
            mouse_diff,
            local_mouse_pos,
            screen_size,
        );
    }

    Ok(())
}

fn process_light_auto_target(ctx: &mut EcsContext) {
    if ctx.gui_data.move_light_to == LightMoveTarget::None {
        return;
    }

    let camera_position =
        crate::ecs::compute_camera_position(&ctx.camera());
    let move_light_to = ctx.gui_data.move_light_to;
    let mut rt_debug = ctx.rt_debug_mut();
    update_light_auto_target(
        &mut *rt_debug,
        &ctx.mesh_positions,
        camera_position,
        move_light_to,
    );
    drop(rt_debug);
    ctx.gui_data.move_light_to = LightMoveTarget::None;
}

fn viewport_local_mouse_pos(
    ctx: &EcsContext,
) -> cgmath::Vector2<f32> {
    cgmath::Vector2::new(
        ctx.gui_data.mouse_pos[0]
            - ctx.gui_data.viewport_position[0],
        ctx.gui_data.mouse_pos[1]
            - ctx.gui_data.viewport_position[1],
    )
}

fn process_gizmo_interaction(
    ctx: &mut EcsContext,
    is_first_left_click: bool,
) -> Result<()> {
    let mouse_pos = viewport_local_mouse_pos(ctx);

    if !ctx.gui_data.imgui_wants_mouse
        && ctx.gui_data.is_left_clicked
    {
        ctx.light_gizmo_mut().draggable.just_selected = false;

        if is_first_left_click {
            let camera = ctx.camera();
            let camera_pos =
                crate::ecs::compute_camera_position(&camera);
            let camera_dir =
                crate::ecs::compute_camera_direction(&camera);
            let camera_up =
                crate::ecs::compute_camera_up(&camera);
            let fov_y = camera.fov_y;
            let near_plane = camera.near_plane;
            drop(camera);
            {
                let mut gizmo_ref = ctx.light_gizmo_mut();
                let light_gizmo = &mut *gizmo_ref;
                let position = light_gizmo.position.clone();
                gizmo_try_select(
                    &position,
                    &mut light_gizmo.selectable,
                    &mut light_gizmo.draggable,
                    mouse_pos,
                    camera_pos,
                    camera_dir,
                    camera_up,
                    ctx.swapchain_extent,
                    ctx.gui_data.billboard_click_rect,
                    fov_y,
                    near_plane,
                );
            }
        }

        let (is_selected, just_selected) = {
            let gizmo = ctx.light_gizmo();
            (
                gizmo.selectable.is_selected,
                gizmo.draggable.just_selected,
            )
        };

        if is_selected
            && ctx.gui_data.is_left_clicked
            && !just_selected
        {
            update_light_gizmo_position(ctx, mouse_pos)?;
        }
    } else if !ctx.gui_data.is_wheel_clicked {
        if ctx.light_gizmo().selectable.is_selected {
            gizmo_handle_mouse_release(ctx);
        }
    }

    Ok(())
}

fn gizmo_handle_mouse_release(ctx: &mut EcsContext) {
    crate::log!("Mouse released - resetting light gizmo state");
    let mut gizmo = ctx.light_gizmo_mut();
    gizmo.selectable.is_selected = false;
    gizmo.selectable.selected_axis = GizmoAxis::None;
    gizmo.draggable.drag_axis = GizmoAxis::None;
    gizmo.draggable.just_selected = false;
    gizmo.draggable.initial_position =
        Vector3::new(0.0, 0.0, 0.0);
}

fn update_light_gizmo_position(
    ctx: &mut EcsContext,
    mouse_pos: cgmath::Vector2<f32>,
) -> Result<()> {
    use crate::math::coordinate_system::perspective_infinite_reverse;
    use cgmath::InnerSpace;

    let camera = ctx.camera();
    let camera_pos =
        crate::ecs::compute_camera_position(&camera);
    let camera_dir =
        crate::ecs::compute_camera_direction(&camera);
    let camera_up =
        crate::ecs::compute_camera_up(&camera);
    let fov_y = camera.fov_y;
    let near_plane = camera.near_plane;
    drop(camera);

    let view = unsafe {
        crate::math::view(camera_pos, camera_dir, camera_up)
    };
    let aspect = ctx.swapchain_extent.0 as f32
        / ctx.swapchain_extent.1 as f32;
    let proj =
        perspective_infinite_reverse(fov_y, aspect, near_plane);
    let screen_size = cgmath::Vector2::new(
        ctx.swapchain_extent.0 as f32,
        ctx.swapchain_extent.1 as f32,
    );

    let (ray_origin, ray_direction) =
        screen_to_world_ray(mouse_pos, screen_size, view, proj);

    let light_pos = ctx.rt_debug().light_position;
    let plane_point = light_pos;
    let plane_normal = -camera_dir;

    let denom = plane_normal.dot(ray_direction);

    if denom.abs() > std::f32::EPSILON {
        let t = (plane_point - ray_origin).dot(plane_normal)
            / denom;

        if t >= 0.0 {
            let intersection =
                ray_origin + ray_direction * t;

            {
                let mut gizmo = ctx.light_gizmo_mut();
                let draggable = gizmo.draggable.clone();
                gizmo_update_position_with_constraint(
                    &mut gizmo.position,
                    intersection,
                    &draggable,
                    ctx.gui_data.is_ctrl_pressed,
                );
            }

            ctx.rt_debug_mut().light_position =
                ctx.light_gizmo().position.position;
        }
    }

    Ok(())
}

fn process_bone_selection(
    ctx: &mut EcsContext,
) -> Result<()> {
    if !ctx.world.contains_resource::<BoneGizmoData>() {
        return Ok(());
    }

    let (visible, display_style) = {
        let bone_gizmo = ctx.world.resource::<BoneGizmoData>();
        (bone_gizmo.visible, bone_gizmo.display_style)
    };

    if !visible || display_style != BoneDisplayStyle::Octahedral {
        return Ok(());
    }

    if ctx.light_gizmo().selectable.is_selected {
        return Ok(());
    }

    let (ray_origin, ray_direction) = compute_bone_pick_ray(ctx);

    let (skeleton_id, transforms, offsets, mesh_scale) = {
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

    let Some(skeleton) =
        ctx.assets.get_skeleton_by_skeleton_id(skel_id)
    else {
        return Ok(());
    };
    let skeleton = skeleton.clone();

    let hit = select_bone_by_ray(
        ray_origin, ray_direction, &skeleton, &transforms, &offsets, mesh_scale,
    );

    let is_shift = ctx.gui_data.is_shift_pressed;
    let new_active_bone = apply_bone_selection_result(
        ctx, &skeleton, hit, is_shift,
    );

    sync_bone_selection_to_hierarchy(ctx, new_active_bone, is_shift);

    Ok(())
}

fn compute_bone_pick_ray(
    ctx: &EcsContext,
) -> (Vector3<f32>, Vector3<f32>) {
    let mouse_pos = viewport_local_mouse_pos(ctx);
    let screen_size = cgmath::Vector2::new(
        ctx.swapchain_extent.0 as f32,
        ctx.swapchain_extent.1 as f32,
    );

    let camera = ctx.camera();
    let camera_pos =
        crate::ecs::compute_camera_position(&camera);
    let camera_dir =
        crate::ecs::compute_camera_direction(&camera);
    let camera_up =
        crate::ecs::compute_camera_up(&camera);
    let fov_y = camera.fov_y;
    let near_plane = camera.near_plane;
    drop(camera);

    let view = unsafe {
        crate::math::view(camera_pos, camera_dir, camera_up)
    };
    let aspect = screen_size.x / screen_size.y;
    let proj =
        crate::math::coordinate_system::perspective_infinite_reverse(
            fov_y, aspect, near_plane,
        );

    let (ray_origin, ray_direction) =
        screen_to_world_ray(mouse_pos, screen_size, view, proj);

    crate::log!(
        "bone_select: viewport_pos=({:.0},{:.0}) viewport_size=({:.0},{:.0}) mouse_raw=({:.0},{:.0}) mouse_local=({:.1},{:.1}) ray_origin=({:.2},{:.2},{:.2}) ray_dir=({:.3},{:.3},{:.3})",
        ctx.gui_data.viewport_position[0],
        ctx.gui_data.viewport_position[1],
        ctx.gui_data.viewport_size[0],
        ctx.gui_data.viewport_size[1],
        ctx.gui_data.mouse_pos[0],
        ctx.gui_data.mouse_pos[1],
        mouse_pos.x,
        mouse_pos.y,
        ray_origin.x, ray_origin.y, ray_origin.z,
        ray_direction.x, ray_direction.y, ray_direction.z,
    );

    (ray_origin, ray_direction)
}

fn apply_bone_selection_result(
    ctx: &mut EcsContext,
    skeleton: &crate::animation::Skeleton,
    hit: Option<(usize, f32)>,
    is_shift: bool,
) -> Option<BoneId> {
    let mut selection = ctx.bone_selection_mut();

    match hit {
        Some((bone_idx, _distance)) => {
            let bone_id = bone_idx as BoneId;
            let descendants = skeleton.collect_descendants(bone_id);

            let bone_name = skeleton
                .bones
                .iter()
                .find(|b| b.id as usize == bone_idx)
                .map(|b| b.name.as_str())
                .unwrap_or("unknown");

            if is_shift {
                if selection.selected_bone_indices.contains(&bone_idx) {
                    selection.selected_bone_indices.remove(&bone_idx);
                    for desc_id in &descendants {
                        selection.selected_bone_indices.remove(&(*desc_id as usize));
                    }
                    if selection.active_bone_index == Some(bone_idx) {
                        selection.active_bone_index = selection
                            .selected_bone_indices
                            .iter()
                            .copied()
                            .next();
                    }
                } else {
                    selection.selected_bone_indices.insert(bone_idx);
                    for desc_id in &descendants {
                        selection.selected_bone_indices.insert(*desc_id as usize);
                    }
                    selection.active_bone_index = Some(bone_idx);
                }
            } else {
                selection.selected_bone_indices.clear();
                selection.selected_bone_indices.insert(bone_idx);
                for desc_id in &descendants {
                    selection.selected_bone_indices.insert(*desc_id as usize);
                }
                selection.active_bone_index = Some(bone_idx);
            }

            crate::log!(
                "Bone selected: [{}] '{}' (active={:?}, total={}, descendants={})",
                bone_idx, bone_name,
                selection.active_bone_index,
                selection.selected_bone_indices.len(),
                descendants.len()
            );

            Some(bone_id)
        }
        None => {
            if !is_shift {
                selection.selected_bone_indices.clear();
                selection.active_bone_index = None;
            }
            None
        }
    }
}

fn sync_bone_selection_to_hierarchy(
    ctx: &mut EcsContext,
    new_active_bone: Option<BoneId>,
    is_shift: bool,
) {
    if let Some(bone_id) = new_active_bone {
        let mut hierarchy = ctx.hierarchy_state_mut();
        hierarchy.selected_bone_id = Some(bone_id);
        hierarchy.display_mode = HierarchyDisplayMode::Bones;
    } else if !is_shift {
        ctx.hierarchy_state_mut().selected_bone_id = None;
    }
}
