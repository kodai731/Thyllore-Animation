use std::ffi::c_void;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{anyhow, Result};
use cgmath::SquareMatrix;
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;
use crate::asset::{AnimationClipAsset, AssetStorage, MeshAsset, NodeAsset, SkeletonAsset};
use crate::debugview::gizmo::{BoneGizmoData, ConstraintGizmoData};
use crate::ecs::resource::{
    AnimationType, ClipLibrary, MeshAssets, ModelState,
    NodeAssets, TimelineState,
};
use crate::animation::editable::SourceClipId;
use crate::ecs::component::{AnimationMeta, ClipSchedule, EntityIcon};
use crate::ecs::world::{Animator, Transform, World};
use crate::loader::texture::load_png_image;
use crate::loader::{ModelLoadResult, TextureSource};
use crate::render::MaterialUBO;
use crate::app::billboard::BillboardData;
use crate::app::graphics_resource::{GraphicsResources, MaterialId, MeshBuffer, NodeData};
use crate::app::raytracing::RayTracingData;
use crate::vulkanr::buffer::{RRIndexBuffer, RRVertexBuffer};
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::data as vulkan_data;
use crate::vulkanr::data::VertexData;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::image::{
    create_image_view, create_texture_image_pixel, create_texture_sampler,
};
use crate::vulkanr::raytracing::acceleration::RRAccelerationStructure;
use crate::vulkanr::swapchain::RRSwapchain;
use crate::vulkanr::vulkan::Instance;

pub unsafe fn load_model_from_file_system(
    path: &str,
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    crate::log!("=== Loading model from path: {} ===", path);

    let load_result = load_model_data(path)?;

    apply_model_to_resources(
        &load_result,
        path,
        instance,
        device,
        command_pool,
        swapchain,
        graphics,
        raytracing,
        world,
        assets,
    )?;

    crate::log!("=== Model loaded successfully ===");
    Ok(())
}

unsafe fn load_model_data(path: &str) -> Result<ModelLoadResult> {
    let path_lower = path.to_lowercase();

    if path_lower.ends_with(".fbx") {
        let result = crate::loader::fbx::load_fbx_to_graphics_resources(path)?;
        Ok(ModelLoadResult::from_fbx(result))
    } else if path_lower.ends_with(".gltf") || path_lower.ends_with(".glb") {
        let result = crate::loader::gltf::load_gltf_file(path);
        Ok(ModelLoadResult::from_gltf(result))
    } else {
        Err(anyhow!(
            "Unsupported file format. Only FBX and glTF/GLB are supported."
        ))
    }
}

unsafe fn apply_model_to_resources(
    load_result: &ModelLoadResult,
    model_name: &str,
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    cleanup_resources(device, graphics, raytracing, world, assets)?;

    let mesh_count = load_result.meshes.len();
    let reserved_scene_objects = 4;
    let required_materials = mesh_count as u32 + reserved_scene_objects as u32;
    let required_objects =
        graphics.objects.get_next_slot() + mesh_count + reserved_scene_objects;

    graphics
        .materials
        .ensure_capacity(device, required_materials)?;
    graphics.objects.ensure_capacity(
        instance,
        device,
        swapchain.swapchain_images.len(),
        required_objects,
    )?;

    setup_animation_system(world, load_result, assets);
    setup_nodes(world, load_result);

    for (i, loaded_mesh) in load_result.meshes.iter().enumerate() {
        let mesh_buffer =
            create_mesh_buffer(instance, device, command_pool, graphics, loaded_mesh, i, model_name)?;
        let material_id = create_material_for_mesh(instance, device, graphics, &mesh_buffer, i)?;

        graphics.meshes.push(mesh_buffer);
        graphics.mesh_material_ids.push(material_id);
    }

    apply_initial_pose(instance, device, command_pool, graphics, world, assets, load_result)?;
    rebuild_acceleration_structures(instance, device, command_pool, graphics, raytracing)?;
    update_ray_query_descriptor(device, raytracing)?;

    {
        let mut billboard = world.resource_mut::<BillboardData>();
        update_billboard_descriptor(device, swapchain, &mut *billboard)?;
    }

    let animation_type = if load_result.has_skinned_meshes {
        AnimationType::Skeletal
    } else if !load_result.animation_system.clips.is_empty() {
        AnimationType::Node
    } else {
        AnimationType::None
    };
    let node_animation_scale = load_result.node_animation_scale;

    create_ecs_entities(
        model_name,
        graphics,
        world,
        assets,
        animation_type,
        node_animation_scale,
    );

    apply_loaded_constraints(load_result, world);
    initialize_bone_gizmo_visibility(world, assets, graphics);
    initialize_constraint_gizmo_visibility(world);

    Ok(())
}

unsafe fn cleanup_resources(
    device: &RRDevice,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    crate::log!("Cleaning up model resources...");

    device.device.device_wait_idle()?;

    if let Some(ref mut accel) = raytracing.acceleration_structure {
        accel.destroy(&device.device);
    }
    raytracing.acceleration_structure = None;

    graphics.clear_meshes(device);
    graphics.mesh_material_ids.clear();
    graphics.materials.clear_materials(&device.device);
    graphics.objects.reset_to(4);

    if world.contains_resource::<ClipLibrary>() {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        clip_library.clear();
    }

    if world.contains_resource::<TimelineState>() {
        let mut timeline_state = world.resource_mut::<TimelineState>();
        timeline_state.current_clip_id = None;
        timeline_state.current_time = 0.0;
        timeline_state.selected_keyframes.clear();
        timeline_state.expanded_tracks.clear();
    }

    if world.contains_resource::<MeshAssets>() {
        let mut mesh_assets = world.resource_mut::<MeshAssets>();
        mesh_assets.meshes.clear();
    }

    if world.contains_resource::<NodeAssets>() {
        let mut node_assets = world.resource_mut::<NodeAssets>();
        node_assets.nodes.clear();
    }

    world.clear();
    assets.clear();

    crate::log!("Model resources cleaned up");
    Ok(())
}

pub unsafe fn cleanup_model_resources(rrdevice: &RRDevice, data: &mut AppData) {
    crate::log!("Cleaning up model resources...");

    rrdevice.device.device_wait_idle().ok();

    if let Some(ref mut accel) = data.raytracing.acceleration_structure {
        accel.destroy(&rrdevice.device);
        crate::log!("Destroyed acceleration structure");
    }
    data.raytracing.acceleration_structure = None;

    data.graphics_resources.clear_meshes(rrdevice);
    data.graphics_resources.mesh_material_ids.clear();
    data.graphics_resources
        .materials
        .clear_materials(&rrdevice.device);
    crate::log!("Cleared materials");

    crate::log!("Model resources cleaned up");
}

fn setup_animation_system(
    world: &mut World,
    load_result: &ModelLoadResult,
    assets: &mut AssetStorage,
) {
    if world.contains_resource::<ClipLibrary>() {
        let mut clip_library = world.resource_mut::<ClipLibrary>();
        clip_library.animation = load_result.animation_system.clone();
        clip_library.morph_animation = load_result.morph_animation.clone();
    }

    for skeleton in &load_result.skeletons {
        let skeleton_asset = SkeletonAsset {
            id: 0,
            skeleton_id: skeleton.id,
            skeleton: skeleton.clone(),
        };
        assets.add_skeleton(skeleton_asset);
    }

    if world.contains_resource::<ModelState>() {
        let mut model_state = world.resource_mut::<ModelState>();
        model_state.has_skinned_meshes = load_result.has_skinned_meshes;
    }
}

fn setup_nodes(world: &mut World, load_result: &ModelLoadResult) {
    let nodes: Vec<NodeData> = load_result
        .nodes
        .iter()
        .map(|n| NodeData {
            index: n.index,
            name: n.name.clone(),
            parent_index: n.parent_index,
            local_transform: n.local_transform,
            global_transform: cgmath::Matrix4::identity(),
        })
        .collect();

    let node_count = nodes.len();

    if world.contains_resource::<NodeAssets>() {
        let mut node_assets = world.resource_mut::<NodeAssets>();
        node_assets.nodes = nodes;
    }

    crate::log!("Loaded {} nodes into NodeAssets", node_count);
}

unsafe fn create_mesh_buffer(
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    graphics: &mut GraphicsResources,
    loaded_mesh: &crate::loader::LoadedMesh,
    mesh_index: usize,
    model_path: &str,
) -> Result<MeshBuffer> {
    let mut mesh = MeshBuffer::default();

    match &loaded_mesh.texture {
        Some(TextureSource::Embedded(tex)) => {
            (mesh.image, mesh.image_memory, mesh.mip_level) = create_texture_image_pixel(
                instance,
                device,
                command_pool,
                &tex.data,
                tex.width,
                tex.height,
            )?;
        }
        Some(TextureSource::File(texture_path)) => {
            let resolved = resolve_texture_path(texture_path, model_path);
            let load_path = resolved.to_string_lossy();
            match load_png_image(&load_path) {
                Ok((image_data, width, height)) => {
                    (mesh.image, mesh.image_memory, mesh.mip_level) =
                        create_texture_image_pixel(
                            instance,
                            device,
                            command_pool,
                            &image_data,
                            width,
                            height,
                        )?;
                }
                Err(e) => {
                    crate::log!("Failed to load texture {}: {}", load_path, e);
                    let white_pixel = vec![255u8, 255, 255, 255];
                    (mesh.image, mesh.image_memory, mesh.mip_level) =
                        create_texture_image_pixel(
                            instance, device, command_pool, &white_pixel, 1, 1,
                        )?;
                }
            }
        }
        None => {
            let white_pixel = vec![255u8, 255, 255, 255];
            (mesh.image, mesh.image_memory, mesh.mip_level) =
                create_texture_image_pixel(instance, device, command_pool, &white_pixel, 1, 1)?;
        }
    }

    mesh.image_view = create_image_view(
        device,
        mesh.image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageAspectFlags::COLOR,
        mesh.mip_level,
    )?;
    mesh.sampler = create_texture_sampler(device, mesh.mip_level)?;

    mesh.vertex_data = loaded_mesh.vertex_data.clone();
    mesh.skin_data = loaded_mesh.skin_data.clone();
    mesh.skeleton_id = loaded_mesh.skeleton_id;
    mesh.node_index = loaded_mesh.node_index;
    mesh.base_vertices = loaded_mesh.local_vertices.clone();

    mesh.vertex_buffer = RRVertexBuffer::new(
        instance,
        device,
        command_pool,
        (size_of::<vulkan_data::Vertex>() * mesh.vertex_data.vertices.len()) as vk::DeviceSize,
        mesh.vertex_data.vertices.as_ptr() as *const c_void,
        mesh.vertex_data.vertices.len(),
    );

    mesh.index_buffer = RRIndexBuffer::new(
        instance,
        device,
        command_pool,
        (size_of::<u32>() * mesh.vertex_data.indices.len()) as u64,
        mesh.vertex_data.indices.as_ptr() as *const c_void,
        mesh.vertex_data.indices.len(),
    );

    mesh.object_index = graphics.objects.allocate_slot();
    crate::log!(
        "Allocated object_index {} for mesh {}",
        mesh.object_index,
        mesh_index
    );

    Ok(mesh)
}

unsafe fn create_material_for_mesh(
    instance: &Instance,
    device: &RRDevice,
    graphics: &mut GraphicsResources,
    mesh: &MeshBuffer,
    mesh_index: usize,
) -> Result<MaterialId> {
    let material_name = format!("material_{}", mesh_index);
    let material_properties = MaterialUBO::default();

    let material_id = graphics.materials.create_material_with_texture(
        instance,
        device,
        &material_name,
        mesh.image_view,
        mesh.sampler,
        material_properties,
    )?;

    crate::log!("Created material {} for mesh {}", material_id, mesh_index);
    Ok(material_id)
}

unsafe fn apply_initial_pose(
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    graphics: &mut GraphicsResources,
    world: &mut World,
    assets: &AssetStorage,
    load_result: &ModelLoadResult,
) -> Result<()> {
    use crate::ecs::{
        create_pose_from_rest, compute_pose_global_transforms,
        sample_clip_to_pose,
    };

    if load_result.animation_system.clips.is_empty() {
        return Ok(());
    }

    crate::log!("Applying initial pose (time=0) for animation...");

    if !load_result.animation_system.clips.is_empty() {
        if world.contains_resource::<TimelineState>() {
            let mut timeline = world.resource_mut::<TimelineState>();
            timeline.playing = false;
            timeline.current_time = 0.0;
        }
    }

    let skeleton_id = graphics.meshes.first().and_then(|m| m.skeleton_id);

    if let Some(skel_id) = skeleton_id {
        let (current_time, looping) = if world.contains_resource::<TimelineState>() {
            let timeline = world.resource::<TimelineState>();
            (timeline.current_time, timeline.looping)
        } else {
            (0.0, true)
        };
        let clip_library = world.resource::<ClipLibrary>();

        let skeleton = assets.get_skeleton_by_skeleton_id(skel_id);
        let clip = clip_library.animation.clips.first();

        if let (Some(skeleton), Some(clip)) = (skeleton, clip) {
            let mut pose = create_pose_from_rest(skeleton);
            sample_clip_to_pose(
                clip,
                current_time,
                skeleton,
                &mut pose,
                looping,
            );
            let globals =
                compute_pose_global_transforms(skeleton, &pose);
            let skeleton_clone = skeleton.clone();
            drop(clip_library);

            for mesh_idx in 0..graphics.meshes.len() {
                apply_skinning_to_mesh(
                    instance,
                    device,
                    command_pool,
                    graphics,
                    &globals,
                    &skeleton_clone,
                    mesh_idx,
                )?;
            }
        } else {
            drop(clip_library);
        }
    }

    let has_node_animation =
        !load_result.has_skinned_meshes && !graphics.meshes.is_empty();
    if has_node_animation {
        let clip_library = world.resource::<ClipLibrary>();
        let mut node_assets = world.resource_mut::<NodeAssets>();
        let node_animation_scale = load_result.node_animation_scale;

        let skel_id =
            graphics.meshes.first().and_then(|m| m.skeleton_id);
        let skeleton_clone =
            skel_id.and_then(|id| assets.get_skeleton_by_skeleton_id(id).cloned());
        let clip_clone =
            clip_library.animation.clips.first().cloned();
        drop(clip_library);

        let updated_meshes =
            if let (Some(skeleton), Some(clip)) =
                (&skeleton_clone, &clip_clone)
            {
                let mut pose = create_pose_from_rest(skeleton);
                sample_clip_to_pose(
                    clip, 0.0, skeleton, &mut pose, false,
                );

                graphics.prepare_node_animation(
                    &mut node_assets.nodes,
                    skeleton,
                    &pose,
                    node_animation_scale,
                )
            } else {
                Vec::new()
            };

        for mesh_idx in updated_meshes {
            if let Err(e) = upload_mesh_vertices(
                instance,
                device,
                command_pool,
                graphics,
                mesh_idx,
            ) {
                crate::log!(
                    "Failed to upload initial node animation mesh {}: {}",
                    mesh_idx,
                    e
                );
            }
        }
    }

    crate::log!("Initial pose applied successfully");
    Ok(())
}

unsafe fn apply_skinning_to_mesh(
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    graphics: &mut GraphicsResources,
    global_transforms: &[cgmath::Matrix4<f32>],
    skeleton: &crate::animation::Skeleton,
    mesh_idx: usize,
) -> Result<()> {
    use crate::ecs::apply_skinning;

    let skin_data = {
        let mesh = &graphics.meshes[mesh_idx];
        mesh.skin_data.clone()
    };

    if let Some(skin_data) = skin_data {
        let vertex_count = skin_data.base_positions.len();
        let mut skinned_positions =
            vec![cgmath::Vector3::new(0.0, 0.0, 0.0); vertex_count];
        let mut skinned_normals =
            vec![cgmath::Vector3::new(0.0, 1.0, 0.0); vertex_count];

        apply_skinning(
            &skin_data,
            global_transforms,
            skeleton,
            &mut skinned_positions,
            &mut skinned_normals,
        );

        let mesh = &mut graphics.meshes[mesh_idx];
        for (i, pos) in skinned_positions.iter().enumerate() {
            if i < mesh.vertex_data.vertices.len() {
                mesh.vertex_data.vertices[i].pos.x = pos.x;
                mesh.vertex_data.vertices[i].pos.y = pos.y;
                mesh.vertex_data.vertices[i].pos.z = pos.z;
            }
        }
        for (i, normal) in skinned_normals.iter().enumerate() {
            if i < mesh.vertex_data.vertices.len() {
                mesh.vertex_data.vertices[i].normal.x = normal.x;
                mesh.vertex_data.vertices[i].normal.y = normal.y;
                mesh.vertex_data.vertices[i].normal.z = normal.z;
            }
        }

        if let Err(e) = mesh.vertex_buffer.update(
            instance,
            device,
            command_pool,
            (size_of::<vulkan_data::Vertex>() * mesh.vertex_data.vertices.len())
                as vk::DeviceSize,
            mesh.vertex_data.vertices.as_ptr() as *const c_void,
            mesh.vertex_data.vertices.len(),
        ) {
            crate::log!(
                "Failed to update vertex buffer for mesh {}: {}",
                mesh_idx,
                e
            );
        }
    }

    Ok(())
}

unsafe fn upload_mesh_vertices(
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    graphics: &mut GraphicsResources,
    mesh_idx: usize,
) -> Result<()> {
    if mesh_idx >= graphics.meshes.len() {
        return Ok(());
    }

    let mesh = &mut graphics.meshes[mesh_idx];
    let vertices = &mesh.vertex_data.vertices;
    let vertex_count = vertices.len();
    let vertex_stride = size_of::<vulkan_data::Vertex>();

    mesh.vertex_buffer.update(
        instance,
        device,
        command_pool,
        (vertex_stride * vertex_count) as vk::DeviceSize,
        vertices.as_ptr() as *const c_void,
        vertex_count,
    )?;

    Ok(())
}

pub unsafe fn rebuild_acceleration_structures(
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    graphics: &GraphicsResources,
    raytracing: &mut RayTracingData,
) -> Result<()> {
    crate::log!("Rebuilding acceleration structures...");

    let mut acceleration_structure = RRAccelerationStructure::new();

    for mesh in &graphics.meshes {
        let blas = RRAccelerationStructure::create_blas(
            instance,
            device,
            command_pool.as_ref(),
            &mesh.vertex_buffer.buffer,
            mesh.vertex_data.vertices.len() as u32,
            std::mem::size_of::<vulkan_data::Vertex>() as u32,
            &mesh.index_buffer.buffer,
            mesh.vertex_data.indices.len() as u32,
        )?;

        acceleration_structure.blas_list.push(blas);
        crate::log!("Created BLAS for mesh");
    }

    if !acceleration_structure.blas_list.is_empty() {
        let tlas = RRAccelerationStructure::create_tlas(
            instance,
            device,
            command_pool.as_ref(),
            &acceleration_structure.blas_list,
        )?;
        acceleration_structure.tlas = tlas;
        crate::log!(
            "Created TLAS with {} instances",
            acceleration_structure.blas_list.len()
        );
    }

    raytracing.acceleration_structure = Some(acceleration_structure);
    crate::log!("Acceleration structures rebuilt successfully");
    Ok(())
}

pub unsafe fn rebuild_acceleration_structures_from_data(
    instance: &Instance,
    rrdevice: &RRDevice,
    data: &mut AppData,
    rrcommand_pool: &Rc<RRCommandPool>,
) -> Result<()> {
    rebuild_acceleration_structures(
        instance,
        rrdevice,
        rrcommand_pool,
        &data.graphics_resources,
        &mut data.raytracing,
    )
}

unsafe fn update_ray_query_descriptor(
    device: &RRDevice,
    raytracing: &mut RayTracingData,
) -> Result<()> {
    if let Some(ref accel_struct) = raytracing.acceleration_structure {
        if let Some(tlas) = accel_struct.tlas.acceleration_structure {
            if let Some(ref mut ray_query_desc) = raytracing.ray_query_descriptor {
                ray_query_desc.update_tlas(device, tlas)?;
                crate::log!("Updated ray_query_descriptor with new TLAS");
            }
        }
    }
    Ok(())
}

unsafe fn update_billboard_descriptor(
    device: &RRDevice,
    swapchain: &RRSwapchain,
    billboard: &mut BillboardData,
) -> Result<()> {
    let texture_clone = billboard.render_state.texture.clone();
    if let Some(ref billboard_texture) = texture_clone {
        billboard
            .render_state
            .descriptor_set
            .update_descriptor_sets(device, swapchain, billboard_texture)?;
        crate::log!("Re-updated billboard.render_state.descriptor_set after model reload");
    }
    Ok(())
}

fn create_ecs_entities(
    model_name: &str,
    graphics: &GraphicsResources,
    world: &mut World,
    assets: &mut AssetStorage,
    animation_type: AnimationType,
    node_animation_scale: f32,
) {
    let name = std::path::Path::new(model_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .to_string();

    let (clips, has_animation) = {
        let clip_library = world.resource::<ClipLibrary>();
        let clips = clip_library.animation.clips.clone();
        let has_animation = !clips.is_empty();
        (clips, has_animation)
    };

    for clip in &clips {
        let clip_asset = AnimationClipAsset {
            id: 0,
            clip_id: clip.id,
            clip: clip.clone(),
        };
        assets.add_animation_clip(clip_asset);
    }

    let bone_names: std::collections::HashMap<u32, String> = assets
        .skeletons
        .values()
        .flat_map(|sa| sa.skeleton.bones.iter().map(|b| (b.id, b.name.clone())))
        .collect();

    if !world.contains_resource::<ClipLibrary>() {
        world.insert_resource(ClipLibrary::new());
    }
    if !world.contains_resource::<TimelineState>() {
        world.insert_resource(TimelineState::new());
    }
    if !world.contains_resource::<crate::ecs::resource::KeyframeCopyBuffer>() {
        world.insert_resource(crate::ecs::resource::KeyframeCopyBuffer::default());
    }
    if !world.contains_resource::<crate::ecs::resource::EditHistory>() {
        world.insert_resource(crate::ecs::resource::EditHistory::new(100));
    }
    if !world.contains_resource::<crate::ecs::resource::ClipBrowserState>() {
        world.insert_resource(crate::ecs::resource::ClipBrowserState::default());
    }

    let mut first_editable_clip_id = None;
    {
        let mut clip_manager = world.resource_mut::<ClipLibrary>();
        for clip in &clips {
            let editable_id = clip_manager.create_from_imported(clip, &bone_names);
            if first_editable_clip_id.is_none() {
                first_editable_clip_id = Some(editable_id);
            }
            crate::log!(
                "Registered editable clip '{}' (editable_id={}, original_id={})",
                clip.name,
                editable_id,
                clip.id
            );
        }
    }

    if let Some(editable_id) = first_editable_clip_id {
        let mut timeline_state = world.resource_mut::<TimelineState>();
        timeline_state.current_clip_id = Some(editable_id);
        crate::log!("Set timeline current_clip_id to {}", editable_id);
    }

    {
        let node_assets = world.resource::<NodeAssets>();
        for node in &node_assets.nodes {
            let node_asset = NodeAsset {
                id: node.index as u64,
                name: node.name.clone(),
                parent_id: node.parent_index.map(|i| i as u64),
                local_transform: node.local_transform,
            };
            assets.add_node(node_asset);
        }
    }

    let parent_entity = world
        .entity()
        .with_name(&name)
        .with_transform(Transform::default())
        .with_visible(true)
        .with_editor_display(EntityIcon::Model, true)
        .build();

    crate::log!(
        "Created parent entity '{}': entity_id={}",
        name,
        parent_entity
    );

    let initial_schedule = if has_animation {
        build_initial_clip_schedule(first_editable_clip_id, world)
    } else {
        ClipSchedule::new()
    };

    for (mesh_idx, mesh) in graphics.meshes.iter().enumerate() {
        let entity_name = format!("{}_{:02}", name, mesh_idx + 1);

        let mesh_asset = MeshAsset {
            id: 0,
            name: entity_name.clone(),
            graphics_mesh_index: mesh_idx,
            object_index: mesh.object_index,
            material_id: graphics.mesh_material_ids.get(mesh_idx).copied(),
            skeleton_id: mesh.skeleton_id,
            node_index: mesh.node_index,
            render_to_gbuffer: mesh.render_to_gbuffer,
        };
        let asset_id = assets.add_mesh(mesh_asset);

        let mut builder = world
            .entity()
            .with_name(&entity_name)
            .with_transform(Transform::default())
            .with_visible(true)
            .with_parent(parent_entity)
            .with_editor_display(EntityIcon::Mesh, false)
            .with_mesh(asset_id, mesh.object_index);

        if has_animation {
            let animator = Animator::new();
            let meta = AnimationMeta {
                animation_type: animation_type.clone(),
                node_animation_scale,
            };
            builder = builder
                .with_animator(animator)
                .with_clip_schedule(initial_schedule.clone())
                .with_animation_meta(meta);
        }

        let entity = builder.build();
        crate::log!(
            "Created ECS entity {} (asset_id={}) for mesh {}: entity_id={}, parent={}",
            entity_name,
            asset_id,
            mesh_idx,
            entity,
            parent_entity
        );
    }

    crate::log!(
        "Created {} ECS entities, {} mesh assets, {} skeletons, {} clips, {} nodes",
        world.entity_count(),
        assets.meshes.len(),
        assets.skeletons.len(),
        assets.animation_clips.len(),
        assets.nodes.len()
    );
}

fn initialize_bone_gizmo_visibility(
    world: &mut World,
    assets: &AssetStorage,
    graphics: &GraphicsResources,
) {
    if !world.contains_resource::<BoneGizmoData>() {
        return;
    }

    let has_skeleton = !assets.skeletons.is_empty();
    let mut bone_gizmo = world.resource_mut::<BoneGizmoData>();

    if has_skeleton {
        bone_gizmo.visible = true;

        let first_skeleton = assets.skeletons.values().next();
        if let Some(skel_asset) = first_skeleton {
            bone_gizmo.cached_skeleton_id = Some(skel_asset.skeleton_id);

            let skeleton = &skel_asset.skeleton;
            let rest_globals = crate::ecs::compute_pose_global_transforms(
                skeleton,
                &crate::ecs::create_pose_from_rest(skeleton),
            );

            bone_gizmo.bone_local_offsets =
                crate::ecs::compute_bone_local_offsets(skeleton, &rest_globals);
            bone_gizmo.cached_global_transforms = rest_globals;
        }
    } else {
        bone_gizmo.visible = false;
        bone_gizmo.cached_skeleton_id = None;
        bone_gizmo.cached_global_transforms.clear();
        bone_gizmo.bone_local_offsets.clear();
    }
}

fn apply_loaded_constraints(
    load_result: &ModelLoadResult,
    world: &mut World,
) {
    use crate::ecs::component::{ConstraintSet, Constrained};

    if load_result.constraints.is_empty() {
        return;
    }

    let animator_entities = world.component_entities::<Animator>();
    if animator_entities.is_empty() {
        return;
    }

    let model_entity = animator_entities[0];

    let mut constraint_set = ConstraintSet::new();
    for loaded in &load_result.constraints {
        constraint_set.add_constraint(
            loaded.constraint_type.clone(),
            loaded.priority,
        );
    }

    world.insert_component(model_entity, constraint_set);
    world.insert_component(model_entity, Constrained);

    crate::log!(
        "Applied {} constraints to entity {}",
        load_result.constraints.len(),
        model_entity
    );
}

fn initialize_constraint_gizmo_visibility(world: &mut World) {
    if !world.contains_resource::<ConstraintGizmoData>() {
        return;
    }

    let has_bone_gizmo_visible = world
        .get_resource::<BoneGizmoData>()
        .map(|bg| bg.visible)
        .unwrap_or(false);

    let has_constraints =
        world.iter_constrained_entities().next().is_some();

    let mut cg = world.resource_mut::<ConstraintGizmoData>();
    cg.visible = has_bone_gizmo_visible && has_constraints;
}

fn build_initial_clip_schedule(
    first_source_id: Option<SourceClipId>,
    world: &World,
) -> ClipSchedule {
    let mut schedule = ClipSchedule::new();

    let Some(source_id) = first_source_id else {
        return schedule;
    };

    let clip_library = world.resource::<ClipLibrary>();
    let duration = clip_library
        .get(source_id)
        .map(|c| c.duration)
        .unwrap_or(1.0);
    drop(clip_library);

    schedule.add_instance(source_id, duration);
    schedule
}

fn resolve_texture_path(texture_path: &str, model_path: &str) -> PathBuf {
    let original = Path::new(texture_path);
    if original.exists() {
        return original.to_path_buf();
    }

    let file_stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let file_name = original
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let model_dir = Path::new(model_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let model_root = model_dir
        .parent()
        .unwrap_or(model_dir);

    let search_dirs = [
        model_dir.to_path_buf(),
        model_dir.join("textures"),
        model_root.join("textures"),
    ];

    let candidate_names: Vec<String> = vec![
        file_name.to_string(),
        format!("{}.png", file_name),
        format!("{}.png", file_stem),
        format!("{}.jpg", file_stem),
    ];

    for dir in &search_dirs {
        for name in &candidate_names {
            let candidate = dir.join(name);
            if candidate.exists() {
                crate::log!(
                    "Resolved texture: {} -> {}",
                    texture_path,
                    candidate.display()
                );
                return candidate;
            }
        }
    }

    crate::log!("Texture not found, using original path: {}", texture_path);
    original.to_path_buf()
}
