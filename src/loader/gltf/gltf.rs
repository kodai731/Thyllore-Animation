use crate::log;
use crate::math::*;
use anyhow::Result;
use cgmath::{Matrix4, Quaternion, Vector3, Vector4};
use core::result::Result::Ok;
use gltf::buffer::Data;
use gltf::{Document, Node};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct GltfModel {
    pub gltf_data: Vec<GltfData>,
    pub morph_animations: Vec<MorphAnimation>,
    pub joints: Vec<Joint>,
    pub joint_animations: Vec<Vec<JointAnimation>>,
    pub node_animations: Vec<NodeAnimation>,
    pub node_joint_map: NodeJointMap,
    pub rrnodes: Vec<RRNode>,
    pub has_skinned_meshes: bool,
    pub has_armature: bool,
    pub skeleton_root_transform: Option<[[f32; 4]; 4]>,
}

impl GltfModel {
    pub unsafe fn load_model(path: &str) -> Self {
        let mut gltf_model = GltfModel::default();
        gltf_model.morph_animations = Vec::new();
        load_gltf(&mut gltf_model, path);
        gltf_model
    }

    pub fn set_joints(self: &mut Self, skin: &gltf::Skin, buffers: &Vec<Data>) {
        self.joints.clear();
        if self.node_joint_map.node_to_joint.len() <= 0 {
            self.node_joint_map.make_from_skin(skin);
        }

        let temp_joints: Vec<_> = skin.joints().collect();
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
            if let Some(iter) = reader.read_inverse_bind_matrices() {
                log!("Inverse bind poses: {:?}", iter.len());
                for (i, mat) in iter.enumerate() {
                    let inverse_bind_pose = mat4_from_array(mat);
                    self.joints[i].inverse_bind_pose = array_from_mat4(inverse_bind_pose);
                }
            }
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
    pub has_joints: bool,
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
    pub node_id: u16,
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
pub struct RRNode {
    pub index: u16,
    pub name: String,
    pub vertex_indices: Vec<JointVertexIndex>,
    pub transform: [[f32; 4]; 4],
    pub children: Vec<u16>,
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

#[derive(Clone, Debug)]
pub struct NodeAnimation {
    pub node_index: usize,
    pub translation_keyframes: Vec<f32>,
    pub translations: Vec<Vector3<f32>>,
    pub rotation_keyframes: Vec<f32>,
    pub rotations: Vec<cgmath::Quaternion<f32>>,
    pub scale_keyframes: Vec<f32>,
    pub scales: Vec<Vector3<f32>>,
    pub default_translation: Vector3<f32>,
    pub default_rotation: cgmath::Quaternion<f32>,
    pub default_scale: Vector3<f32>,
}

impl Default for NodeAnimation {
    fn default() -> Self {
        NodeAnimation {
            node_index: 0,
            translation_keyframes: Vec::new(),
            translations: Vec::new(),
            rotation_keyframes: Vec::new(),
            rotations: Vec::new(),
            scale_keyframes: Vec::new(),
            scales: Vec::new(),
            default_translation: Vector3::new(0.0, 0.0, 0.0),
            default_rotation: cgmath::Quaternion::new(1.0, 0.0, 0.0, 0.0),
            default_scale: Vector3::new(1.0, 1.0, 1.0),
        }
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

    pub fn get_node_index(&self, joint_index: u16) -> Option<u16> {
        self.joint_to_node.get(&joint_index).copied()
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
            has_joints: false,
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

fn build_node_parent_map(gltf: &Document) -> HashMap<usize, usize> {
    let mut parent_map = HashMap::new();
    for node in gltf.nodes() {
        for child in node.children() {
            parent_map.insert(child.index(), node.index());
        }
    }
    parent_map
}

fn compute_node_global_transform(
    gltf: &Document,
    node_index: usize,
    parent_map: &HashMap<usize, usize>,
) -> Matrix4<f32> {
    let mut transform_chain = Vec::new();
    let mut current_index = Some(node_index);

    while let Some(idx) = current_index {
        if let Some(node) = gltf.nodes().nth(idx) {
            transform_chain.push(mat4_from_array(node.transform().matrix()));
        }
        current_index = parent_map.get(&idx).copied();
    }

    let mut global = Matrix4::identity();
    for transform in transform_chain.iter().rev() {
        global = global * transform;
    }
    global
}

fn determine_skeleton_root_transform(
    gltf: &Document,
    skin: &gltf::Skin,
    parent_map: &HashMap<usize, usize>,
) -> Option<[[f32; 4]; 4]> {
    if let Some(skeleton_node) = skin.skeleton() {
        let global_transform =
            compute_node_global_transform(gltf, skeleton_node.index(), parent_map);
        log!(
            "Skeleton root transform from skin.skeleton: node {} ({:?})",
            skeleton_node.index(),
            skeleton_node.name()
        );
        return Some(array_from_mat4(global_transform));
    }

    let joint_indices: std::collections::HashSet<usize> =
        skin.joints().map(|j| j.index()).collect();
    let mut root_joint_node: Option<gltf::Node> = None;

    for joint in skin.joints() {
        let has_parent_joint = parent_map
            .get(&joint.index())
            .map(|parent_idx| joint_indices.contains(parent_idx))
            .unwrap_or(false);

        if !has_parent_joint {
            root_joint_node = Some(joint);
            break;
        }
    }

    if let Some(root_joint) = root_joint_node {
        if let Some(&parent_index) = parent_map.get(&root_joint.index()) {
            let parent_global = compute_node_global_transform(gltf, parent_index, parent_map);
            log!(
                "Skeleton root transform from root joint's parent: node {}",
                parent_index
            );
            return Some(array_from_mat4(parent_global));
        }
    }

    None
}

unsafe fn load_gltf(gltf_model: &mut GltfModel, path: &str) {
    log!("Loading glTF file: {}", path);
    let (gltf, buffers, images) = gltf::import(format!("{}", path)).expect("Failed to load model");

    log!(
        "glTF: {} skins, {} nodes, {} meshes, {} animations",
        gltf.skins().count(),
        gltf.nodes().count(),
        gltf.meshes().count(),
        gltf.animations().count()
    );

    let node_parent_map = build_node_parent_map(&gltf);

    let has_armature = gltf.skins().count() > 0;
    gltf_model.has_armature = has_armature;

    gltf.skins().enumerate().for_each(|(i, skin)| {
        log!(
            "Skin {}: name={:?}, {} joints",
            i,
            skin.name(),
            skin.joints().count()
        );
        gltf_model.node_joint_map.make_from_skin(&skin);
        gltf_model.set_joints(&skin, &buffers);

        let root_transform = determine_skeleton_root_transform(&gltf, &skin, &node_parent_map);
        gltf_model.skeleton_root_transform = root_transform;
    });

    for scene in gltf.scenes() {
        for node in scene.nodes() {
            process_node(
                &gltf,
                &buffers,
                &images,
                &node,
                gltf_model,
                &Matrix4::identity(),
                None,
            )
            .unwrap();
        }
    }

    input_joint_vertex(gltf_model);
    load_white_texture_if_none(gltf_model);

    initialize_joint_animation(gltf_model);
    for animation in gltf.animations() {
        process_animation(&gltf, &buffers, animation, gltf_model).unwrap();
    }

    log!(
        "Loaded: has_skinned_meshes={}, {} node_animations, {} joint_animations",
        gltf_model.has_skinned_meshes,
        gltf_model.node_animations.len(),
        gltf_model.joint_animations.len()
    );
}

unsafe fn process_node(
    gltf: &Document,
    buffers: &Vec<Data>,
    images: &Vec<gltf::image::Data>,
    node: &Node,
    gltf_model: &mut GltfModel,
    parent_transform: &Matrix4<f32>,
    parent_node_index: Option<usize>,
) -> Result<()> {
    let node_transform = mat4_from_array(node.transform().matrix());
    let cumulative_transform = *parent_transform * node_transform;

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let mut gltf_data = GltfData::new();

            if let Some(iter) = reader.read_positions() {
                for (i, position) in iter.enumerate() {
                    let mut pos = [position[0], position[1], position[2], 1f32].to_vec4();
                    pos = cumulative_transform * pos;

                    let mut vertex = Vertex::default();
                    vertex.index = i;
                    vertex.position = [pos.x, pos.y, pos.z];
                    vertex.node_id = node.index() as u16;
                    gltf_data.vertices.push(vertex);
                }
            }

            if let Some(iter) = reader.read_normals() {
                for (i, normal) in iter.enumerate() {
                    if i < gltf_data.vertices.len() {
                        gltf_data.vertices[i].normal = normal;
                    }
                }
            }

            if let Some(iter) = reader.read_tangents() {
                for (i, tangent) in iter.enumerate() {
                    if i < gltf_data.vertices.len() {
                        gltf_data.vertices[i].tangent = [tangent[0], tangent[1], tangent[2]];
                    }
                }
            }

            if let Some(iter) = reader.read_tex_coords(0) {
                for (i, tex_coord) in iter.into_f32().enumerate() {
                    if i < gltf_data.vertices.len() {
                        gltf_data.vertices[i].tex_coord = tex_coord;
                    }
                }
            }

            if let Some(iter) = reader.read_joints(0) {
                gltf_data.has_joints = true;
                gltf_model.has_skinned_meshes = true;
                for (i, joints) in iter.into_u16().enumerate() {
                    if i < gltf_data.vertices.len() {
                        gltf_data.vertices[i].joint_indices = joints;
                    }
                }
            }

            if let Some(iter) = reader.read_weights(0) {
                for (i, weights) in iter.into_f32().enumerate() {
                    if i < gltf_data.vertices.len() {
                        gltf_data.vertices[i].joint_weights = weights;
                    }
                }
            }

            if let Some(iter) = reader.read_indices() {
                gltf_data.indices = iter.into_u32().collect();
            }

            let morph_targets = reader.read_morph_targets();
            for (positions, normals, tangents) in morph_targets {
                let mut morph_target = MorphTarget::new();
                if let Some(position_iter) = positions {
                    for position in position_iter {
                        morph_target.positions.push(position);
                    }
                }
                if let Some(normal_iter) = normals {
                    for normal in normal_iter {
                        morph_target.normals.push(normal);
                    }
                }
                if let Some(tangent_iter) = tangents {
                    for tangent in tangent_iter {
                        morph_target.tangents.push(tangent);
                    }
                }
                gltf_data.morph_targets.push(morph_target);
            }

            if let Some(material) = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
            {
                let texture = material.texture();
                let source = texture.source();
                let image_index = source.index();

                if image_index < images.len() {
                    let image = &images[image_index];
                    let mut image_data = ImageData::default();

                    match image.format {
                        gltf::image::Format::R8G8B8A8 => {
                            image_data.data = image.pixels.clone();
                        }
                        gltf::image::Format::R8G8B8 => {
                            let mut rgba_data = Vec::with_capacity(image.pixels.len() / 3 * 4);
                            for chunk in image.pixels.chunks(3) {
                                rgba_data.push(chunk[0]);
                                rgba_data.push(chunk[1]);
                                rgba_data.push(chunk[2]);
                                rgba_data.push(255);
                            }
                            image_data.data = rgba_data;
                        }
                        _ => {
                            log!("Unsupported image format: {:?}", image.format);
                        }
                    }

                    image_data.width = image.width;
                    image_data.height = image.height;
                    image_data.size = image_data.data.len() as u64;
                    gltf_data.image_data.push(image_data);
                }
            }

            gltf_model.gltf_data.push(gltf_data);

            let gltf_data_index = gltf_model.gltf_data.len() - 1;
            let gltf_data = gltf_model.gltf_data.last_mut().unwrap();

            for (vertex_idx, _) in gltf_data.vertices.iter().enumerate() {
                let joint_vertex_index = JointVertexIndex {
                    gltf_data_index,
                    vertex_index: vertex_idx,
                };

                if let Some(rrnode) = gltf_model
                    .rrnodes
                    .iter_mut()
                    .find(|n| n.index == node.index() as u16)
                {
                    if !rrnode.vertex_indices.contains(&joint_vertex_index) {
                        rrnode.vertex_indices.push(joint_vertex_index);
                    }
                }
            }
        }
    }

    let rrnode = RRNode {
        index: node.index() as u16,
        name: node.name().unwrap_or("").to_string(),
        vertex_indices: Vec::new(),
        transform: array_from_mat4(node_transform),
        children: node.children().map(|c| c.index() as u16).collect(),
    };
    gltf_model.rrnodes.push(rrnode);

    if gltf_model
        .node_joint_map
        .contain_node_index(node.index() as u16)
    {
        let joint_index = gltf_model
            .node_joint_map
            .node_to_joint
            .get(&(node.index() as u16))
            .unwrap();
        let joint = &mut gltf_model.joints[*joint_index as usize];
        joint.transform = array_from_mat4(node_transform);

        if let Some(parent_index) = parent_node_index {
            if gltf_model
                .node_joint_map
                .contain_node_index(parent_index as u16)
            {
                let parent_joint_index = gltf_model
                    .node_joint_map
                    .node_to_joint
                    .get(&(parent_index as u16))
                    .unwrap();
                gltf_model.joints[*parent_joint_index as usize]
                    .child_joint_indices
                    .push(*joint_index);
            }
        }
    }

    for child in node.children() {
        process_node(
            gltf,
            buffers,
            images,
            &child,
            gltf_model,
            &cumulative_transform,
            Some(node.index()),
        )?;
    }

    Ok(())
}

fn load_white_texture_if_none(gltf_model: &mut GltfModel) {
    for gltf_data in &mut gltf_model.gltf_data {
        if gltf_data.image_data.is_empty() {
            if let Ok(white_texture) = load_white_texture() {
                gltf_data.image_data.push(white_texture);
            }
        }
    }
}

fn load_white_texture() -> Result<ImageData> {
    let width = 1u32;
    let height = 1u32;
    let white_pixel: Vec<u8> = vec![255, 255, 255, 255];

    Ok(ImageData {
        data: white_pixel,
        size: 4,
        width,
        height,
    })
}

unsafe fn input_joint_vertex(gltf_model: &mut GltfModel) {
    for (i, gltf_data) in &mut gltf_model.gltf_data.iter().enumerate() {
        for (vertex_index, vertex) in gltf_data.vertices.iter().enumerate() {
            for joint_index in vertex.joint_indices {
                if (joint_index as usize) < gltf_model.joints.len() {
                    let joint_vertex_index = JointVertexIndex {
                        gltf_data_index: i,
                        vertex_index,
                    };
                    if !gltf_model.joints[joint_index as usize]
                        .vertex_indices
                        .contains(&joint_vertex_index)
                    {
                        gltf_model.joints[joint_index as usize]
                            .vertex_indices
                            .push(joint_vertex_index);
                    }
                }
            }
        }
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
    use gltf::animation::util::ReadOutputs;

    let gltf_data_index = gltf_model.gltf_data.len().saturating_sub(1);
    let gltf_data = if gltf_model.gltf_data.is_empty() {
        return Ok(());
    } else {
        &gltf_model.gltf_data[gltf_data_index]
    };

    for channel in animation.channels() {
        let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
        let key_frames: Vec<f32> = reader.read_inputs().unwrap().collect();

        let mut joint_translations: Vec<Mat4> = Vec::new();
        let mut joint_rotations: Vec<Mat4> = Vec::new();
        let mut joint_rotation_quats: Vec<cgmath::Quaternion<f32>> = Vec::new();
        let mut joint_scales: Vec<Mat4> = Vec::new();
        let mut node_translations: Vec<Vector3<f32>> = Vec::new();

        if let Some(outputs) = reader.read_outputs() {
            match outputs {
                ReadOutputs::Translations(translations) => {
                    for translation in translations {
                        let matrix = cgmath::Matrix4::from_translation(Vector3::new(
                            translation[0],
                            translation[1],
                            translation[2],
                        ));
                        joint_translations.push(matrix);
                        node_translations.push(Vector3::new(
                            translation[0],
                            translation[1],
                            translation[2],
                        ));
                    }
                }
                ReadOutputs::Rotations(rotations) => {
                    for rotation in rotations.into_f32() {
                        let quat = cgmath::Quaternion::new(
                            rotation[3],
                            rotation[0],
                            rotation[1],
                            rotation[2],
                        );
                        joint_rotation_quats.push(quat);
                        let matrix = cgmath::Matrix4::from(quat);
                        joint_rotations.push(matrix);
                    }
                }
                ReadOutputs::Scales(scales) => {
                    for scale in scales {
                        let matrix =
                            cgmath::Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
                        joint_scales.push(matrix);
                    }
                }
                ReadOutputs::MorphTargetWeights(morph_target_weights) => {
                    let morph_target_length = gltf_data.morph_targets.len();
                    let mut weight = Vec::new();
                    let mut weights = Vec::new();

                    for morph_target_weight in morph_target_weights.into_f32() {
                        weight.push(morph_target_weight);
                        if weight.len() >= morph_target_length {
                            weights.push(weight.clone());
                            weight.clear();
                        }
                    }

                    for (i, weight_set) in weights.iter().enumerate() {
                        if i < key_frames.len() {
                            let mut morph_animation = MorphAnimation::new();
                            morph_animation.key_frame = key_frames[i];
                            morph_animation.weights = weight_set.clone();
                            gltf_model.morph_animations.push(morph_animation);
                        }
                    }
                }
            }
        }

        let target = channel.target();
        let node = target.node();

        let is_joint_node = gltf_model
            .node_joint_map
            .node_to_joint
            .contains_key(&(node.index() as u16));

        if is_joint_node && gltf_model.has_skinned_meshes {
            let joint_id = gltf_model
                .node_joint_map
                .node_to_joint
                .get(&(node.index() as u16))
                .unwrap();

            let mut joint_animation = JointAnimation::default();
            joint_animation.key_frames = key_frames.clone();
            joint_animation.translations = joint_translations.clone();
            joint_animation.rotations = joint_rotations.clone();
            joint_animation.scales = joint_scales.clone();
            gltf_model.joint_animations[*joint_id as usize].push(joint_animation);
        } else {
            let existing = gltf_model
                .node_animations
                .iter_mut()
                .find(|na| na.node_index == node.index());

            let node_animation = if let Some(na) = existing {
                na
            } else {
                let mut na = NodeAnimation::default();
                na.node_index = node.index();
                let (default_trans, default_rot, default_scale) =
                    decompose(&mat4_from_array(node.transform().matrix()));
                na.default_translation = default_trans;
                na.default_rotation = default_rot;
                na.default_scale = default_scale;
                gltf_model.node_animations.push(na);
                gltf_model.node_animations.last_mut().unwrap()
            };

            if !node_translations.is_empty() {
                for (i, &kf) in key_frames.iter().enumerate() {
                    if i < node_translations.len() {
                        node_animation.translation_keyframes.push(kf);
                        node_animation.translations.push(node_translations[i]);
                    }
                }
            }

            if !joint_rotation_quats.is_empty() {
                for (i, &kf) in key_frames.iter().enumerate() {
                    if i < joint_rotation_quats.len() {
                        node_animation.rotation_keyframes.push(kf);
                        node_animation.rotations.push(joint_rotation_quats[i]);
                    }
                }
            }

            if !joint_scales.is_empty() {
                for (i, &kf) in key_frames.iter().enumerate() {
                    if i < joint_scales.len() {
                        node_animation.scale_keyframes.push(kf);
                        let mat = joint_scales[i];
                        let scale = Vector3::new(mat[0][0], mat[1][1], mat[2][2]);
                        node_animation.scales.push(scale);
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_default() {
        let vertex = Vertex::default();
        assert_eq!(vertex.position, [0.0, 0.0, 0.0]);
        assert_eq!(vertex.normal, [0.0, 0.0, 0.0]);
        assert_eq!(vertex.tex_coord, [0.0, 0.0]);
        assert_eq!(vertex.joint_indices, [0, 0, 0, 0]);
        assert_eq!(vertex.joint_weights, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_gltf_data_default() {
        let data = GltfData::default();
        assert_eq!(data.vertices.len(), 0);
        assert_eq!(data.indices.len(), 0);
        assert_eq!(data.morph_targets.len(), 0);
        assert_eq!(data.has_joints, false);
    }

    #[test]
    fn test_morph_target_default() {
        let morph = MorphTarget::default();
        assert_eq!(morph.positions.len(), 0);
        assert_eq!(morph.normals.len(), 0);
        assert_eq!(morph.tangents.len(), 0);
    }
}
