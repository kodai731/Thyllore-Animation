/*
reference from bevy_mod_fbx, FizzWizZleDazzle
https://github.com/FizzWizZleDazzle/bevy_mod_fbx/blob/main/src/loader.rs#L217
 */
use crate::log;
use crate::math::math::*;
use anyhow::{anyhow, Context, Result};
use cgmath::{Matrix4, Quaternion, Deg, Rad, EuclideanSpace, Point3};
use fbxcel::tree::v7400::NodeHandle;
use fbxcel_dom::any::AnyDocument;
use fbxcel_dom::v7400::data::{
    mesh::{
        layer::TypedLayerElementHandle, ControlPointIndex, PolygonVertexIndex, PolygonVertices,
    },
    texture::WrapMode,
};
use fbxcel_dom::v7400::object::property::loaders::{StrictF64Loader, F64Arr3Loader, F64Arr16Loader};
use fbxcel_dom::v7400::{
    object::{
        model::{ModelHandle, TypedModelHandle},
        ObjectHandle, TypedObjectHandle,
        deformer::{ClusterHandle, TypedDeformerHandle},
    },
    Document,
};
use std::collections::HashMap;

/// Get local transform matrix from FBX model node
fn get_local_transform(model: &ModelHandle) -> Matrix4<f32> {
    let mesh_name = model.name().unwrap_or("");

    // Get transform properties from FBX file
    let model_obj: &ObjectHandle = &**model;
    let props = model_obj.properties_by_native_typename("FbxNode");

    // Helper function to extract Vec3 from property
    let get_vec3 = |name: &str, default: [f32; 3]| -> [f32; 3] {
        if let Some(prop) = props.get_property(name) {
            if let Ok(values) = prop.load_value(F64Arr3Loader) {
                return [values[0] as f32, values[1] as f32, values[2] as f32];
            }
        }
        default
    };

    let translation = get_vec3("Lcl Translation", [0.0, 0.0, 0.0]);
    let rotation = get_vec3("Lcl Rotation", [0.0, 0.0, 0.0]);
    let scaling = get_vec3("Lcl Scaling", [1.0, 1.0, 1.0]);

    log!(
        "Mesh: {}, Local transform - Translation: {:?}, Rotation (deg): {:?}, Scaling: {:?}",
        mesh_name,
        translation,
        rotation,
        scaling
    );

    // Build transform matrix: T * R * S
    let translation_matrix = Matrix4::from_translation(vec3(
        translation[0],
        translation[1],
        translation[2],
    ));

    // FBX rotation is in degrees, convert to radians
    // FBX uses XYZ Euler rotation order by default (applied as Z*Y*X in matrix multiplication)
    let rotation_x = Matrix4::from_angle_x(Rad((rotation[0] as f32).to_radians()));
    let rotation_y = Matrix4::from_angle_y(Rad((rotation[1] as f32).to_radians()));
    let rotation_z = Matrix4::from_angle_z(Rad((rotation[2] as f32).to_radians()));
    // Apply in reverse order for XYZ Euler: Z * Y * X
    let rotation_matrix = rotation_z * rotation_y * rotation_x;

    let scale_matrix = Matrix4::from_nonuniform_scale(
        scaling[0],
        scaling[1],
        scaling[2],
    );

    translation_matrix * rotation_matrix * scale_matrix
}

/// Get world transform matrix by traversing parent hierarchy
fn get_world_transform(model: &ModelHandle, doc: &Document) -> Matrix4<f32> {
    let mesh_name = model.name().unwrap_or("");

    // Get local transform first
    let local_transform = get_local_transform(model);

    // Try to find parent model in the hierarchy
    let model_obj: &ObjectHandle = &**model;

    // Check if this mesh is a child of another mesh/model
    // In FBX, destination_objects() returns objects that this object connects TO (parents)
    for conn in model_obj.destination_objects() {
        if let Some(parent_obj) = conn.object_handle() {
            // Check if parent is a Model node
            if let TypedObjectHandle::Model(parent_model_typed) = parent_obj.get_typed() {
                let parent_name = parent_obj.name().unwrap_or("");

                // For mesh parents, use their transform
                match parent_model_typed {
                    TypedModelHandle::Mesh(parent_mesh) => {
                        log!("  Mesh {} has parent mesh: {}", mesh_name, parent_name);

                        // Recursively get parent's world transform
                        let parent_world = get_world_transform(&parent_mesh, doc);

                        // Combine: parent_world * local_transform
                        let world_transform = parent_world * local_transform;
                        return world_transform;
                    }
                    TypedModelHandle::Null(_) | TypedModelHandle::LimbNode(_) => {
                        log!("  Mesh {} has parent node: {}", mesh_name, parent_name);

                        // Get parent transform
                        let parent_world = get_world_transform_for_object(&parent_obj, doc);

                        // Combine: parent_world * local_transform
                        let world_transform = parent_world * local_transform;
                        return world_transform;
                    }
                    _ => {}
                }
            }
        }
    }

    // No parent found, local transform is world transform
    log!("  No parent for {}, using local as world", mesh_name);
    local_transform
}

/// Helper function to get world transform for any ObjectHandle
fn get_world_transform_for_object(obj: &ObjectHandle, doc: &Document) -> Matrix4<f32> {
    if let TypedObjectHandle::Model(model_typed) = obj.get_typed() {
        match model_typed {
            TypedModelHandle::Mesh(mesh_model) => {
                get_world_transform(&mesh_model, doc)
            }
            TypedModelHandle::Null(null_model) => {
                get_world_transform(&null_model, doc)
            }
            TypedModelHandle::LimbNode(limb_model) => {
                get_world_transform(&limb_model, doc)
            }
            _ => {
                // For other model types, use identity transform
                Matrix4::from_scale(1.0)
            }
        }
    } else {
        Matrix4::from_scale(1.0)
    }
}

pub unsafe fn load_fbx(path: &str) -> anyhow::Result<(FbxModel)> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut fbx_model = FbxModel::default();
    // TODO: multi FbxData per material
    fbx_model.fbx_data.push(FbxData::new());
    match AnyDocument::from_reader(reader).expect("failed to load FBX document") {
        AnyDocument::V7400(fbx_ver, doc) => {
            // First, log all objects to understand the hierarchy
            log!("=== FBX Object Hierarchy ===");
            for object in doc.objects() {
                let obj_name = object.name().unwrap_or("unnamed");
                log!("Object: name='{}', class='{}', subclass='{}'",
                    obj_name, object.class(), object.subclass());

                // Log parent connections (source = parent)
                for conn in object.source_objects() {
                    if let Some(src) = conn.object_handle() {
                        log!("  <- Parent: {}", src.name().unwrap_or("unnamed"));
                    }
                }
            }
            log!("=== Loading Meshes ===");

            for object in doc.objects() {
                if let TypedObjectHandle::Model(TypedModelHandle::Mesh(mesh)) = object.get_typed() {
                    log!("Loading mesh {:?}", mesh);
                    let mesh_name = mesh.name().expect("mesh name not found").to_string();
                    log!("mesh node name {}", mesh_name);

                    // Get world transform matrix for this mesh
                    let world_transform = get_world_transform(&mesh, &doc);
                    log!("World transform for {}: {:?}", mesh_name, world_transform);

                    let mesh_handle = mesh.geometry().context("failed to get geometry handle")?;
                    let polygon_vertices = mesh_handle
                        .polygon_vertices()
                        .context("failed to get polygon vertices")?;
                    let triangle_indices = polygon_vertices.triangulate_each(triangulate)?;
                    log!("polygon vertices {:?}", triangle_indices);

                    let mut indices: Vec<u32> = triangle_indices
                        .triangle_vertex_indices()
                        .map(|t| t.to_usize() as u32)
                        .collect();
                    let offset = fbx_model.fbx_data[0].positions.len() as u32;
                    for index in indices.iter_mut() {
                        *index += offset;
                    }
                    log!("indices: count={}, {:?}", indices.len(), indices);
                    fbx_model.fbx_data[0].indices.extend(indices);

                    // ローカル座標の頂点位置を取得
                    let get_local_position =
                        |pos: Option<ControlPointIndex>| -> Result<_, anyhow::Error> {
                            let cpi =
                                pos.ok_or_else(|| anyhow!("failed to get position handle"))?;
                            let point = polygon_vertices.control_point(cpi).ok_or_else(|| {
                                anyhow!("failed to get point handle cpi: {:?}", cpi)
                            })?;
                            Ok(Vector3::new(point.x as f32, point.y as f32, point.z as f32))
                        };

                    let local_positions = triangle_indices
                        .iter_control_point_indices()
                        .map(get_local_position)
                        .collect::<Result<Vec<_>, _>>()
                        .context("failed to get local position")?;

                    // ワールド座標の頂点位置を取得（表示用）
                    let get_position =
                        |pos: Option<ControlPointIndex>| -> Result<_, anyhow::Error> {
                            let cpi =
                                pos.ok_or_else(|| anyhow!("failed to get position handle"))?;
                            let point = polygon_vertices.control_point(cpi).ok_or_else(|| {
                                anyhow!("failed to get point handle cpi: {:?}", cpi)
                            })?;

                            // Apply world transform to vertex position
                            let local_pos = Vector3::new(point.x as f32, point.y as f32, point.z as f32);
                            let world_pos = world_transform.transform_point(Point3::from_vec(local_pos));

                            Ok(Vector3::new(world_pos.x, world_pos.y, world_pos.z))
                        };
                    let positions = triangle_indices
                        .iter_control_point_indices()
                        .map(get_position)
                        .collect::<Result<Vec<_>, _>>()
                        .context("failed to get position")?;

                    log!("positions (transformed): {} {:?}", mesh_name, positions);
                    fbx_model.fbx_data[0].local_positions.extend(local_positions);
                    fbx_model.fbx_data[0].positions.extend(positions);

                    // Skin Deformerを探してクラスター情報を取得
                    let mesh_obj: &ObjectHandle = &*mesh;
                    for conn in mesh_obj.source_objects() {
                        if let Some(deformer_obj) = conn.object_handle() {
                            if let TypedObjectHandle::Deformer(TypedDeformerHandle::Skin(skin)) = deformer_obj.get_typed() {
                                log!("Found Skin Deformer for mesh: {}", mesh_name);

                                // 各クラスターを処理
                                for cluster in skin.clusters() {
                                    match extract_cluster_data(&cluster) {
                                        Ok(cluster_info) => {
                                            log!("Successfully extracted cluster data for bone: {}", cluster_info.bone_name);
                                            fbx_model.fbx_data[0].clusters.push(cluster_info);
                                        }
                                        Err(e) => {
                                            log!("Warning: Failed to extract cluster data: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // AnimStackを探してアニメーションを抽出
            log!("=== Loading Animations ===");
            for object in doc.objects() {
                if object.class() == "AnimStack" {
                    log!("Found AnimStack: {:?}", object.name());
                    match extract_anim_stack(&object, &doc) {
                        Ok(animation) => {
                            log!("Successfully extracted animation: {}", animation.name);
                            fbx_model.animations.push(animation);
                        }
                        Err(e) => {
                            log!("Warning: Failed to extract AnimStack: {}", e);
                        }
                    }
                }
            }

            log!("Loaded {} animations", fbx_model.animations.len());
        }
        _ => log!("unsupported FBX version"),
    }
    Ok(fbx_model)
}
pub fn triangulate(
    pvs: &PolygonVertices<'_>,
    poly_pvis: &[PolygonVertexIndex],
    results: &mut Vec<[PolygonVertexIndex; 3]>,
) -> anyhow::Result<()> {
    macro_rules! get_vec {
        ($pvii:expr) => {
            get_vec(pvs, poly_pvis[$pvii])
        };
    }

    match poly_pvis.len() {
        3 => {
            // Got a triangle, no need of triangulation.
            results.push([poly_pvis[0], poly_pvis[1], poly_pvis[2]]);

            Ok(())
        }
        4 => {
            // p0, p1, p2, p3: vertices of the quadrangle (angle{0..3}).
            let p0 = get_vec!(0)?;
            let p1 = get_vec!(1)?;
            let p2 = get_vec!(2)?;
            let p3 = get_vec!(3)?;

            // n1: Normal vector calculated with two edges of the angle1.
            // n3: Normal vector calculated with two edges of the angle3.
            let n1 = (p0 - p1).cross(p1 - p2);
            let n3 = (p2 - p3).cross(p3 - p0);

            // If both angle1 and angle3 are concave, vectors n1 and n3 are
            // oriented in the same direction and `n1.dot(n3)` will be positive.
            // If either angle1 or angle3 is concave, vector n1 and n3 are
            // oriented in the opposite directions and `n1.dot(n3)` will be
            // negative.
            // It does not matter when the vertices of quadrangle is not on the
            // same plane, because whichever diagonal you choose, the cut will
            // be inaccurate.
            if n1.dot(n3) >= 0.0 {
                // Both angle1 and angle3 are concave.
                // This means that either angle0 or angle2 can be convex.
                // Cut from p0 to p2.
                results.extend_from_slice(&[
                    [poly_pvis[0], poly_pvis[1], poly_pvis[2]],
                    [poly_pvis[2], poly_pvis[3], poly_pvis[0]],
                ]);
            } else {
                // Either angle1 or angle3 is convex.
                // Cut from p1 to p3.
                results.extend_from_slice(&[
                    [poly_pvis[0], poly_pvis[1], poly_pvis[3]],
                    [poly_pvis[3], poly_pvis[1], poly_pvis[2]],
                ]);
            }
            Ok(())
        }
        n => {
            let points = (0..n).map(|i| get_vec!(i)).collect::<Result<Vec<_>, _>>()?;
            let points_2d: Vec<_> = {
                // Reduce dimensions for faster computation.
                // This helps treat points which are not on a single plane.
                let (min, max) =
                    bounding_box(&points).expect("Should never happen: there are 5 or more points");

                let width = max - min;

                match smallest_direction(&width) {
                    (x) if x.x > 0.0 => points
                        .into_iter()
                        .map(|v| Vector2::new(v[1], v[2]))
                        .collect(),
                    (x) if x.y > 0.0 => points
                        .into_iter()
                        .map(|v| Vector2::new(v[0], v[2]))
                        .collect(),
                    (x) => points
                        .into_iter()
                        .map(|v| Vector2::new(v[0], v[1]))
                        .collect(),
                }
            };
            // Normal directions.
            let normal_directions = {
                // 0 ... n-1
                let iter_cur = points_2d.iter();

                // n-1, 0, ... n-2
                let iter_prev = points_2d.iter().cycle().skip(n - 1);

                // 1, ... n-1, 0
                let iter_next = points_2d.iter().cycle().skip(1);

                iter_cur
                    .zip(iter_prev)
                    .zip(iter_next)
                    .map(|((cur, prev), next)| {
                        let prev_cur = *prev - *cur;
                        let cur_next = *cur - *next;
                        prev_cur.perp_dot(cur_next) > 0.0
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(normal_directions.len(), n);

            let dirs_true_count = normal_directions.iter().filter(|&&v| v).count();

            if dirs_true_count <= 1 || dirs_true_count >= n - 1 {
                // Zero or one angles are concave.
                let minor_sign = dirs_true_count <= 1;

                // If there are no concave angles, use 0 as center.
                let convex_index = normal_directions
                    .iter()
                    .position(|&sign| sign == minor_sign)
                    .unwrap_or(0);

                let convex_pvi = poly_pvis[convex_index];

                let iter1 = (0..n)
                    .cycle()
                    .skip(convex_index + 1)
                    .take(n - 2)
                    .map(|i| poly_pvis[i]);

                let iter2 = (0..n).cycle().skip(convex_index + 2).map(|i| poly_pvis[i]);

                for (pvi1, pvi2) in iter1.zip(iter2) {
                    results.push([convex_pvi, pvi1, pvi2]);
                }

                Ok(())
            } else {
                log!(
                    "Unsupported polygon: {}-gon with two or more concave angles",
                    n
                );
                Err(anyhow!("Unsupported polygon"))
            }
        }
    }
}

fn get_vec(pvs: &PolygonVertices<'_>, pvi: PolygonVertexIndex) -> anyhow::Result<Vector3<f32>> {
    pvs.control_point(pvi)
        .map(|p| vec3_from_array([p.x as f32, p.y as f32, p.z as f32]))
        .ok_or_else(|| anyhow!("Index out of range: {pvi:?}"))
}

fn bounding_box<'a>(
    points: impl IntoIterator<Item=&'a Vector3<f32>>,
) -> Option<(Vector3<f32>, Vector3<f32>)> {
    points.into_iter().fold(None, |minmax, point| {
        minmax.map_or_else(
            || Some((*point, *point)),
            |(min, max)| {
                Some((
                    Vector3::new(min.x.min(point.x), min.y.min(point.y), min.z.min(point.z)),
                    Vector3::new(max.x.max(point.x), max.y.max(point.y), max.z.max(point.z)),
                ))
            },
        )
    })
}

fn smallest_direction(v: &Vector3<f32>) -> Vector3<f32> {
    match () {
        () if v.x < v.y && v.z < v.x => Vector3::new(0.0, 0.0, 1.0),
        () if v.x < v.y => Vector3::new(1.0, 0.0, 0.0),
        () if v.z < v.y => Vector3::new(0.0, 0.0, 1.0),
        () => Vector3::new(0.0, 1.0, 0.0),
    }
}

#[derive(Clone, Debug, Default)]
pub struct FbxModel {
    pub fbx_data: Vec<FbxData>,
    pub animations: Vec<FbxAnimation>,
}

impl FbxModel {
    /// 指定したアニメーションで頂点を更新
    ///
    /// # Arguments
    /// * `animation_index` - アニメーションのインデックス
    /// * `time` - アニメーション時間（秒）
    ///
    /// # Example
    /// ```ignore
    /// // アニメーションを0.5秒の位置で更新
    /// fbx_model.update_animation(0, 0.5);
    /// ```
    pub fn update_animation(&mut self, animation_index: usize, time: f32) {
        if let Some(animation) = self.animations.get(animation_index) {
            for fbx_data in &mut self.fbx_data {
                fbx_data.update_animation(animation, time);
            }
        }
    }

    /// アニメーションの長さ（秒）を取得
    pub fn get_animation_duration(&self, animation_index: usize) -> Option<f32> {
        self.animations.get(animation_index).map(|anim| anim.duration)
    }

    /// アニメーション数を取得
    pub fn animation_count(&self) -> usize {
        self.animations.len()
    }
}

/// FBXアニメーション全体
#[derive(Clone, Debug)]
pub struct FbxAnimation {
    pub name: String,
    pub duration: f32,  // アニメーションの長さ（秒）
    pub bone_animations: std::collections::HashMap<String, BoneAnimation>,
}

/// ボーンごとのアニメーション
#[derive(Clone, Debug)]
pub struct BoneAnimation {
    pub bone_name: String,
    pub translation_keys: Vec<KeyFrame<[f32; 3]>>,
    pub rotation_keys: Vec<KeyFrame<[f32; 3]>>,
    pub scale_keys: Vec<KeyFrame<[f32; 3]>>,
}

/// キーフレーム
#[derive(Clone, Debug)]
pub struct KeyFrame<T> {
    pub time: f32,  // 秒
    pub value: T,
}

/// 指定時間でのボーン変換行列を計算
fn evaluate_bone_transform_at_time(
    bone_anim: &BoneAnimation,
    time: f32,
) -> Matrix4<f32> {
    // Translation を補間
    let translation = if !bone_anim.translation_keys.is_empty() {
        interpolate_vec3_at_time(&bone_anim.translation_keys, time)
    } else {
        [0.0, 0.0, 0.0]
    };

    // Rotation を補間（度数法）
    let rotation = if !bone_anim.rotation_keys.is_empty() {
        interpolate_vec3_at_time(&bone_anim.rotation_keys, time)
    } else {
        [0.0, 0.0, 0.0]
    };

    // Scale を補間
    let scale = if !bone_anim.scale_keys.is_empty() {
        interpolate_vec3_at_time(&bone_anim.scale_keys, time)
    } else {
        [1.0, 1.0, 1.0]
    };

    // T * R * S の順で行列を構築
    let translation_matrix = Matrix4::from_translation(vec3(translation[0], translation[1], translation[2]));

    // FBX rotation is in degrees, convert to radians
    let rotation_x = Matrix4::from_angle_x(Rad((rotation[0] as f32).to_radians()));
    let rotation_y = Matrix4::from_angle_y(Rad((rotation[1] as f32).to_radians()));
    let rotation_z = Matrix4::from_angle_z(Rad((rotation[2] as f32).to_radians()));
    let rotation_matrix = rotation_z * rotation_y * rotation_x;

    let scale_matrix = Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);

    translation_matrix * rotation_matrix * scale_matrix
}

/// Vec3キーフレームを補間
fn interpolate_vec3_at_time(keyframes: &[KeyFrame<[f32; 3]>], time: f32) -> [f32; 3] {
    if keyframes.is_empty() {
        return [0.0, 0.0, 0.0];
    }

    if keyframes.len() == 1 {
        return keyframes[0].value;
    }

    // 時間がキーフレームの範囲外の場合
    if time <= keyframes[0].time {
        return keyframes[0].value;
    }
    if time >= keyframes[keyframes.len() - 1].time {
        return keyframes[keyframes.len() - 1].value;
    }

    // 線形補間
    for i in 0..keyframes.len() - 1 {
        if time >= keyframes[i].time && time <= keyframes[i + 1].time {
            let t = (time - keyframes[i].time) / (keyframes[i + 1].time - keyframes[i].time);
            return [
                keyframes[i].value[0] + t * (keyframes[i + 1].value[0] - keyframes[i].value[0]),
                keyframes[i].value[1] + t * (keyframes[i + 1].value[1] - keyframes[i].value[1]),
                keyframes[i].value[2] + t * (keyframes[i + 1].value[2] - keyframes[i].value[2]),
            ];
        }
    }

    keyframes[0].value
}

/// スキニングされた頂点位置を計算
pub fn apply_skinning(
    original_positions: &[Vector3<f32>],
    clusters: &[ClusterInfo],
    animation: &FbxAnimation,
    time: f32,
) -> Vec<Vector3<f32>> {
    let num_vertices = original_positions.len();
    let mut skinned_positions = original_positions.to_vec();

    // 各ボーンのスキニング行列を計算
    let mut bone_matrices: HashMap<String, Matrix4<f32>> = HashMap::new();

    for cluster in clusters {
        let bone_name = &cluster.bone_name;

        // アニメーションからボーン変換を取得
        let bone_transform = if let Some(bone_anim) = animation.bone_animations.get(bone_name) {
            evaluate_bone_transform_at_time(bone_anim, time)
        } else {
            // アニメーションがない場合は単位行列
            Matrix4::identity()
        };

        // スキニング行列 = BoneTransform * InverseBindPose
        let skinning_matrix = bone_transform * cluster.inverse_bind_pose;
        bone_matrices.insert(bone_name.clone(), skinning_matrix);
    }

    // 各頂点にスキニングを適用
    let mut vertex_transforms: Vec<Vec<(usize, f32)>> = vec![Vec::new(); num_vertices];

    // クラスターから頂点への影響を収集
    for (cluster_idx, cluster) in clusters.iter().enumerate() {
        for (i, &vertex_idx) in cluster.vertex_indices.iter().enumerate() {
            if vertex_idx < num_vertices && i < cluster.vertex_weights.len() {
                let weight = cluster.vertex_weights[i];
                vertex_transforms[vertex_idx].push((cluster_idx, weight));
            }
        }
    }

    // 各頂点を変換
    for (vertex_idx, transforms) in vertex_transforms.iter().enumerate() {
        if transforms.is_empty() {
            continue; // スキニングの影響を受けない頂点
        }

        let original_pos = original_positions[vertex_idx];
        let mut weighted_pos = Vector3::new(0.0, 0.0, 0.0);
        let mut total_weight = 0.0;

        for &(cluster_idx, weight) in transforms {
            let cluster = &clusters[cluster_idx];
            if let Some(bone_matrix) = bone_matrices.get(&cluster.bone_name) {
                // 頂点を変換
                let transformed = bone_matrix.transform_point(Point3::from_vec(original_pos));
                weighted_pos += transformed.to_vec() * weight;
                total_weight += weight;
            }
        }

        // ウェイトを正規化
        if total_weight > 0.0 {
            skinned_positions[vertex_idx] = weighted_pos / total_weight;
        }
    }

    skinned_positions
}

/// AnimCurveからキーフレームデータを抽出
fn extract_animation_curve(curve_obj: &ObjectHandle) -> Result<Vec<KeyFrame<f32>>> {
    let node = curve_obj.node();

    let mut key_times: Vec<f64> = Vec::new();
    let mut key_values: Vec<f32> = Vec::new();

    // 子ノードから"KeyTime"と"KeyValueFloat"を探す
    for child in node.children() {
        let name = child.name();
        match name {
            "KeyTime" => {
                if let Some(attr) = child.attributes().get(0) {
                    if let Ok(arr) = attr.get_arr_i64_or_type() {
                        // FBX時間は内部的に整数で保存され、46186158000で1秒
                        const FBX_TIME_UNIT: f64 = 46186158000.0;
                        key_times = arr.iter().map(|&t| t as f64 / FBX_TIME_UNIT).collect();
                    }
                }
            }
            "KeyValueFloat" => {
                if let Some(attr) = child.attributes().get(0) {
                    if let Ok(arr) = attr.get_arr_f32_or_type() {
                        key_values = arr.to_vec();
                    }
                }
            }
            _ => {}
        }
    }

    // キーフレームを構築
    let mut keyframes = Vec::new();
    let count = key_times.len().min(key_values.len());
    for i in 0..count {
        keyframes.push(KeyFrame {
            time: key_times[i] as f32,
            value: key_values[i],
        });
    }

    if keyframes.is_empty() {
        log!("      Warning: No keyframes extracted from AnimCurve");
    }

    Ok(keyframes)
}

/// AnimCurveNodeからアニメーションカーブを抽出（X, Y, Z成分）
fn extract_anim_curve_node(curve_node_obj: &ObjectHandle, doc: &Document) -> Result<(Vec<KeyFrame<f32>>, Vec<KeyFrame<f32>>, Vec<KeyFrame<f32>>)> {
    let mut curves_x = Vec::new();
    let mut curves_y = Vec::new();
    let mut curves_z = Vec::new();

    let mut curve_count = 0;
    // AnimCurveNodeに接続されているAnimCurveを探す
    for conn in curve_node_obj.source_objects() {
        if let Some(curve_obj) = conn.object_handle() {
            if curve_obj.class() == "AnimCurve" {
                curve_count += 1;
                // 接続ラベル（"d|X", "d|Y", "d|Z"など）でどの軸か判別
                let label = conn.label().unwrap_or("");

                match extract_animation_curve(&curve_obj) {
                    Ok(keyframes) => {
                        log!("    Extracted {} keyframes for label '{}'", keyframes.len(), label);
                        if label.contains("X") {
                            curves_x = keyframes;
                        } else if label.contains("Y") {
                            curves_y = keyframes;
                        } else if label.contains("Z") {
                            curves_z = keyframes;
                        }
                    }
                    Err(e) => {
                        log!("Warning: Failed to extract curve for label {}: {}", label, e);
                    }
                }
            }
        }
    }

    log!("    Found {} AnimCurves (X:{}, Y:{}, Z:{} keyframes)", curve_count, curves_x.len(), curves_y.len(), curves_z.len());

    Ok((curves_x, curves_y, curves_z))
}

/// AnimLayerからボーンごとのアニメーションを抽出
fn extract_anim_layer(layer_obj: &ObjectHandle, doc: &Document) -> Result<HashMap<String, BoneAnimation>> {
    let mut bone_animations = HashMap::new();

    log!("Processing AnimLayer: {}", layer_obj.name().unwrap_or("Unnamed"));

    // AnimLayerに接続されているAnimCurveNodeを探す
    let mut curve_node_count = 0;
    for conn in layer_obj.source_objects() {
        if let Some(curve_node_obj) = conn.object_handle() {
            log!("  Checking connection: class='{}', name='{}'", curve_node_obj.class(), curve_node_obj.name().unwrap_or(""));
            if curve_node_obj.class() == "AnimCurveNode" {
                curve_node_count += 1;
                // AnimCurveNodeが影響するモデル（ボーン）を探す
                for target_conn in curve_node_obj.destination_objects() {
                    if let Some(target_obj) = target_conn.object_handle() {
                        if target_obj.class() == "Model" {
                            let bone_name = target_obj.name().unwrap_or("Unknown").to_string();
                            let curve_node_name = curve_node_obj.name().unwrap_or("");

                            log!("Found AnimCurveNode '{}' for bone '{}'", curve_node_name, bone_name);

                            // カーブノードのタイプを判別（T=Translation, R=Rotation, S=Scaling）
                            let (curves_x, curves_y, curves_z) = extract_anim_curve_node(&curve_node_obj, doc)?;

                            // ボーンアニメーションを取得または作成
                            let bone_anim = bone_animations.entry(bone_name.clone()).or_insert_with(|| {
                                BoneAnimation {
                                    bone_name: bone_name.clone(),
                                    translation_keys: Vec::new(),
                                    rotation_keys: Vec::new(),
                                    scale_keys: Vec::new(),
                                }
                            });

                            // プロパティ名で判別
                            if curve_node_name.contains("T") || curve_node_name.contains("Lcl Translation") {
                                // Translationカーブを統合
                                bone_anim.translation_keys = merge_xyz_curves(curves_x, curves_y, curves_z);
                            } else if curve_node_name.contains("R") || curve_node_name.contains("Lcl Rotation") {
                                // Rotationカーブを統合
                                bone_anim.rotation_keys = merge_xyz_curves(curves_x, curves_y, curves_z);
                            } else if curve_node_name.contains("S") || curve_node_name.contains("Lcl Scaling") {
                                // Scalingカーブを統合
                                bone_anim.scale_keys = merge_xyz_curves(curves_x, curves_y, curves_z);
                            }
                        }
                    }
                }
            }
        }
    }

    log!("  Found {} AnimCurveNodes, extracted {} bone animations", curve_node_count, bone_animations.len());
    Ok(bone_animations)
}

/// X, Y, Zの個別カーブを[f32; 3]のカーブに統合
fn merge_xyz_curves(
    curves_x: Vec<KeyFrame<f32>>,
    curves_y: Vec<KeyFrame<f32>>,
    curves_z: Vec<KeyFrame<f32>>,
) -> Vec<KeyFrame<[f32; 3]>> {
    // すべての時間を収集してソート
    let mut all_times: Vec<f32> = Vec::new();
    for kf in &curves_x {
        if !all_times.contains(&kf.time) {
            all_times.push(kf.time);
        }
    }
    for kf in &curves_y {
        if !all_times.contains(&kf.time) {
            all_times.push(kf.time);
        }
    }
    for kf in &curves_z {
        if !all_times.contains(&kf.time) {
            all_times.push(kf.time);
        }
    }
    all_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // 各時間でのX, Y, Z値を補間して統合
    let mut merged = Vec::new();
    for &time in &all_times {
        let x = interpolate_at_time(&curves_x, time);
        let y = interpolate_at_time(&curves_y, time);
        let z = interpolate_at_time(&curves_z, time);

        merged.push(KeyFrame {
            time,
            value: [x, y, z],
        });
    }

    merged
}

/// 指定時間での値を線形補間で取得
fn interpolate_at_time(keyframes: &[KeyFrame<f32>], time: f32) -> f32 {
    if keyframes.is_empty() {
        return 0.0;
    }

    if keyframes.len() == 1 {
        return keyframes[0].value;
    }

    // 時間がキーフレームの範囲外の場合
    if time <= keyframes[0].time {
        return keyframes[0].value;
    }
    if time >= keyframes[keyframes.len() - 1].time {
        return keyframes[keyframes.len() - 1].value;
    }

    // 線形補間
    for i in 0..keyframes.len() - 1 {
        if time >= keyframes[i].time && time <= keyframes[i + 1].time {
            let t = (time - keyframes[i].time) / (keyframes[i + 1].time - keyframes[i].time);
            return keyframes[i].value + t * (keyframes[i + 1].value - keyframes[i].value);
        }
    }

    keyframes[0].value
}

/// AnimStackからアニメーションデータを抽出
fn extract_anim_stack(stack_obj: &ObjectHandle, doc: &Document) -> Result<FbxAnimation> {
    let name = stack_obj.name().unwrap_or("DefaultAnimation").to_string();
    log!("Processing AnimStack: {}", name);

    let mut all_bone_animations = HashMap::new();
    let mut duration = 0.0f32;

    // AnimStackに接続されているAnimLayerを探す
    let mut layer_count = 0;
    for conn in stack_obj.source_objects() {
        if let Some(layer_obj) = conn.object_handle() {
            log!("  Checking source object: class='{}', name='{}'", layer_obj.class(), layer_obj.name().unwrap_or(""));
            if layer_obj.class() == "AnimLayer" {
                layer_count += 1;
                match extract_anim_layer(&layer_obj, doc) {
                    Ok(bone_anims) => {
                        // レイヤーのアニメーションをマージ
                        for (bone_name, bone_anim) in bone_anims {
                            // 最大時間を更新
                            for kf in &bone_anim.translation_keys {
                                if kf.time > duration {
                                    duration = kf.time;
                                }
                            }
                            for kf in &bone_anim.rotation_keys {
                                if kf.time > duration {
                                    duration = kf.time;
                                }
                            }
                            for kf in &bone_anim.scale_keys {
                                if kf.time > duration {
                                    duration = kf.time;
                                }
                            }

                            all_bone_animations.insert(bone_name, bone_anim);
                        }
                    }
                    Err(e) => {
                        log!("Warning: Failed to extract AnimLayer: {}", e);
                    }
                }
            }
        }
    }

    log!("AnimStack '{}': found {} layers, duration: {} seconds, {} bones", name, layer_count, duration, all_bone_animations.len());

    Ok(FbxAnimation {
        name,
        duration,
        bone_animations: all_bone_animations,
    })
}

/// Clusterから頂点インデックスとウェイトを抽出
fn extract_cluster_data(cluster: &ClusterHandle) -> Result<ClusterInfo> {
    // ClusterHandleからObjectHandleへの参照を取得
    let cluster_obj: &ObjectHandle = &**cluster;

    // ボーン名を取得（クラスターが接続されているモデルノード）
    let bone_name = cluster_obj
        .source_objects()
        .filter(|obj| obj.label().is_none())
        .filter_map(|obj| obj.object_handle())
        .filter_map(|obj| match obj.get_typed() {
            TypedObjectHandle::Model(_) => Some(obj.name().unwrap_or("Unknown")),
            _ => None,
        })
        .next()
        .unwrap_or("Unknown")
        .to_string();

    log!("Processing cluster for bone: {}", bone_name);

    // 低レベルノードにアクセス
    let node = cluster_obj.node();

    // 頂点インデックスとウェイトを格納する配列
    let mut vertex_indices = Vec::new();
    let mut vertex_weights = Vec::new();

    // 子ノードを走査して"Indexes"と"Weights"を探す
    for child in node.children() {
        let name = child.name();
        match name {
            "Indexes" => {
                // インデックス配列を取得
                if let Some(attr) = child.attributes().get(0) {
                    if let Ok(arr) = attr.get_arr_i32_or_type() {
                        vertex_indices = arr.iter().map(|&i| i as usize).collect();
                        log!("Found {} vertex indices", vertex_indices.len());
                    }
                }
            }
            "Weights" => {
                // ウェイト配列を取得
                if let Some(attr) = child.attributes().get(0) {
                    if let Ok(arr) = attr.get_arr_f64_or_type() {
                        vertex_weights = arr.iter().map(|&w| w as f32).collect();
                        log!("Found {} vertex weights", vertex_weights.len());
                    }
                }
            }
            _ => {}
        }
    }

    // TransformとTransformLinkマトリクスを取得
    let props = cluster_obj.properties_by_native_typename("Cluster");

    let transform = if let Some(prop) = props.get_property("Transform") {
        if let Ok(values) = prop.load_value(F64Arr16Loader) {
            // FBX stores matrices in row-major, cgmath uses column-major
            Matrix4::new(
                values[0] as f32, values[4] as f32, values[8] as f32, values[12] as f32,
                values[1] as f32, values[5] as f32, values[9] as f32, values[13] as f32,
                values[2] as f32, values[6] as f32, values[10] as f32, values[14] as f32,
                values[3] as f32, values[7] as f32, values[11] as f32, values[15] as f32,
            )
        } else {
            Matrix4::identity()
        }
    } else {
        Matrix4::identity()
    };

    let transform_link = if let Some(prop) = props.get_property("TransformLink") {
        if let Ok(values) = prop.load_value(F64Arr16Loader) {
            Matrix4::new(
                values[0] as f32, values[4] as f32, values[8] as f32, values[12] as f32,
                values[1] as f32, values[5] as f32, values[9] as f32, values[13] as f32,
                values[2] as f32, values[6] as f32, values[10] as f32, values[14] as f32,
                values[3] as f32, values[7] as f32, values[11] as f32, values[15] as f32,
            )
        } else {
            Matrix4::identity()
        }
    } else {
        Matrix4::identity()
    };

    // 逆バインドポーズ行列を計算: inverse(TransformLink) * Transform
    let inverse_bind_pose = if let Some(inv_tl) = transform_link.invert() {
        inv_tl * transform
    } else {
        log!("Warning: Could not invert TransformLink matrix for bone {}", bone_name);
        Matrix4::identity()
    };

    log!(
        "Cluster {} - Transform: {:?}, TransformLink: {:?}",
        bone_name,
        transform,
        transform_link
    );

    Ok(ClusterInfo {
        bone_name,
        transform,
        transform_link,
        inverse_bind_pose,
        vertex_indices,
        vertex_weights,
    })
}

/// Cluster情報（バインドポーズ）を保持する構造体
#[derive(Clone, Debug)]
pub struct ClusterInfo {
    pub bone_name: String,
    pub transform: Matrix4<f32>,           // メッシュの初期変換
    pub transform_link: Matrix4<f32>,      // ボーンの初期変換（バインドポーズ）
    pub inverse_bind_pose: Matrix4<f32>,   // 計算済み逆バインドポーズ
    pub vertex_indices: Vec<usize>,        // 影響を受ける頂点のインデックス
    pub vertex_weights: Vec<f32>,          // 各頂点のウェイト値
}

#[derive(Clone, Debug)]
pub struct FbxData {
    pub positions: Vec<Vector3<f32>>,        // ワールド座標の頂点位置（表示用）
    pub local_positions: Vec<Vector3<f32>>,  // ローカル座標の頂点位置（スキニング用）
    pub indices: Vec<u32>,
    pub clusters: Vec<ClusterInfo>,          // スキニング情報
}

impl FbxData {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            local_positions: Vec::new(),
            indices: Vec::new(),
            clusters: Vec::new(),
        }
    }

    /// アニメーション時間に基づいて頂点位置を更新
    pub fn update_animation(&mut self, animation: &FbxAnimation, time: f32) {
        if self.clusters.is_empty() || self.local_positions.is_empty() {
            return; // スキニング情報がない場合はスキップ
        }

        // スキニングを適用
        self.positions = apply_skinning(&self.local_positions, &self.clusters, animation, time);
    }
}
