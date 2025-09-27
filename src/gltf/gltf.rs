use crate::log;
use anyhow::{anyhow, Result};
use core::result::Result::Ok;
use glium::buffer::Content;
use gltf::buffer::Data;
use gltf::{image, Document, Gltf, Node};

#[derive(Clone, Debug, Default)]
pub struct GltfModel {
    pub gltf_data: Vec<GltfData>,
    pub morph_animations: Vec<MorphAnimation>,
}

impl GltfModel {
    pub unsafe fn load_model(path: &str) -> Self {
        let mut gltf_model = GltfModel::default();
        gltf_model.morph_animations = Vec::new();
        load_gltf(&mut gltf_model, path);
        gltf_model
    }

    pub fn morph_target_index(&self, time: f32) -> usize {
        let end = self
            .morph_animations
            .last()
            .expect("morph_animations is empty");
        let start = self
            .morph_animations
            .first()
            .expect("morph_animations is empty");
        let mod_time = time % (end.key_frame - start.key_frame);
        let mut index = 0;
        // TODO: fix animation index
        let end_index = self.morph_animations.len();
        for i in 0..end_index {
            let morph_animation = &self.morph_animations[i];
            if mod_time <= morph_animation.key_frame {
                index = i;
                break;
            }
        }
        index
    }
}

#[derive(Clone, Debug, Default)]
pub struct GltfData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub image_indices: Vec<[u16; 4]>,
    pub image_data: Vec<ImageData>,
    pub joints: Vec<Joint>,
    pub morph_targets: Vec<MorphTarget>,
}

#[derive(Clone, Debug, Default)]
pub struct Vertex {
    pub index: usize,
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub tex_coord: [f32; 2],
    pub joint_indices: [u16; 4],
    pub joint_weights: [f32; 4],
}

#[derive(Clone, Debug, Default)]
pub struct Joint {
    pub index: usize,
    pub vertex_indices: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct MorphTarget {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, Default)]
pub struct MorphAnimation {
    pub key_frame: f32,
    pub weights: Vec<f32>,
}

impl GltfData {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            image_indices: Vec::new(),
            image_data: Vec::new(),
            joints: Vec::new(),
            morph_targets: Vec::new(),
        }
    }
}

impl MorphTarget {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            tangents: Vec::new(),
        }
    }
}

impl MorphAnimation {
    fn new() -> Self {
        Self {
            key_frame: 0.0,
            weights: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImageData {
    pub data: Vec<u8>,
    pub size: u64,
    pub width: u32,
    pub height: u32,
}

unsafe fn load_gltf(gltf_model: &mut GltfModel, path: &str) {
    log!("Loading glTF file");
    let (gltf, buffers, images) = gltf::import(format!("{}", path)).expect("Failed to load model");
    for scene in gltf.scenes() {
        for node in scene.nodes() {
            process_node(&gltf, &buffers, &images, &node, gltf_model).unwrap();
        }
    }
    for animation in gltf.animations() {
        process_animation(&gltf, &buffers, animation, gltf_model).unwrap();
    }
}

unsafe fn process_node(
    gltf: &Document,
    buffers: &Vec<Data>,
    images: &Vec<gltf::image::Data>,
    node: &Node,
    gltf_model: &mut GltfModel,
) -> Result<()> {
    log!("Node {} {}", node.index().to_string(), node.name().unwrap());
    // meshes
    if let Some(mesh) = node.mesh() {
        log!("mesh found");
        let primitives = mesh.primitives();
        let mut normals = Vec::new();
        let mut joint_count = 0;

        // primitive
        primitives.for_each(|primitive| {
            log!("primitive found");
            let mut gltf_data = GltfData::new();
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            log!("Topology: {:?}", primitive.mode());

            let index_offset = gltf_data.indices.len();
            if let Some(gltf::mesh::util::ReadIndices::U16(gltf::accessor::Iter::Standard(iter))) =
                reader.read_indices()
            {
                for index in iter {
                    gltf_data
                        .indices
                        .push((index_offset + index as usize) as u32);
                }
            }

            if let Some(iter) = reader.read_positions() {
                log!("positions count {:?}", iter.len());
                for (i, position) in iter.enumerate() {
                    let mut vertex = Vertex::default();
                    vertex.index = i;
                    let mut position_converted = position;
                    position_converted[1] = 1.0 - position_converted[1];
                    vertex.position = position_converted;
                    gltf_data.vertices.push(vertex);
                }
            }

            if let Some(gltf::mesh::util::ReadTexCoords::F32(gltf::accessor::Iter::Standard(
                iter,
            ))) = reader.read_tex_coords(0)
            {
                for (i, texture_coord) in iter.enumerate() {
                    gltf_data.vertices[i].tex_coord = texture_coord;
                }
            }

            if let Some(iter) = reader.read_normals() {
                for normal in iter {
                    normals.push(normal);
                }
            }

            // texture
            if let Some(material) = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
            {
                let texture = material.texture();
                let image_index = texture.source().index();
                let image = &images[image_index];

                let size = (size_of::<u8>() * image.pixels.len()) as u64;
                let (width, height) = (image.width, image.height);
                let image_data = ImageData {
                    data: image.pixels.clone(),
                    size: size,
                    width: width,
                    height: height,
                };
                gltf_data.image_data.push(image_data);
            }

            // joint
            if let Some(iter) = reader.read_joints(0) {
                for (i, joint) in iter.into_u16().enumerate() {
                    log!("Joint {}: {:?}", i, joint);
                    gltf_data.vertices[i].joint_indices = joint;
                }
            }

            if let Some(iter) = reader.read_weights(0) {
                for (i, weight) in iter.into_f32().enumerate() {
                    log!("weight {}: {:?}", i, weight);
                    gltf_data.vertices[i].joint_weights = weight;
                }
            }

            // morph targets
            if let morph_targets = reader.read_morph_targets() {
                log!("morph targets count {:?}", morph_targets.len());
                for target in morph_targets {
                    let mut morph_target = MorphTarget::new();
                    let (positions, normals, tangents) = target;
                    // positions
                    if let Some(position_iter) = positions {
                        log!("morph positions count {:?}", position_iter.len());
                        for position in position_iter {
                            morph_target.positions.push(position);
                        }
                    }
                    // normals
                    if let Some(normal_iter) = normals {
                        log!("morph normals count {:?}", normal_iter.len());
                        for normal in normal_iter {
                            morph_target.normals.push(normal);
                        }
                    }
                    // tangents
                    if let Some(tangent_iter) = tangents {
                        log!("morph tangents count {:?}", tangent_iter.len());
                        for tangent in tangent_iter {
                            morph_target.tangents.push(tangent);
                        }
                    }
                    gltf_data.morph_targets.push(morph_target);
                }
            }

            // validate
            log!("vertex count {}", gltf_data.vertices.len());
            log!("indices count {}", gltf_data.indices.len());
            log!("morph targets count {}", gltf_data.morph_targets.len());
            gltf_model.gltf_data.push(gltf_data);
        });
    }
    for child in node.children() {
        process_node(gltf, buffers, images, &child, gltf_model)?;
    }

    Ok(())
}

unsafe fn process_animation(
    gltf: &Document,
    buffers: &Vec<Data>,
    animation: gltf::Animation,
    gltf_model: &mut GltfModel,
) -> Result<()> {
    for channel in animation.channels() {
        let gltf_data_index = channel.animation().index();
        let gltf_data = &gltf_model.gltf_data[gltf_data_index];
        let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
        let mut key_frames = Vec::new();
        let mut weights = Vec::new();
        if let Some(inputs) = reader.read_inputs() {
            log!("KeyFrame Count: {:?}", inputs.len());
            for input in inputs {
                log!("KeyFrame input {:?}", input);
                key_frames.push(input);
            }
        }

        if let Some(outputs) = reader.read_outputs() {
            use gltf::animation::util::ReadOutputs;
            match outputs {
                ReadOutputs::Translations(translations) => {
                    for (i, translation) in translations.enumerate() {
                        log!("Translation {}: {:?}", i, translation);
                    }
                }
                ReadOutputs::Rotations(rotations) => {
                    for (i, rotation) in rotations.into_f32().enumerate() {
                        log!("Rotation {}: {:?}", i, rotation);
                    }
                }
                ReadOutputs::Scales(scales) => {
                    for (i, scale) in scales.enumerate() {
                        log!("Scale {}: {:?}", i, scale);
                    }
                }
                ReadOutputs::MorphTargetWeights(morph_target_weights) => {
                    let mut weight = Vec::new();
                    // TODO: multi data
                    let morph_target_length = gltf_data.morph_targets.len();
                    for (i, morph_target_weight) in morph_target_weights.into_f32().enumerate() {
                        log!("Morph Target Weight: {} {:?}", i, morph_target_weight);
                        weight.push(morph_target_weight);
                        if weight.len() >= morph_target_length {
                            weights.push(weight);
                            weight = Vec::new();
                        }
                    }
                }
            }
        }

        if key_frames.len() != weights.len() {
            log!("KeyFrame Count != Weight Count");
        }

        if key_frames.len() != 0 && weights.len() != 0 && key_frames.len() == weights.len() {
            for i in 0..key_frames.len() {
                let mut morph_animation = MorphAnimation::new();
                morph_animation.key_frame = key_frames[i];
                morph_animation.weights = weights[i].clone();
                gltf_model.morph_animations.push(morph_animation);
            }
        }

        // validate
        for i in 0..gltf_model.morph_animations.len() {
            for j in 0..gltf_model.morph_animations[i].weights.len() {
                let morph_animation = &gltf_model.morph_animations[i];
                log!(
                    "Morph Animation {} KeyFrame {:?} Weight {} {:?}",
                    i,
                    morph_animation.key_frame,
                    j,
                    morph_animation.weights[j]
                );
            }
        }
        log!("vertex count {:?}", gltf_data.vertices.len());
        if gltf_data.morph_targets.len() > 0 {
            // morphing
            log!(
                "target0 position count {:?}",
                gltf_data.morph_targets[0].positions.len()
            );
            log!(
                "morph animation0 weights count {:?}",
                gltf_model.morph_animations[0].weights.len()
            );
        }
    }
    Ok(())
}
