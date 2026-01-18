use crate::app::model_loader::rebuild_acceleration_structures;
use crate::app::{App, AppData};
use crate::ecs::{playback_play, AnimationState, Transform};
use crate::loader::fbx::load_fbx_to_graphics_resources;
use crate::loader::gltf::load_gltf_file;
use crate::loader::texture::load_png_image;
use crate::math::*;
use crate::scene::graphics_resource::{MaterialUBO, MeshBuffer};
use crate::scene::Scene;
use crate::vulkanr::buffer::*;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::data as vulkan_data;
use crate::vulkanr::device::*;
use crate::vulkanr::image::*;
use crate::vulkanr::vulkan::*;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use std::ffi::c_void;

use crate::vulkanr::render::RRRender;
use crate::vulkanr::swapchain::RRSwapchain;

impl App {
    pub(crate) unsafe fn load_model_from_path_with_resources(
        instance: &Instance,
        rrdevice: &RRDevice,
        data: &mut AppData,
        scene: &Scene,
        rrcommand_pool: &Rc<RRCommandPool>,
        rrswapchain: &RRSwapchain,
        rrrender: &RRRender,
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
        data.graphics_resources.clear_meshes(rrdevice);
        data.graphics_resources
            .materials
            .clear_materials(&rrdevice.device);
        data.graphics_resources.mesh_material_ids.clear();
        data.graphics_resources.objects.reset_to(3);
        crate::log!("Cleared existing data (meshes and materials), reset object slots to 3");

        if is_fbx {
            crate::log!("Loading FBX model...");

            data.graphics_resources.animation.clear();
            crate::log!("Cleared animation data");

            let fbx_result = load_fbx_to_graphics_resources(model_path)?;
            crate::log!(
                "Loaded FBX: {} meshes, {} skeletons, {} clips",
                fbx_result.meshes.len(),
                fbx_result.animation_system.skeletons.len(),
                fbx_result.animation_system.clips.len()
            );

            data.graphics_resources.animation = fbx_result.animation_system;
            data.graphics_resources.has_skinned_meshes = fbx_result.has_skinned_meshes;
            data.graphics_resources.node_animation_scale = 1.0;

            if let Some(model_info) = data.ecs_world.get_resource_mut::<crate::ecs::ModelInfo>() {
                model_info.has_skinned_meshes = fbx_result.has_skinned_meshes;
                model_info.node_animation_scale = 1.0;
            } else {
                let mut model_info = crate::ecs::ModelInfo::new();
                model_info.has_skinned_meshes = fbx_result.has_skinned_meshes;
                model_info.node_animation_scale = 1.0;
                data.ecs_world.insert_resource(model_info);
            }

            data.graphics_resources.nodes = fbx_result
                .nodes
                .iter()
                .map(|n| crate::scene::graphics_resource::NodeData {
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

                let mut mesh = MeshBuffer::default();

                if let Some(texture_path) = &fbx_mesh.texture_path {
                    crate::log!("Loading texture: {}", texture_path);
                    match load_png_image(texture_path) {
                        Ok((image_data, width, height)) => {
                            match create_texture_image_pixel(
                                instance,
                                rrdevice,
                                rrcommand_pool,
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
                                            rrcommand_pool,
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
                                    rrcommand_pool,
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
                        rrcommand_pool,
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

                data.graphics_resources.meshes.push(mesh);
            }

            if !data.graphics_resources.animation.clips.is_empty() {
                crate::log!(
                    "FBX animation loaded: {} clips",
                    data.graphics_resources.animation.clips.len()
                );
            }
        } else if is_gltf {
            crate::log!("Loading glTF model...");

            data.graphics_resources.animation.clear();
            crate::log!("Cleared animation state");

            let gltf_result = load_gltf_file(model_path);
            crate::log!(
                "Converted glTF: {} meshes, {} skeletons, {} clips",
                gltf_result.meshes.len(),
                gltf_result.animation_system.skeletons.len(),
                gltf_result.animation_system.clips.len()
            );

            data.graphics_resources.animation = gltf_result.animation_system;
            data.graphics_resources.has_skinned_meshes = gltf_result.has_skinned_meshes;
            data.graphics_resources.morph_animation = gltf_result.morph_animation;
            let node_animation_scale = if gltf_result.has_armature { 0.01 } else { 1.0 };
            data.graphics_resources.node_animation_scale = node_animation_scale;

            if let Some(model_info) = data.ecs_world.get_resource_mut::<crate::ecs::ModelInfo>() {
                model_info.has_skinned_meshes = gltf_result.has_skinned_meshes;
                model_info.node_animation_scale = node_animation_scale;
            } else {
                let mut model_info = crate::ecs::ModelInfo::new();
                model_info.has_skinned_meshes = gltf_result.has_skinned_meshes;
                model_info.node_animation_scale = node_animation_scale;
                data.ecs_world.insert_resource(model_info);
            }

            data.graphics_resources.nodes = gltf_result
                .nodes
                .iter()
                .map(|n| crate::scene::graphics_resource::NodeData {
                    index: n.index,
                    name: n.name.clone(),
                    parent_index: n.parent_index,
                    local_transform: n.local_transform,
                    global_transform: cgmath::Matrix4::identity(),
                })
                .collect();
            crate::log!(
                "Loaded {} nodes into graphics_resources",
                data.graphics_resources.nodes.len()
            );

            for (i, gltf_mesh) in gltf_result.meshes.iter().enumerate() {
                crate::log!(
                    "Creating Mesh for glTF mesh {}: {} vertices",
                    i,
                    gltf_mesh.vertex_data.vertices.len()
                );

                let mut mesh = MeshBuffer::default();

                if !gltf_mesh.image_data.is_empty() {
                    let img = &gltf_mesh.image_data[0];
                    let expected_rgba_size = img.width as usize * img.height as usize * 4;
                    crate::log!("Loading texture for mesh {}: {}x{}, data_len={}, expected_rgba={}, is_rgba={}",
                        i, img.width, img.height, img.data.len(), expected_rgba_size, img.data.len() == expected_rgba_size);
                    match create_texture_image_pixel(
                        instance,
                        rrdevice,
                        rrcommand_pool,
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
                                    rrcommand_pool,
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
                        rrcommand_pool,
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

                data.graphics_resources.meshes.push(mesh);
            }

            if !data.graphics_resources.animation.clips.is_empty() {
                crate::log!(
                    "glTF animation loaded: {} clips",
                    data.graphics_resources.animation.clips.len()
                );
            }
        }

        crate::log!("Recreating buffers...");
        for i in 0..data.graphics_resources.meshes.len() {
            let mesh = &mut data.graphics_resources.meshes[i];

            mesh.vertex_buffer = RRVertexBuffer::new(
                &instance,
                &rrdevice,
                rrcommand_pool,
                (size_of::<vulkan_data::Vertex>() * mesh.vertex_data.vertices.len())
                    as vk::DeviceSize,
                mesh.vertex_data.vertices.as_ptr() as *const c_void,
                mesh.vertex_data.vertices.len(),
            );

            mesh.index_buffer = RRIndexBuffer::new(
                &instance,
                &rrdevice,
                rrcommand_pool,
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

            mesh.object_index = data.graphics_resources.objects.allocate_slot();
            crate::log!(
                "Allocated object_index {} for mesh {}",
                mesh.object_index,
                i
            );

            let material_name = format!("material_{}", i);
            let material_properties = MaterialUBO::default();
            let material_id = data
                .graphics_resources
                .materials
                .create_material_with_texture(
                    instance,
                    rrdevice,
                    &material_name,
                    mesh.image_view,
                    mesh.sampler,
                    material_properties,
                )?;
            data.graphics_resources.mesh_material_ids.push(material_id);
            crate::log!("Created material {} for mesh {}", material_id, i);
        }

        let is_gltf = model_path.ends_with(".glb") || model_path.ends_with(".gltf");
        let is_fbx = model_path.ends_with(".fbx");
        let has_node_animation = (is_gltf || is_fbx)
            && !data.graphics_resources.meshes.is_empty()
            && !data.graphics_resources.has_skinned_meshes;

        if has_node_animation {
            crate::log!(
                "Node animation detected - using initial mesh positions (no node transform)"
            );
        }

        if !data.graphics_resources.animation.clips.is_empty() {
            crate::log!("Applying initial pose (time=0) for skeletal animation...");

            let first_clip_id = data.graphics_resources.animation.clips.first().map(|c| c.id);
            if let Some(clip_id) = first_clip_id {
                if let Some(playback) = data.ecs_world.get_resource_mut::<crate::ecs::AnimationPlayback>() {
                    playback_play(playback, clip_id);
                } else {
                    let mut playback = crate::ecs::AnimationPlayback::new();
                    playback_play(&mut playback, clip_id);
                    data.ecs_world.insert_resource(playback);
                }
            }

            let skeleton_id = data
                .graphics_resources
                .meshes
                .first()
                .and_then(|m| m.skeleton_id);

            if let Some(skel_id) = skeleton_id {
                let playback = data.ecs_world.resource::<crate::ecs::AnimationPlayback>();
                data.graphics_resources
                    .animation
                    .apply_to_skeleton(skel_id, playback);

                for mesh_idx in 0..data.graphics_resources.meshes.len() {
                    let (skin_data, skel_id) = {
                        let mesh = &data.graphics_resources.meshes[mesh_idx];
                        (mesh.skin_data.clone(), mesh.skeleton_id)
                    };

                    if let (Some(skin_data), Some(skel_id)) = (skin_data, skel_id) {
                        if let Some(skeleton) =
                            data.graphics_resources.animation.get_skeleton(skel_id)
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

                            let mesh = &mut data.graphics_resources.meshes[mesh_idx];
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
                                rrcommand_pool,
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
                    if let Err(e) = data.graphics_resources.update_node_animation(
                        &instance,
                        &rrdevice,
                        rrcommand_pool,
                        &mut None,
                    ) {
                        crate::log!("Failed to apply initial node animation: {}", e);
                    }
                }
            }

            crate::log!("Initial pose applied successfully");
        }

        rebuild_acceleration_structures(instance, rrdevice, data, rrcommand_pool)?;

        if let Some(ref accel_struct) = data.raytracing.acceleration_structure {
            if let Some(tlas) = accel_struct.tlas.acceleration_structure {
                if let Some(ref mut ray_query_desc) = data.raytracing.ray_query_descriptor {
                    ray_query_desc.update_tlas(rrdevice, tlas)?;
                    crate::log!("Updated ray_query_descriptor with new TLAS");
                }
            }
        }

        if let Err(e) = Self::create_ray_tracing_pipelines_with_resources(
            instance,
            rrdevice,
            data,
            scene,
            rrswapchain,
            rrrender,
        ) {
            crate::log!("Failed to create ray tracing pipelines: {:?}", e);
        }

        Self::create_ecs_entities_from_meshes(data, model_path);

        crate::log!("=== Model loaded successfully ===");
        Ok(())
    }

    fn create_ecs_entities_from_meshes(data: &mut AppData, model_path: &str) {
        use crate::asset::{AnimationClipAsset, MeshAsset, NodeAsset, SkeletonAsset};

        data.ecs_world.clear();
        data.ecs_assets.clear();

        let model_name = std::path::Path::new(model_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("model")
            .to_string();

        for skeleton in &data.graphics_resources.animation.skeletons {
            let skeleton_asset = SkeletonAsset {
                id: 0,
                skeleton_id: skeleton.id,
                skeleton: skeleton.clone(),
            };
            data.ecs_assets.add_skeleton(skeleton_asset);
        }
        crate::log!(
            "Added {} skeletons to ecs_assets",
            data.ecs_assets.skeletons.len()
        );

        for clip in &data.graphics_resources.animation.clips {
            let clip_asset = AnimationClipAsset {
                id: 0,
                clip_id: clip.id,
                clip: clip.clone(),
            };
            data.ecs_assets.add_animation_clip(clip_asset);
        }
        crate::log!(
            "Added {} animation clips to ecs_assets",
            data.ecs_assets.animation_clips.len()
        );

        for node in &data.graphics_resources.nodes {
            let node_asset = NodeAsset {
                id: node.index as u64,
                name: node.name.clone(),
                parent_id: node.parent_index.map(|i| i as u64),
                local_transform: node.local_transform,
            };
            data.ecs_assets.add_node(node_asset);
        }
        crate::log!("Added {} nodes to ecs_assets", data.ecs_assets.nodes.len());

        let has_animation = !data.graphics_resources.animation.clips.is_empty();
        let first_clip_id = data
            .graphics_resources
            .animation
            .clips
            .first()
            .map(|c| c.id);

        for (mesh_idx, mesh) in data.graphics_resources.meshes.iter().enumerate() {
            let entity_name = format!("{}_{}", model_name, mesh_idx);

            let mesh_asset = MeshAsset {
                id: 0,
                name: entity_name.clone(),
                graphics_mesh_index: mesh_idx,
                object_index: mesh.object_index,
                material_id: data
                    .graphics_resources
                    .mesh_material_ids
                    .get(mesh_idx)
                    .copied(),
                skeleton_id: mesh.skeleton_id,
                node_index: mesh.node_index,
                render_to_gbuffer: mesh.render_to_gbuffer,
            };
            let asset_id = data.ecs_assets.add_mesh(mesh_asset);

            let mut builder = data.ecs_world.entity();
            builder = builder
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
            data.ecs_world.entity_count(),
            data.ecs_assets.meshes.len(),
            data.ecs_assets.skeletons.len(),
            data.ecs_assets.animation_clips.len(),
            data.ecs_assets.nodes.len()
        );
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn update_vertex_buffer(
        _instance: &Instance,
        _rrdevice: &RRDevice,
        _data: &mut AppData,
    ) -> Result<()> {
        Ok(())
    }

    pub fn dump_debug_info(&self) {
        crate::log!("========== DUMP DEBUG INFORMATION ==========");

        crate::log!("--- Model Info ---");
        crate::log!(
            "  current_model_path: {}",
            self.animation_playback().model_path
        );
        crate::log!(
            "  meshes count: {}",
            self.data.graphics_resources.meshes.len()
        );
        crate::log!(
            "  has_skinned_meshes: {}",
            self.data.graphics_resources.has_skinned_meshes
        );
        crate::log!(
            "  animation clips count: {}",
            self.data.graphics_resources.animation.clips.len()
        );
        crate::log!(
            "  morph_animations count: {}",
            self.data
                .graphics_resources
                .morph_animation
                .animations
                .len()
        );
        crate::log!(
            "  skeletons count: {}",
            self.data.graphics_resources.animation.skeletons.len()
        );

        crate::log!("--- GraphicsResources Info ---");
        crate::log!(
            "  meshes count: {}",
            self.data.graphics_resources.meshes.len()
        );
        crate::log!(
            "  materials count: {}",
            self.data.graphics_resources.materials.materials.len()
        );
        crate::log!(
            "  mesh_material_ids: {:?}",
            self.data.graphics_resources.mesh_material_ids
        );

        for (i, mesh) in self.data.graphics_resources.meshes.iter().enumerate() {
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
        crate::log!("  position: {:?}", self.data.camera.position);

        crate::log!("--- Animation Info ---");
        crate::log!("  animation_playing: {}", self.animation_playback().playing);
        crate::log!(
            "  clips count: {}",
            self.data.graphics_resources.animation.clips.len()
        );

        crate::log!("========== END DEBUG INFORMATION ==========");
    }
}
