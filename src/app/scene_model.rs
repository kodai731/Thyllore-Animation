use crate::app::model_loader::rebuild_acceleration_structures;
use crate::app::{App, AppData};
use crate::loader::fbx::load_fbx_to_render_resources;
use crate::loader::gltf::load_gltf_file;
use crate::loader::texture::load_png_image;
use crate::math::*;
use crate::scene::render_resource::{MaterialUBO, Mesh};
use crate::vulkanr::buffer::*;
use crate::vulkanr::data as vulkan_data;
use crate::vulkanr::device::*;
use crate::vulkanr::image::*;
use crate::vulkanr::vulkan::*;

use anyhow::{anyhow, Result};
use std::borrow::BorrowMut;
use std::mem::size_of;
use std::os::raw::c_void;
use vulkanalia::prelude::v1_0::*;

impl App {
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
        data.render_resources
            .materials
            .clear_materials(&rrdevice.device);
        data.render_resources.mesh_material_ids.clear();
        data.render_resources.objects.reset_to(2);
        crate::log!("Cleared existing data (meshes and materials), reset object slots to 2");

        if is_fbx {
            crate::log!("Loading FBX model...");

            data.render_resources.animation.clear();
            crate::log!("Cleared animation data");

            let fbx_result = load_fbx_to_render_resources(model_path)?;
            crate::log!(
                "Loaded FBX: {} meshes, {} skeletons, {} clips",
                fbx_result.meshes.len(),
                fbx_result.animation_system.skeletons.len(),
                fbx_result.animation_system.clips.len()
            );

            data.render_resources.animation = fbx_result.animation_system;
            data.render_resources.has_skinned_meshes = fbx_result.has_skinned_meshes;
            data.render_resources.node_animation_scale = 1.0;

            data.render_resources.nodes = fbx_result
                .nodes
                .iter()
                .map(|n| crate::scene::render_resource::NodeData {
                    index: n.index,
                    name: n.name.clone(),
                    parent_index: n.parent_index,
                    local_transform: n.local_transform,
                    global_transform: cgmath::Matrix4::identity(),
                })
                .collect();

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
                mesh.node_index = fbx_mesh.node_index;
                mesh.base_vertices = fbx_mesh.local_vertices.clone();

                crate::log!(
                    "FBX Mesh[{}]: node_index={:?}, base_vertices={}, skin_data={}",
                    mesh_idx,
                    mesh.node_index,
                    mesh.base_vertices.len(),
                    mesh.skin_data.is_some()
                );

                if !mesh.base_vertices.is_empty() {
                    let first = &mesh.base_vertices[0];
                    crate::log!(
                        "  base_vertices[0] = ({:.4}, {:.4}, {:.4})",
                        first.pos.x,
                        first.pos.y,
                        first.pos.z
                    );
                }

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

            let gltf_result = load_gltf_file(model_path);
            crate::log!(
                "Converted glTF: {} meshes, {} skeletons, {} clips",
                gltf_result.meshes.len(),
                gltf_result.animation_system.skeletons.len(),
                gltf_result.animation_system.clips.len()
            );

            data.render_resources.animation = gltf_result.animation_system;
            data.render_resources.has_skinned_meshes = gltf_result.has_skinned_meshes;
            data.render_resources.morph_animation = gltf_result.morph_animation;
            data.render_resources.node_animation_scale =
                if gltf_result.has_armature { 0.01 } else { 1.0 };

            data.render_resources.nodes = gltf_result
                .nodes
                .iter()
                .map(|n| crate::scene::render_resource::NodeData {
                    index: n.index,
                    name: n.name.clone(),
                    parent_index: n.parent_index,
                    local_transform: n.local_transform,
                    global_transform: cgmath::Matrix4::identity(),
                })
                .collect();
            crate::log!(
                "Loaded {} nodes into render_resources",
                data.render_resources.nodes.len()
            );

            for (i, gltf_mesh) in gltf_result.meshes.iter().enumerate() {
                crate::log!(
                    "Creating Mesh for glTF mesh {}: {} vertices",
                    i,
                    gltf_mesh.vertex_data.vertices.len()
                );

                let mut mesh = Mesh::default();

                if !gltf_mesh.image_data.is_empty() {
                    let img = &gltf_mesh.image_data[0];
                    let expected_rgba_size = img.width as usize * img.height as usize * 4;
                    crate::log!("Loading texture for mesh {}: {}x{}, data_len={}, expected_rgba={}, is_rgba={}",
                        i, img.width, img.height, img.data.len(), expected_rgba_size, img.data.len() == expected_rgba_size);
                    match create_texture_image_pixel(
                        instance,
                        rrdevice,
                        data.rrcommand_pool.borrow_mut(),
                        &gltf_mesh.image_data[0].data,
                        gltf_mesh.image_data[0].width,
                        gltf_mesh.image_data[0].height,
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
                    (mesh.image, mesh.image_memory, mesh.mip_level) = create_texture_image_pixel(
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
                mesh.node_index = gltf_mesh.node_index;
                mesh.base_vertices = gltf_mesh.local_vertices.clone();

                if i == 2 && !gltf_mesh.local_vertices.is_empty() {
                    crate::log!(
                        "  gltf_mesh[2].local_vertices[0]=({:.3},{:.3},{:.3})",
                        gltf_mesh.local_vertices[0].pos.x,
                        gltf_mesh.local_vertices[0].pos.y,
                        gltf_mesh.local_vertices[0].pos.z
                    );
                }

                crate::log!(
                    "Mesh[{}]: node_index={:?}, base_vertices={}, skin_data={}",
                    i,
                    mesh.node_index,
                    mesh.base_vertices.len(),
                    mesh.skin_data.is_some()
                );

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
            crate::log!(
                "Allocated object_index {} for mesh {}",
                mesh.object_index,
                i
            );

            let material_name = format!("material_{}", i);
            let material_properties = MaterialUBO::default();
            let material_id = data
                .render_resources
                .materials
                .create_material_with_texture(
                    instance,
                    rrdevice,
                    &material_name,
                    mesh.image_view,
                    mesh.sampler,
                    material_properties,
                )?;
            data.render_resources.mesh_material_ids.push(material_id);
            crate::log!("Created material {} for mesh {}", material_id, i);
        }

        let is_gltf =
            data.current_model_path.ends_with(".glb") || data.current_model_path.ends_with(".gltf");
        let is_fbx = data.current_model_path.ends_with(".fbx");
        let has_node_animation = (is_gltf || is_fbx)
            && !data.render_resources.meshes.is_empty()
            && !data.render_resources.has_skinned_meshes;

        if has_node_animation {
            crate::log!(
                "Node animation detected - using initial mesh positions (no node transform)"
            );
        }

        if !data.render_resources.animation.clips.is_empty() {
            crate::log!("Applying initial pose (time=0) for skeletal animation...");

            data.render_resources.animation.play(0);
            data.render_resources.animation.player.time = 0.0;

            let skeleton_id = data
                .render_resources
                .meshes
                .first()
                .and_then(|m| m.skeleton_id);

            if let Some(skel_id) = skeleton_id {
                data.render_resources.animation.apply_to_skeleton(skel_id);

                for mesh_idx in 0..data.render_resources.meshes.len() {
                    let (skin_data, skel_id) = {
                        let mesh = &data.render_resources.meshes[mesh_idx];
                        (mesh.skin_data.clone(), mesh.skeleton_id)
                    };

                    if let (Some(skin_data), Some(skel_id)) = (skin_data, skel_id) {
                        if let Some(skeleton) =
                            data.render_resources.animation.get_skeleton(skel_id)
                        {
                            let vertex_count = skin_data.base_positions.len();
                            let mut skinned_positions =
                                vec![cgmath::Vector3::new(0.0, 0.0, 0.0); vertex_count];
                            let mut skinned_normals =
                                vec![cgmath::Vector3::new(0.0, 1.0, 0.0); vertex_count];

                            skin_data.apply_skinning(
                                skeleton,
                                &mut skinned_positions,
                                &mut skinned_normals,
                            );

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

            if has_node_animation {
                unsafe {
                    if let Err(e) = data.render_resources.update_node_animation(
                        &instance,
                        &rrdevice,
                        &data.rrcommand_pool,
                        &mut None,
                    ) {
                        crate::log!("Failed to apply initial node animation: {}", e);
                    }
                }
            }

            crate::log!("Initial pose applied successfully");
        }

        data.current_model_path = model_path.to_string();

        rebuild_acceleration_structures(instance, rrdevice, data)?;

        if let Some(ref accel_struct) = data.raytracing.acceleration_structure {
            if let Some(tlas) = accel_struct.tlas.acceleration_structure {
                if let Some(ref mut ray_query_desc) = data.raytracing.ray_query_descriptor {
                    ray_query_desc.update_tlas(rrdevice, tlas)?;
                    crate::log!("Updated ray_query_descriptor with new TLAS");
                }
            }
        }

        if let Err(e) = Self::create_ray_tracing_pipelines(instance, rrdevice, data) {
            crate::log!("Failed to create ray tracing pipelines: {:?}", e);
        }

        crate::log!("=== Model loaded successfully ===");
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn update_vertex_buffer(
        _instance: &Instance,
        _rrdevice: &RRDevice,
        _data: &mut AppData,
    ) -> Result<()> {
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
                (size_of::<vulkan_data::Vertex>() * mesh.vertex_data.vertices.len())
                    as vk::DeviceSize,
                mesh.vertex_data.vertices.as_ptr() as *const c_void,
                mesh.vertex_data.vertices.len(),
            ) {
                crate::log!(
                    "Failed to update skinned vertex buffer for mesh {}: {}",
                    mesh_idx,
                    e
                );
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
                accel_struct.update_all(
                    instance,
                    rrdevice,
                    &data.rrcommand_pool,
                    &vertex_buffers,
                )?;
            }
        }

        Ok(())
    }

    pub fn dump_debug_info(&self) {
        crate::log!("========== DUMP DEBUG INFORMATION ==========");

        crate::log!("--- Model Info ---");
        crate::log!("  current_model_path: {}", self.data.current_model_path);
        crate::log!(
            "  meshes count: {}",
            self.data.render_resources.meshes.len()
        );
        crate::log!(
            "  has_skinned_meshes: {}",
            self.data.render_resources.has_skinned_meshes
        );
        crate::log!(
            "  animation clips count: {}",
            self.data.render_resources.animation.clips.len()
        );
        crate::log!(
            "  morph_animations count: {}",
            self.data.render_resources.morph_animation.animations.len()
        );
        crate::log!(
            "  skeletons count: {}",
            self.data.render_resources.animation.skeletons.len()
        );

        crate::log!("--- RenderResources Info ---");
        crate::log!(
            "  meshes count: {}",
            self.data.render_resources.meshes.len()
        );
        crate::log!(
            "  materials count: {}",
            self.data.render_resources.materials.materials.len()
        );
        crate::log!(
            "  mesh_material_ids: {:?}",
            self.data.render_resources.mesh_material_ids
        );

        for (i, mesh) in self.data.render_resources.meshes.iter().enumerate() {
            crate::log!(
                "  mesh[{}]: render_to_gbuffer={}, vertex_buffer={:?}, indices={}",
                i,
                mesh.render_to_gbuffer,
                mesh.vertex_buffer.buffer,
                mesh.index_buffer.indices
            );
            crate::log!(
                "    vertex_data.vertices count: {}",
                mesh.vertex_data.vertices.len()
            );
            crate::log!("    object_index: {}", mesh.object_index);

            if !mesh.vertex_data.vertices.is_empty() {
                let v = &mesh.vertex_data.vertices[0];
                crate::log!(
                    "    vertex_data[0].pos: ({:.4}, {:.4}, {:.4})",
                    v.pos.x,
                    v.pos.y,
                    v.pos.z
                );

                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                let mut min_z = f32::MAX;
                let mut max_z = f32::MIN;
                for v in &mesh.vertex_data.vertices {
                    min_x = min_x.min(v.pos.x);
                    max_x = max_x.max(v.pos.x);
                    min_y = min_y.min(v.pos.y);
                    max_y = max_y.max(v.pos.y);
                    min_z = min_z.min(v.pos.z);
                    max_z = max_z.max(v.pos.z);
                }
                crate::log!(
                    "    bounds: X[{:.2}, {:.2}], Y[{:.2}, {:.2}], Z[{:.2}, {:.2}]",
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                    min_z,
                    max_z
                );
            }
        }

        crate::log!("--- Camera Info ---");
        crate::log!("  position: {:?}", self.data.camera.position());

        crate::log!("--- Animation Info ---");
        crate::log!("  animation_playing: {}", self.data.animation_playing);
        crate::log!(
            "  clips count: {}",
            self.data.render_resources.animation.clips.len()
        );

        crate::log!("========== END DEBUG INFORMATION ==========");
    }
}
