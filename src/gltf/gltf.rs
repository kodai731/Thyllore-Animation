use crate::log;
use anyhow::{anyhow, Result};
use core::result::Result::Ok;
use glium::buffer::Content;
use gltf::buffer::Data;
use gltf::{image, Document, Gltf, Node};

#[derive(Clone, Debug, Default)]
pub struct GltfData {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub tex_coords: Vec<[f32; 2]>,
    pub joint_indices: Vec<[u16; 4]>,
    pub joint_weights: Vec<[f32; 4]>,
    pub morph_targets: Vec<MorphTarget>,
    pub morph_animations: Vec<MorphAnimation>,
    pub image_indices: Vec<[u16; 4]>,
    pub image_data: Vec<ImageData>,
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
            positions: Vec::new(),
            indices: Vec::new(),
            tex_coords: Vec::new(),
            joint_indices: Vec::new(),
            joint_weights: Vec::new(),
            morph_targets: Vec::new(),
            morph_animations: Vec::new(),
            image_indices: Vec::new(),
            image_data: Vec::new(),
        }
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
        let end_index = self.morph_targets.len();
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

pub unsafe fn load_gltf(path: &str) -> Result<GltfData> {
    let (gltf, buffers, images) = gltf::import(format!("{}", path)).expect("Failed to load grass");
    let mut gltf_data = GltfData::new();
    for scene in gltf.scenes() {
        for node in scene.nodes() {
            process_node(&gltf, &buffers, &images, &node, &mut gltf_data)?;
        }
    }
    for animation in gltf.animations() {
        process_animation(&gltf, &buffers, animation, &mut gltf_data)?;
    }
    Ok(gltf_data)
}

unsafe fn process_node(
    gltf: &Document,
    buffers: &Vec<Data>,
    images: &Vec<gltf::image::Data>,
    node: &Node,
    gltf_data: &mut GltfData,
) -> Result<()> {
    println!("Node {} {}", node.index().to_string(), node.name().unwrap());
    // meshes
    if let Some(mesh) = node.mesh() {
        println!("mesh found");
        let primitives = mesh.primitives();
        let mut normals = Vec::new();
        let mut joint_indices = Vec::new();
        let mut joint_weights = Vec::new();

        // primitive
        primitives.for_each(|primitive| {
            println!("primitive found");
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            println!("Topology: {:?}", primitive.mode());

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
                println!("positions count {:?}", iter.len());
                for position in iter {
                    let mut position_converted = position;
                    position_converted[1] = 1.0 - position_converted[1];
                    gltf_data.positions.push(position_converted);
                }
            }

            if let Some(gltf::mesh::util::ReadTexCoords::F32(gltf::accessor::Iter::Standard(
                iter,
            ))) = reader.read_tex_coords(0)
            {
                for texture_coord in iter {
                    gltf_data.tex_coords.push(texture_coord);
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
                let image = &images[texture.source().index()];

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
                for joint in iter.into_u16() {
                    print!("Joint: {:?}", joint);
                    joint_indices.push(joint);
                    gltf_data.joint_indices.push(joint);
                }
            }

            if let Some(iter) = reader.read_weights(0) {
                for weight in iter.into_f32() {
                    println!("weight: {:?}", weight);
                    joint_weights.push(weight);
                    gltf_data.joint_weights.push(weight);
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
                        println!("morph positions count {:?}", position_iter.len());
                        for position in position_iter {
                            morph_target.positions.push(position);
                        }
                    }
                    // normals
                    if let Some(normal_iter) = normals {
                        println!("morph normals count {:?}", normal_iter.len());
                        for normal in normal_iter {
                            morph_target.normals.push(normal);
                        }
                    }
                    // tangents
                    if let Some(tangent_iter) = tangents {
                        println!("morph tangents count {:?}", tangent_iter.len());
                        for tangent in tangent_iter {
                            morph_target.tangents.push(tangent);
                        }
                    }
                    gltf_data.morph_targets.push(morph_target);
                }
            }
        });
    }
    for child in node.children() {
        process_node(gltf, buffers, images, &child, gltf_data)?;
    }

    // validate
    println!("indices count {}", gltf_data.indices.len());
    println!("joint indices count {}", gltf_data.joint_indices.len());
    println!("joint weights count {}", gltf_data.joint_weights.len());
    println!("positions count {}", gltf_data.positions.len());
    println!("tex coords count {}", gltf_data.tex_coords.len());
    println!("morph targets count {}", gltf_data.morph_targets.len());

    Ok(())
}

unsafe fn process_animation(
    gltf: &Document,
    buffers: &Vec<Data>,
    animation: gltf::Animation,
    gltf_data: &mut GltfData,
) -> Result<()> {
    for channel in animation.channels() {
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
                    println!("Translations");
                    for translation in translations {
                        println!("Translation: {:?}", translation);
                    }
                }
                ReadOutputs::Rotations(rotations) => {
                    println!("Rotations");
                    // if let rotations = outputs {
                    //     for rotation in rotations {
                    //         println!("Rotation: {:?}", rotation);
                    //     }
                    // }
                }
                ReadOutputs::Scales(scales) => {
                    println!("Scales");
                    for scale in scales {
                        println!("Scale: {:?}", scale);
                    }
                }
                ReadOutputs::MorphTargetWeights(morph_target_weights) => {
                    println!("Morph Target Weights");
                    let mut weight = Vec::new();
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
            eprintln!("KeyFrame Count != Weight Count");
        }

        if key_frames.len() != 0 && weights.len() != 0 && key_frames.len() == weights.len() {
            for i in 0..key_frames.len() {
                let mut morph_animation = MorphAnimation::new();
                morph_animation.key_frame = key_frames[i];
                morph_animation.weights = weights[i].clone();
                gltf_data.morph_animations.push(morph_animation);
            }
        }

        // validate
        for i in 0..gltf_data.morph_animations.len() {
            for j in 0..gltf_data.morph_animations[i].weights.len() {
                let morph_animation = &gltf_data.morph_animations[i];
                log!(
                    "Morph Target {} KeyFrame {:?} Weight {} {:?}",
                    i,
                    morph_animation.key_frame,
                    j,
                    morph_animation.weights[j]
                );
            }
        }
        log!("position count {:?}", gltf_data.positions.len());
        log!(
            "target0 position count {:?}",
            gltf_data.morph_targets[0].positions.len()
        );
        log!(
            "morph animation0 weights count {:?}",
            gltf_data.morph_animations[0].weights.len()
        );
    }
    Ok(())
}
