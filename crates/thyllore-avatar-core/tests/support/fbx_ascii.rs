#![cfg(test)]
use std::fmt::Write;

use cgmath::{InnerSpace, Matrix3, Matrix4, Quaternion, SquareMatrix, Vector3};

use super::rig_convention::{ContainerNode, RigConvention};
use super::rig_nodes::build_rig_nodes;

fn is_armature_container(convention: &RigConvention, node_index: usize) -> bool {
    node_index == 0 && matches!(convention.container, ContainerNode::Armature { .. })
}

const BONE_MODEL_BASE: i32 = 1000;
const ATTRIBUTE_BASE: i32 = 11000;
const MESH_MODEL_ID: i32 = 20000;
const GEOMETRY_ID: i32 = 20001;
const SKIN_ID: i32 = 20002;
const CLUSTER_BASE: i32 = 21000;
const BIND_POSE_ID: i32 = 30000;
const MATERIAL_ID: i32 = 40000;

fn bone_model_id(i: usize) -> i32 {
    BONE_MODEL_BASE + i as i32
}

fn attribute_id(i: usize) -> i32 {
    ATTRIBUTE_BASE + i as i32
}

fn cluster_id(ci: usize) -> i32 {
    CLUSTER_BASE + ci as i32
}

pub struct RigMesh {
    pub vertices: Vec<[f64; 3]>,
    pub polygon_vertex_index: Vec<i32>,
    pub cluster_vertex_indices: Vec<Vec<i32>>,
    pub cluster_nodes: Vec<usize>,
    pub diffuse_color: Option<[f64; 3]>,
    pub cloth: Option<ClothMaterial>,
}

pub struct ClothMaterial {
    pub color: [f64; 3],
    pub first_polygon: usize,
}

pub fn write_rig_fbx(convention: &RigConvention) -> String {
    let nodes = build_rig_nodes(convention);
    let mesh = build_box_mesh(convention, &nodes);
    write_rig_fbx_with_mesh(convention, &nodes, &mesh)
}

fn fbx_header(out: &mut String, convention: &RigConvention) {
    out.push_str("FBXHeaderExtension:  {\n");
    out.push_str("  FBXVersion: 7400\n");
    out.push_str("}\n");

    out.push_str("GlobalSettings:  {\n");
    out.push_str("  Properties70:  {\n");
    write!(
        out,
        "    P: \"UpAxis\", \"int\", \"Integer\", \"\",{}\n",
        convention.up_axis
    )
    .unwrap();
    out.push_str("    P: \"UpAxisSign\", \"int\", \"Integer\", \"\",1\n");
    write!(
        out,
        "    P: \"UnitScaleFactor\", \"double\", \"Number\", \"\",{}\n",
        fmt(convention.unit_scale_factor)
    )
    .unwrap();
    write!(
        out,
        "    P: \"OriginalUnitScaleFactor\", \"double\", \"Number\", \"\",{}\n",
        fmt(convention.unit_scale_factor)
    )
    .unwrap();
    out.push_str("  }\n");
    out.push_str("}\n");
}

fn definitions(out: &mut String, mesh: &RigMesh) {
    let has_materials = mesh.diffuse_color.is_some() || mesh.cloth.is_some();
    let count = if has_materials { 7 } else { 6 };
    out.push_str("Definitions:  {\n");
    write!(out, "  Count: {}\n", count).unwrap();
    out.push_str("  Version: 100\n");
    out.push_str("  ObjectType: \"GlobalSettings\"\n");
    out.push_str("  ObjectType: \"Model\"\n");
    out.push_str("  ObjectType: \"NodeAttribute\"\n");
    out.push_str("  ObjectType: \"Geometry\"\n");
    out.push_str("  ObjectType: \"Deformer\"\n");
    out.push_str("  ObjectType: \"Pose\"\n");
    if has_materials {
        out.push_str("  ObjectType: \"Material\"\n");
    }
    out.push_str("}\n");
}

pub fn build_box_mesh(convention: &RigConvention, nodes: &[super::rig_nodes::RigNode]) -> RigMesh {
    let bones = super::canonical_bones();
    let positions = super::rig_positions::file_positions(convention);
    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let mut polygon_vertex_index: Vec<i32> = Vec::new();
    let mut cluster_vertex_indices: Vec<Vec<i32>> = Vec::new();
    let mut vertex_offset = 0;

    for (bi, _bone) in bones.iter().enumerate() {
        let child_idx = super::rig_nodes::first_child_bone_index(&bones, bi);
        let dir = super::rig_nodes::bone_direction(&positions, bi, child_idx);
        let len = match child_idx {
            Some(ci) => {
                let p: Vector3<f64> = [positions[bi][0], positions[bi][1], positions[bi][2]].into();
                let c: Vector3<f64> = [positions[ci][0], positions[ci][1], positions[ci][2]].into();
                (c - p).magnitude()
            }
            None => 0.1 * convention.body_scale / (convention.unit_scale_factor / 100.0),
        };
        let half = 0.02 * convention.body_scale / (convention.unit_scale_factor / 100.0);

        let p: Vector3<f64> = [positions[bi][0], positions[bi][1], positions[bi][2]].into();
        let verts = cube_vertices(p, dir, len, half);
        for v in &verts {
            vertices.push(*v);
        }

        let base = vertex_offset as i32;
        polygon_vertex_index.extend_from_slice(&stick_polygon_indices(base));
        vertex_offset += 8;
    }

    let role_nodes: Vec<(usize, &super::rig_nodes::RigNode)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.role.is_some())
        .collect();

    for (ci, (_node_idx, _)) in role_nodes.iter().enumerate() {
        let start = ci * 8;
        let end = (ci + 1) * 8;
        cluster_vertex_indices.push((start as i32..end as i32).collect());
    }

    let cluster_nodes: Vec<usize> = role_nodes.iter().map(|(idx, _)| *idx).collect();

    RigMesh {
        vertices,
        polygon_vertex_index,
        cluster_vertex_indices,
        cluster_nodes,
        diffuse_color: None,
        cloth: None,
    }
}

pub fn write_rig_fbx_with_mesh(
    convention: &RigConvention,
    nodes: &[super::rig_nodes::RigNode],
    mesh: &RigMesh,
) -> String {
    let mut out = String::new();

    out.push_str("; FBX 7.4.0 project file\n");

    fbx_header(&mut out, convention);

    definitions(&mut out, mesh);

    objects(&mut out, convention, nodes, mesh);

    connections(&mut out, convention, nodes, mesh);

    out
}

fn objects(
    out: &mut String,
    convention: &RigConvention,
    nodes: &[super::rig_nodes::RigNode],
    mesh: &RigMesh,
) {
    out.push_str("Objects:  {\n");

    for (i, node) in nodes.iter().enumerate() {
        let is_container = is_armature_container(convention, i);
        let model_type = if is_container { "Null" } else { "LimbNode" };

        write!(
            out,
            "  Model: {}, \"Model::{}\", \"{}\" {{\n",
            bone_model_id(i),
            node.name,
            model_type
        )
        .unwrap();

        let parent_world_inv = parent_world_inverse(nodes, i);
        let local_transform = parent_world_inv * world_matrix(node);
        let (local_pos, local_rot) = decompose(&local_transform);
        let euler = matrix_to_euler_degrees(rotation_matrix(&local_rot));

        out.push_str("    Properties70:  {\n");
        write!(
            out,
            "      P: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",{},{},{}\n",
            fmt(local_pos[0]),
            fmt(local_pos[1]),
            fmt(local_pos[2])
        )
        .unwrap();
        if convention.uses_pre_rotation {
            out.push_str("      P: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",0,0,0\n");
            write!(
                out,
                "      P: \"PreRotation\", \"Vector3D\", \"Vector\", \"\",{},{},{}\n",
                fmt(euler[0]),
                fmt(euler[1]),
                fmt(euler[2])
            )
            .unwrap();
            out.push_str("      P: \"RotationActive\", \"bool\", \"\", \"\",1\n");
        } else {
            write!(
                out,
                "      P: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",{},{},{}\n",
                fmt(euler[0]),
                fmt(euler[1]),
                fmt(euler[2])
            )
            .unwrap();
        }

        out.push_str("    }\n");
        out.push_str("  }\n");

        if !is_container {
            write!(
                out,
                "  NodeAttribute: {}, \"NodeAttribute::\", \"LimbNode\" {{\n",
                attribute_id(i)
            )
            .unwrap();
            out.push_str("    TypeFlags: \"Skeleton\"\n");
            out.push_str("  }\n");
        }
    }

    if let Some(color) = mesh.diffuse_color {
        write!(
            out,
            "  Material: {}, \"Material::White\", \"\" {{\n",
            MATERIAL_ID
        )
        .unwrap();
        out.push_str("    Version: 102\n");
        out.push_str("    ShadingModel: \"lambert\"\n");
        out.push_str("    Properties70:  {\n");
        write!(
            out,
            "      P: \"DiffuseColor\", \"Color\", \"\", \"A\",{},{},{}\n",
            fmt(color[0]),
            fmt(color[1]),
            fmt(color[2])
        )
        .unwrap();
        out.push_str("    }\n");
        out.push_str("  }\n");
    }

    if let Some(cloth) = &mesh.cloth {
        write!(
            out,
            "  Material: {}, \"Material::Cloth\", \"\" {{\n",
            MATERIAL_ID + 1
        )
        .unwrap();
        out.push_str("    Version: 102\n");
        out.push_str("    ShadingModel: \"lambert\"\n");
        out.push_str("    Properties70:  {\n");
        write!(
            out,
            "      P: \"DiffuseColor\", \"Color\", \"\", \"A\",{},{},{}\n",
            fmt(cloth.color[0]),
            fmt(cloth.color[1]),
            fmt(cloth.color[2])
        )
        .unwrap();
        out.push_str("    }\n");
        out.push_str("  }\n");
    }

    write_mesh_and_skin(out, convention, nodes, mesh);

    out.push_str("}\n");
}

fn connections(
    out: &mut String,
    convention: &RigConvention,
    nodes: &[super::rig_nodes::RigNode],
    mesh: &RigMesh,
) {
    out.push_str("Connections:  {\n");

    for (i, node) in nodes.iter().enumerate() {
        let is_container = is_armature_container(convention, i);
        if !is_container {
            write!(
                out,
                "  C: \"OO\",{},{}\n",
                attribute_id(i),
                bone_model_id(i)
            )
            .unwrap();
        }

        match node.parent {
            Some(parent_idx) => {
                write!(
                    out,
                    "  C: \"OO\",{},{}\n",
                    bone_model_id(i),
                    bone_model_id(parent_idx)
                )
                .unwrap();
            }
            None => {
                write!(out, "  C: \"OO\",{},0\n", bone_model_id(i)).unwrap();
            }
        }
    }

    write!(out, "  C: \"OO\",{},0\n", MESH_MODEL_ID).unwrap();
    write!(out, "  C: \"OO\",{},{}\n", GEOMETRY_ID, MESH_MODEL_ID).unwrap();
    write!(out, "  C: \"OO\",{},{}\n", SKIN_ID, GEOMETRY_ID).unwrap();

    for (ci, &bone_node_idx) in mesh.cluster_nodes.iter().enumerate() {
        let cluster_id = cluster_id(ci);
        let bone_model_id = bone_model_id(bone_node_idx);
        write!(out, "  C: \"OO\",{},{}\n", cluster_id, SKIN_ID).unwrap();
        write!(out, "  C: \"OO\",{},{}\n", bone_model_id, cluster_id).unwrap();
    }

    if mesh.diffuse_color.is_some() {
        write!(out, "  C: \"OO\",{},{}\n", MATERIAL_ID, MESH_MODEL_ID).unwrap();
    }

    if mesh.cloth.is_some() {
        write!(out, "  C: \"OO\",{},{}\n", MATERIAL_ID + 1, MESH_MODEL_ID).unwrap();
    }

    out.push_str("}\n");
}

fn write_mesh_and_skin(
    out: &mut String,
    _convention: &RigConvention,
    nodes: &[super::rig_nodes::RigNode],
    mesh: &RigMesh,
) {
    write!(
        out,
        "  Model: {}, \"Model::Body\", \"Mesh\" {{\n",
        MESH_MODEL_ID
    )
    .unwrap();
    out.push_str("    Properties70:  {\n");
    out.push_str("      P: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",0,0,0\n");
    out.push_str("      P: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",0,0,0\n");
    out.push_str("    }\n");
    out.push_str("  }\n");

    let vert_count = mesh.vertices.len();
    write!(
        out,
        "  Geometry: {}, \"Geometry::Body\", \"Mesh\" {{\n",
        GEOMETRY_ID
    )
    .unwrap();
    out.push_str("    Vertices: *");
    write!(out, "{}", 3 * vert_count).unwrap();
    out.push_str(" { a: ");
    for (i, v) in mesh.vertices.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write!(out, "{},{},{}", fmt(v[0]), fmt(v[1]), fmt(v[2])).unwrap();
    }
    out.push_str(" }\n");

    out.push_str("    PolygonVertexIndex: *");
    write!(out, "{}", mesh.polygon_vertex_index.len()).unwrap();
    out.push_str(" { a: ");
    for (i, v) in mesh.polygon_vertex_index.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write!(out, "{}", v).unwrap();
    }
    out.push_str(" }\n");

    if mesh.diffuse_color.is_some() {
        out.push_str("    LayerElementMaterial: 0 {\n");
        out.push_str("      Version: 101\n");
        if let Some(cloth) = &mesh.cloth {
            let mut polygon_count = 0;
            for &v in &mesh.polygon_vertex_index {
                if v < 0 {
                    polygon_count += 1;
                }
            }
            out.push_str("      MappingInformationType: \"ByPolygon\"\n");
            out.push_str("      ReferenceInformationType: \"IndexToDirect\"\n");
            write!(out, "      Materials: *{} {{ a: ", polygon_count).unwrap();
            for pi in 0..polygon_count {
                if pi > 0 {
                    out.push(',');
                }
                let mat_idx = if pi < cloth.first_polygon { 0 } else { 1 };
                write!(out, "{}", mat_idx).unwrap();
            }
            out.push_str(" }\n");
        } else {
            out.push_str("      MappingInformationType: \"AllSame\"\n");
            out.push_str("      ReferenceInformationType: \"IndexToDirect\"\n");
            out.push_str("      Materials: *1 { a: 0 }\n");
        }
        out.push_str("    }\n");
        out.push_str("    Layer: 0 {\n");
        out.push_str("      Version: 100\n");
        out.push_str("      LayerElement:  {\n");
        out.push_str("        Type: \"LayerElementMaterial\"\n");
        out.push_str("        TypedIndex: 0\n");
        out.push_str("      }\n");
        out.push_str("    }\n");
    }

    out.push_str("  }\n");

    write_skin_and_clusters(out, nodes, mesh);

    write_bind_pose(out, nodes, mesh);
}

fn world_matrix(node: &super::rig_nodes::RigNode) -> Matrix4<f64> {
    let pos = Vector3::new(
        node.world_position.x,
        node.world_position.y,
        node.world_position.z,
    );
    let q = node.world_rotation;
    let rot: Matrix3<f64> = Matrix3::from(q);
    let mut m = Matrix4::identity();
    for r in 0..3 {
        for c in 0..3 {
            m[r][c] = rot[r][c];
        }
    }
    m[3][0] = pos.x;
    m[3][1] = pos.y;
    m[3][2] = pos.z;
    m
}

fn parent_world_inverse(nodes: &[super::rig_nodes::RigNode], index: usize) -> Matrix4<f64> {
    match nodes[index].parent {
        Some(parent_idx) => world_matrix(&nodes[parent_idx]).invert().unwrap(),
        None => Matrix4::identity(),
    }
}

fn decompose(m: &Matrix4<f64>) -> ([f64; 3], Quaternion<f64>) {
    let translation = [m[3][0], m[3][1], m[3][2]];
    let rot_matrix = Matrix3::new(
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2],
    );
    let q = Quaternion::from(rot_matrix);
    (translation, q)
}

fn rotation_matrix(q: &Quaternion<f64>) -> Matrix3<f64> {
    Matrix3::from(*q)
}

fn matrix_to_euler_degrees(m: Matrix3<f64>) -> [f64; 3] {
    fn r(m: Matrix3<f64>, row: usize, col: usize) -> f64 {
        m[col][row] as f64
    }

    let sy = (r(m, 0, 0) * r(m, 0, 0) + r(m, 1, 0) * r(m, 1, 0)).sqrt();
    let epsilon = 1e-12;

    if sy > epsilon {
        let x = r(m, 2, 1).atan2(r(m, 2, 2));
        let y = (-r(m, 2, 0)).atan2(sy);
        let z = r(m, 1, 0).atan2(r(m, 0, 0));
        [x.to_degrees(), y.to_degrees(), z.to_degrees()]
    } else {
        let x = (-r(m, 1, 2)).atan2(r(m, 1, 1));
        let y = (-r(m, 2, 0)).atan2(sy);
        let z = 0.0;
        [x.to_degrees(), y.to_degrees(), z]
    }
}

fn fmt(v: f64) -> String {
    format!("{:.9}", v)
}

fn matrix4_flat(m: &Matrix4<f64>) -> Vec<f64> {
    let mut flat = Vec::with_capacity(16);
    for c in 0..4 {
        for r in 0..4 {
            flat.push(m[c][r]);
        }
    }
    flat
}

pub fn cube_vertices(
    center: Vector3<f64>,
    dir: Vector3<f64>,
    length: f64,
    half: f64,
) -> [[f64; 3]; 8] {
    let axis = perpendicular_to(dir);
    let perp = dir.cross(axis).normalize();

    let tip = center + dir * length;
    let base = [center, center, center, center];
    let top = [tip, tip, tip, tip];

    [
        [
            base[0][0] - axis.x * half,
            base[0][1] - axis.y * half,
            base[0][2] - axis.z * half,
        ],
        [
            base[1][0] + axis.x * half,
            base[1][1] + axis.y * half,
            base[1][2] + axis.z * half,
        ],
        [
            base[2][0] - perp.x * half,
            base[2][1] - perp.y * half,
            base[2][2] - perp.z * half,
        ],
        [
            base[3][0] + perp.x * half,
            base[3][1] + perp.y * half,
            base[3][2] + perp.z * half,
        ],
        [
            top[0][0] - axis.x * half,
            top[0][1] - axis.y * half,
            top[0][2] - axis.z * half,
        ],
        [
            top[1][0] + axis.x * half,
            top[1][1] + axis.y * half,
            top[1][2] + axis.z * half,
        ],
        [
            top[2][0] - perp.x * half,
            top[2][1] - perp.y * half,
            top[2][2] - perp.z * half,
        ],
        [
            top[3][0] + perp.x * half,
            top[3][1] + perp.y * half,
            top[3][2] + perp.z * half,
        ],
    ]
}

fn perpendicular_to(v: Vector3<f64>) -> Vector3<f64> {
    let ax = v.x.abs();
    let ay = v.y.abs();
    let az = v.z.abs();
    if ax < ay && ax < az {
        Vector3::new(1.0, 0.0, 0.0).cross(v).normalize()
    } else if ay < az {
        Vector3::new(0.0, 1.0, 0.0).cross(v).normalize()
    } else {
        Vector3::new(0.0, 0.0, 1.0).cross(v).normalize()
    }
}

pub fn stick_polygon_indices(base: i32) -> [i32; 24] {
    [
        base,
        base + 1,
        base + 3,
        -(base + 2 + 1),
        base + 4,
        base + 5,
        base + 7,
        -(base + 6 + 1),
        base,
        base + 4,
        base + 5,
        -(base + 1 + 1),
        base + 1,
        base + 2,
        base + 6,
        -(base + 5 + 1),
        base + 2,
        base + 3,
        base + 7,
        -(base + 6 + 1),
        base + 3,
        base + 0,
        base + 4,
        -(base + 7 + 1),
    ]
}

fn write_bind_pose(out: &mut String, nodes: &[super::rig_nodes::RigNode], mesh: &RigMesh) {
    let bone_count = mesh.cluster_nodes.len();

    let pose_node_count = bone_count + 1;
    write!(
        out,
        "  Pose: {}, \"Pose::BindPose\", \"BindPose\" {{\n",
        BIND_POSE_ID
    )
    .unwrap();
    out.push_str("    Type: \"BindPose\"\n");
    out.push_str("    Version: 100\n");
    write!(out, "    NbPoseNodes: {}\n", pose_node_count).unwrap();

    let mesh_world = Matrix4::identity();
    let flat_mesh: Vec<f64> = matrix4_flat(&mesh_world);
    out.push_str("    PoseNode: {\n");
    write!(out, "      Node: {}\n", MESH_MODEL_ID).unwrap();
    out.push_str("      Matrix: *16 { a: ");
    for (i, v) in flat_mesh.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write!(out, "{}", fmt(*v)).unwrap();
    }
    out.push_str(" }\n");
    out.push_str("    }\n");

    for &bone_node_idx in &mesh.cluster_nodes {
        let bone_node = &nodes[bone_node_idx];
        let world_mat = world_matrix(bone_node);
        let flat: Vec<f64> = matrix4_flat(&world_mat);
        out.push_str("    PoseNode: {\n");
        write!(out, "      Node: {}\n", bone_model_id(bone_node_idx)).unwrap();
        out.push_str("      Matrix: *16 { a: ");
        for (i, v) in flat.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write!(out, "{}", fmt(*v)).unwrap();
        }
        out.push_str(" }\n");
        out.push_str("    }\n");
    }

    out.push_str("  }\n");
}

fn write_skin_and_clusters(out: &mut String, nodes: &[super::rig_nodes::RigNode], mesh: &RigMesh) {
    write!(
        out,
        "  Deformer: {}, \"Deformer::Skin\", \"Skin\" {{\n",
        SKIN_ID
    )
    .unwrap();
    out.push_str("    Version: 101\n");
    out.push_str("  }\n");

    for (ci, &bone_node_idx) in mesh.cluster_nodes.iter().enumerate() {
        let bone_node = &nodes[bone_node_idx];
        let cluster_id = cluster_id(ci);
        let bone_name = &bone_node.name;
        write!(
            out,
            "  Deformer: {}, \"SubDeformer::{}\", \"Cluster\" {{\n",
            cluster_id, bone_name
        )
        .unwrap();
        out.push_str("    Version: 100\n");

        let indices = &mesh.cluster_vertex_indices[ci];
        write!(out, "    Indexes: *{} {{ a: ", indices.len()).unwrap();
        for (i, v) in indices.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write!(out, "{}", v).unwrap();
        }
        out.push_str(" }\n");

        write!(out, "    Weights: *{} {{ a: ", indices.len()).unwrap();
        for i in 0..indices.len() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("1");
        }
        out.push_str(" }\n");

        let world_mat = world_matrix(bone_node);
        let flat: Vec<f64> = matrix4_flat(&world_mat);
        out.push_str("    TransformLink: *16 { a: ");
        for (i, v) in flat.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write!(out, "{}", fmt(*v)).unwrap();
        }
        out.push_str(" }\n");

        let mesh_to_bone = world_mat.invert().unwrap();
        let flat_mesh_to_bone: Vec<f64> = matrix4_flat(&mesh_to_bone);
        out.push_str("    Transform: *16 { a: ");
        for (i, v) in flat_mesh_to_bone.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write!(out, "{}", fmt(*v)).unwrap();
        }
        out.push_str(" }\n");

        out.push_str("  }\n");
    }
}
