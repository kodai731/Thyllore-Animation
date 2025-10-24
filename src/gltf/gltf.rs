use crate::log;
use crate::math::math::*;
use anyhow::{anyhow, Result};
use cgmath::{Matrix4, Quaternion};
use core::result::Result::Ok;
use gltf::buffer::Data;
use gltf::{image, Document, Gltf, Node};
use std::collections::HashMap;
use std::fs::File;
use std::ptr::null;

#[derive(Clone, Debug, Default)]
pub struct GltfModel {
    pub gltf_data: Vec<GltfData>,
    pub morph_animations: Vec<MorphAnimation>,
    pub joints: Vec<Joint>, // order by joint id
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

    pub fn set_joints(self: &mut Self, skin: &gltf::Skin, buffers: &Vec<Data>) {
        self.joints.clear();
        if self.node_joint_map.node_to_joint.len() <= 0 {
            self.node_joint_map.make_from_skin(skin);
        }

        let mut temp_joints: Vec<_> = skin.joints().collect();
        for (joint_index, node) in temp_joints.iter().enumerate() {
            let mut joint = Joint::default();
            joint.index = joint_index as u16;
            joint.name = node.name().unwrap().to_string();
            let joint_transform = mat4_from_array(node.transform().matrix());
            joint.transform = array_from_mat4(joint_transform);
            let node_index = self.node_joint_map.get_node_index(joint.index).unwrap();
            log!(
                "Joint Pushed: Node Index: {}, Node Name: {}, Joint Index: {}",
                node_index,
                joint.name,
                joint.index
            );
            self.joints.push(joint);
        }

        if let Some(_) = skin.inverse_bind_matrices() {
            let reader = skin.reader(|buffer| Some(&buffers[buffer.index()]));
            // convert milli meter to meter
            if let Some(iter) = reader.read_inverse_bind_matrices() {
                log!("Inverse bind poses: {:?}", iter.len());
                for (i, mat) in iter.enumerate() {
                    let mut inverse_bind_pose = mat4_from_array(mat);
                    // convert milli meter to meter
                    inverse_bind_pose = inverse_bind_pose * Matrix4::from_scale(0.001f32);
                    self.joints[i].inverse_bind_pose = array_from_mat4(inverse_bind_pose);
                    log!("Inverse bind pose {}: {:?}", i, inverse_bind_pose);
                }
            }
        }
    }

    pub fn apply_animation(self: &mut Self, time: f32, target_joint_id: usize, transform: Mat4) {
        // joints[0] = Root
        let joint = &self.joints[target_joint_id];
        let joint_animations = &self.joint_animations[target_joint_id];
        let mut joint_translate = Mat4::identity();
        let mut joint_rotation = Mat4::identity();
        let mut joint_scale = Mat4::identity();
        for joint_animation in joint_animations {
            let key_frame_id = joint_animation.identify_key_frame_index(time);
            if joint_animation.scales.len() > key_frame_id {
                joint_scale = joint_animation.scales[key_frame_id] * joint_scale;
            }
            if joint_animation.rotations.len() > key_frame_id {
                joint_rotation = joint_animation.rotations[key_frame_id] * joint_rotation;
            }
            if joint_animation.translations.len() > key_frame_id {
                joint_translate = joint_animation.translations[key_frame_id] * joint_translate;
            }
        }

        let joint_inverse_bind_pose = mat4_from_array(joint.inverse_bind_pose);
        let joint_transform =
            (transform * joint_translate * joint_rotation * joint_scale).transpose();
        for joint_vertex_id in &joint.vertex_indices {
            let vertex = &mut self.gltf_data[joint_vertex_id.gltf_data_index].vertices
                [joint_vertex_id.vertex_index];
            let weight = vertex.get_weight_from_joint_id(joint.index);
            let mut pos = vec4_from_array([
                vertex.position[0],
                vertex.position[1],
                vertex.position[2],
                1f32,
            ]);
            pos = weight * joint_transform * joint_inverse_bind_pose.transpose() * pos;
            vertex.animation_position[0] += pos.x;
            vertex.animation_position[1] += pos.y;
            vertex.animation_position[2] += pos.z;
        }

        let child_indices = joint.child_joint_indices.clone();
        for child_index in child_indices {
            self.apply_animation(time, child_index as usize, joint_transform)
        }
    }

    // debug
    pub fn validate_vertex_position(self: &Self) {
        for (i, gltf_data) in self.gltf_data.iter().enumerate() {
            for vertex in &gltf_data.vertices {
                if !approx_equal_array3(&vertex.position, &vertex.animation_position) {
                    log!(
                        "invalid vertex animation position: joint id {:?}, gltf data index {}, vertex id {}",
                        vertex.joint_indices,
                        i,
                        vertex.index
                    );
                }
            }
        }
    }

    pub fn reset_vertices_animation_position(self: &mut Self) {
        self.gltf_data.iter_mut().for_each(|gltf_data| {
            gltf_data.vertices.iter_mut().for_each(|vertex| {
                vertex.animation_position = vertex.position;
            })
        })
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
    pub animation_position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub tex_coord: [f32; 2],
    pub joint_indices: [u16; 4],
    pub joint_weights: [f32; 4],
}

impl Vertex {
    pub fn identify_index_from_joint_id(&self, joint_id: u16) -> usize {
        for (i, vertex_joint_id) in self.joint_indices.iter().enumerate() {
            if *vertex_joint_id == joint_id {
                return i;
            }
        }
        log!(
            "invalid: this vertex {} is not included in joint {}",
            self.index,
            joint_id
        );
        3
    }

    pub fn get_weight_from_joint_id(&self, joint_id: u16) -> f32 {
        let index = self.identify_index_from_joint_id(joint_id);
        self.joint_weights[index]
    }
}

#[derive(Clone, Debug, Default)]
pub struct Joint {
    pub index: u16,
    pub name: String,
    pub vertex_indices: Vec<JointVertexIndex>,
    pub child_joint_indices: Vec<u16>,
    pub inverse_bind_pose: [[f32; 4]; 4],
    pub transform: [[f32; 4]; 4],
}

#[derive(Clone, Debug, Default)]
pub struct JointVertexIndex {
    pub gltf_data_index: usize,
    pub vertex_index: usize,
}

impl PartialEq for JointVertexIndex {
    fn eq(&self, other: &Self) -> bool {
        self.gltf_data_index == other.gltf_data_index && self.vertex_index == other.vertex_index
    }
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

impl JointAnimation {
    pub fn identify_key_frame_index(&self, time: f32) -> usize {
        let period = self.key_frames.last().unwrap();
        let time = time.rem_euclid(*period);
        for (i, key_frame) in self.key_frames.iter().enumerate() {
            if time < *key_frame {
                return i;
            }
        }
        return self.key_frames.len() - 1;
    }
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

    pub fn contain_node_index(&self, node_index: u16) -> bool {
        self.node_to_joint.contains_key(&node_index)
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
        gltf_model.set_joints(&skin, &buffers);
    });
    for scene in gltf.scenes() {
        for node in scene.nodes() {
            process_node(&gltf, &buffers, &images, &node, gltf_model).unwrap();
        }
    }
    input_joint_vertex(gltf_model);
    log_node_hierarchy(gltf_model);
    validate_inverse_bind_pose(gltf_model, 0, fix_coord());
    load_white_texture_if_none(gltf_model);

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
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            log!("Topology: {:?}", primitive.mode());

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
                let mut gltf_data = GltfData::default();
                gltf_data.image_data.push(image_data);
                gltf_model.gltf_data.push(gltf_data);
            }

            if gltf_model.gltf_data.len() < 1 {
                gltf_model.gltf_data.push(GltfData::default());
            }

            let gltf_data = gltf_model.gltf_data.last_mut().unwrap();

            let index_offset = gltf_data.vertices.len();
            if let Some(iter) = reader.read_indices() {
                for index in iter.into_u32() {
                    gltf_data
                        .indices
                        .push((index_offset + index as usize) as u32);
                }
            }

            if let Some(iter) = reader.read_positions() {
                log!("positions count {:?}", iter.len());
                for position in iter {
                    let mut vertex = Vertex::default();
                    vertex.index = gltf_data.vertices.len();
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
        });
    }

    if gltf_model
        .node_joint_map
        .contain_node_index(node.index() as u16)
    {
        let joint_index = gltf_model
            .node_joint_map
            .get_joint_index(node.index() as u16)
            .unwrap();
        let joint = &mut gltf_model.joints[joint_index as usize];

        for child in node.children() {
            if gltf_model
                .node_joint_map
                .contain_node_index(child.index() as u16)
            {
                joint.child_joint_indices.push(
                    gltf_model
                        .node_joint_map
                        .get_joint_index(child.index() as u16)
                        .unwrap(),
                );
            }
        }
    }

    for child in node.children() {
        process_node(gltf, buffers, images, &child, gltf_model)?;
    }

    Ok(())
}

fn load_white_texture_if_none(gltf_model: &mut GltfModel) {
    for gltf_data in &mut gltf_model.gltf_data {
        if gltf_data.image_data.len() == 0 {
            let image_data = load_white_texture().unwrap();
            gltf_data.image_data.push(image_data);
        }
    }
}

// TODO: use vulkanr::image
fn load_white_texture() -> Result<ImageData> {
    let image = File::open("src/resources/white.png")?;
    let decoder = png::Decoder::new(image);
    let mut reader = decoder.read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut pixels)?;
    let size = reader.info().raw_bytes() as u64;
    let (width, height) = reader.info().size();
    Ok(ImageData {
        data: pixels,
        size,
        width,
        height,
    })
}

unsafe fn input_joint_vertex(gltf_model: &mut GltfModel) {
    for (i, gltf_data) in &mut gltf_model.gltf_data.iter().enumerate() {
        if gltf_data.vertices.len() <= 0 {
            return;
        }
        if gltf_data.vertices[0].joint_indices.len() <= 0 {
            return;
        }

        gltf_data.vertices.iter().for_each(|vertex| {
            vertex.joint_indices.iter().for_each(|joint_index| {
                let mut joint_vertex_index = JointVertexIndex::default();
                joint_vertex_index.gltf_data_index = i;
                joint_vertex_index.vertex_index = vertex.index;
                if !gltf_model.joints[*joint_index as usize]
                    .vertex_indices
                    .contains(&joint_vertex_index)
                {
                    gltf_model.joints[*joint_index as usize]
                        .vertex_indices
                        .push(joint_vertex_index);
                }
            })
        });

        // validate
        for vertex in &gltf_data.vertices {
            let joint_indices = vertex.joint_indices;
            let mut is_vertex_found = false;
            for (j, joint_index) in joint_indices.iter().enumerate() {
                let target_joint = &gltf_model.joints[*joint_index as usize];
                let mut is_joint_found = false;
                for joint_vertex_index in &target_joint.vertex_indices {
                    if joint_vertex_index.vertex_index == vertex.index {
                        is_joint_found = true;
                        is_vertex_found = true;
                        break;
                    }
                }
                if !is_joint_found {
                    log!(
                        "invalid: joint index not found: Gltf Index {}, Joint Index Id {}, Joint Index {}",
                        i,
                        j,
                        joint_index
                    );
                }
            }
            if !is_vertex_found {
                log!("invalid: vertex {} is not included in Joint", vertex.index);
            }
        }
    }
}

unsafe fn log_node_hierarchy(gltf_model: &GltfModel) {
    for (i, joint) in gltf_model.joints.iter().enumerate() {
        let node_index = gltf_model
            .node_joint_map
            .get_node_index(joint.index)
            .unwrap();
        log!(
            "joint name: {}, node Id: {}, joint Id: {}, child joint indices: {:?}",
            joint.name,
            node_index,
            joint.index,
            joint.child_joint_indices
        );
    }
}

// debug test
unsafe fn validate_inverse_bind_pose(gltf_model: &GltfModel, joint_index: u16, transform: Mat4) {
    let joint = &gltf_model.joints[joint_index as usize];
    let inverse_bind_pose = mat4_from_array(joint.inverse_bind_pose);
    let joint_transform = mat4_from_array(joint.transform);
    log!("joint transform {}: {:?}", joint_index, joint_transform);
    let transform = transform * joint_transform;
    let multiplied = transform * inverse_bind_pose;
    if !approx_equal_mat4(&multiplied, &Mat4::identity()) {
        log!(
            "invalid: inverse transform is not invertible, joint id {}, multi product {:?}",
            joint_index,
            multiplied,
        );
    }
    for child in &joint.child_joint_indices {
        validate_inverse_bind_pose(gltf_model, *child, transform);
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
        log!("node name {}", node.name().unwrap());
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
                        // convert milli meter to meter
                        let joint_translation = vec3_from_array(translation) * 0.00001f32;
                        let joint_translation_mat = Mat4::from_translation(joint_translation);
                        joint_translations.push(joint_translation_mat);
                        log!("Translation Matrix {}: {:?}", i, joint_translation_mat);
                    }
                }
                ReadOutputs::Rotations(rotations) => {
                    for (i, rotation) in rotations.into_f32().enumerate() {
                        log!("Rotation {}: {:?}", i, rotation);
                        let joint_rotaion_mat = Mat4::from(
                            Quaternion::new(rotation[0], rotation[1], rotation[2], rotation[3])
                                .normalize(),
                        );
                        joint_rotations.push(joint_rotaion_mat);
                    }
                }
                ReadOutputs::Scales(scales) => {
                    for (i, scale) in scales.enumerate() {
                        log!("Scale {}: {:?}", i, scale);
                        let joint_scale_mat =
                            Mat4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
                        log!("Scale Matrix {} : {:?}", i, joint_scale_mat);
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
