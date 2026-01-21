use std::ffi::c_void;
use std::mem::size_of;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use cgmath::SquareMatrix;
use vulkanalia::prelude::v1_0::*;

use crate::asset::{AnimationClipAsset, AssetStorage, MeshAsset, NodeAsset, SkeletonAsset};
use crate::debugview::DebugViewData;
use crate::ecs::playback_play;
use crate::ecs::resource::{
    AnimationPlayback, AnimationRegistry, MeshAssets, ModelState, NodeAssets,
};
use crate::ecs::world::{AnimationState, Transform, World};
use crate::loader::texture::load_png_image;
use crate::loader::{ModelLoadResult, TextureSource};
use crate::scene::billboard::BillboardData;
use crate::render::MaterialUBO;
use crate::scene::graphics_resource::{GraphicsResources, MaterialId, MeshBuffer, NodeData};
use crate::scene::raytracing::RayTracingData;
use crate::scene::CubeModel;
use crate::vulkanr::buffer::{RRIndexBuffer, RRVertexBuffer};
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::data as vulkan_data;
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

pub unsafe fn load_cube_model_system(
    size: f32,
    position: [f32; 3],
    instance: &Instance,
    device: &RRDevice,
    command_pool: &Rc<RRCommandPool>,
    swapchain: &RRSwapchain,
    graphics: &mut GraphicsResources,
    raytracing: &mut RayTracingData,
    debug_view: &mut DebugViewData,
    world: &mut World,
    assets: &mut AssetStorage,
) -> Result<()> {
    crate::log!(
        "=== Loading cube model: size={}, position=({}, {}, {}) ===",
        size,
        position[0],
        position[1],
        position[2]
    );

    let load_result = crate::loader::cube::create_cube(size, position);

    apply_model_to_resources(
        &load_result,
        "cube",
        instance,
        device,
        command_pool,
        swapchain,
        graphics,
        raytracing,
        world,
        assets,
    )?;

    debug_view.cube_model = Some(CubeModel::new_at_position(size, position));

    crate::log!("=== Cube model loaded successfully ===");
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

    setup_animation_system(world, load_result);
    setup_nodes(world, load_result);

    for (i, loaded_mesh) in load_result.meshes.iter().enumerate() {
        let mesh_buffer =
            create_mesh_buffer(instance, device, command_pool, graphics, loaded_mesh, i)?;
        let material_id = create_material_for_mesh(instance, device, graphics, &mesh_buffer, i)?;

        graphics.meshes.push(mesh_buffer);
        graphics.mesh_material_ids.push(material_id);
    }

    apply_initial_pose(instance, device, command_pool, graphics, world, load_result)?;
    rebuild_acceleration_structures(instance, device, command_pool, graphics, raytracing)?;
    update_ray_query_descriptor(device, raytracing)?;

    {
        let mut billboard = world.resource_mut::<BillboardData>();
        update_billboard_descriptor(device, swapchain, &mut *billboard)?;
    }

    create_ecs_entities(model_name, graphics, world, assets);

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
    graphics.objects.reset_to(3);

    if world.contains_resource::<AnimationRegistry>() {
        let mut anim_registry = world.resource_mut::<AnimationRegistry>();
        anim_registry.clear();
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

fn setup_animation_system(world: &mut World, load_result: &ModelLoadResult) {
    if world.contains_resource::<AnimationRegistry>() {
        let mut anim_registry = world.resource_mut::<AnimationRegistry>();
        anim_registry.animation = load_result.animation_system.clone();
        anim_registry.morph_animation = load_result.morph_animation.clone();
    }

    if world.contains_resource::<ModelState>() {
        let mut model_state = world.resource_mut::<ModelState>();
        model_state.has_skinned_meshes = load_result.has_skinned_meshes;
        model_state.node_animation_scale = load_result.node_animation_scale;
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
        Some(TextureSource::File(path)) => match load_png_image(path) {
            Ok((image_data, width, height)) => {
                (mesh.image, mesh.image_memory, mesh.mip_level) = create_texture_image_pixel(
                    instance,
                    device,
                    command_pool,
                    &image_data,
                    width,
                    height,
                )?;
            }
            Err(e) => {
                crate::log!("Failed to load texture {}: {}", path, e);
                let white_pixel = vec![255u8, 255, 255, 255];
                (mesh.image, mesh.image_memory, mesh.mip_level) =
                    create_texture_image_pixel(instance, device, command_pool, &white_pixel, 1, 1)?;
            }
        },
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
    load_result: &ModelLoadResult,
) -> Result<()> {
    if load_result.animation_system.clips.is_empty() {
        return Ok(());
    }

    crate::log!("Applying initial pose (time=0) for animation...");

    let first_clip_id = load_result.animation_system.clips.first().map(|c| c.id);
    if let Some(clip_id) = first_clip_id {
        let has_playback = world.contains_resource::<AnimationPlayback>();
        if has_playback {
            let mut playback = world.resource_mut::<AnimationPlayback>();
            playback_play(&mut playback, clip_id);
        } else {
            let mut playback = AnimationPlayback::new();
            playback_play(&mut playback, clip_id);
            world.insert_resource(playback);
        }
    }

    let skeleton_id = graphics.meshes.first().and_then(|m| m.skeleton_id);

    if let Some(skel_id) = skeleton_id {
        let playback = world.resource::<AnimationPlayback>();
        let mut anim_registry = world.resource_mut::<AnimationRegistry>();
        anim_registry.animation.apply_to_skeleton(skel_id, &*playback);
        drop(anim_registry);
        drop(playback);

        let anim_registry = world.resource::<AnimationRegistry>();
        for mesh_idx in 0..graphics.meshes.len() {
            apply_skinning_to_mesh(
                instance,
                device,
                command_pool,
                graphics,
                &anim_registry.animation,
                mesh_idx,
            )?;
        }
    }

    let has_node_animation = !load_result.has_skinned_meshes && !graphics.meshes.is_empty();
    if has_node_animation {
        let anim_registry = world.resource::<AnimationRegistry>();
        let model_state = world.resource::<ModelState>();
        let mut node_assets = world.resource_mut::<NodeAssets>();
        let animation = anim_registry.animation.clone();
        let node_animation_scale = model_state.node_animation_scale;
        drop(anim_registry);
        drop(model_state);

        let updated_meshes =
            graphics.prepare_node_animation(&mut node_assets.nodes, &animation, node_animation_scale);

        for mesh_idx in updated_meshes {
            if let Err(e) = upload_mesh_vertices(instance, device, command_pool, graphics, mesh_idx)
            {
                crate::log!("Failed to upload initial node animation mesh {}: {}", mesh_idx, e);
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
    animation: &crate::animation::AnimationSystem,
    mesh_idx: usize,
) -> Result<()> {
    let (skin_data, skel_id) = {
        let mesh = &graphics.meshes[mesh_idx];
        (mesh.skin_data.clone(), mesh.skeleton_id)
    };

    if let (Some(skin_data), Some(skel_id)) = (skin_data, skel_id) {
        if let Some(skeleton) = animation.get_skeleton(skel_id) {
            let vertex_count = skin_data.base_positions.len();
            let mut skinned_positions = vec![cgmath::Vector3::new(0.0, 0.0, 0.0); vertex_count];
            let mut skinned_normals = vec![cgmath::Vector3::new(0.0, 1.0, 0.0); vertex_count];

            skin_data.apply_skinning(skeleton, &mut skinned_positions, &mut skinned_normals);

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
                    "Failed to update vertex buffer for mesh {} with initial pose: {}",
                    mesh_idx,
                    e
                );
            }
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
    let texture_clone = billboard.render.texture.clone();
    if let Some(ref billboard_texture) = texture_clone {
        billboard
            .render
            .descriptor_set
            .update_descriptor_sets(device, swapchain, billboard_texture)?;
        crate::log!("Re-updated billboard.render.descriptor_set after model reload");
    }
    Ok(())
}

fn create_ecs_entities(
    model_name: &str,
    graphics: &GraphicsResources,
    world: &mut World,
    assets: &mut AssetStorage,
) {
    let name = std::path::Path::new(model_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .to_string();

    let (skeletons, clips, has_animation, first_clip_id) = {
        let anim_registry = world.resource::<AnimationRegistry>();
        let skeletons = anim_registry.animation.skeletons.clone();
        let clips = anim_registry.animation.clips.clone();
        let has_animation = !clips.is_empty();
        let first_clip_id = clips.first().map(|c| c.id);
        (skeletons, clips, has_animation, first_clip_id)
    };

    for skeleton in &skeletons {
        let skeleton_asset = SkeletonAsset {
            id: 0,
            skeleton_id: skeleton.id,
            skeleton: skeleton.clone(),
        };
        assets.add_skeleton(skeleton_asset);
    }

    for clip in &clips {
        let clip_asset = AnimationClipAsset {
            id: 0,
            clip_id: clip.id,
            clip: clip.clone(),
        };
        assets.add_animation_clip(clip_asset);
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

    for (mesh_idx, mesh) in graphics.meshes.iter().enumerate() {
        let entity_name = format!("{}_{}", name, mesh_idx);

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
            .with_mesh(asset_id, mesh.object_index);

        if has_animation {
            let mut anim_state = AnimationState::new();
            anim_state.current_clip_id = first_clip_id;
            builder = builder.with_animation_state(anim_state);
        }

        let entity = builder.build();
        crate::log!(
            "Created ECS entity {} (asset_id={}) for mesh {}: entity_id={}",
            entity_name,
            asset_id,
            mesh_idx,
            entity
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
