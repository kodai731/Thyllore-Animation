use std::io::{Seek, Write};
use std::path::Path;

use cgmath::Matrix4;
use fbxcel::low::FbxVersion;
use fbxcel::writer::v7400::binary::{FbxFooter, Writer};

use crate::animation::Skeleton;
use crate::animation::editable::EditableAnimationClip;
use crate::loader::fbx::fbx::{FbxData, FbxModel};

use super::fbx_animation::{
    FbxBoneExport, FbxChannel, FbxConnection, FbxCurveExport, FbxCurveNodeExport, FbxExportData,
    FbxWriteResult, UidAllocator, build_bone_export_list, build_channel_exports,
    decompose_matrix_to_trs, seconds_to_ktime, write_anim_curve, write_anim_curve_node,
    write_anim_layer, write_anim_stack, write_bone_model, write_connections,
    write_documents, write_global_settings, write_header_extension, write_node_attribute,
    write_object_type, write_property_f64, write_property_f64x3, write_property_i32,
    write_references,
};

struct FbxGeometryExport {
    uid: i64,
    mesh_model_uid: i64,
    positions: Vec<f64>,
    polygon_vertex_index: Vec<i32>,
    normals: Vec<f64>,
    uv_values: Vec<f64>,
    uv_indices: Vec<i32>,
}

struct FbxMeshModelExport {
    uid: i64,
    name: String,
    parent_bone_uid: Option<i64>,
    translation: [f64; 3],
    rotation: [f64; 3],
    scaling: [f64; 3],
}

struct FbxMaterialExport {
    uid: i64,
    name: String,
    mesh_model_uid: i64,
    diffuse_color: [f64; 3],
}

struct FbxTextureExport {
    texture_uid: i64,
    video_uid: i64,
    material_uid: i64,
    filename: String,
}

struct FbxSkinExport {
    skin_uid: i64,
    geometry_uid: i64,
    clusters: Vec<FbxClusterExport>,
}

struct FbxClusterExport {
    uid: i64,
    bone_model_uid: i64,
    indices: Vec<i32>,
    weights: Vec<f64>,
    transform: [f64; 16],
    transform_link: [f64; 16],
}

struct FullFbxExportData {
    anim_data: FbxExportData,
    geometries: Vec<FbxGeometryExport>,
    mesh_models: Vec<FbxMeshModelExport>,
    materials: Vec<FbxMaterialExport>,
    textures: Vec<FbxTextureExport>,
    skins: Vec<FbxSkinExport>,
}

pub fn export_full_fbx(
    fbx_model: &FbxModel,
    clip: Option<&EditableAnimationClip>,
    skeleton: &Skeleton,
    path: &Path,
) -> anyhow::Result<()> {
    let export_data = build_full_export_data(fbx_model, clip, skeleton)?;

    let file = std::fs::File::create(path)?;
    let writer = Writer::new(file, FbxVersion::V7_4)
        .map_err(|e| anyhow::anyhow!("FBX writer init failed: {}", e))?;

    write_full_fbx_binary(writer, &export_data)
        .map_err(|e| anyhow::anyhow!("FBX write failed: {}", e))?;

    Ok(())
}

fn build_full_export_data(
    fbx_model: &FbxModel,
    clip: Option<&EditableAnimationClip>,
    skeleton: &Skeleton,
) -> anyhow::Result<FullFbxExportData> {
    let inv_unit_scale = 1.0_f32 / fbx_model.unit_scale;

    let mesh_node_names: std::collections::HashSet<String> = fbx_model
        .fbx_data
        .iter()
        .filter_map(|d| d.mesh_node_name.clone())
        .collect();

    let needs_coord_conversion = fbx_model.fbx_data.iter().any(|d| !d.clusters.is_empty());

    let mut uid_alloc = UidAllocator::new();
    let bones = build_bone_export_list(
        skeleton,
        &mut uid_alloc,
        &mesh_node_names,
        inv_unit_scale,
        needs_coord_conversion,
    );

    let stack_uid = uid_alloc.allocate();
    let layer_uid = uid_alloc.allocate();
    let document_uid = uid_alloc.allocate();

    let duration_ktime = clip
        .map(|c| seconds_to_ktime(c.duration))
        .unwrap_or_else(|| {
            fbx_model
                .animations
                .first()
                .map(|a| seconds_to_ktime(a.duration))
                .unwrap_or(0)
        });

    let clip_name = clip
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "DefaultAnimation".to_string());

    let mut name_to_model_uid: std::collections::HashMap<String, i64> = bones
        .iter()
        .map(|b| (b.name.clone(), b.model_uid))
        .collect();

    let geometries = build_geometry_exports(
        &fbx_model.fbx_data,
        &mut uid_alloc,
        inv_unit_scale,
    );

    let mesh_models = build_mesh_model_exports(
        &fbx_model.fbx_data,
        &geometries,
        &name_to_model_uid,
        &fbx_model.nodes,
        &mut uid_alloc,
        inv_unit_scale,
    );

    for mesh_model in &mesh_models {
        name_to_model_uid.insert(mesh_model.name.clone(), mesh_model.uid);
    }

    let (curve_nodes, curves) =
        build_animation_curves(clip, &name_to_model_uid, &mut uid_alloc, inv_unit_scale);

    let materials = build_material_exports(&fbx_model.fbx_data, &mesh_models, &mut uid_alloc);
    let textures = build_texture_exports(&fbx_model.fbx_data, &materials, &mut uid_alloc);

    let skins = build_skin_exports(
        &fbx_model.fbx_data,
        &geometries,
        &name_to_model_uid,
        &mut uid_alloc,
        inv_unit_scale,
    );

    let mut connections = Vec::new();
    generate_bone_connections(&bones, &mut connections);
    generate_mesh_connections(
        &mesh_models,
        &geometries,
        &materials,
        &textures,
        &skins,
        &mut connections,
    );
    generate_animation_connections(
        stack_uid,
        layer_uid,
        &curve_nodes,
        &mut connections,
    );

    let anim_data = FbxExportData {
        clip_name,
        duration_ktime,
        needs_coord_conversion,
        axes: fbx_model.axes.clone(),
        fps: fbx_model.fps,
        bones,
        stack_uid,
        layer_uid,
        document_uid,
        curve_nodes,
        curves,
        connections,
    };

    Ok(FullFbxExportData {
        anim_data,
        geometries,
        mesh_models,
        materials,
        textures,
        skins,
    })
}

fn build_animation_curves(
    clip: Option<&EditableAnimationClip>,
    bone_name_to_model_uid: &std::collections::HashMap<String, i64>,
    uid_alloc: &mut UidAllocator,
    inv_unit_scale: f32,
) -> (Vec<FbxCurveNodeExport>, Vec<FbxCurveExport>) {
    let mut curve_nodes = Vec::new();
    let mut curves = Vec::new();

    let Some(clip) = clip else {
        return (curve_nodes, curves);
    };

    for track in clip.tracks.values() {
        let bone_model_uid = match bone_name_to_model_uid.get(track.bone_name.as_str()) {
            Some(&uid) => uid,
            None => continue,
        };

        if let Some((node, node_curves)) = build_channel_exports(
            [
                &track.translation_x,
                &track.translation_y,
                &track.translation_z,
            ],
            bone_model_uid,
            FbxChannel::Translation,
            uid_alloc,
            inv_unit_scale,
        ) {
            curve_nodes.push(node);
            curves.extend(node_curves);
        }

        if let Some((node, node_curves)) = build_channel_exports(
            [&track.rotation_x, &track.rotation_y, &track.rotation_z],
            bone_model_uid,
            FbxChannel::Rotation,
            uid_alloc,
            inv_unit_scale,
        ) {
            curve_nodes.push(node);
            curves.extend(node_curves);
        }

        if let Some((node, node_curves)) = build_channel_exports(
            [&track.scale_x, &track.scale_y, &track.scale_z],
            bone_model_uid,
            FbxChannel::Scale,
            uid_alloc,
            inv_unit_scale,
        ) {
            curve_nodes.push(node);
            curves.extend(node_curves);
        }
    }

    (curve_nodes, curves)
}

fn build_geometry_exports(
    fbx_data_list: &[FbxData],
    uid_alloc: &mut UidAllocator,
    inv_unit_scale: f32,
) -> Vec<FbxGeometryExport> {
    let mut geometries = Vec::new();

    for fbx_data in fbx_data_list {
        let geometry_uid = uid_alloc.allocate();
        let mesh_model_uid = uid_alloc.allocate();

        let positions = convert_positions_to_fbx(fbx_data, inv_unit_scale);
        let polygon_vertex_index = encode_triangle_polygon_indices(&fbx_data.indices);
        let normals = convert_normals_to_fbx(fbx_data);
        let (uv_values, uv_indices) = convert_uvs_to_fbx(fbx_data);

        geometries.push(FbxGeometryExport {
            uid: geometry_uid,
            mesh_model_uid,
            positions,
            polygon_vertex_index,
            normals,
            uv_values,
            uv_indices,
        });
    }

    geometries
}

fn convert_positions_to_fbx(fbx_data: &FbxData, inv_unit_scale: f32) -> Vec<f64> {
    let source = if !fbx_data.local_positions.is_empty() {
        &fbx_data.local_positions
    } else {
        &fbx_data.positions
    };

    source
        .iter()
        .flat_map(|p| {
            [
                (p.x * inv_unit_scale) as f64,
                (p.y * inv_unit_scale) as f64,
                (p.z * inv_unit_scale) as f64,
            ]
        })
        .collect()
}

fn convert_normals_to_fbx(fbx_data: &FbxData) -> Vec<f64> {
    let source = if !fbx_data.local_normals.is_empty() {
        &fbx_data.local_normals
    } else {
        &fbx_data.normals
    };

    source
        .iter()
        .flat_map(|n| [n.x as f64, n.y as f64, n.z as f64])
        .collect()
}

fn convert_uvs_to_fbx(fbx_data: &FbxData) -> (Vec<f64>, Vec<i32>) {
    let uv_values: Vec<f64> = fbx_data
        .tex_coords
        .iter()
        .flat_map(|uv| [uv[0] as f64, (1.0 - uv[1]) as f64])
        .collect();

    let uv_indices: Vec<i32> = (0..fbx_data.tex_coords.len() as i32).collect();

    (uv_values, uv_indices)
}

fn encode_triangle_polygon_indices(indices: &[u32]) -> Vec<i32> {
    indices
        .chunks(3)
        .flat_map(|tri| {
            if tri.len() == 3 {
                vec![tri[0] as i32, tri[1] as i32, -(tri[2] as i32 + 1)]
            } else {
                tri.iter().map(|&i| i as i32).collect()
            }
        })
        .collect()
}

fn build_mesh_model_exports(
    fbx_data_list: &[FbxData],
    geometries: &[FbxGeometryExport],
    bone_name_to_model_uid: &std::collections::HashMap<String, i64>,
    nodes: &std::collections::HashMap<String, crate::loader::fbx::fbx::BoneNode>,
    uid_alloc: &mut UidAllocator,
    inv_unit_scale: f32,
) -> Vec<FbxMeshModelExport> {
    let mut mesh_models = Vec::new();
    let mut mesh_name_to_uid: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let scale = inv_unit_scale as f64;

    for (i, fbx_data) in fbx_data_list.iter().enumerate() {
        let uid = if i < geometries.len() {
            geometries[i].mesh_model_uid
        } else {
            uid_alloc.allocate()
        };

        let mesh_name = fbx_data
            .mesh_node_name
            .clone()
            .unwrap_or_else(|| format!("MeshModel_{}", i));

        mesh_name_to_uid.insert(mesh_name.clone(), uid);

        let (mut translation, rotation, scaling) = fbx_data
            .mesh_node_name
            .as_ref()
            .and_then(|name| nodes.get(name))
            .map(|node| decompose_matrix_to_trs(&node.local_transform))
            .unwrap_or(([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));

        translation[0] *= scale;
        translation[1] *= scale;
        translation[2] *= scale;

        mesh_models.push(FbxMeshModelExport {
            uid,
            name: mesh_name,
            parent_bone_uid: None,
            translation,
            rotation,
            scaling,
        });
    }

    resolve_mesh_parent_uids(
        &mut mesh_models,
        fbx_data_list,
        nodes,
        bone_name_to_model_uid,
        &mesh_name_to_uid,
    );

    mesh_models
}

fn resolve_mesh_parent_uids(
    mesh_models: &mut [FbxMeshModelExport],
    fbx_data_list: &[FbxData],
    nodes: &std::collections::HashMap<String, crate::loader::fbx::fbx::BoneNode>,
    bone_name_to_model_uid: &std::collections::HashMap<String, i64>,
    mesh_name_to_uid: &std::collections::HashMap<String, i64>,
) {
    for (i, fbx_data) in fbx_data_list.iter().enumerate() {
        let parent_uid = fbx_data
            .mesh_node_name
            .as_ref()
            .and_then(|name| nodes.get(name))
            .and_then(|node| node.parent.as_ref())
            .and_then(|parent| {
                bone_name_to_model_uid
                    .get(parent.as_str())
                    .or_else(|| mesh_name_to_uid.get(parent.as_str()))
                    .copied()
            });

        if i < mesh_models.len() {
            mesh_models[i].parent_bone_uid = parent_uid;
        }
    }
}

fn build_material_exports(
    fbx_data_list: &[FbxData],
    mesh_models: &[FbxMeshModelExport],
    uid_alloc: &mut UidAllocator,
) -> Vec<FbxMaterialExport> {
    let mut materials = Vec::new();

    for (i, fbx_data) in fbx_data_list.iter().enumerate() {
        let mat_uid = uid_alloc.allocate();
        let mat_name = fbx_data
            .material_name
            .clone()
            .unwrap_or_else(|| format!("Material_{}", i));

        let mesh_model_uid = if i < mesh_models.len() {
            mesh_models[i].uid
        } else {
            0
        };

        materials.push(FbxMaterialExport {
            uid: mat_uid,
            name: mat_name,
            mesh_model_uid,
            diffuse_color: [0.8, 0.8, 0.8],
        });
    }

    materials
}

fn build_texture_exports(
    fbx_data_list: &[FbxData],
    materials: &[FbxMaterialExport],
    uid_alloc: &mut UidAllocator,
) -> Vec<FbxTextureExport> {
    let mut textures = Vec::new();

    for (i, fbx_data) in fbx_data_list.iter().enumerate() {
        if let Some(ref tex_path) = fbx_data.diffuse_texture {
            let texture_uid = uid_alloc.allocate();
            let video_uid = uid_alloc.allocate();

            let material_uid = if i < materials.len() {
                materials[i].uid
            } else {
                0
            };

            textures.push(FbxTextureExport {
                texture_uid,
                video_uid,
                material_uid,
                filename: tex_path.clone(),
            });
        }
    }

    textures
}

fn build_skin_exports(
    fbx_data_list: &[FbxData],
    geometries: &[FbxGeometryExport],
    bone_name_to_model_uid: &std::collections::HashMap<String, i64>,
    uid_alloc: &mut UidAllocator,
    inv_unit_scale: f32,
) -> Vec<FbxSkinExport> {
    let mut skins = Vec::new();

    for (i, fbx_data) in fbx_data_list.iter().enumerate() {
        if fbx_data.clusters.is_empty() {
            continue;
        }

        let skin_uid = uid_alloc.allocate();
        let geometry_uid = if i < geometries.len() {
            geometries[i].uid
        } else {
            continue;
        };

        let clusters: Vec<FbxClusterExport> = fbx_data
            .clusters
            .iter()
            .filter_map(|cluster| {
                let bone_model_uid =
                    bone_name_to_model_uid.get(cluster.bone_name.as_str()).copied()?;

                let cluster_uid = uid_alloc.allocate();

                let indices: Vec<i32> =
                    cluster.vertex_indices.iter().map(|&i| i as i32).collect();
                let weights: Vec<f64> =
                    cluster.vertex_weights.iter().map(|&w| w as f64).collect();

                let transform = matrix4_to_flat_f64_scaled(
                    &cluster.inverse_bind_pose,
                    inv_unit_scale,
                );
                let transform_link = matrix4_to_flat_f64_scaled(
                    &cluster.transform_link,
                    inv_unit_scale,
                );

                Some(FbxClusterExport {
                    uid: cluster_uid,
                    bone_model_uid,
                    indices,
                    weights,
                    transform,
                    transform_link,
                })
            })
            .collect();

        if !clusters.is_empty() {
            skins.push(FbxSkinExport {
                skin_uid,
                geometry_uid,
                clusters,
            });
        }
    }

    skins
}

fn matrix4_to_flat_f64_scaled(m: &Matrix4<f32>, inv_unit_scale: f32) -> [f64; 16] {
    [
        m[0][0] as f64,
        m[0][1] as f64,
        m[0][2] as f64,
        m[0][3] as f64,
        m[1][0] as f64,
        m[1][1] as f64,
        m[1][2] as f64,
        m[1][3] as f64,
        m[2][0] as f64,
        m[2][1] as f64,
        m[2][2] as f64,
        m[2][3] as f64,
        (m[3][0] * inv_unit_scale) as f64,
        (m[3][1] * inv_unit_scale) as f64,
        (m[3][2] * inv_unit_scale) as f64,
        m[3][3] as f64,
    ]
}

fn generate_bone_connections(bones: &[FbxBoneExport], connections: &mut Vec<FbxConnection>) {
    for bone in bones {
        let parent_uid = bone.parent_model_uid.unwrap_or(0);
        connections.push(FbxConnection::OO {
            child: bone.model_uid,
            parent: parent_uid,
        });

        connections.push(FbxConnection::OO {
            child: bone.node_attr_uid,
            parent: bone.model_uid,
        });
    }
}

fn generate_mesh_connections(
    mesh_models: &[FbxMeshModelExport],
    geometries: &[FbxGeometryExport],
    materials: &[FbxMaterialExport],
    textures: &[FbxTextureExport],
    skins: &[FbxSkinExport],
    connections: &mut Vec<FbxConnection>,
) {
    for (i, mesh_model) in mesh_models.iter().enumerate() {
        let parent_uid = mesh_model.parent_bone_uid.unwrap_or(0);
        connections.push(FbxConnection::OO {
            child: mesh_model.uid,
            parent: parent_uid,
        });

        if i < geometries.len() {
            connections.push(FbxConnection::OO {
                child: geometries[i].uid,
                parent: mesh_model.uid,
            });
        }
    }

    for material in materials {
        connections.push(FbxConnection::OO {
            child: material.uid,
            parent: material.mesh_model_uid,
        });
    }

    for texture in textures {
        connections.push(FbxConnection::OP {
            child: texture.texture_uid,
            parent: texture.material_uid,
            property: "DiffuseColor".to_string(),
        });

        connections.push(FbxConnection::OO {
            child: texture.video_uid,
            parent: texture.texture_uid,
        });
    }

    for skin in skins {
        connections.push(FbxConnection::OO {
            child: skin.skin_uid,
            parent: skin.geometry_uid,
        });

        for cluster in &skin.clusters {
            connections.push(FbxConnection::OO {
                child: cluster.uid,
                parent: skin.skin_uid,
            });

            connections.push(FbxConnection::OO {
                child: cluster.bone_model_uid,
                parent: cluster.uid,
            });
        }
    }
}

fn generate_animation_connections(
    stack_uid: i64,
    layer_uid: i64,
    curve_nodes: &[FbxCurveNodeExport],
    connections: &mut Vec<FbxConnection>,
) {
    connections.push(FbxConnection::OO {
        child: stack_uid,
        parent: 0,
    });

    connections.push(FbxConnection::OO {
        child: layer_uid,
        parent: stack_uid,
    });

    for cn in curve_nodes {
        connections.push(FbxConnection::OO {
            child: cn.uid,
            parent: layer_uid,
        });

        connections.push(FbxConnection::OP {
            child: cn.uid,
            parent: cn.bone_model_uid,
            property: cn.channel.property_name().to_string(),
        });

        let axis_names = ["d|X", "d|Y", "d|Z"];
        for (i, axis) in axis_names.iter().enumerate() {
            if let Some(curve_uid) = cn.curve_uids[i] {
                connections.push(FbxConnection::OP {
                    child: curve_uid,
                    parent: cn.uid,
                    property: axis.to_string(),
                });
            }
        }
    }
}

fn write_full_definitions<W: Write + Seek>(
    writer: &mut Writer<W>,
    data: &FullFbxExportData,
) -> FbxWriteResult<()> {
    let model_count =
        data.anim_data.bones.len() as i32 + data.mesh_models.len() as i32;
    let node_attr_count = data.anim_data.bones.len() as i32;
    let geometry_count = data.geometries.len() as i32;
    let material_count = data.materials.len() as i32;
    let texture_count = data.textures.len() as i32;
    let video_count = data.textures.len() as i32;
    let deformer_count = data.skins.len() as i32;
    let sub_deformer_count: i32 =
        data.skins.iter().map(|s| s.clusters.len() as i32).sum();
    let curve_node_count = data.anim_data.curve_nodes.len() as i32;
    let curve_count = data.anim_data.curves.len() as i32;

    let total = 1 + model_count + node_attr_count + geometry_count
        + material_count + texture_count + video_count
        + deformer_count + sub_deformer_count
        + 1 + 1 + curve_node_count + curve_count;

    drop(writer.new_node("Definitions")?);

    {
        let mut attrs = writer.new_node("Version")?;
        attrs.append_i32(100)?;
        drop(attrs);
        writer.close_node()?;
    }

    {
        let mut attrs = writer.new_node("Count")?;
        attrs.append_i32(total)?;
        drop(attrs);
        writer.close_node()?;
    }

    write_object_type(writer, "GlobalSettings", 1)?;
    write_object_type(writer, "Model", model_count)?;
    write_object_type(writer, "NodeAttribute", node_attr_count)?;

    if geometry_count > 0 {
        write_object_type(writer, "Geometry", geometry_count)?;
    }
    if material_count > 0 {
        write_object_type(writer, "Material", material_count)?;
    }
    if texture_count > 0 {
        write_object_type(writer, "Texture", texture_count)?;
    }
    if video_count > 0 {
        write_object_type(writer, "Video", video_count)?;
    }
    if deformer_count > 0 {
        write_object_type(writer, "Deformer", deformer_count + sub_deformer_count)?;
    }

    write_object_type(writer, "AnimationStack", 1)?;
    write_object_type(writer, "AnimationLayer", 1)?;

    if curve_node_count > 0 {
        write_object_type(writer, "AnimationCurveNode", curve_node_count)?;
    }
    if curve_count > 0 {
        write_object_type(writer, "AnimationCurve", curve_count)?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_mesh_model<W: Write + Seek>(
    writer: &mut Writer<W>,
    mesh: &FbxMeshModelExport,
) -> FbxWriteResult<()> {
    let fbx_name = format!("{}\x00\x01Model", mesh.name);
    let mut attrs = writer.new_node("Model")?;
    attrs.append_i64(mesh.uid)?;
    attrs.append_string_direct(&fbx_name)?;
    attrs.append_string_direct("Mesh")?;
    drop(attrs);

    {
        let mut va = writer.new_node("Version")?;
        va.append_i32(232)?;
        drop(va);
        writer.close_node()?;
    }

    drop(writer.new_node("Properties70")?);
    write_property_f64x3(
        writer,
        "Lcl Translation",
        "Lcl Translation",
        "",
        "A",
        mesh.translation[0],
        mesh.translation[1],
        mesh.translation[2],
    )?;
    write_property_f64x3(
        writer,
        "Lcl Rotation",
        "Lcl Rotation",
        "",
        "A",
        mesh.rotation[0],
        mesh.rotation[1],
        mesh.rotation[2],
    )?;
    write_property_f64x3(
        writer,
        "Lcl Scaling",
        "Lcl Scaling",
        "",
        "A",
        mesh.scaling[0],
        mesh.scaling[1],
        mesh.scaling[2],
    )?;
    writer.close_node()?;

    writer.close_node()?;
    Ok(())
}

fn write_geometry<W: Write + Seek>(
    writer: &mut Writer<W>,
    geo: &FbxGeometryExport,
) -> FbxWriteResult<()> {
    let fbx_name = format!("\x00\x01Geometry");
    let mut attrs = writer.new_node("Geometry")?;
    attrs.append_i64(geo.uid)?;
    attrs.append_string_direct(&fbx_name)?;
    attrs.append_string_direct("Mesh")?;
    drop(attrs);

    {
        let mut va = writer.new_node("Vertices")?;
        va.append_arr_f64_from_iter(None, geo.positions.iter().copied())?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("PolygonVertexIndex")?;
        va.append_arr_i32_from_iter(None, geo.polygon_vertex_index.iter().copied())?;
        drop(va);
        writer.close_node()?;
    }

    if !geo.normals.is_empty() {
        write_layer_element_normal(writer, &geo.normals)?;
    }

    if !geo.uv_values.is_empty() {
        write_layer_element_uv(writer, &geo.uv_values, &geo.uv_indices)?;
    }

    if !geo.normals.is_empty() || !geo.uv_values.is_empty() {
        write_layer(writer, !geo.normals.is_empty(), !geo.uv_values.is_empty())?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_layer_element_normal<W: Write + Seek>(
    writer: &mut Writer<W>,
    normals: &[f64],
) -> FbxWriteResult<()> {
    drop(writer.new_node("LayerElementNormal")?);

    {
        let mut va = writer.new_node("Version")?;
        va.append_i32(101)?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("Name")?;
        va.append_string_direct("")?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("MappingInformationType")?;
        va.append_string_direct("ByVertice")?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("ReferenceInformationType")?;
        va.append_string_direct("Direct")?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("Normals")?;
        va.append_arr_f64_from_iter(None, normals.iter().copied())?;
        drop(va);
        writer.close_node()?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_layer_element_uv<W: Write + Seek>(
    writer: &mut Writer<W>,
    uv_values: &[f64],
    uv_indices: &[i32],
) -> FbxWriteResult<()> {
    drop(writer.new_node("LayerElementUV")?);

    {
        let mut va = writer.new_node("Version")?;
        va.append_i32(101)?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("Name")?;
        va.append_string_direct("UVMap")?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("MappingInformationType")?;
        va.append_string_direct("ByVertice")?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("ReferenceInformationType")?;
        va.append_string_direct("IndexToDirect")?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("UV")?;
        va.append_arr_f64_from_iter(None, uv_values.iter().copied())?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("UVIndex")?;
        va.append_arr_i32_from_iter(None, uv_indices.iter().copied())?;
        drop(va);
        writer.close_node()?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_layer<W: Write + Seek>(
    writer: &mut Writer<W>,
    has_normal: bool,
    has_uv: bool,
) -> FbxWriteResult<()> {
    drop(writer.new_node("Layer")?);

    {
        let mut va = writer.new_node("Version")?;
        va.append_i32(100)?;
        drop(va);
        writer.close_node()?;
    }

    if has_normal {
        drop(writer.new_node("LayerElement")?);

        {
            let mut va = writer.new_node("Type")?;
            va.append_string_direct("LayerElementNormal")?;
            drop(va);
            writer.close_node()?;
        }

        {
            let mut va = writer.new_node("TypedIndex")?;
            va.append_i32(0)?;
            drop(va);
            writer.close_node()?;
        }

        writer.close_node()?;
    }

    if has_uv {
        drop(writer.new_node("LayerElement")?);

        {
            let mut va = writer.new_node("Type")?;
            va.append_string_direct("LayerElementUV")?;
            drop(va);
            writer.close_node()?;
        }

        {
            let mut va = writer.new_node("TypedIndex")?;
            va.append_i32(0)?;
            drop(va);
            writer.close_node()?;
        }

        writer.close_node()?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_material<W: Write + Seek>(
    writer: &mut Writer<W>,
    mat: &FbxMaterialExport,
) -> FbxWriteResult<()> {
    let fbx_name = format!("{}\x00\x01Material", mat.name);
    let mut attrs = writer.new_node("Material")?;
    attrs.append_i64(mat.uid)?;
    attrs.append_string_direct(&fbx_name)?;
    attrs.append_string_direct("")?;
    drop(attrs);

    {
        let mut va = writer.new_node("Version")?;
        va.append_i32(102)?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("ShadingModel")?;
        va.append_string_direct("phong")?;
        drop(va);
        writer.close_node()?;
    }

    drop(writer.new_node("Properties70")?);
    write_property_f64x3(
        writer,
        "DiffuseColor",
        "Color",
        "",
        "A",
        mat.diffuse_color[0],
        mat.diffuse_color[1],
        mat.diffuse_color[2],
    )?;
    writer.close_node()?;

    writer.close_node()?;
    Ok(())
}

fn write_texture<W: Write + Seek>(
    writer: &mut Writer<W>,
    tex: &FbxTextureExport,
) -> FbxWriteResult<()> {
    let fbx_name = format!("\x00\x01Texture");
    let mut attrs = writer.new_node("Texture")?;
    attrs.append_i64(tex.texture_uid)?;
    attrs.append_string_direct(&fbx_name)?;
    attrs.append_string_direct("")?;
    drop(attrs);

    {
        let mut va = writer.new_node("Type")?;
        va.append_string_direct("TextureDiffuse")?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("FileName")?;
        va.append_string_direct(&tex.filename)?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("RelativeFilename")?;
        let relative = Path::new(&tex.filename)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&tex.filename);
        va.append_string_direct(relative)?;
        drop(va);
        writer.close_node()?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_video<W: Write + Seek>(
    writer: &mut Writer<W>,
    tex: &FbxTextureExport,
) -> FbxWriteResult<()> {
    let fbx_name = format!("\x00\x01Video");
    let mut attrs = writer.new_node("Video")?;
    attrs.append_i64(tex.video_uid)?;
    attrs.append_string_direct(&fbx_name)?;
    attrs.append_string_direct("Clip")?;
    drop(attrs);

    {
        let mut va = writer.new_node("FileName")?;
        va.append_string_direct(&tex.filename)?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("RelativeFilename")?;
        let relative = Path::new(&tex.filename)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&tex.filename);
        va.append_string_direct(relative)?;
        drop(va);
        writer.close_node()?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_skin_deformer<W: Write + Seek>(
    writer: &mut Writer<W>,
    skin: &FbxSkinExport,
) -> FbxWriteResult<()> {
    let mut attrs = writer.new_node("Deformer")?;
    attrs.append_i64(skin.skin_uid)?;
    attrs.append_string_direct("\x00\x01Deformer")?;
    attrs.append_string_direct("Skin")?;
    drop(attrs);

    {
        let mut va = writer.new_node("Version")?;
        va.append_i32(101)?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("Link_DeformAcuracy")?;
        va.append_f64(50.0)?;
        drop(va);
        writer.close_node()?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_cluster<W: Write + Seek>(
    writer: &mut Writer<W>,
    cluster: &FbxClusterExport,
) -> FbxWriteResult<()> {
    let mut attrs = writer.new_node("Deformer")?;
    attrs.append_i64(cluster.uid)?;
    attrs.append_string_direct("\x00\x01SubDeformer")?;
    attrs.append_string_direct("Cluster")?;
    drop(attrs);

    {
        let mut va = writer.new_node("Version")?;
        va.append_i32(100)?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("Indexes")?;
        va.append_arr_i32_from_iter(None, cluster.indices.iter().copied())?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("Weights")?;
        va.append_arr_f64_from_iter(None, cluster.weights.iter().copied())?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("Transform")?;
        va.append_arr_f64_from_iter(None, cluster.transform.iter().copied())?;
        drop(va);
        writer.close_node()?;
    }

    {
        let mut va = writer.new_node("TransformLink")?;
        va.append_arr_f64_from_iter(None, cluster.transform_link.iter().copied())?;
        drop(va);
        writer.close_node()?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_full_objects<W: Write + Seek>(
    writer: &mut Writer<W>,
    data: &FullFbxExportData,
) -> FbxWriteResult<()> {
    drop(writer.new_node("Objects")?);

    for bone in &data.anim_data.bones {
        write_bone_model(writer, bone)?;
        write_node_attribute(writer, bone)?;
    }

    for geo in &data.geometries {
        write_geometry(writer, geo)?;
    }

    for mesh_model in &data.mesh_models {
        write_mesh_model(writer, mesh_model)?;
    }

    for material in &data.materials {
        write_material(writer, material)?;
    }

    for texture in &data.textures {
        write_texture(writer, texture)?;
        write_video(writer, texture)?;
    }

    for skin in &data.skins {
        write_skin_deformer(writer, skin)?;
        for cluster in &skin.clusters {
            write_cluster(writer, cluster)?;
        }
    }

    write_anim_stack(writer, &data.anim_data)?;
    write_anim_layer(writer, data.anim_data.layer_uid)?;

    for cn in &data.anim_data.curve_nodes {
        write_anim_curve_node(writer, cn)?;
    }

    for curve in &data.anim_data.curves {
        write_anim_curve(writer, curve)?;
    }

    writer.close_node()?;
    Ok(())
}

fn write_full_fbx_binary<W: Write + Seek>(
    mut writer: Writer<W>,
    data: &FullFbxExportData,
) -> FbxWriteResult<()> {
    write_header_extension(&mut writer)?;
    write_global_settings(&mut writer, data.anim_data.duration_ktime, &data.anim_data.axes, data.anim_data.fps)?;
    write_documents(&mut writer, data.anim_data.document_uid)?;
    write_references(&mut writer)?;
    write_full_definitions(&mut writer, data)?;
    write_full_objects(&mut writer, data)?;
    write_connections(&mut writer, &data.anim_data)?;
    writer.finalize_and_flush(&FbxFooter::default())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_triangle_polygon_indices() {
        let indices = vec![0, 1, 2, 3, 4, 5];
        let encoded = encode_triangle_polygon_indices(&indices);
        assert_eq!(encoded, vec![0, 1, -3, 3, 4, -6]);
    }

    #[test]
    fn test_encode_triangle_polygon_indices_single() {
        let indices = vec![0, 1, 2];
        let encoded = encode_triangle_polygon_indices(&indices);
        assert_eq!(encoded, vec![0, 1, -3]);
    }

    #[test]
    fn test_convert_uvs_to_fbx_flip() {
        let mut fbx_data = FbxData::new();
        fbx_data.tex_coords = vec![[0.5, 0.3]];
        let (uv_values, _) = convert_uvs_to_fbx(&fbx_data);
        assert!((uv_values[0] - 0.5).abs() < 1e-6);
        assert!((uv_values[1] - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_matrix4_to_flat_f64_scaled() {
        use cgmath::SquareMatrix;
        let identity = Matrix4::<f32>::identity();
        let flat = matrix4_to_flat_f64_scaled(&identity, 2.0);
        assert!((flat[0] - 1.0).abs() < 1e-8);
        assert!((flat[5] - 1.0).abs() < 1e-8);
        assert!((flat[10] - 1.0).abs() < 1e-8);
        assert!((flat[15] - 1.0).abs() < 1e-8);
        assert!((flat[12] - 0.0).abs() < 1e-8);
    }

    #[test]
    fn test_convert_positions_to_fbx_no_scale() {
        let mut fbx_data = FbxData::new();
        fbx_data.positions = vec![cgmath::Vector3::new(0.01, 0.02, 0.03)];
        fbx_data.local_positions = vec![];

        let positions = convert_positions_to_fbx(&fbx_data, 1.0);
        assert!((positions[0] - 0.01).abs() < 1e-6);
        assert!((positions[1] - 0.02).abs() < 1e-6);
        assert!((positions[2] - 0.03).abs() < 1e-6);
    }

    #[test]
    fn test_fbx_roundtrip_stickman() {
        let original_path = "assets/models/stickman/stickman_bin.fbx";
        if !std::path::Path::new(original_path).exists() {
            eprintln!("Skipping roundtrip test: {} not found", original_path);
            return;
        }

        let fbx_model =
            crate::loader::fbx::fbx::load_fbx_with_ufbx(original_path).expect("Failed to load original FBX");

        let result = crate::loader::fbx::loader::load_fbx_to_graphics_resources(original_path)
            .expect("Failed to load graphics resources");
        let (load_result, _) = result;

        let skeleton = load_result
            .animation_system
            .get_skeleton(0)
            .expect("No skeleton found")
            .clone();

        let export_dir = std::path::Path::new("assets/exports");
        std::fs::create_dir_all(export_dir).ok();
        let export_path = export_dir.join("roundtrip_test.fbx");

        export_full_fbx(&fbx_model, None, &skeleton, &export_path)
            .expect("Failed to export FBX");

        let original_scene = ufbx::load_file(original_path, ufbx::LoadOpts::default())
            .expect("Failed to load original with ufbx");
        let exported_scene = ufbx::load_file(export_path.to_str().unwrap(), ufbx::LoadOpts::default())
            .expect("Failed to load exported with ufbx");

        let orig_axes = &original_scene.settings.axes;
        let exp_axes = &exported_scene.settings.axes;
        assert_eq!(
            orig_axes.up as i32, exp_axes.up as i32,
            "UpAxis mismatch: original={:?}, exported={:?}",
            orig_axes.up, exp_axes.up
        );
        assert_eq!(
            orig_axes.front as i32, exp_axes.front as i32,
            "FrontAxis mismatch: original={:?}, exported={:?}",
            orig_axes.front, exp_axes.front
        );
        assert_eq!(
            orig_axes.right as i32, exp_axes.right as i32,
            "CoordAxis mismatch: original={:?}, exported={:?}",
            orig_axes.right, exp_axes.right
        );

        let orig_non_root_nodes: Vec<_> = original_scene
            .nodes
            .iter()
            .filter(|n| !n.is_root)
            .collect();
        let exp_non_root_nodes: Vec<_> = exported_scene
            .nodes
            .iter()
            .filter(|n| !n.is_root)
            .collect();

        let orig_names: std::collections::HashSet<String> = orig_non_root_nodes
            .iter()
            .map(|n| n.element.name.to_string())
            .collect();
        let exp_names: std::collections::HashSet<String> = exp_non_root_nodes
            .iter()
            .map(|n| n.element.name.to_string())
            .collect();

        let missing_in_export: Vec<_> = orig_names.difference(&exp_names).collect();
        let extra_in_export: Vec<_> = exp_names.difference(&orig_names).collect();
        assert!(
            missing_in_export.is_empty(),
            "Nodes missing in exported FBX: {:?}",
            missing_in_export
        );
        if !extra_in_export.is_empty() {
            eprintln!("Extra nodes in export (acceptable): {:?}", extra_in_export);
        }

        for orig_node in &orig_non_root_nodes {
            let name = orig_node.element.name.to_string();
            let exp_node = exp_non_root_nodes
                .iter()
                .find(|n| n.element.name.to_string() == name);

            let Some(exp_node) = exp_node else {
                continue;
            };

            let orig_t = &orig_node.local_transform;
            let exp_t = &exp_node.local_transform;

            let position_tolerance = 0.1;
            let orig_pos = [orig_t.translation.x, orig_t.translation.y, orig_t.translation.z];
            let exp_pos = [exp_t.translation.x, exp_t.translation.y, exp_t.translation.z];

            for axis in 0..3 {
                let diff = (orig_pos[axis] - exp_pos[axis]).abs();
                assert!(
                    diff < position_tolerance,
                    "Node '{}' position[{}] mismatch: original={}, exported={}, diff={}",
                    name, axis, orig_pos[axis], exp_pos[axis], diff
                );
            }
        }

        assert!(
            !exported_scene.anim_stacks.is_empty(),
            "Exported FBX has no animation stacks"
        );

        std::fs::remove_file(&export_path).ok();
    }

    #[test]
    fn test_fbx_roundtrip_stickman_with_animation() {
        let original_path = "assets/models/stickman/stickman_bin.fbx";
        if !std::path::Path::new(original_path).exists() {
            eprintln!("Skipping: {} not found", original_path);
            return;
        }

        let fbx_model = crate::loader::fbx::fbx::load_fbx_with_ufbx(original_path)
            .expect("Failed to load original FBX");
        let (load_result, _) =
            crate::loader::fbx::loader::load_fbx_to_graphics_resources(original_path)
                .expect("Failed to load graphics resources");

        let skeleton = load_result
            .animation_system
            .get_skeleton(0)
            .expect("No skeleton found")
            .clone();
        let anim_clip = load_result.clips.first().expect("No animation clip found");

        let bone_names: std::collections::HashMap<u32, String> = skeleton
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| (i as u32, b.name.clone()))
            .collect();
        let editable = crate::animation::editable::EditableAnimationClip::from_animation_clip(
            1, anim_clip, &bone_names,
        );
        assert!(editable.duration > 0.0);
        assert!(!editable.tracks.is_empty());

        let export_dir = std::path::Path::new("assets/exports");
        std::fs::create_dir_all(export_dir).ok();
        let export_path = export_dir.join("roundtrip_anim_test.fbx");

        export_full_fbx(&fbx_model, Some(&editable), &skeleton, &export_path)
            .expect("Failed to export FBX with animation");

        let original_scene = ufbx::load_file(original_path, ufbx::LoadOpts::default())
            .expect("Failed to load original with ufbx");
        let exported_scene =
            ufbx::load_file(export_path.to_str().unwrap(), ufbx::LoadOpts::default())
                .expect("Failed to load exported with ufbx");

        assert!(!exported_scene.anim_stacks.is_empty());
        assert!(
            (exported_scene.settings.frames_per_second
                - original_scene.settings.frames_per_second)
                .abs()
                < 1.0
        );

        let anim_stack = &exported_scene.anim_stacks[0];
        let time_span = anim_stack.time_end - anim_stack.time_begin;
        assert!(time_span > 0.1, "Animation time span too short: {:.4}s", time_span);

        let baked = ufbx::bake_anim(
            &exported_scene,
            &exported_scene.anim_stacks[0].anim,
            ufbx::BakeOpts::default(),
        )
        .expect("Failed to bake exported animation");

        let orig_baked = ufbx::bake_anim(
            &original_scene,
            &original_scene.anim_stacks[0].anim,
            ufbx::BakeOpts::default(),
        )
        .expect("Failed to bake original animation");

        let animated_count = baked
            .nodes
            .iter()
            .filter(|n| n.rotation_keys.len() > 1)
            .count();
        assert!(animated_count > 0, "Exported FBX has no animated nodes");

        let mesh_node_names: Vec<String> = fbx_model
            .fbx_data
            .iter()
            .filter_map(|d| d.mesh_node_name.clone())
            .collect();

        for mesh_name in &mesh_node_names {
            let orig_parent = original_scene
                .nodes
                .iter()
                .find(|n| n.element.name.to_string() == *mesh_name)
                .and_then(|n| n.parent.as_ref())
                .map(|p| p.element.name.to_string())
                .unwrap_or_default();
            let exp_parent = exported_scene
                .nodes
                .iter()
                .find(|n| n.element.name.to_string() == *mesh_name)
                .and_then(|n| n.parent.as_ref())
                .map(|p| p.element.name.to_string())
                .unwrap_or_default();
            assert_eq!(orig_parent, exp_parent, "Parent mismatch for mesh '{}'", mesh_name);
        }

        for orig_bn in &orig_baked.nodes {
            let orig_idx = orig_bn.typed_id as usize;
            if orig_idx >= original_scene.nodes.len() || orig_bn.rotation_keys.len() <= 2 {
                continue;
            }
            let name = original_scene.nodes[orig_idx].element.name.to_string();

            let exp_bn = baked.nodes.iter().find(|bn| {
                let idx = bn.typed_id as usize;
                idx < exported_scene.nodes.len()
                    && exported_scene.nodes[idx].element.name.to_string() == name
            });
            let exp_bn = match exp_bn {
                Some(b) => b,
                None => continue,
            };

            assert_eq!(
                orig_bn.rotation_keys.len(),
                exp_bn.rotation_keys.len(),
                "Rotation key count mismatch for bone '{}'",
                name
            );

            let sample_indices = [0, 100, 500, 1000];
            for &idx in &sample_indices {
                if idx >= orig_bn.rotation_keys.len() {
                    break;
                }
                let o = &orig_bn.rotation_keys[idx];
                let e = &exp_bn.rotation_keys[idx];
                let max_diff = (o.value.w - e.value.w)
                    .abs()
                    .max((o.value.x - e.value.x).abs())
                    .max((o.value.y - e.value.y).abs())
                    .max((o.value.z - e.value.z).abs());
                assert!(
                    max_diff < 0.01,
                    "Rotation value mismatch for bone '{}' at key {}: diff={}",
                    name, idx, max_diff
                );
            }
        }

        std::fs::remove_file(&export_path).ok();
    }
}
