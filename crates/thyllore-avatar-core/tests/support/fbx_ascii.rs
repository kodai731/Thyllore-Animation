use std::fmt::Write;

use cgmath::{Matrix3, Matrix4, Quaternion, SquareMatrix, Vector3};

use super::rig_convention::{ContainerNode, RigConvention};
use super::rig_nodes::build_rig_nodes;

fn is_armature_container(convention: &RigConvention, node_index: usize) -> bool {
    node_index == 0 && matches!(convention.container, ContainerNode::Armature { .. })
}

pub fn write_skeleton_fbx(convention: &RigConvention) -> String {
    let nodes = build_rig_nodes(convention);
    let mut out = String::new();

    out.push_str("; FBX 7.4.0 project file\n");

    fbx_header(&mut out, convention);

    definitions(&mut out, convention, &nodes);

    objects(&mut out, convention, &nodes);

    connections(&mut out, &nodes);

    out
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

fn definitions(
    out: &mut String,
    _convention: &RigConvention,
    _nodes: &[super::rig_nodes::RigNode],
) {
    out.push_str("Definitions:  {\n");
    out.push_str("  Count: 3\n");
    out.push_str("  Version: 100\n");
    out.push_str("  ObjectType: \"GlobalSettings\"\n");
    out.push_str("  ObjectType: \"Model\"\n");
    out.push_str("  ObjectType: \"NodeAttribute\"\n");
    out.push_str("}\n");
}

fn objects(out: &mut String, convention: &RigConvention, nodes: &[super::rig_nodes::RigNode]) {
    out.push_str("Objects:  {\n");

    let mut id = 1000;
    for (i, node) in nodes.iter().enumerate() {
        let is_container = is_armature_container(convention, i);
        let model_type = if is_container { "Null" } else { "LimbNode" };

        write!(
            out,
            "  Model: {}, \"Model::{}\", \"{}\" {{\n",
            id, node.name, model_type
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
                id + 10000
            )
            .unwrap();
            out.push_str("    TypeFlags: \"Skeleton\"\n");
            out.push_str("  }\n");
        }

        id += 1;
    }

    out.push_str("}\n");
}

fn connections(out: &mut String, nodes: &[super::rig_nodes::RigNode]) {
    out.push_str("Connections:  {\n");

    let mut id = 1000;
    for (i, node) in nodes.iter().enumerate() {
        let is_container = i == 0;
        if !is_container {
            write!(out, "  C: \"OO\",{},{}\n", id + 10000, id).unwrap();
        }

        match node.parent {
            Some(parent_idx) => {
                let parent_id = 1000 + parent_idx;
                write!(out, "  C: \"OO\",{},{}\n", id, parent_id).unwrap();
            }
            None => {
                write!(out, "  C: \"OO\",{},0\n", id).unwrap();
            }
        }

        id += 1;
    }

    out.push_str("}\n");
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
