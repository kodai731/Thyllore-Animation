use crate::app::{App, AppData};
use rust_rendering::vulkanr::buffer::*;
use rust_rendering::vulkanr::command::*;
use rust_rendering::vulkanr::data as vulkan_data;
use rust_rendering::vulkanr::data::*;
use rust_rendering::vulkanr::descriptor::*;
use rust_rendering::vulkanr::device::*;
use rust_rendering::vulkanr::image::*;
use rust_rendering::vulkanr::pipeline::{
    PipelineBuilder, RRPipeline, VertexInputConfig, DepthTestConfig, BlendConfig, PushConstantConfig,
};
use rust_rendering::vulkanr::render::*;
use rust_rendering::vulkanr::swapchain::*;
use rust_rendering::vulkanr::vulkan::*;

use rust_rendering::loader::gltf::gltf::*;
use rust_rendering::loader::fbx::fbx::{FbxModel, load_fbx, load_fbx_with_russimp};
use rust_rendering::loader::texture::load_png_image;
use rust_rendering::math::*;
use rust_rendering::logger::logger::*;

use anyhow::{anyhow, Result};
use std::mem::size_of;
use std::os::raw::c_void;
use std::borrow::BorrowMut;
use cgmath::Matrix4;
use vulkanalia::prelude::v1_0::*;

impl App {
    pub(crate) unsafe fn load_model(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        let model_path_fbx = "assets/models/phoenix-bird/source/fly.fbx";
        data.fbx_model = load_fbx_with_russimp(model_path_fbx)?;

        if data.fbx_model.animation_count() > 0 {
            log!("Applying initial pose (time=0) for FBX skeletal animation...");
            data.fbx_model.update_animation(0, 0.0);
        }

        for (mesh_idx, fbx_data) in data.fbx_model.fbx_data.iter().enumerate() {
            log!("Creating RRData for FBX mesh {}: {} vertices, texture: {:?}",
                mesh_idx, fbx_data.positions.len(), fbx_data.diffuse_texture);

            if !fbx_data.positions.is_empty() {
                let first_pos = &fbx_data.positions[0];
                log!("DEBUG: Mesh {} first vertex position: ({}, {}, {})", mesh_idx, first_pos.x, first_pos.y, first_pos.z);
            }

            let rrdata_name = format!("fbx_mesh_{}", mesh_idx);
            let mut rrdata = RRData::new(&instance, &rrdevice, &data.rrswapchain, &rrdata_name);

            if let Some(texture_path) = &fbx_data.diffuse_texture {
                log!("Loading texture: {}", texture_path);
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
                                rrdata.image = image;
                                rrdata.image_memory = image_memory;
                                rrdata.mip_level = mip_level;
                                log!("Texture loaded successfully for mesh {}", mesh_idx);
                            }
                            Err(e) => {
                                log!("Failed to create texture image for mesh {}: {}", mesh_idx, e);
                                (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
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
                        log!("Failed to load texture file {}: {}", texture_path, e);
                        (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
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
                log!("No texture specified for mesh {}, using white", mesh_idx);
                (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                    instance,
                    rrdevice,
                    data.rrcommand_pool.borrow_mut(),
                    &vec![255u8, 255, 255, 255],
                    1,
                    1,
                )?;
            }

            rrdata.vertex_data = VertexData::default();
            for (i, position) in fbx_data.positions.iter().enumerate() {
                let uv = if i < fbx_data.tex_coords.len() {
                    fbx_data.tex_coords[i]
                } else {
                    [0.5, 0.5]
                };

                let normal = if i < fbx_data.normals.len() {
                    let n = &fbx_data.normals[i];
                    Vec3::new(n.x, n.y, n.z)
                } else {
                    Vec3::new(0.0, 1.0, 0.0)
                };

                let vertex = vulkan_data::Vertex::new_with_normal(
                    Vec3::new(position.x, position.y, position.z),
                    Vec4::new(1.0, 1.0, 1.0, 1.0),
                    Vec2::new_array(uv),
                    normal,
                );
                rrdata.vertex_data.vertices.push(vertex);
            }

            rrdata.vertex_data.indices = fbx_data.indices.clone();
            data.model_descriptor_set.rrdata.push(rrdata);
        }

        if data.fbx_model.animation_count() > 0 {
            data.animation_playing = true;
            data.current_animation_index = 0;
            data.animation_time = 0.0;
            log!("FBX animation loaded: {} animations", data.fbx_model.animation_count());
            if let Some(duration) = data.fbx_model.get_animation_duration(0) {
                log!("Animation 0 duration: {} seconds", duration);
            }
        }

        let model_path_fbx = "assets/models/phoenix-bird/source/fly.fbx";
        data.current_model_path = model_path_fbx.to_string();

        log!("=== FBX model loaded successfully ===");
        Ok(())
    }

    pub(crate) unsafe fn load_model_from_path(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        model_path: &str,
    ) -> Result<()> {
        log!("=== Loading model from path: {} ===", model_path);

        let path_lower = model_path.to_lowercase();
        let is_fbx = path_lower.ends_with(".fbx");
        let is_gltf = path_lower.ends_with(".gltf") || path_lower.ends_with(".glb");

        if !is_fbx && !is_gltf {
            return Err(anyhow!("Unsupported file format. Only FBX and glTF/GLB are supported."));
        }

        log!("Cleaning up existing model data...");
        data.model_descriptor_set.delete_data(rrdevice);
        data.model_descriptor_set.rrdata.clear();
        log!("Cleared existing data, descriptor pool will be reused");

        if is_fbx {
            log!("Loading FBX model...");

            data.gltf_model = GltfModel::default();
            log!("Cleared glTF model data");

            if model_path.contains("stickman_bin.fbx") {
                log!("Using fbxcel loader for stickman_bin.fbx");
                unsafe {
                    data.fbx_model = load_fbx(model_path)?;
                }
            } else {
                log!("Using russimp loader");
                data.fbx_model = load_fbx_with_russimp(model_path)?;
            }

            for (mesh_idx, fbx_data) in data.fbx_model.fbx_data.iter().enumerate() {
                log!("Creating RRData for FBX mesh {}: {} vertices, texture: {:?}",
                    mesh_idx, fbx_data.positions.len(), fbx_data.diffuse_texture);

                let rrdata_name = format!("gltf_mesh_{}", mesh_idx);
                let mut rrdata = RRData::new(&instance, &rrdevice, &data.rrswapchain, &rrdata_name);

                if let Some(texture_path) = &fbx_data.diffuse_texture {
                    log!("Loading texture: {}", texture_path);
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
                                    rrdata.image = image;
                                    rrdata.image_memory = image_memory;
                                    rrdata.mip_level = mip_level;
                                    log!("Texture loaded successfully for mesh {}", mesh_idx);
                                }
                                Err(e) => {
                                    log!("Failed to create texture image for mesh {}: {}", mesh_idx, e);
                                    (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
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
                            log!("Failed to load texture file {}: {}", texture_path, e);
                            (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
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
                    log!("No texture specified for mesh {}, using white", mesh_idx);
                    (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                        instance,
                        rrdevice,
                        data.rrcommand_pool.borrow_mut(),
                        &vec![255u8, 255, 255, 255],
                        1,
                        1,
                    )?;
                }

                rrdata.vertex_data = VertexData::default();
                for (i, position) in fbx_data.positions.iter().enumerate() {
                    let uv = if i < fbx_data.tex_coords.len() {
                        fbx_data.tex_coords[i]
                    } else {
                        [0.5, 0.5]
                    };

                    let normal = if i < fbx_data.normals.len() {
                        let n = &fbx_data.normals[i];
                        Vec3::new(n.x, n.y, n.z)
                    } else {
                        Vec3::new(0.0, 1.0, 0.0)
                    };

                    let vertex = vulkan_data::Vertex::new_with_normal(
                        Vec3::new(position.x, position.y, position.z),
                        Vec4::new(1.0, 1.0, 1.0, 1.0),
                        Vec2::new_array(uv),
                        normal,
                    );
                    rrdata.vertex_data.vertices.push(vertex);
                }

                rrdata.vertex_data.indices = fbx_data.indices.clone();
                data.model_descriptor_set.rrdata.push(rrdata);
            }

            if data.fbx_model.animation_count() > 0 {
                data.animation_playing = true;
                data.current_animation_index = 0;
                data.animation_time = 0.0;
                log!("FBX animation loaded: {} animations", data.fbx_model.animation_count());
            }

        } else if is_gltf {
            log!("Loading glTF model...");

            data.fbx_model = FbxModel::default();
            data.animation_playing = false;
            data.current_animation_index = 0;
            data.animation_time = 0.0;
            log!("Cleared FBX model data and animation state");

            data.gltf_model = GltfModel::load_model(model_path);

            for (i, gltf_data) in data.gltf_model.gltf_data.iter().enumerate() {
                log!("Creating RRData for glTF mesh {}: {} vertices", i, gltf_data.vertices.len());

                let rrdata_name = format!("gltf_mesh2_{}", i);
                let mut rrdata = RRData::new(&instance, &rrdevice, &data.rrswapchain, &rrdata_name);

                if !gltf_data.image_data.is_empty() {
                    log!("Loading texture from glTF image data for mesh {}", i);
                    match create_texture_image_pixel(
                        instance,
                        rrdevice,
                        data.rrcommand_pool.borrow_mut(),
                        &gltf_data.image_data[0].data,
                        gltf_data.image_data[0].width,
                        gltf_data.image_data[0].height,
                    ) {
                        Ok((image, image_memory, mip_level)) => {
                            rrdata.image = image;
                            rrdata.image_memory = image_memory;
                            rrdata.mip_level = mip_level;
                            log!("Texture loaded successfully for mesh {}", i);
                        }
                        Err(e) => {
                            log!("Failed to create texture image for mesh {}: {}", i, e);
                            (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
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
                    log!("No texture data for mesh {}, using white", i);
                    (rrdata.image, rrdata.image_memory, rrdata.mip_level) = create_texture_image_pixel(
                        instance,
                        rrdevice,
                        data.rrcommand_pool.borrow_mut(),
                        &vec![255u8, 255, 255, 255],
                        1,
                        1,
                    )?;
                }

                rrdata.vertex_data = VertexData::default();
                for gltf_vertex in &gltf_data.vertices {
                    rrdata
                        .vertex_data
                        .vertices
                        .push(vulkan_data::Vertex::default());
                }

                for gltf_vertex in &gltf_data.vertices {
                    let vertex = vulkan_data::Vertex::new(
                        Vec3::new_array(gltf_vertex.position),
                        Vec4::new(0.0, 1.0, 0.0, 1.0),
                        Vec2::new_array(gltf_vertex.tex_coord),
                    );
                    rrdata.vertex_data.vertices[gltf_vertex.index] = vertex;
                }

                rrdata.vertex_data.indices = gltf_data.indices.clone();
                data.model_descriptor_set.rrdata.push(rrdata);
            }
        }

        log!("Recreating buffers and descriptor sets...");
        for i in 0..data.model_descriptor_set.rrdata.len() {
            let rrdata = &mut data.model_descriptor_set.rrdata[i];

            rrdata.vertex_buffer = RRVertexBuffer::new(
                &instance,
                &rrdevice,
                &data.rrcommand_pool,
                (size_of::<vulkan_data::Vertex>() * rrdata.vertex_data.vertices.len())
                    as vk::DeviceSize,
                rrdata.vertex_data.vertices.as_ptr() as *const c_void,
                rrdata.vertex_data.vertices.len(),
            );

            rrdata.index_buffer = RRIndexBuffer::new(
                &instance,
                &rrdevice,
                &data.rrcommand_pool,
                (size_of::<u32>() * rrdata.vertex_data.indices.len()) as u64,
                rrdata.vertex_data.indices.as_ptr() as *const c_void,
                rrdata.vertex_data.indices.len(),
            );

            let buffer_name = format!("recreate_mesh_{}", i);
            RRData::create_uniform_buffers(rrdata, &instance, &rrdevice, &data.rrswapchain, &buffer_name);

            rrdata.image_view = create_image_view(
                &rrdevice,
                rrdata.image,
                vk::Format::R8G8B8A8_SRGB,
                vk::ImageAspectFlags::COLOR,
                rrdata.mip_level,
            )?;

            rrdata.sampler = create_texture_sampler(&rrdevice, rrdata.mip_level)?;
        }

        if is_gltf && (!data.gltf_model.joint_animations.is_empty() || !data.gltf_model.node_animations.is_empty()) {
            if data.gltf_model.has_skinned_meshes {
                log!("Applying initial pose (time=0) for glTF skeletal animation...");
                data.gltf_model.reset_vertices_animation_position(0.0);
                data.gltf_model.apply_animation(0.0, 0, Matrix4::identity());
                log!("Initial pose applied successfully for glTF");
            } else {
                log!("Applying initial pose (time=0) for glTF node animation...");
                data.gltf_model.reset_vertices_animation_position(0.0);
                log!("Initial pose applied successfully for glTF");
            }

            for i in 0..data.gltf_model.gltf_data.len() {
                if i >= data.model_descriptor_set.rrdata.len() {
                    break;
                }

                let rrdata = &mut data.model_descriptor_set.rrdata[i];
                let vertex_data = &mut rrdata.vertex_data;
                let gltf_data = &data.gltf_model.gltf_data[i];

                for vertex in &gltf_data.vertices {
                    vertex_data.vertices[vertex.index].pos.x = vertex.animation_position[0];
                    vertex_data.vertices[vertex.index].pos.y = vertex.animation_position[1];
                    vertex_data.vertices[vertex.index].pos.z = vertex.animation_position[2];
                }

                if let Err(e) = rrdata.vertex_buffer.update(
                    &instance,
                    &rrdevice,
                    &data.rrcommand_pool,
                    (size_of::<vulkan_data::Vertex>() * vertex_data.vertices.len()) as vk::DeviceSize,
                    vertex_data.vertices.as_ptr() as *const c_void,
                    vertex_data.vertices.len(),
                ) {
                    log!("Failed to update vertex buffer for glTF mesh {} with initial pose: {}", i, e);
                }
            }
            log!("Initial pose applied successfully for glTF");
        }

        if is_fbx && data.fbx_model.animation_count() > 0 {
            log!("Applying initial pose (time=0) for FBX skeletal animation...");
            data.fbx_model.update_animation(0, 0.0);

            for (mesh_idx, fbx_data) in data.fbx_model.fbx_data.iter().enumerate() {
                if let Some(rrdata) = data.model_descriptor_set.rrdata.get_mut(mesh_idx) {
                    let vertex_data = &mut rrdata.vertex_data;

                    for (vertex_idx, pos) in fbx_data.positions.iter().enumerate() {
                        if vertex_idx < vertex_data.vertices.len() {
                            vertex_data.vertices[vertex_idx].pos.x = pos.x;
                            vertex_data.vertices[vertex_idx].pos.y = pos.y;
                            vertex_data.vertices[vertex_idx].pos.z = pos.z;
                        }
                    }

                    if let Err(e) = rrdata.vertex_buffer.update(
                        &instance,
                        &rrdevice,
                        &data.rrcommand_pool,
                        (size_of::<vulkan_data::Vertex>() * vertex_data.vertices.len()) as vk::DeviceSize,
                        vertex_data.vertices.as_ptr() as *const c_void,
                        vertex_data.vertices.len(),
                    ) {
                        log!("Failed to update vertex buffer for mesh {} with initial pose: {}", mesh_idx, e);
                    }
                }
            }
            log!("Initial pose applied successfully for FBX");
        }

        log!("Creating descriptor sets...");
        if let Err(e) = RRDescriptorSet::create_descriptor_set(
            &rrdevice,
            &data.rrswapchain,
            &mut data.model_descriptor_set,
        ) {
            log!("Failed to create model descriptor set: {:?}", e);
            return Err(anyhow!("Failed to create descriptor sets: {:?}", e));
        }

        log!("Recreating command buffers...");
        let mut rrbind_info = Vec::new();
        rrbind_info.push(RRBindInfo::new(
            &data.grid.pipeline,
            &data.grid.descriptor_set,
            &data.grid.vertex_buffer,
            &data.grid.index_buffer,
            0,
            0,
            0,
        ));

        for i in 0..data.model_descriptor_set.rrdata.len() {
            rrbind_info.push(RRBindInfo::new(
                &data.model_pipeline,
                &data.model_descriptor_set,
                &data.model_descriptor_set.rrdata[i].vertex_buffer,
                &data.model_descriptor_set.rrdata[i].index_buffer,
                0,
                0,
                i,
            ));
        }

        for i in 0..data.rrrender.framebuffers.len() {
            if let Err(e) = RRCommandBuffer::bind_command(
                &rrdevice,
                &data.rrrender,
                &data.rrswapchain,
                &rrbind_info,
                &mut data.rrcommand_buffer,
                i,
            ) {
                log!("Failed to bind command for framebuffer {}: {:?}", i, e);
                return Err(anyhow!("Failed to bind command: {:?}", e));
            }
        }

        data.current_model_path = model_path.to_string();

        log!("=== Model loaded successfully ===");
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
            if i >= data.model_descriptor_set.rrdata.len() {
                break;
            }

            let rrdata = &mut data.model_descriptor_set.rrdata[i];
            let vertex_data = &mut rrdata.vertex_data;
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
                    log!("=== update_vertex_buffer Debug (mesh {}) ===", i);
                    log!("gltf_data.vertices count: {}", gltf_data.vertices.len());
                    log!("vertex_data.vertices count: {}", vertex_data.vertices.len());
                    if !vertex_data.vertices.is_empty() {
                        let v0 = &vertex_data.vertices[0];
                        log!("vertex_data[0].pos: ({:.2}, {:.2}, {:.2})", v0.pos.x, v0.pos.y, v0.pos.z);
                        if vertex_data.vertices.len() > 100 {
                            let v100 = &vertex_data.vertices[100];
                            log!("vertex_data[100].pos: ({:.2}, {:.2}, {:.2})", v100.pos.x, v100.pos.y, v100.pos.z);
                        }
                    }
                    if !gltf_data.vertices.is_empty() {
                        let v = &gltf_data.vertices[0];
                        log!("gltf_data[0].index: {}", v.index);
                        log!("gltf_data[0].animation_position: ({:.2}, {:.2}, {:.2})",
                            v.animation_position[0], v.animation_position[1], v.animation_position[2]);
                        log!("gltf_data[0].position (original): ({:.2}, {:.2}, {:.2})",
                            v.position[0], v.position[1], v.position[2]);
                    }
                    log!("==================================");
                }
            }

            if let Err(e) = rrdata.vertex_buffer.update(
                instance,
                rrdevice,
                &data.rrcommand_pool,
                (size_of::<vulkan_data::Vertex>() * rrdata.vertex_data.vertices.len())
                    as vk::DeviceSize,
                rrdata.vertex_data.vertices.as_ptr() as *const c_void,
                rrdata.vertex_data.vertices.len(),
            ) {
                eprintln!("Failed to update vertex buffer: {}", e);
            }
        }
        Ok(())
    }

    pub(crate) unsafe fn update_fbx_vertex_buffer(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
    ) -> Result<()> {
        if data.fbx_model.fbx_data.is_empty() {
            return Ok(());
        }

        for (mesh_idx, fbx_data) in data.fbx_model.fbx_data.iter().enumerate() {
            if let Some(rrdata) = data.model_descriptor_set.rrdata.get_mut(mesh_idx) {
                let vertex_data = &mut rrdata.vertex_data;

                if mesh_idx == 0 {
                    static mut VERTEX_BUFFER_LOG_COUNTER: u32 = 0;
                    unsafe {
                        VERTEX_BUFFER_LOG_COUNTER += 1;
                        if VERTEX_BUFFER_LOG_COUNTER % 60 == 0 {
                            log!("GPU Vertex Buffer (first 5 vertices being sent to GPU):");
                            for i in 0..5.min(fbx_data.positions.len()) {
                                log!("  GPU[{}]: ({:.2}, {:.2}, {:.2})",
                                     i, fbx_data.positions[i].x, fbx_data.positions[i].y, fbx_data.positions[i].z);
                            }
                        }
                    }
                }

                for (vertex_idx, pos) in fbx_data.positions.iter().enumerate() {
                    if vertex_idx < vertex_data.vertices.len() {
                        vertex_data.vertices[vertex_idx].pos.x = pos.x;
                        vertex_data.vertices[vertex_idx].pos.y = pos.y;
                        vertex_data.vertices[vertex_idx].pos.z = pos.z;

                        if vertex_idx < fbx_data.normals.len() {
                            let normal = &fbx_data.normals[vertex_idx];
                            vertex_data.vertices[vertex_idx].normal.x = normal.x;
                            vertex_data.vertices[vertex_idx].normal.y = normal.y;
                            vertex_data.vertices[vertex_idx].normal.z = normal.z;
                        }
                    }
                }

                if let Err(e) = rrdata.vertex_buffer.update(
                    instance,
                    rrdevice,
                    &data.rrcommand_pool,
                    (size_of::<vulkan_data::Vertex>() * vertex_data.vertices.len()) as vk::DeviceSize,
                    vertex_data.vertices.as_ptr() as *const c_void,
                    vertex_data.vertices.len(),
                ) {
                    eprintln!("Failed to update FBX vertex buffer for mesh {}: {}", mesh_idx, e);
                }
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
            let vertex_buffers: Vec<_> = data.model_descriptor_set.rrdata.iter()
                .map(|rrdata| {
                    (
                        &rrdata.vertex_buffer.buffer,
                        rrdata.vertex_data.vertices.len() as u32,
                        std::mem::size_of::<vulkan_data::Vertex>() as u32,
                        &rrdata.index_buffer.buffer,
                        rrdata.vertex_data.indices.len() as u32,
                    )
                })
                .collect();

            accel_struct.update_all(
                instance,
                rrdevice,
                &data.rrcommand_pool,
                &vertex_buffers,
            )?;
        }

        Ok(())
    }
}
