use crate::app::{App, AppData};
use crate::loader::fbx::load_fbx_to_render_resources;
use crate::loader::gltf::gltf::*;
use crate::loader::gltf::convert_gltf_to_render_resources;
use crate::loader::texture::load_png_image;
use crate::math::*;
use crate::scene::render_resource::{MaterialUBO, Mesh};
use crate::vulkanr::buffer::*;
use crate::vulkanr::data as vulkan_data;
use crate::vulkanr::data::{Vertex, VertexData};
use crate::vulkanr::device::*;
use crate::vulkanr::image::*;
use crate::vulkanr::vulkan::*;

use anyhow::{anyhow, Result};
use cgmath::Matrix4;
use std::borrow::BorrowMut;
use std::mem::size_of;
use std::os::raw::c_void;
use vulkanalia::prelude::v1_0::*;

impl App {
    pub(crate) unsafe fn load_model(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        let model_path_fbx = "assets/models/phoenix-bird/source/fly.fbx";

        let fbx_result = load_fbx_to_render_resources(model_path_fbx)?;
        crate::log!(
            "Loaded FBX: {} meshes, {} skeletons, {} clips",
            fbx_result.meshes.len(),
            fbx_result.animation_system.skeletons.len(),
            fbx_result.animation_system.clips.len()
        );

        data.render_resources.animation = fbx_result.animation_system;

        for (mesh_idx, fbx_mesh) in fbx_result.meshes.iter().enumerate() {
            crate::log!(
                "Creating Mesh {}: {} vertices, texture: {:?}",
                mesh_idx,
                fbx_mesh.vertex_data.vertices.len(),
                fbx_mesh.texture_path
            );

            let mut mesh = Mesh::default();

            if let Some(texture_path) = &fbx_mesh.texture_path {
                crate::log!("Loading texture: {}", texture_path);
                match load_png_image(texture_path) {
                    Ok((image_data, width, height)) => {
                        match create_texture_image_pixel(
                            instance,
                            rrdevice,
                            data.rrcommand_pool.borrow_mut(),
                            &image_data,
                            width,
                            height,
                        ) {
                            Ok((image, image_memory, mip_level)) => {
                                mesh.image = image;
                                mesh.image_memory = image_memory;
                                mesh.mip_level = mip_level;
                                crate::log!("Texture loaded successfully for mesh {}", mesh_idx);
                            }
                            Err(e) => {
                                crate::log!("Failed to create texture image: {}", e);
                                (mesh.image, mesh.image_memory, mesh.mip_level) =
                                    create_texture_image_pixel(
                                        instance,
                                        rrdevice,
                                        data.rrcommand_pool.borrow_mut(),
                                        &vec![255u8, 255, 255, 255],
                                        1,
                                        1,
                                    )?;
                            }
                        }
                    }
                    Err(e) => {
                        crate::log!("Failed to load texture file: {}", e);
                        (mesh.image, mesh.image_memory, mesh.mip_level) =
                            create_texture_image_pixel(
                                instance,
                                rrdevice,
                                data.rrcommand_pool.borrow_mut(),
                                &vec![255u8, 255, 255, 255],
                                1,
                                1,
                            )?;
                    }
                }
            } else {
                (mesh.image, mesh.image_memory, mesh.mip_level) = create_texture_image_pixel(
                    instance,
                    rrdevice,
                    data.rrcommand_pool.borrow_mut(),
                    &vec![255u8, 255, 255, 255],
                    1,
                    1,
                )?;
            }

            mesh.vertex_data = fbx_mesh.vertex_data.clone();
            mesh.skin_data = fbx_mesh.skin_data.clone();
            mesh.skeleton_id = fbx_mesh.skeleton_id;

            data.render_resources.meshes.push(mesh);
        }

        if !data.render_resources.animation.clips.is_empty() {
            data.animation_playing = true;
            data.current_animation_index = 0;
            data.animation_time = 0.0;

            data.render_resources.animation.play(0);

            let clip_count = data.render_resources.animation.clips.len();
            crate::log!("Animation loaded: {} clips", clip_count);

            if let Some(clip) = data.render_resources.animation.clips.first() {
                crate::log!("Animation 0 duration: {} seconds", clip.duration);
            }
        }

        data.current_model_path = model_path_fbx.to_string();

        crate::log!("=== FBX model loaded successfully ===");
        Ok(())
    }

    pub(crate) unsafe fn load_model_from_path(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        model_path: &str,
    ) -> Result<()> {
        crate::log!("=== Loading model from path: {} ===", model_path);

        let path_lower = model_path.to_lowercase();
        let is_fbx = path_lower.ends_with(".fbx");
        let is_gltf = path_lower.ends_with(".gltf") || path_lower.ends_with(".glb");

        if !is_fbx && !is_gltf {
            return Err(anyhow!(
                "Unsupported file format. Only FBX and glTF/GLB are supported."
            ));
        }

        crate::log!("Cleaning up existing model data...");
        data.render_resources.clear_meshes(rrdevice);
        data.render_resources.mesh_material_ids.clear();
        crate::log!("Cleared existing data");

        if is_fbx {
            crate::log!("Loading FBX model...");

            data.gltf_model = GltfModel::default();
            data.render_resources.animation.clear();
            crate::log!("Cleared glTF model and animation data");

            let fbx_result = load_fbx_to_render_resources(model_path)?;
            crate::log!(
                "Loaded FBX: {} meshes, {} skeletons, {} clips",
                fbx_result.meshes.len(),
                fbx_result.animation_system.skeletons.len(),
                fbx_result.animation_system.clips.len()
            );

            data.render_resources.animation = fbx_result.animation_system;

            for (mesh_idx, fbx_mesh) in fbx_result.meshes.iter().enumerate() {
                crate::log!(
                    "Creating Mesh for FBX mesh {}: {} vertices, texture: {:?}",
                    mesh_idx,
                    fbx_mesh.vertex_data.vertices.len(),
                    fbx_mesh.texture_path
                );

                let mut mesh = Mesh::default();

                if let Some(texture_path) = &fbx_mesh.texture_path {
                    crate::log!("Loading texture: {}", texture_path);
                    match load_png_image(texture_path) {
                        Ok((image_data, width, height)) => {
                            match create_texture_image_pixel(
                                instance,
                                rrdevice,
                                data.rrcommand_pool.borrow_mut(),
                                &image_data,
                                width,
                                height,
                            ) {
                                Ok((image, image_memory, mip_level)) => {
                                    mesh.image = image;
                                    mesh.image_memory = image_memory;
                                    mesh.mip_level = mip_level;
                                    crate::log!(
                                        "Texture loaded successfully for mesh {}",
                                        mesh_idx
                                    );
                                }
                                Err(e) => {
                                    crate::log!(
                                        "Failed to create texture image for mesh {}: {}",
                                        mesh_idx,
                                        e
                                    );
                                    (mesh.image, mesh.image_memory, mesh.mip_level) =
                                        create_texture_image_pixel(
                                            instance,
                                            rrdevice,
                                            data.rrcommand_pool.borrow_mut(),
                                            &vec![255u8, 255, 255, 255],
                                            1,
                                            1,
                                        )?;
                                }
                            }
                        }
                        Err(e) => {
                            crate::log!("Failed to load texture file {}: {}", texture_path, e);
                            (mesh.image, mesh.image_memory, mesh.mip_level) =
                                create_texture_image_pixel(
                                    instance,
                                    rrdevice,
                                    data.rrcommand_pool.borrow_mut(),
                                    &vec![255u8, 255, 255, 255],
                                    1,
                                    1,
                                )?;
                        }
                    }
                } else {
                    crate::log!("No texture specified for mesh {}, using white", mesh_idx);
                    (mesh.image, mesh.image_memory, mesh.mip_level) =
                        create_texture_image_pixel(
                            instance,
                            rrdevice,
                            data.rrcommand_pool.borrow_mut(),
                            &vec![255u8, 255, 255, 255],
                            1,
                            1,
                        )?;
                }

                mesh.vertex_data = fbx_mesh.vertex_data.clone();
                mesh.skin_data = fbx_mesh.skin_data.clone();
                mesh.skeleton_id = fbx_mesh.skeleton_id;

                data.render_resources.meshes.push(mesh);
            }

            if !data.render_resources.animation.clips.is_empty() {
                data.animation_playing = true;
                data.current_animation_index = 0;
                data.animation_time = 0.0;
                crate::log!(
                    "FBX animation loaded: {} clips",
                    data.render_resources.animation.clips.len()
                );
            }
        } else if is_gltf {
            crate::log!("Loading glTF model...");

            data.render_resources.animation.clear();
            data.animation_playing = false;
            data.current_animation_index = 0;
            data.animation_time = 0.0;
            crate::log!("Cleared animation state");

            data.gltf_model = GltfModel::load_model(model_path);

            let gltf_result = convert_gltf_to_render_resources(&data.gltf_model);
            crate::log!(
                "Converted glTF: {} meshes, {} skeletons, {} clips",
                gltf_result.meshes.len(),
                gltf_result.animation_system.skeletons.len(),
                gltf_result.animation_system.clips.len()
            );

            data.render_resources.animation = gltf_result.animation_system;

            for (i, gltf_mesh) in gltf_result.meshes.iter().enumerate() {
                crate::log!(
                    "Creating Mesh for glTF mesh {}: {} vertices",
                    i,
                    gltf_mesh.vertex_data.vertices.len()
                );

                let mut mesh = Mesh::default();

                let gltf_data = &data.gltf_model.gltf_data[i];
                if !gltf_data.image_data.is_empty() {
                    crate::log!("Loading texture from glTF image data for mesh {}", i);
                    match create_texture_image_pixel(
                        instance,
                        rrdevice,
                        data.rrcommand_pool.borrow_mut(),
                        &gltf_data.image_data[0].data,
                        gltf_data.image_data[0].width,
                        gltf_data.image_data[0].height,
                    ) {
                        Ok((image, image_memory, mip_level)) => {
                            mesh.image = image;
                            mesh.image_memory = image_memory;
                            mesh.mip_level = mip_level;
                            crate::log!("Texture loaded successfully for mesh {}", i);
                        }
                        Err(e) => {
                            crate::log!("Failed to create texture image for mesh {}: {}", i, e);
                            (mesh.image, mesh.image_memory, mesh.mip_level) =
                                create_texture_image_pixel(
                                    instance,
                                    rrdevice,
                                    data.rrcommand_pool.borrow_mut(),
                                    &vec![255u8, 255, 255, 255],
                                    1,
                                    1,
                                )?;
                        }
                    }
                } else {
                    crate::log!("No texture data for mesh {}, using white", i);
                    (mesh.image, mesh.image_memory, mesh.mip_level) =
                        create_texture_image_pixel(
                            instance,
                            rrdevice,
                            data.rrcommand_pool.borrow_mut(),
                            &vec![255u8, 255, 255, 255],
                            1,
                            1,
                        )?;
                }

                mesh.vertex_data = gltf_mesh.vertex_data.clone();
                mesh.skin_data = gltf_mesh.skin_data.clone();
                mesh.skeleton_id = gltf_mesh.skeleton_id;

                data.render_resources.meshes.push(mesh);
            }

            if !data.render_resources.animation.clips.is_empty() {
                data.animation_playing = true;
                data.current_animation_index = 0;
                data.animation_time = 0.0;
                crate::log!(
                    "glTF animation loaded: {} clips",
                    data.render_resources.animation.clips.len()
                );
            }
        }

        crate::log!("Recreating buffers...");
        for i in 0..data.render_resources.meshes.len() {
            let mesh = &mut data.render_resources.meshes[i];

            mesh.vertex_buffer = RRVertexBuffer::new(
                &instance,
                &rrdevice,
                &data.rrcommand_pool,
                (size_of::<vulkan_data::Vertex>() * mesh.vertex_data.vertices.len())
                    as vk::DeviceSize,
                mesh.vertex_data.vertices.as_ptr() as *const c_void,
                mesh.vertex_data.vertices.len(),
            );

            mesh.index_buffer = RRIndexBuffer::new(
                &instance,
                &rrdevice,
                &data.rrcommand_pool,
                (size_of::<u32>() * mesh.vertex_data.indices.len()) as u64,
                mesh.vertex_data.indices.as_ptr() as *const c_void,
                mesh.vertex_data.indices.len(),
            );

            mesh.image_view = create_image_view(
                &rrdevice,
                mesh.image,
                vk::Format::R8G8B8A8_SRGB,
                vk::ImageAspectFlags::COLOR,
                mesh.mip_level,
            )?;

            mesh.sampler = create_texture_sampler(&rrdevice, mesh.mip_level)?;

            mesh.object_index = data.render_resources.objects.allocate_slot();
            crate::log!("Allocated object_index {} for mesh {}", mesh.object_index, i);

            let material_name = format!("material_{}", i);
            let material_properties = MaterialUBO::default();
            let material_id = data.render_resources.materials.create_material_with_texture(
                instance,
                rrdevice,
                &material_name,
                mesh.image_view,
                mesh.sampler,
                material_properties,
            )?;
            data.render_resources.mesh_material_ids.push(material_id);
        }

        if !data.render_resources.animation.clips.is_empty() {
            crate::log!("Applying initial pose (time=0) for skeletal animation...");

            data.render_resources.animation.play(0);
            data.render_resources.animation.player.time = 0.0;

            let skeleton_id = data.render_resources.meshes.first()
                .and_then(|m| m.skeleton_id);

            if let Some(skel_id) = skeleton_id {
                data.render_resources.animation.apply_to_skeleton(skel_id);

                for mesh_idx in 0..data.render_resources.meshes.len() {
                    let (skin_data, skel_id) = {
                        let mesh = &data.render_resources.meshes[mesh_idx];
                        (mesh.skin_data.clone(), mesh.skeleton_id)
                    };

                    if let (Some(skin_data), Some(skel_id)) = (skin_data, skel_id) {
                        if let Some(skeleton) = data.render_resources.animation.get_skeleton(skel_id) {
                            let vertex_count = skin_data.base_positions.len();
                            let mut skinned_positions = vec![cgmath::Vector3::new(0.0, 0.0, 0.0); vertex_count];
                            let mut skinned_normals = vec![cgmath::Vector3::new(0.0, 1.0, 0.0); vertex_count];

                            skin_data.apply_skinning(skeleton, &mut skinned_positions, &mut skinned_normals);

                            let mesh = &mut data.render_resources.meshes[mesh_idx];
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
                                &instance,
                                &rrdevice,
                                &data.rrcommand_pool,
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
                }
            }
            crate::log!("Initial pose applied successfully");
        }

        data.current_model_path = model_path.to_string();

        crate::log!("=== Model loaded successfully ===");
        Ok(())
    }

    pub(crate) unsafe fn update_vertex_buffer(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        if data.gltf_model.gltf_data.is_empty() {
            return Ok(());
        }

        let gltf_mesh_count = data.gltf_model.gltf_data.len();
        for i in 0..gltf_mesh_count {
            if i >= data.render_resources.meshes.len() {
                break;
            }

            let mesh = &mut data.render_resources.meshes[i];
            let vertex_data = &mut mesh.vertex_data;
            let gltf_data = &data.gltf_model.gltf_data[i];

            for vertex in &gltf_data.vertices {
                vertex_data.vertices[vertex.index].pos.x = vertex.animation_position[0];
                vertex_data.vertices[vertex.index].pos.y = vertex.animation_position[1];
                vertex_data.vertices[vertex.index].pos.z = vertex.animation_position[2];
            }

            static mut UPDATE_LOG_COUNTER: u32 = 0;
            unsafe {
                UPDATE_LOG_COUNTER += 1;
                if UPDATE_LOG_COUNTER <= 5 {
                    crate::log!("=== update_vertex_buffer Debug (mesh {}) ===", i);
                    crate::log!("gltf_data.vertices count: {}", gltf_data.vertices.len());
                    crate::log!("vertex_data.vertices count: {}", vertex_data.vertices.len());
                    if !vertex_data.vertices.is_empty() {
                        let v0 = &vertex_data.vertices[0];
                        crate::log!(
                            "vertex_data[0].pos: ({:.2}, {:.2}, {:.2})",
                            v0.pos.x,
                            v0.pos.y,
                            v0.pos.z
                        );
                        if vertex_data.vertices.len() > 100 {
                            let v100 = &vertex_data.vertices[100];
                            crate::log!(
                                "vertex_data[100].pos: ({:.2}, {:.2}, {:.2})",
                                v100.pos.x,
                                v100.pos.y,
                                v100.pos.z
                            );
                        }
                    }
                    if !gltf_data.vertices.is_empty() {
                        let v = &gltf_data.vertices[0];
                        crate::log!("gltf_data[0].index: {}", v.index);
                        crate::log!(
                            "gltf_data[0].animation_position: ({:.2}, {:.2}, {:.2})",
                            v.animation_position[0],
                            v.animation_position[1],
                            v.animation_position[2]
                        );
                        crate::log!(
                            "gltf_data[0].position (original): ({:.2}, {:.2}, {:.2})",
                            v.position[0],
                            v.position[1],
                            v.position[2]
                        );
                    }
                    crate::log!("==================================");
                }
            }

            if let Err(e) = mesh.vertex_buffer.update(
                instance,
                rrdevice,
                &data.rrcommand_pool,
                (size_of::<vulkan_data::Vertex>() * mesh.vertex_data.vertices.len())
                    as vk::DeviceSize,
                mesh.vertex_data.vertices.as_ptr() as *const c_void,
                mesh.vertex_data.vertices.len(),
            ) {
                eprintln!("Failed to update vertex buffer: {}", e);
            }
        }
        Ok(())
    }

    pub(crate) unsafe fn update_skinned_vertex_buffers(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        for mesh_idx in 0..data.render_resources.meshes.len() {
            let (skin_data, skeleton_id) = {
                let mesh = &data.render_resources.meshes[mesh_idx];
                (mesh.skin_data.clone(), mesh.skeleton_id)
            };

            let Some(skin_data) = skin_data else {
                continue;
            };

            let Some(skeleton_id) = skeleton_id else {
                continue;
            };

            let Some(skeleton) = data.render_resources.animation.get_skeleton(skeleton_id) else {
                continue;
            };

            let vertex_count = skin_data.base_positions.len();
            let mut skinned_positions = vec![cgmath::Vector3::new(0.0, 0.0, 0.0); vertex_count];
            let mut skinned_normals = vec![cgmath::Vector3::new(0.0, 1.0, 0.0); vertex_count];

            skin_data.apply_skinning(skeleton, &mut skinned_positions, &mut skinned_normals);

            let mesh = &mut data.render_resources.meshes[mesh_idx];
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
                rrdevice,
                &data.rrcommand_pool,
                (size_of::<vulkan_data::Vertex>() * mesh.vertex_data.vertices.len()) as vk::DeviceSize,
                mesh.vertex_data.vertices.as_ptr() as *const c_void,
                mesh.vertex_data.vertices.len(),
            ) {
                crate::log!("Failed to update skinned vertex buffer for mesh {}: {}", mesh_idx, e);
            }
        }

        Ok(())
    }

    pub(crate) unsafe fn update_acceleration_structures(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        if let Some(ref accel_struct) = data.raytracing.acceleration_structure {
            let vertex_buffers: Vec<_> = data
                .render_resources
                .meshes
                .iter()
                .filter(|mesh| mesh.vertex_buffer.buffer != vk::Buffer::null())
                .map(|mesh| {
                    (
                        &mesh.vertex_buffer.buffer,
                        mesh.vertex_data.vertices.len() as u32,
                        std::mem::size_of::<vulkan_data::Vertex>() as u32,
                        &mesh.index_buffer.buffer,
                        mesh.vertex_data.indices.len() as u32,
                    )
                })
                .collect();

            if !vertex_buffers.is_empty() {
                accel_struct.update_all(instance, rrdevice, &data.rrcommand_pool, &vertex_buffers)?;
            }
        }

        Ok(())
    }
}
