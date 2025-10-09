use crate::log;
use crate::math::math::{vec3_from_array, Mat4};
use anyhow::{anyhow, Result};
use cgmath::Quaternion;
use core::result::Result::Ok;
use glium::buffer::Content;
use gltf::buffer::Data;
use gltf::{image, Document, Gltf, Node};
use std::collections::HashMap;
use std::ptr::null;

#[derive(Clone, Debug, Default)]
pub struct GltfModel {
    pub gltf_data: Vec<GltfData>,
    pub morph_animations: Vec<MorphAnimation>,
    pub joints: Vec<Joint>, // order by node id
    pub joint_animations: Vec<Vec<JointAnimation>>,
    pub node_joint_map: NodeJointMap,
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

    pub fn set_joints(self: &mut Self, skin: &gltf::Skin) {
        self.joints.clear();
        if self.node_joint_map.node_to_joint.len() <= 0 {
            self.node_joint_map.make_from_skin(skin);
        }

        let mut temp_joints: Vec<_> = skin.joints().collect();
        temp_joints.sort_by_key(|node| node.index());
        for (node_index, node) in temp_joints.iter().enumerate() {
            let mut joint = Joint::default();
            joint.index = self
                .node_joint_map
                .get_joint_index(node_index as u16)
                .unwrap();
            joint.name = node.name().unwrap().to_string();
            log!(
                "Joint Pushed: Node Index: {}, Node Name: {}, Joint Index: {}",
                node_index,
                joint.name,
                joint.index
            );
            self.joints.push(joint);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GltfData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub image_indices: Vec<[u16; 4]>,
    pub image_data: Vec<ImageData>,
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
    pub index: u16,
    pub name: String,
    pub vertex_indices: Vec<u16>,
    pub child_joint_indices: Vec<u16>,
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

#[derive(Clone, Debug, Default)]
pub struct JointAnimation {
    pub key_frames: Vec<f32>,
    pub translations: Vec<Mat4>,
    pub rotations: Vec<Mat4>,
    pub scales: Vec<Mat4>,
}

#[derive(Clone, Debug, Default)]
pub struct NodeJointMap {
    pub node_to_joint: HashMap<u16, u16>,
    pub joint_to_node: HashMap<u16, u16>,
}

impl NodeJointMap {
    pub fn make_from_skin(&mut self, skin: &gltf::Skin) {
        self.node_to_joint.clear();
        self.joint_to_node.clear();
        self.node_to_joint = HashMap::new();
        self.joint_to_node = HashMap::new();
        for (joint_index, joint_node) in skin.joints().enumerate() {
            self.node_to_joint
                .insert(joint_node.index() as u16, joint_index as u16);
            self.joint_to_node
                .insert(joint_index as u16, joint_node.index() as u16);
            log!(
                "Node Joint Map, node name: {}, node index: {}, joint: {}",
                joint_node.name().unwrap(),
                joint_node.index(),
                joint_index
            );
        }
    }

    pub fn get_joint_index(&self, node_index: u16) -> Option<u16> {
        match self.node_to_joint.get(&node_index) {
            Some(&joint_index) => Some(joint_index),
            None => {
                log!("Error: node {} is not in map", node_index);
                None
            }
        }
    }

    pub fn get_node_index(&self, joint_index: u16) -> Option<u16> {
        match self.joint_to_node.get(&joint_index) {
            Some(&node_index) => Some(node_index),
            None => {
                log!("Error: node {} is not in map", joint_index);
                None
            }
        }
    }
}

impl GltfData {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            image_indices: Vec::new(),
            image_data: Vec::new(),
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
    gltf.skins().enumerate().for_each(|(i, skin)| {
        gltf_model.node_joint_map.make_from_skin(&skin);
        gltf_model.set_joints(&skin);
    });
    for scene in gltf.scenes() {
        for node in scene.nodes() {
            process_node(&gltf, &buffers, &images, &node, gltf_model).unwrap();
        }
    }
    log_node_hierarchy(gltf_model);
    initialize_joint_animation(gltf_model);
    for animation in gltf.animations() {
        process_animation(&gltf, &buffers, animation, gltf_model).unwrap();
    }
    // validation
    for (i, joint_animation_joint) in gltf_model.joint_animations.iter().enumerate() {
        for (j, joint_animation_animation) in joint_animation_joint.iter().enumerate() {
            log!(
                "Joint Id {}, Animation Index {}, KeyFrameLength {}, TranslationLength {}, RotationLength {}, ScaleLength {}",
                i,
                j,
                joint_animation_animation.key_frames.len(),
                joint_animation_animation.translations.len(),
                joint_animation_animation.rotations.len(),
                joint_animation_animation.scales.len()
            );
        }
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
                    log!("Vertex {}, Joint: {:?}", i, joint);
                    gltf_data.vertices[i].joint_indices = joint;
                }
            }

            if let Some(iter) = reader.read_weights(0) {
                for (i, weight) in iter.into_f32().enumerate() {
                    log!("Vertex {}, Weight: {:?}", i, weight);
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

            input_joint_vertex(gltf_model, &mut gltf_data, node);

            gltf_model.gltf_data.push(gltf_data);
        });
    }

    // TODO: more hard comparison
    if node.index() < gltf_model.joints.len() {
        let joint = &mut gltf_model.joints[node.index()];

        for child in node.children() {
            joint.child_joint_indices.push(
                gltf_model
                    .node_joint_map
                    .get_joint_index(child.index() as u16)
                    .unwrap(),
            );
        }
    }

    for child in node.children() {
        process_node(gltf, buffers, images, &child, gltf_model)?;
    }

    Ok(())
}

unsafe fn input_joint_vertex(gltf_model: &mut GltfModel, gltf_data: &mut GltfData, node: &Node) {
    if gltf_data.vertices.len() <= 0 {
        return;
    }
    if gltf_data.vertices[0].joint_indices.len() <= 0 {
        return;
    }

    for (i, joint) in gltf_model.joints.iter_mut().enumerate() {
        for j in 0..gltf_data.vertices.len() {
            let joint_indices = &gltf_data.vertices[j].joint_indices;
            if joint_indices.contains(&joint.index) {
                joint.vertex_indices.push(j as u16);
            }
        }
    }

    // validate
    for i in 0..gltf_data.vertices.len() {
        let joint_indices = &gltf_data.vertices[i].joint_indices;
        for j in 0..joint_indices.len() {
            let joint_index = &joint_indices[j];
            if joint_indices[j] == 0 {
                continue;
            }
            let node_id = gltf_model
                .node_joint_map
                .get_node_index(joint_indices[j])
                .unwrap();
            let target_joint = &gltf_model.joints[node_id as usize];
            if !target_joint.vertex_indices.contains(&(i as u16)) {
                log!(
                    "invalid joint id, vertex id {}, joint id {}",
                    i,
                    joint_index
                )
            }
        }
    }
}

unsafe fn log_node_hierarchy(gltf_model: &GltfModel) {
    for (i, joint) in gltf_model.joints.iter().enumerate() {
        log!(
            "joint name: {}, node Id: {}, joint Id: {}, child joint indices: {:?}",
            joint.name,
            i,
            joint.index,
            joint.child_joint_indices
        );
    }
}

unsafe fn initialize_joint_animation(gltf_model: &mut GltfModel) {
    for _ in 0..gltf_model.joints.len() {
        gltf_model.joint_animations.push(Vec::default())
    }
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
        let target = channel.target();
        let node = target.node();
        log!("target animation index {}", target.animation().index());
        log!("node index {}", node.index());
        let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
        let mut key_frames = Vec::new();
        let mut weights = Vec::new();
        let mut joint_translations = Vec::new();
        let mut joint_rotations = Vec::new();
        let mut joint_scales = Vec::new();
        if let Some(inputs) = reader.read_inputs() {
            log!("KeyFrame Count: {:?}", inputs.len());
            for (i, input) in inputs.enumerate() {
                log!("KeyFrame input {}: {:?}", i, input);
                key_frames.push(input);
            }
        }

        if let Some(outputs) = reader.read_outputs() {
            use gltf::animation::util::ReadOutputs;
            match outputs {
                ReadOutputs::Translations(translations) => {
                    for (i, translation) in translations.enumerate() {
                        log!("Translation {}: {:?}", i, translation);
                        let joint_translation = vec3_from_array(translation);
                        let joint_translation_mat = Mat4::from_translation(joint_translation);
                        joint_translations.push(joint_translation_mat);
                    }
                }
                ReadOutputs::Rotations(rotations) => {
                    for (i, rotation) in rotations.into_f32().enumerate() {
                        log!("Rotation {}: {:?}", i, rotation);
                        let joint_rotaion_mat = Mat4::from(Quaternion::new(
                            rotation[0],
                            rotation[1],
                            rotation[2],
                            rotation[3],
                        ));
                        joint_rotations.push(joint_rotaion_mat);
                    }
                }
                ReadOutputs::Scales(scales) => {
                    for (i, scale) in scales.enumerate() {
                        log!("Scale {}: {:?}", i, scale);
                        let joint_scale_mat =
                            Mat4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
                        joint_scales.push(joint_scale_mat);
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

        if key_frames.len() > 0
            && (joint_translations.len() > 0
                || joint_rotations.len() > 0
                || joint_rotations.len() > 0)
        {
            let mut joint_animation = JointAnimation::default();
            let joint_id = gltf_model
                .node_joint_map
                .get_joint_index(node.index() as u16)
                .unwrap();
            for i in 0..key_frames.len() {
                joint_animation.key_frames.push(key_frames[i]);
                if joint_translations.len() > 0 {
                    joint_animation.translations.push(joint_translations[i]);
                }
                if joint_rotations.len() > 0 {
                    joint_animation.rotations.push(joint_rotations[i]);
                }
                if joint_scales.len() > 0 {
                    joint_animation.scales.push(joint_scales[i]);
                }
            }
            gltf_model.joint_animations[joint_id as usize].push(joint_animation);
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
        log!("key frame count {:?}", key_frames.len());
        log!("joint translation count {:?}", joint_translations.len());
        log!("joint rotation count {:?}", joint_rotations.len());
        log!("joint scale count {:?}", joint_scales.len());
    }
    Ok(())
}
