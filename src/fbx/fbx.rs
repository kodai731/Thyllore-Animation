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
use russimp::scene::{PostProcess, Scene};
use cgmath::Vector3;

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
    let pre_rotation = get_vec3("PreRotation", [0.0, 0.0, 0.0]);
    let post_rotation = get_vec3("PostRotation", [0.0, 0.0, 0.0]);
    let geometric_translation = get_vec3("GeometricTranslation", [0.0, 0.0, 0.0]);
    let geometric_rotation = get_vec3("GeometricRotation", [0.0, 0.0, 0.0]);
    let geometric_scaling = get_vec3("GeometricScaling", [1.0, 1.0, 1.0]);

    // Read RotationOrder (0: XYZ, 1: XZY, 2: YZX, 3: YXZ, 4: ZXY, 5: ZYX)
    let rotation_order = if let Some(prop) = props.get_property("RotationOrder") {
        prop.load_value(StrictF64Loader).ok().map(|v| v as i32).unwrap_or(0)
    } else {
        0 // Default to XYZ
    };

    log!(
        "Mesh: {}, Local transform - Translation: {:?}, Rotation (deg): {:?}, Scaling: {:?}, RotationOrder: {}",
        mesh_name,
        translation,
        rotation,
        scaling,
        rotation_order
    );

    // Log additional properties for all meshes (useful for debugging)
    log!("  {} additional properties:", mesh_name);
    log!("    PreRotation: {:?}", pre_rotation);
    log!("    PostRotation: {:?}", post_rotation);
    log!("    GeometricTranslation: {:?}", geometric_translation);
    log!("    GeometricRotation: {:?}", geometric_rotation);
    log!("    GeometricScaling: {:?}", geometric_scaling);

    // Optional: Test rotation patterns for debugging coordinate system issues
    // This can be enabled/disabled via environment variable or removed entirely
    if std::env::var("FBX_DEBUG_ROTATION").is_ok() {
        log!("  DEBUG: Testing rotation patterns for {}:", mesh_name);
        log!("    FBX Rotation: {:?}", rotation);

        // Test coordinate system conversion
        // FBX might have Z-up to Y-up conversion baked in
        // Try converting rotation from Z-up to Y-up (or vice versa)

        // Pattern 1: Direct use of FBX rotation
        let rx1 = Matrix4::from_angle_x(Rad((rotation[0] as f32).to_radians()));
        let ry1 = Matrix4::from_angle_y(Rad((rotation[1] as f32).to_radians()));
        let rz1 = Matrix4::from_angle_z(Rad((rotation[2] as f32).to_radians()));
        let s = Matrix4::from_nonuniform_scale(scaling[0], scaling[1], scaling[2]);
        let mat1 = rx1 * ry1 * rz1 * s;
        log!("    Pattern 1 (X*Y*Z): [{:?}, {:?}, {:?}]", mat1.x, mat1.y, mat1.z);

        // Pattern 2: Negate Y rotation (convert between coordinate systems)
        let rx2 = Matrix4::from_angle_x(Rad((rotation[0] as f32).to_radians()));
        let ry2 = Matrix4::from_angle_y(Rad((-rotation[1] as f32).to_radians()));
        let rz2 = Matrix4::from_angle_z(Rad((rotation[2] as f32).to_radians()));
        let mat2 = rx2 * ry2 * rz2 * s;
        log!("    Pattern 2 (X*-Y*Z): [{:?}, {:?}, {:?}]", mat2.x, mat2.y, mat2.z);

        // Pattern 3: Negate Z rotation
        let rx3 = Matrix4::from_angle_x(Rad((rotation[0] as f32).to_radians()));
        let ry3 = Matrix4::from_angle_y(Rad((rotation[1] as f32).to_radians()));
        let rz3 = Matrix4::from_angle_z(Rad((-rotation[2] as f32).to_radians()));
        let mat3 = rx3 * ry3 * rz3 * s;
        log!("    Pattern 3 (X*Y*-Z): [{:?}, {:?}, {:?}]", mat3.x, mat3.y, mat3.z);

        // Pattern 4: Swap Y and Z components
        let converted_rot = [rotation[0], rotation[2], rotation[1]]; // Swap Y and Z
        let rx4 = Matrix4::from_angle_x(Rad((converted_rot[0] as f32).to_radians()));
        let ry4 = Matrix4::from_angle_y(Rad((converted_rot[1] as f32).to_radians()));
        let rz4 = Matrix4::from_angle_z(Rad((converted_rot[2] as f32).to_radians()));
        let mat4 = rx4 * ry4 * rz4 * s;
        log!("    Pattern 4 (Swap Y<->Z): [{:?}, {:?}, {:?}]", mat4.x, mat4.y, mat4.z);

        // Pattern 5: Apply Z-up to Y-up coordinate conversion AFTER rotation
        // Conversion matrix: swap Y and Z, negate new Z
        let mat1_copy = mat1.clone();
        let zup_to_yup = Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, -1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        );
        let mat5 = zup_to_yup * mat1_copy;
        log!("    Pattern 5 (Z-up->Y-up * X*Y*Z): [{:?}, {:?}, {:?}]", mat5.x, mat5.y, mat5.z);

        // Pattern 6: Apply Z-up to Y-up coordinate conversion BEFORE rotation
        let mat6 = mat1_copy * zup_to_yup;
        log!("    Pattern 6 (X*Y*Z * Z-up->Y-up): [{:?}, {:?}, {:?}]", mat6.x, mat6.y, mat6.z);

        // Pattern 7: Z-up to Y-up with X negation
        let zup_to_yup_negx = Matrix4::new(
            -1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, -1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        );
        let mat7 = zup_to_yup_negx * mat1_copy;
        log!("    Pattern 7 (Z-up->Y-up with -X): [{:?}, {:?}, {:?}]", mat7.x, mat7.y, mat7.z);

        // Pattern 8: Different Z-up to Y-up conversion
        let zup_to_yup_alt = Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, 0.0, -1.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        );
        let mat8 = zup_to_yup_alt * mat1_copy;
        log!("    Pattern 8 (Alt Z-up->Y-up): [{:?}, {:?}, {:?}]", mat8.x, mat8.y, mat8.z);

        // Pattern 9: Correct Z*Y*X rotation order (what we now use)
        let mat9 = rz1 * ry1 * rx1 * s;
        log!("    Pattern 9 (Z*Y*X): [{:?}, {:?}, {:?}]", mat9.x, mat9.y, mat9.z);

        // Pattern 10: Z*Y*X with Z-up to Y-up conversion
        let mat10_conv = zup_to_yup * mat9;
        log!("    Pattern 10 (Z-up->Y-up * Z*Y*X): [{:?}, {:?}, {:?}]", mat10_conv.x, mat10_conv.y, mat10_conv.z);

        // Pattern 11-16: More systematic testing
        // Blender col0=[0, 0, -37.549], col1=[-37.549, 0, 0], col2=[0, 37.549, 0]
        // Pattern 9 col0=[~0, ~0, 37.549], col1=[-37.549, ~0, ~0], col2=[0, -37.549, ~0]
        // Need to negate Z component of col0 and Y component of col2

        let test_matrices = [
            ("Rot 180° around Y", Matrix4::new(-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0)),
            ("Rot 90° around X", Matrix4::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)),
            ("Rot -90° around X", Matrix4::new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)),
            ("Rot 90° around Z", Matrix4::new(0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0)),
            ("Rot -90° around Z", Matrix4::new(0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0)),
            ("Negate X and Z", Matrix4::new(-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0)),
        ];
        for (idx, (name, test_mat)) in test_matrices.iter().enumerate() {
            let result = *test_mat * mat9;
            log!("    Pattern {} ({}): [{:?}, {:?}, {:?}]", 11 + idx, name, result.x, result.y, result.z);
        }

        log!("    Note: Compare with expected values from your 3D editor");
    }

    // Build transform matrix: T * R * S
    let translation_matrix = Matrix4::from_translation(vec3(
        translation[0],
        translation[1],
        translation[2],
    ));

    // FBX rotation is in degrees, convert to radians
    let rotation_x = Matrix4::from_angle_x(Rad((rotation[0] as f32).to_radians()));
    let rotation_y = Matrix4::from_angle_y(Rad((rotation[1] as f32).to_radians()));
    let rotation_z = Matrix4::from_angle_z(Rad((rotation[2] as f32).to_radians()));

    // Apply rotation in the order specified by RotationOrder
    // FBX RotationOrder specifies Euler angle application order
    // Matrix multiplication is in REVERSE order of angle application
    // 0: XYZ (angles) -> Z*Y*X (matrices), 1: XZY -> Y*Z*X, 2: YZX -> X*Z*Y, etc.
    let rotation_matrix = match rotation_order {
        0 => rotation_z * rotation_y * rotation_x, // XYZ angles -> Z*Y*X matrices
        1 => rotation_y * rotation_z * rotation_x, // XZY angles -> Y*Z*X matrices
        2 => rotation_x * rotation_z * rotation_y, // YZX angles -> X*Z*Y matrices
        3 => rotation_z * rotation_x * rotation_y, // YXZ angles -> Z*X*Y matrices
        4 => rotation_y * rotation_x * rotation_z, // ZXY angles -> Y*X*Z matrices
        5 => rotation_x * rotation_y * rotation_z, // ZYX angles -> X*Y*Z matrices
        _ => rotation_z * rotation_y * rotation_x, // Default to XYZ
    };

    // Apply PreRotation and PostRotation
    // FBX transform: T * Rpre * R * Rpost * S
    let pre_rot_x = Matrix4::from_angle_x(Rad((pre_rotation[0] as f32).to_radians()));
    let pre_rot_y = Matrix4::from_angle_y(Rad((pre_rotation[1] as f32).to_radians()));
    let pre_rot_z = Matrix4::from_angle_z(Rad((pre_rotation[2] as f32).to_radians()));
    let pre_rotation_matrix = pre_rot_z * pre_rot_y * pre_rot_x;

    let post_rot_x = Matrix4::from_angle_x(Rad((post_rotation[0] as f32).to_radians()));
    let post_rot_y = Matrix4::from_angle_y(Rad((post_rotation[1] as f32).to_radians()));
    let post_rot_z = Matrix4::from_angle_z(Rad((post_rotation[2] as f32).to_radians()));
    let post_rotation_matrix = post_rot_z * post_rot_y * post_rot_x;

    let scale_matrix = Matrix4::from_nonuniform_scale(
        scaling[0],
        scaling[1],
        scaling[2],
    );

    translation_matrix * pre_rotation_matrix * rotation_matrix * post_rotation_matrix * scale_matrix
}

/// メッシュの親ボーンを探す
fn find_parent_bone(mesh: &ModelHandle, _doc: &Document) -> Option<String> {
    let mesh_obj: &ObjectHandle = &**mesh;

    // 親オブジェクトを探す
    for conn in mesh_obj.destination_objects() {
        if let Some(parent_obj) = conn.object_handle() {
            // 親がModelの場合
            if parent_obj.class() == "Model" {
                let subclass = parent_obj.node().attributes()
                    .get(2)
                    .and_then(|attr| attr.get_string_or_type().ok())
                    .unwrap_or("");

                // LimbNodeまたはNullはボーン
                if subclass == "LimbNode" || subclass == "Null" {
                    if let Some(bone_name) = parent_obj.name() {
                        return Some(bone_name.to_string());
                    }
                }
            }
        }
    }

    None
}

/// 親ボーンに対するメッシュの相対変換を計算
fn calculate_mesh_local_transform(
    mesh: &ModelHandle,
    parent_bone_name: &str,
    doc: &Document,
) -> Matrix4<f32> {
    // メッシュのワールド変換を取得
    let mesh_world = get_world_transform(mesh, doc);

    // 親ボーンを探す
    for object in doc.objects() {
        if let Some(name) = object.name() {
            if name == parent_bone_name && object.class() == "Model" {
                if let TypedObjectHandle::Model(model) = object.get_typed() {
                    // 親ボーンのワールド変換を取得
                    let bone_world = get_world_transform(&model, doc);

                    // 相対変換 = inverse(親ボーンワールド) × メッシュワールド
                    if let Some(bone_world_inv) = bone_world.invert() {
                        return bone_world_inv * mesh_world;
                    }
                }
            }
        }
    }

    // 親ボーンが見つからない場合は単位行列
    mesh_world
}

/// Find a transform-bearing parent by skipping AnimCurve/AnimCurveNode objects
/// Returns the first parent that is a Model node (Mesh, Null, LimbNode, etc.)
fn find_transform_bearing_parent<'a>(obj: &ObjectHandle<'a>, doc: &'a Document) -> Option<ObjectHandle<'a>> {
    for conn in obj.destination_objects() {
        if let Some(parent_obj) = conn.object_handle() {
            let parent_class = parent_obj.class();

            // Skip materials and Geometry (Geometry doesn't have transforms)
            if parent_class == "Material" || parent_class == "Geometry" {
                continue;
            }

            // If parent is AnimCurve/AnimCurveNode, recursively search its parents
            if parent_class == "AnimCurve" || parent_class == "AnimCurveNode" {
                if let Some(transform_parent) = find_transform_bearing_parent(&parent_obj, doc) {
                    return Some(transform_parent);
                }
                continue;
            }

            // Check if this is a Model node (which can have transforms)
            if let TypedObjectHandle::Model(_) = parent_obj.get_typed() {
                return Some(parent_obj);
            }

            // For other object types, try to check if they have transform properties
            // by checking if they have FbxNode properties
            let props = parent_obj.properties_by_native_typename("FbxNode");
            if props.get_property("Lcl Translation").is_some()
                || props.get_property("Lcl Rotation").is_some()
                || props.get_property("Lcl Scaling").is_some() {
                // This object has transform properties
                return Some(parent_obj);
            }

            // If this object doesn't have transforms, recursively search its parents
            if let Some(transform_parent) = find_transform_bearing_parent(&parent_obj, doc) {
                return Some(transform_parent);
            }
        }
    }

    None
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
            let parent_name = parent_obj.name().unwrap_or("<no name>");
            let parent_class = parent_obj.class();

            // Skip materials and Geometry (Geometry doesn't have transforms)
            if parent_class == "Material" || parent_class == "Geometry" {
                continue;
            }

            // For AnimCurve/AnimCurveNode, skip to their parent
            if parent_class == "AnimCurve" || parent_class == "AnimCurveNode" {
                log!("  Mesh {} has AnimCurve/AnimCurveNode parent, searching for transform parent...", mesh_name);
                // Recursively search for a transform-bearing parent
                if let Some(transform_parent) = find_transform_bearing_parent(&parent_obj, doc) {
                    log!("    Found transform parent through AnimCurve chain");
                    let parent_world = get_world_transform_for_object(&transform_parent, doc);
                    let world_transform = parent_world * local_transform;
                    return world_transform;
                } else {
                    // No transform parent found through this AnimCurve, continue to next connection
                    continue;
                }
            }

            // Check if parent is a Model node
            if let TypedObjectHandle::Model(parent_model_typed) = parent_obj.get_typed() {
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
            } else {
                // Parent is not a Model type, but might still have transform properties
                log!("  Mesh {} has non-model parent: {} (class: {})", mesh_name, parent_name, parent_class);

                // Try to get transform from any object type
                let parent_world = get_object_transform(&parent_obj, doc);
                if parent_world != Matrix4::identity() {
                    log!("    Applied non-model parent transform");
                    let world_transform = parent_world * local_transform;
                    return world_transform;
                }
            }
        }
    }

    // No parent found, local transform is world transform
    log!("  No parent for {}, using local as world", mesh_name);

    // Apply Z-up to Y-up coordinate system conversion to all root meshes (no parent)
    // This is needed because FBX files may have coordinate transformations baked in
    // Pattern 12: Rotate 90° around X axis (swaps Y and Z, negates new Y)
    let zup_to_yup = Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 0.0, -1.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );
    log!("  Applying Z-up to Y-up conversion (Rot +90° X) to root mesh {}", mesh_name);
    zup_to_yup * local_transform
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

/// Get transform from any object type by reading properties directly
fn get_object_transform(obj: &ObjectHandle, doc: &Document) -> Matrix4<f32> {
    // Try to read transform properties from any object type
    let props = obj.properties_by_native_typename("FbxNode");

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

    // Read RotationOrder (0: XYZ, 1: XZY, 2: YZX, 3: YXZ, 4: ZXY, 5: ZYX)
    let rotation_order = if let Some(prop) = props.get_property("RotationOrder") {
        prop.load_value(StrictF64Loader).ok().map(|v| v as i32).unwrap_or(0)
    } else {
        0 // Default to XYZ
    };

    // Build local transform
    let translation_matrix = Matrix4::from_translation(vec3(
        translation[0],
        translation[1],
        translation[2],
    ));

    let rotation_x = Matrix4::from_angle_x(Rad((rotation[0] as f32).to_radians()));
    let rotation_y = Matrix4::from_angle_y(Rad((rotation[1] as f32).to_radians()));
    let rotation_z = Matrix4::from_angle_z(Rad((rotation[2] as f32).to_radians()));

    // Apply rotation in the order specified by RotationOrder
    // Note: Trying reversed order to match Blender's output
    let rotation_matrix = match rotation_order {
        0 => rotation_x * rotation_y * rotation_z, // XYZ (reversed from Z*Y*X)
        1 => rotation_x * rotation_z * rotation_y, // XZY
        2 => rotation_y * rotation_z * rotation_x, // YZX
        3 => rotation_y * rotation_x * rotation_z, // YXZ
        4 => rotation_z * rotation_x * rotation_y, // ZXY
        5 => rotation_z * rotation_y * rotation_x, // ZYX
        _ => rotation_x * rotation_y * rotation_z, // Default to XYZ (reversed)
    };

    let scale_matrix = Matrix4::from_nonuniform_scale(
        scaling[0],
        scaling[1],
        scaling[2],
    );

    let local_transform = translation_matrix * rotation_matrix * scale_matrix;

    // Check for parent and recursively get parent transform
    for conn in obj.destination_objects() {
        if let Some(parent_obj) = conn.object_handle() {
            let parent_class = parent_obj.class();

            // Skip materials and Geometry (Geometry doesn't have transforms)
            if parent_class == "Material" || parent_class == "Geometry" {
                continue;
            }

            // For AnimCurve/AnimCurveNode, skip to their parent
            if parent_class == "AnimCurve" || parent_class == "AnimCurveNode" {
                // Recursively search for a transform-bearing parent
                if let Some(transform_parent) = find_transform_bearing_parent(&parent_obj, doc) {
                    let parent_world = get_object_transform(&transform_parent, doc);
                    return parent_world * local_transform;
                } else {
                    // No transform parent found through this AnimCurve, continue to next connection
                    continue;
                }
            }

            // Get parent transform recursively
            let parent_world = get_object_transform(&parent_obj, doc);
            return parent_world * local_transform;
        }
    }

    // No parent, return local transform
    local_transform
}

pub unsafe fn load_fbx(path: &str) -> anyhow::Result<(FbxModel)> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut fbx_model = FbxModel::default();

    log!("=== Loading FBX file: {} ===", path);

    // 各メッシュごとにFbxDataを作成するため、ここでは初期化しない
    let doc = AnyDocument::from_reader(reader)
        .context(format!("Failed to parse FBX file: {}. This may be due to NodeLengthMismatch or other parsing errors.", path))?;

    match doc {
        AnyDocument::V7400(fbx_ver, doc) => {
            // TODO: Read GlobalSettings to detect coordinate system conversion
            // Currently, we assume the FBX file has coordinate system transformations baked in

            // First, log all objects to understand the hierarchy
            log!("=== FBX Object Hierarchy ===");
            for object in doc.objects() {
                let obj_name = object.name().unwrap_or("unnamed");
                log!("Object: name='{}', class='{}', subclass='{}'",
                    obj_name, object.class(), object.subclass());

                // Log parent connections (source = parent)
                for conn in object.source_objects() {
                    if let Some(src) = conn.object_handle() {
                        log!("  <- Parent: name='{}', class='{}'",
                            src.name().unwrap_or(""), src.class());
                    }
                }
            }

            // Populate BoneNodes
            log!("=== Populating Bone Nodes ===");
            for object in doc.objects() {
                if let TypedObjectHandle::Model(model) = object.get_typed() {
                    let name = model.name().unwrap_or("").to_string();
                    if name.is_empty() { continue; }

                    // Find parent
                    let mut parent = None;
                    for conn in object.destination_objects() {
                        if let Some(parent_obj) = conn.object_handle() {
                            if parent_obj.class() == "Model" {
                                parent = parent_obj.name().map(|s| s.to_string());
                                break; // Assume single parent
                            }
                        }
                    }

                    // Get transform properties
                    let props = object.properties_by_native_typename("FbxNode");
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

                    // Debug: Log translation for specific bones
                    if name == "b_Head" || name == "b_Neck_2" || name == "b_Root" {
                        log!("DEBUG: Bone '{}' - Lcl Translation (FBX Y-up): [{:.3}, {:.3}, {:.3}]", name, translation[0], translation[1], translation[2]);
                        log!("DEBUG: Bone '{}' - Lcl Rotation: [{:.3}, {:.3}, {:.3}]", name, rotation[0], rotation[1], rotation[2]);
                    }

                    // Build local transform matrix
                    let translation_matrix = Matrix4::from_translation(vec3(translation[0], translation[1], translation[2]));
                    let rotation_x = Matrix4::from_angle_x(Rad((rotation[0] as f32).to_radians()));
                    let rotation_y = Matrix4::from_angle_y(Rad((rotation[1] as f32).to_radians()));
                    let rotation_z = Matrix4::from_angle_z(Rad((rotation[2] as f32).to_radians()));
                    let rotation_matrix = rotation_z * rotation_y * rotation_x;
                    let scale_matrix = Matrix4::from_nonuniform_scale(scaling[0], scaling[1], scaling[2]);

                    let local_transform_fbx = translation_matrix * rotation_matrix * scale_matrix;

                    // Apply coordinate system conversion (Y-up → Z-up) for RootNode and its direct children (armature roots)
                    // This is necessary because inverse_bind_pose contains coordinate conversion,
                    // so bone_transform must also contain it for skinning to work correctly
                    let needs_coord_conversion = parent.is_none()
                        || name == "RootNode"
                        || parent.as_ref().map_or(false, |p| p == "RootNode");

                    let local_transform = if needs_coord_conversion {
                        // Coordinate conversion matrix: Y-up → Z-up (90-degree X-axis rotation)
                        // [1, 0, 0, 0; 0, 0, -1, 0; 0, 1, 0, 0; 0, 0, 0, 1]
                        let coord_conversion = Matrix4::new(
                            1.0, 0.0, 0.0, 0.0,
                            0.0, 0.0, -1.0, 0.0,
                            0.0, 1.0, 0.0, 0.0,
                            0.0, 0.0, 0.0, 1.0,
                        );
                        coord_conversion * local_transform_fbx
                    } else {
                        local_transform_fbx
                    };

                    if name == "b_Head" || name == "b_Neck_2" || name == "b_Root" {
                        log!("DEBUG: Bone '{}' - Local transform position: [{:.3}, {:.3}, {:.3}]",
                             name, local_transform[3][0], local_transform[3][1], local_transform[3][2]);
                    }

                    // オイラー角からクォータニオンに変換（度数法→ラジアン）
                    let quat_x = Quaternion::from_angle_x(Rad(rotation[0].to_radians()));
                    let quat_y = Quaternion::from_angle_y(Rad(rotation[1].to_radians()));
                    let quat_z = Quaternion::from_angle_z(Rad(rotation[2].to_radians()));
                    let rotation_quat = quat_z * quat_y * quat_x;

                    let node = BoneNode {
                        name: name.clone(),
                        parent,
                        local_transform,
                        default_translation: translation,
                        default_rotation: rotation_quat,
                        default_scaling: scaling,
                    };

                    fbx_model.nodes.insert(name, node);
                }
            }
            log!("=== Loading Meshes ===");

            for object in doc.objects() {
                if let TypedObjectHandle::Model(TypedModelHandle::Mesh(mesh)) = object.get_typed() {
                    log!("Loading mesh {:?}", mesh);
                    let mesh_name = mesh.name().expect("mesh name not found").to_string();
                    log!("mesh node name {}", mesh_name);

                    // 各メッシュごとに新しいFbxDataを作成
                    let fbx_data_index = fbx_model.fbx_data.len();
                    fbx_model.fbx_data.push(FbxData::new());
                    log!("Created FbxData[{}] for mesh '{}'", fbx_data_index, mesh_name);

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
                    let offset = fbx_model.fbx_data[fbx_data_index].positions.len() as u32;
                    for index in indices.iter_mut() {
                        *index += offset;
                    }
                    log!("indices: count={}, {:?}", indices.len(), indices);
                    fbx_model.fbx_data[fbx_data_index].indices.extend(indices);

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

                    // 頂点オフセットを計算
                    let vertex_offset = fbx_model.fbx_data[fbx_data_index].positions.len();
                    let vertex_count = local_positions.len();

                    fbx_model.fbx_data[fbx_data_index].local_positions.extend(local_positions.clone());
                    fbx_model.fbx_data[fbx_data_index].positions.extend(positions);

                    // 親ボーンを探してMeshPartを作成（階層アニメーション用）
                    let parent_bone = find_parent_bone(&mesh, &doc);
                    let local_transform = if let Some(ref bone_name) = parent_bone {
                        log!("Mesh '{}' is parented to bone '{}'", mesh_name, bone_name);
                        calculate_mesh_local_transform(&mesh, bone_name, &doc)
                    } else {
                        log!("Mesh '{}' has no parent bone, using world transform", mesh_name);
                        world_transform
                    };

                    let mesh_part = MeshPart {
                        mesh_name: mesh_name.clone(),
                        local_positions,
                        parent_bone,
                        local_transform,
                        vertex_offset,
                        vertex_count,
                    };
                    fbx_model.fbx_data[fbx_data_index].mesh_parts.push(mesh_part);

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
                                            fbx_model.fbx_data[fbx_data_index].clusters.push(cluster_info);
                                        }
                                        Err(e) => {
                                            log!("Warning: Failed to extract cluster data: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // マテリアルとテクスチャ情報を取得
                    log!("=== Loading Material and Texture for mesh '{}' ===", mesh_name);
                    for conn in mesh_obj.source_objects() {
                        if let Some(material_obj) = conn.object_handle() {
                            if material_obj.class() == "Material" {
                                let material_name = material_obj.name().unwrap_or("Unknown").to_string();
                                log!("  Found Material: {}", material_name);
                                fbx_model.fbx_data[fbx_data_index].material_name = Some(material_name.clone());

                                // マテリアルからテクスチャを検索
                                for texture_conn in material_obj.source_objects() {
                                    if let Some(texture_obj) = texture_conn.object_handle() {
                                        if texture_obj.class() == "Texture" {
                                            let texture_name = texture_obj.name().unwrap_or("Unknown").to_string();
                                            log!("    Found Texture: {}", texture_name);

                                            // テクスチャ名を保存（後でテクスチャファイルとマッチングする）
                                            fbx_model.fbx_data[fbx_data_index].diffuse_texture = Some(texture_name);
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

#[derive(Clone, Debug)]
pub struct BoneNode {
    pub name: String,
    pub parent: Option<String>,
    pub local_transform: Matrix4<f32>, // Static local transform (T * R * S)
    pub default_translation: [f32; 3],
    pub default_rotation: Quaternion<f32>,  // クォータニオンで保存
    pub default_scaling: [f32; 3],
}

#[derive(Clone, Debug, Default)]
pub struct FbxModel {
    pub fbx_data: Vec<FbxData>,
    pub animations: Vec<FbxAnimation>,
    pub nodes: HashMap<String, BoneNode>,
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
            log!("[FRAME t={:.4}] ========== Animation update ==========", time);
            for (mesh_idx, fbx_data) in self.fbx_data.iter_mut().enumerate() {
                log!("[FRAME t={:.4}] Processing mesh {}", time, mesh_idx);
                fbx_data.update_animation(animation, &self.nodes, time);
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

/// Calculate global transforms for all bones
fn compute_global_bone_transforms(
    animation: &FbxAnimation,
    nodes: &HashMap<String, BoneNode>,
    time: f32,
) -> HashMap<String, Matrix4<f32>> {
    let mut global_transforms = HashMap::new();

    for name in nodes.keys() {
        resolve_global_transform(name, animation, nodes, time, &mut global_transforms);
    }

    // Debug: Log bone transforms every frame
    if let Some(transform) = global_transforms.get("b_Root") {
        log!("[FRAME t={:.4}] b_Root global transform: [{:.3}, {:.3}, {:.3}] rotation",
             time,
             transform[3][0], transform[3][1], transform[3][2]);
    }

    if let Some(transform) = global_transforms.get("b_Head") {
        log!("[FRAME t={:.4}] b_Head global transform: [{:.3}, {:.3}, {:.3}]",
             time,
             transform[3][0], transform[3][1], transform[3][2]);
    }

    global_transforms
}

fn resolve_global_transform(
    bone_name: &str,
    animation: &FbxAnimation,
    nodes: &HashMap<String, BoneNode>,
    time: f32,
    cache: &mut HashMap<String, Matrix4<f32>>,
) -> Matrix4<f32> {
    if let Some(transform) = cache.get(bone_name) {
        return *transform;
    }

    // Check for circular reference by temporarily inserting identity matrix
    // If we encounter this bone again during parent traversal, we'll detect the cycle
    cache.insert(bone_name.to_string(), Matrix4::identity());

    let transform = if let Some(node) = nodes.get(bone_name) {
        // Calculate local transform
        let local_transform_fbx = if let Some(bone_anim) = animation.bone_animations.get(bone_name) {
            evaluate_bone_transform_at_time(
                bone_anim,
                time,
                node.default_translation,
                node.default_rotation,
                node.default_scaling,
            )
        } else {
            // For nodes without animation, use the bind pose (local_transform)
            // Note: node.local_transform already has coord conversion applied for root bones
            if time < 0.1 && (bone_name == "RootNode" || bone_name == "b_Root") {
                log!("DEBUG: {} using static local_transform (no animation): {:?}", bone_name, node.local_transform);
            }
            node.local_transform
        };

        // Apply coordinate system conversion (Y-up → Z-up) for RootNode and its direct children (armature roots)
        // This is necessary because inverse_bind_pose contains coordinate conversion,
        // so bone_transform must also contain it for skinning to work correctly
        let needs_coord_conversion = node.parent.is_none()
            || bone_name == "RootNode"
            || node.parent.as_ref().map_or(false, |p| p == "RootNode");

        let local_transform = if needs_coord_conversion && animation.bone_animations.contains_key(bone_name) {
            // Only apply conversion if this bone has animation (animated local_transform_fbx needs conversion)
            // Coordinate conversion matrix: Y-up → Z-up (90-degree X-axis rotation)
            let coord_conversion = Matrix4::new(
                1.0, 0.0, 0.0, 0.0,
                0.0, 0.0, -1.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            );
            coord_conversion * local_transform_fbx
        } else {
            // Child bones or static bones: use local_transform as-is
            local_transform_fbx
        };

        // Multiply with parent global transform
        if let Some(parent_name) = &node.parent {
            let parent_global = resolve_global_transform(parent_name, animation, nodes, time, cache);

            // Debug: Log parent multiplication
            if time < 0.1 && bone_name == "b_Root" {
                log!("DEBUG: b_Root - parent_global: {:?}", parent_global);
                log!("DEBUG: b_Root - local_transform: {:?}", local_transform);
            }

            if time < 0.1 && bone_name == "b_Head" {
                log!("DEBUG: b_Head - parent: {}", parent_name);
                log!("DEBUG: b_Head - parent_global: {:?}", parent_global);
                log!("DEBUG: b_Head - local_transform: {:?}", local_transform);
            }

            parent_global * local_transform
        } else {
            local_transform
        }
    } else {
        Matrix4::identity()
    };

    cache.insert(bone_name.to_string(), transform);
    transform
}

/// ボーンごとのアニメーション
#[derive(Clone, Debug)]
pub struct BoneAnimation {
    pub bone_name: String,
    pub translation_keys: Vec<KeyFrame<[f32; 3]>>,
    pub rotation_keys: Vec<KeyFrame<Quaternion<f32>>>,  // クォータニオンで保存
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
    default_translation: [f32; 3],
    default_rotation: Quaternion<f32>,
    default_scaling: [f32; 3],
) -> Matrix4<f32> {
    // Debug logging (only for time < 0.05 to avoid spam)
    let should_log = time < 0.05 && (bone_anim.bone_name.contains("Bone.003") || bone_anim.bone_name.contains("Bone.007"));

    if should_log {
        log!("DEBUG [evaluate_bone_transform_at_time] bone={}, time={:.4}", bone_anim.bone_name, time);
        log!("  translation_keys: {} keys", bone_anim.translation_keys.len());
        log!("  rotation_keys: {} keys", bone_anim.rotation_keys.len());
        log!("  scale_keys: {} keys", bone_anim.scale_keys.len());
        if !bone_anim.translation_keys.is_empty() {
            log!("  First translation key: time={:.4}, value={:?}",
                 bone_anim.translation_keys[0].time, bone_anim.translation_keys[0].value);
            if bone_anim.translation_keys.len() > 1 {
                log!("  Second translation key: time={:.4}, value={:?}",
                     bone_anim.translation_keys[1].time, bone_anim.translation_keys[1].value);
            }
        }
    }

    // Translation を補間
    // Use animation keyframes if available, otherwise use bind pose (default)
    let translation = if !bone_anim.translation_keys.is_empty() {
        interpolate_vec3_at_time(&bone_anim.translation_keys, time)
    } else {
        default_translation
    };

    // Rotation を補間（クォータニオン）
    // Use animation keyframes if available, otherwise use bind pose (default)
    let rotation_quat = if !bone_anim.rotation_keys.is_empty() {
        interpolate_quaternion_at_time(&bone_anim.rotation_keys, time)
    } else {
        default_rotation
    };

    // Scale を補間
    // Use animation keyframes if available, otherwise use bind pose (default)
    let scale = if !bone_anim.scale_keys.is_empty() {
        interpolate_vec3_at_time(&bone_anim.scale_keys, time)
    } else {
        default_scaling
    };

    // T * R * S の順で行列を構築
    let translation_matrix = Matrix4::from_translation(vec3(translation[0], translation[1], translation[2]));

    // クォータニオンから回転行列を作成
    let rotation_matrix = Matrix4::from(rotation_quat);

    let scale_matrix = Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);

    translation_matrix * rotation_matrix * scale_matrix
}

/// クォータニオン補間（SLERP: Spherical Linear Interpolation）
/// 滑らかな回転補間を提供
fn interpolate_quaternion_at_time(keyframes: &[KeyFrame<Quaternion<f32>>], time: f32) -> Quaternion<f32> {
    if keyframes.is_empty() {
        return Quaternion::new(1.0, 0.0, 0.0, 0.0);  // 単位クォータニオン
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

    // 線形補間（SLERP）
    for i in 0..keyframes.len() - 1 {
        if time >= keyframes[i].time && time <= keyframes[i + 1].time {
            let t = (time - keyframes[i].time) / (keyframes[i + 1].time - keyframes[i].time);
            let from = keyframes[i].value;
            let to = keyframes[i + 1].value;

            // cgmathのSLERP (Spherical Linear Interpolation)
            return from.slerp(to, t);
        }
    }

    keyframes[0].value
}

/// Translation/Scaling用の補間関数（Catmull-Rom spline）
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

    // 線形補間（シンプルで安定）
    for i in 0..keyframes.len() - 1 {
        if time >= keyframes[i].time && time <= keyframes[i + 1].time {
            let t = (time - keyframes[i].time) / (keyframes[i + 1].time - keyframes[i].time);
            let from = keyframes[i].value;
            let to = keyframes[i + 1].value;

            return [
                from[0] + (to[0] - from[0]) * t,
                from[1] + (to[1] - from[1]) * t,
                from[2] + (to[2] - from[2]) * t,
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
    nodes: &HashMap<String, BoneNode>,
    time: f32,
) -> Vec<Vector3<f32>> {
    let num_vertices = original_positions.len();
    let mut skinned_positions = original_positions.to_vec();

    // 各ボーンのスキニング行列を計算
    let mut bone_matrices: HashMap<String, Matrix4<f32>> = HashMap::new();

    // 全ボーンのグローバル変換を計算
    let global_transforms = compute_global_bone_transforms(animation, nodes, time);

    let mut missing_bones = 0;
    for cluster in clusters {
        let bone_name = &cluster.bone_name;

        // グローバル変換を取得
        let bone_transform = if let Some(transform) = global_transforms.get(bone_name) {
            *transform
        } else {
            missing_bones += 1;
            if missing_bones <= 3 && time < 0.1 {
                log!("DEBUG: Bone '{}' not found in global_transforms, using identity", bone_name);
            }
            Matrix4::identity()
        };

        // スキニング行列 = BoneTransform * InverseBindPose
        let skinning_matrix = bone_transform * cluster.inverse_bind_pose;
        bone_matrices.insert(bone_name.clone(), skinning_matrix);

        // Debug: Log first bone's matrices
        if bone_name == "b_Root" && time < 0.1 {
            log!("DEBUG: b_Root bone_transform: {:?}", bone_transform);
            log!("DEBUG: b_Root inverse_bind_pose: {:?}", cluster.inverse_bind_pose);
            log!("DEBUG: b_Root skinning_matrix: {:?}", skinning_matrix);
        }

        if bone_name == "b_Head" && time < 0.1 {
            log!("DEBUG: b_Head bone_transform: {:?}", bone_transform);
            log!("DEBUG: b_Head inverse_bind_pose: {:?}", cluster.inverse_bind_pose);
            log!("DEBUG: b_Head skinning_matrix: {:?}", skinning_matrix);
        }
    }

    if missing_bones > 0 && time < 0.1 {
        log!("DEBUG: Total missing bones in global_transforms: {}/{}", missing_bones, clusters.len());
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

    // Debug: Check how many vertices have weights
    if time < 0.1 {
        let vertices_with_weights = vertex_transforms.iter().filter(|v| !v.is_empty()).count();
        log!("DEBUG: Vertices with weights: {}/{}", vertices_with_weights, num_vertices);

        // Debug: Check if all bone matrices are available
        let mut bones_with_matrices = 0;
        let mut bones_without_matrices = 0;
        for cluster in clusters {
            if bone_matrices.contains_key(&cluster.bone_name) {
                bones_with_matrices += 1;
            } else {
                bones_without_matrices += 1;
                if bones_without_matrices <= 3 {
                    log!("DEBUG: Cluster bone '{}' has no matrix!", cluster.bone_name);
                }
            }
        }
        log!("DEBUG: Bones with matrices: {}/{}", bones_with_matrices, clusters.len());

        // Debug: Show first few vertices' weight info
        for i in 0..3.min(num_vertices) {
            if !vertex_transforms[i].is_empty() {
                log!("DEBUG: Vertex {} has {} bone influences", i, vertex_transforms[i].len());
                for &(cluster_idx, weight) in &vertex_transforms[i] {
                    log!("  Bone: {}, Weight: {}", clusters[cluster_idx].bone_name, weight);
                }
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

                // Debug: Log vertex 0 transformation details (every frame)
                if vertex_idx == 0 {
                    log!("[FRAME t={:.4}] Vertex 0 - Bone: {}, Weight: {}", time, cluster.bone_name, weight);
                    log!("  Original pos: {:?}", original_pos);
                    log!("  Transformed pos: {:?}", transformed);
                }
            }
        }

        // ウェイトを正規化
        if total_weight > 0.0 {
            skinned_positions[vertex_idx] = weighted_pos / total_weight;

            // Debug: Log final position for vertex 0 (every frame)
            if vertex_idx == 0 {
                log!("[FRAME t={:.4}] Vertex 0 final skinned position: {:?}", time, skinned_positions[vertex_idx]);
            }
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
                                // Rotationカーブを統合（オイラー角→クォータニオン）
                                bone_anim.rotation_keys = merge_xyz_curves_to_quaternion(curves_x, curves_y, curves_z);
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

/// X, Y, Zの個別Rotationカーブ（度数法のオイラー角）をクォータニオンのカーブに統合
fn merge_xyz_curves_to_quaternion(
    curves_x: Vec<KeyFrame<f32>>,
    curves_y: Vec<KeyFrame<f32>>,
    curves_z: Vec<KeyFrame<f32>>,
) -> Vec<KeyFrame<Quaternion<f32>>> {
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

    // 各時間でのX, Y, Z値（度数法）を補間して、クォータニオンに変換
    let mut merged = Vec::new();
    for &time in &all_times {
        let x_deg = interpolate_at_time(&curves_x, time);
        let y_deg = interpolate_at_time(&curves_y, time);
        let z_deg = interpolate_at_time(&curves_z, time);

        // オイラー角（度数法）からクォータニオンに変換
        let quat_x = Quaternion::from_angle_x(Rad(x_deg.to_radians()));
        let quat_y = Quaternion::from_angle_y(Rad(y_deg.to_radians()));
        let quat_z = Quaternion::from_angle_z(Rad(z_deg.to_radians()));
        let rotation_quat = quat_z * quat_y * quat_x;

        merged.push(KeyFrame {
            time,
            value: rotation_quat,
        });
    }

    merged
}

/// 同じ時間のrotation keyframeをマージ
/// FBXではクォータニオンの成分が別々のキーとして保存されることがある
fn merge_duplicate_rotation_keys(keyframes: &[KeyFrame<Quaternion<f32>>]) -> Vec<KeyFrame<Quaternion<f32>>> {
    if keyframes.is_empty() {
        return Vec::new();
    }

    // まず時間でソート（FBXのキーフレームは時系列順ではない場合がある）
    // NaN値がある場合はEqualとして扱う
    let mut sorted_keyframes = keyframes.to_vec();
    sorted_keyframes.sort_by(|a, b| {
        a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut merged = Vec::new();
    let time_threshold = 0.001; // 1ms以内は同じ時間とみなす

    let mut i = 0;
    while i < sorted_keyframes.len() {
        let current_time = sorted_keyframes[i].time;

        // 同じ時間（または非常に近い時間）のキーフレームを収集
        let mut group = vec![sorted_keyframes[i].value];
        let mut j = i + 1;

        while j < sorted_keyframes.len() && (sorted_keyframes[j].time - current_time).abs() < time_threshold {
            group.push(sorted_keyframes[j].value);
            j += 1;
        }

        // グループから最も妥当なクォータニオンを選択（最も正規化に近いもの）
        let best_quat = group.iter()
            .min_by(|a, b| {
                let len_a = (a.s * a.s + a.v.x * a.v.x + a.v.y * a.v.y + a.v.z * a.v.z).sqrt();
                let len_b = (b.s * b.s + b.v.x * b.v.x + b.v.y * b.v.y + b.v.z * b.v.z).sqrt();
                let diff_a = (len_a - 1.0).abs();
                let diff_b = (len_b - 1.0).abs();
                diff_a.partial_cmp(&diff_b).unwrap()
            })
            .unwrap();

        merged.push(KeyFrame {
            time: current_time,
            value: *best_quat,
        });

        i = j;
    }

    // 無効なキーフレームをフィルタリング
    // - 負の時間
    // - ゼロに近いクォータニオン（単位クォータニオン[0,0,0,1]や[1,0,0,0]以外で長さが非常に小さいもの）
    merged.retain(|key| {
        // 負の時間を除外
        if key.time < 0.0 {
            return false;
        }

        let quat = &key.value;
        let len = (quat.s * quat.s + quat.v.x * quat.v.x + quat.v.y * quat.v.y + quat.v.z * quat.v.z).sqrt();

        // クォータニオンの長さがゼロに近い場合、単位クォータニオンかチェック
        if len < 0.1 {
            // 単位クォータニオンかチェック（[1,0,0,0]または[0,0,0,1]）
            let is_identity =
                ((quat.s - 1.0).abs() < 0.01 && quat.v.x.abs() < 0.01 && quat.v.y.abs() < 0.01 && quat.v.z.abs() < 0.01) ||
                    ((quat.s).abs() < 0.01 && quat.v.x.abs() < 0.01 && quat.v.y.abs() < 0.01 && (quat.v.z - 1.0).abs() < 0.01);

            // 単位クォータニオンで時間が0の場合のみ除外（初期姿勢として無効なデータ）
            if is_identity && key.time.abs() < 0.01 {
                return false;
            }
        }

        true
    });

    // ループをスムーズにするため、最初のキーフレームをt=0.0にコピー
    if !merged.is_empty() && merged[0].time > 0.01 {
        let first_keyframe = merged[0].value;
        merged.insert(0, KeyFrame {
            time: 0.0,
            value: first_keyframe,
        });
    }

    // ループをスムーズにするため、最後のキーフレームも最初のキーフレームと同じ値にする
    // アニメーションの最後でループの最初に戻る際のジャンプを防ぐ
    if merged.len() >= 2 {
        let first_value = merged[0].value;
        let last_index = merged.len() - 1;

        // 最後のキーフレームの値を最初のキーフレームの値で置き換える
        merged[last_index].value = first_value;
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

/// 個別のメッシュパーツ（階層アニメーション用）
#[derive(Clone, Debug)]
pub struct MeshPart {
    pub mesh_name: String,                    // メッシュ名
    pub local_positions: Vec<Vector3<f32>>,  // メッシュローカル空間の頂点（変換前）
    pub parent_bone: Option<String>,          // 親ボーンの名前
    pub local_transform: Matrix4<f32>,        // 親ボーンに対する相対変換
    pub vertex_offset: usize,                 // 結合頂点配列内の開始インデックス
    pub vertex_count: usize,                  // 頂点数
}

#[derive(Clone, Debug)]
pub struct FbxData {
    pub positions: Vec<Vector3<f32>>,        // ワールド座標の頂点位置（表示用）
    pub local_positions: Vec<Vector3<f32>>,  // ローカル座標の頂点位置（スキニング用）
    pub indices: Vec<u32>,
    pub tex_coords: Vec<[f32; 2]>,           // UV座標
    pub clusters: Vec<ClusterInfo>,          // スキニング情報
    pub mesh_parts: Vec<MeshPart>,           // 個別メッシュパーツ（階層アニメーション用）
    pub parent_node: Option<String>,         // 親ノード名（階層アニメーション用）
    pub material_name: Option<String>,       // マテリアル名
    pub diffuse_texture: Option<String>,     // Diffuseテクスチャパス
}

impl FbxData {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            local_positions: Vec::new(),
            indices: Vec::new(),
            tex_coords: Vec::new(),
            clusters: Vec::new(),
            mesh_parts: Vec::new(),
            parent_node: None,
            material_name: None,
            diffuse_texture: None,
        }
    }

    /// アニメーション時間に基づいて頂点位置を更新
    pub fn update_animation(&mut self, animation: &FbxAnimation, nodes: &HashMap<String, BoneNode>, time: f32) {
        // スキニング情報がある場合はスキニングアニメーション
        if !self.clusters.is_empty() && !self.local_positions.is_empty() {
            self.positions = apply_skinning(&self.local_positions, &self.clusters, animation, nodes, time);
            return;
        }

        // メッシュパーツがある場合は階層アニメーション（複数パーツ）
        if !self.mesh_parts.is_empty() {
            self.apply_hierarchy_animation(animation, nodes, time);
            return;
        }

        // 親ノードがある場合は単一ノードの階層アニメーション
        if let Some(parent_node) = self.parent_node.clone() {
            self.apply_single_node_animation(animation, nodes, time, &parent_node);
        }
    }

    /// 単一ノードの階層アニメーションを適用（メッシュ全体を親ノードの変換で変換）
    fn apply_single_node_animation(&mut self, animation: &FbxAnimation, nodes: &HashMap<String, BoneNode>, time: f32, parent_node: &str) {
        // 全ボーンの階層変換を計算
        let bone_transforms = compute_global_bone_transforms(animation, nodes, time);

        // 親ノードから階層を遡って、アニメーションチャンネルを持つ最初の祖先ノードを検索
        // 同時に、メッシュノードからanimated ancestorまでのlocal_transformsを収集
        let mut current_node = Some(parent_node.to_string());
        let mut path_to_animated: Vec<String> = Vec::new();
        let mut animated_ancestor: Option<String> = None;
        let mut visited_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

        if time < 0.1 {
            log!("  [apply_single_node_animation] parent_node={}, searching for animated ancestor...", parent_node);
        }

        while let Some(node_name) = current_node.clone() {
            // Check for circular reference
            if visited_nodes.contains(&node_name) {
                if time < 0.1 {
                    log!("    Circular reference detected at node: {}, stopping search", node_name);
                }
                break;
            }
            visited_nodes.insert(node_name.clone());
            path_to_animated.push(node_name.clone());

            // このノードがアニメーションチャンネルを持っているか確認
            if animation.bone_animations.contains_key(&node_name) {
                // アニメーションチャンネルがある = アニメーションされている
                animated_ancestor = Some(node_name.clone());
                if time < 0.1 {
                    log!("    Found animated ancestor: {}", node_name);
                }
                break;
            } else if time < 0.1 {
                log!("    Checking node: {} (no animation channel)", node_name);
            }

            // 親ノードに遡る
            if let Some(node) = nodes.get(&node_name) {
                current_node = node.parent.clone();
            } else {
                if time < 0.1 {
                    log!("    Node {} not found in nodes map, stopping search", node_name);
                }
                break;
            }
        }

        // 最終的な変換を計算
        let mut final_transform = Matrix4::identity();

        if let Some(ancestor_name) = animated_ancestor {
            // animated ancestorのグローバル変換を取得
            final_transform = bone_transforms.get(&ancestor_name).copied().unwrap_or(Matrix4::identity());

            // メッシュノードからanimated ancestorまでの中間ノードのlocal_transformを累積
            // path_to_animated[0]がメッシュノード、最後がanimated ancestor
            // animated ancestorは既にグローバル変換なので、それより前のノードのlocal_transformを右から掛ける
            for i in 0..(path_to_animated.len() - 1) {
                let node_name = &path_to_animated[i];
                if let Some(node) = nodes.get(node_name) {
                    final_transform = final_transform * node.local_transform;
                    if time < 0.1 {
                        log!("    Accumulating local_transform of: {}", node_name);
                    }
                }
            }
        } else {
            // アニメーションされた祖先が見つからない場合、親ノードの変換をそのまま使用
            final_transform = bone_transforms.get(parent_node).copied().unwrap_or(Matrix4::identity());
            if time < 0.1 {
                log!("    No animated ancestor found, using parent node transform");
            }
        }

        // デバッグ: 最初のフレームで変換行列をログ出力
        if time < 0.1 {
            log!("    Final transform (row-major display):");
            log!("      [{:.4}, {:.4}, {:.4}, {:.4}]", final_transform[0][0], final_transform[1][0], final_transform[2][0], final_transform[3][0]);
            log!("      [{:.4}, {:.4}, {:.4}, {:.4}]", final_transform[0][1], final_transform[1][1], final_transform[2][1], final_transform[3][1]);
            log!("      [{:.4}, {:.4}, {:.4}, {:.4}]", final_transform[0][2], final_transform[1][2], final_transform[2][2], final_transform[3][2]);
            log!("      [{:.4}, {:.4}, {:.4}, {:.4}]", final_transform[0][3], final_transform[1][3], final_transform[2][3], final_transform[3][3]);
        }

        let animated_transform = final_transform;

        // 全頂点を変換
        for i in 0..self.local_positions.len() {
            let local_pos = Point3::new(
                self.local_positions[i].x,
                self.local_positions[i].y,
                self.local_positions[i].z,
            );
            let world_pos = animated_transform.transform_point(local_pos);
            self.positions[i] = Vector3::new(world_pos.x, world_pos.y, world_pos.z);
        }

        // デバッグ: 最初の頂点をログ出力
        if time < 0.1 && !self.local_positions.is_empty() {
            log!("    First vertex: local={:?} -> world={:?}",
                 self.local_positions.get(0), self.positions.get(0));
        }
    }

    /// 階層アニメーションを適用（親ボーンの変換を各メッシュパーツに適用）
    fn apply_hierarchy_animation(&mut self, animation: &FbxAnimation, nodes: &HashMap<String, BoneNode>, time: f32) {
        // 全ボーンの階層変換を計算
        let bone_transforms = compute_global_bone_transforms(animation, nodes, time);

        for mesh_part in &self.mesh_parts {
            // 親ボーンの変換を取得
            let parent_transform = if let Some(bone_name) = &mesh_part.parent_bone {
                bone_transforms.get(bone_name).copied().unwrap_or(Matrix4::identity())
            } else {
                Matrix4::identity()
            };

            // 最終変換 = 親ボーンの変換 × メッシュの相対変換
            let final_transform = parent_transform * mesh_part.local_transform;

            // 各頂点を変換
            for (i, local_pos) in mesh_part.local_positions.iter().enumerate() {
                let world_pos = final_transform.transform_point(Point3::from_vec(*local_pos));
                self.positions[mesh_part.vertex_offset + i] = Vector3::new(world_pos.x, world_pos.y, world_pos.z);
            }
        }
    }
}

// ============================================================================
// Russimp-based FBX loader (more flexible, handles various FBX formats)
// ============================================================================

/// Load FBX file using russimp (Assimp bindings)
/// This is more flexible than fbxcel and can handle FBX files that fbxcel cannot parse
pub fn load_fbx_with_russimp(path: &str) -> Result<FbxModel> {
    log!("=== Loading FBX file with russimp: {} ===", path);

    let scene = Scene::from_file(
        path,
        vec![
            PostProcess::Triangulate,
            PostProcess::GenerateNormals,
            // Don't join identical vertices - this can break animation
            // PostProcess::JoinIdenticalVertices,
        ],
    ).context(format!("Failed to load FBX with russimp: {}", path))?;

    let mut fbx_model = FbxModel::default();

    log!("Loaded scene with {} meshes", scene.meshes.len());

    // Process each mesh
    for (mesh_idx, mesh) in scene.meshes.iter().enumerate() {
        log!("Processing mesh {}: {} vertices, {} faces",
             mesh_idx, mesh.vertices.len(), mesh.faces.len());

        let mut fbx_data = FbxData::new();

        // Extract material and texture information
        let material_index = mesh.material_index as usize;
        log!("  Mesh {} uses material index: {}", mesh_idx, material_index);

        if material_index < scene.materials.len() {
            let material = &scene.materials[material_index];

            // Debug: Log all material properties
            log!("  Material has {} properties:", material.properties.len());
            for (i, prop) in material.properties.iter().enumerate() {
                match &prop.data {
                    russimp::material::PropertyTypeInfo::String(s) => {
                        log!("    Property {}: key='{}', semantic={:?}, data=String('{}')", i, prop.key, prop.semantic, s);
                    }
                    russimp::material::PropertyTypeInfo::FloatArray(arr) => {
                        log!("    Property {}: key='{}', semantic={:?}, data=FloatArray(len={})", i, prop.key, prop.semantic, arr.len());
                    }
                    russimp::material::PropertyTypeInfo::IntegerArray(arr) => {
                        log!("    Property {}: key='{}', semantic={:?}, data=IntegerArray(len={})", i, prop.key, prop.semantic, arr.len());
                    }
                    russimp::material::PropertyTypeInfo::Buffer(buf) => {
                        log!("    Property {}: key='{}', semantic={:?}, data=Buffer(len={})", i, prop.key, prop.semantic, buf.len());
                    }
                }
            }

            // Debug: Log all texture types
            log!("  Material has {} textures:", material.textures.len());
            for (tex_type, texture) in &material.textures {
                let texture_ref = texture.borrow();
                log!("    Texture type: {:?}, filename: '{}'", tex_type, texture_ref.filename);
            }

            // Get material name from properties
            for prop in &material.properties {
                if prop.key.contains("?mat.name") || prop.key == "$mat.name" {
                    if let russimp::material::PropertyTypeInfo::String(name_str) = &prop.data {
                        fbx_data.material_name = Some(name_str.clone());
                        log!("  Material name: {}", name_str);
                        break;
                    }
                }
            }

            // Get diffuse texture (try multiple texture types)
            let texture_types = [
                russimp::material::TextureType::Diffuse,
                russimp::material::TextureType::BaseColor,
                russimp::material::TextureType::Ambient,
            ];

            for tex_type in &texture_types {
                if let Some(texture) = material.textures.get(tex_type) {
                    let texture_ref = texture.borrow();
                    let texture_filename = texture_ref.filename.clone();
                    fbx_data.diffuse_texture = Some(texture_filename.clone());
                    log!("  Found texture ({:?}): {}", tex_type, texture_filename);
                    break;
                }
            }

            if fbx_data.diffuse_texture.is_none() {
                log!("  No diffuse/basecolor/ambient texture found in FBX");

                // Fallback: Try to infer texture filename from material name
                // Pattern: MatI_Ride_FengHuang_01a -> Tex_Ride_FengHuang_01a_D_A.tga.png
                if let Some(mat_name) = &fbx_data.material_name {
                    if mat_name.starts_with("MatI_") {
                        let texture_base = mat_name.replace("MatI_", "Tex_");
                        let texture_filename = format!("{}_D_A.tga.png", texture_base);

                        // Construct relative path from executable location
                        let texture_path = format!("src/resources/phoenix-bird/textures/{}", texture_filename);

                        fbx_data.diffuse_texture = Some(texture_path.clone());
                        log!("  Inferred texture from material name: {} -> {}", mat_name, texture_path);
                    }
                }
            }
        } else {
            log!("  Warning: material_index {} out of bounds (scene has {} materials)",
                 material_index, scene.materials.len());
        }

        // Extract vertices
        for (i, vertex) in mesh.vertices.iter().enumerate() {
            fbx_data.positions.push(Vector3::new(vertex.x, vertex.y, vertex.z));
            fbx_data.local_positions.push(Vector3::new(vertex.x, vertex.y, vertex.z));

            // Debug: Log first 3 vertices to check if positions are correct
            if i < 3 {
                log!("  Vertex[{}]: ({:.3}, {:.3}, {:.3})", i, vertex.x, vertex.y, vertex.z);
            }
        }

        // Extract UV coordinates (texture_coords[0] is the first UV channel)
        if !mesh.texture_coords.is_empty() && mesh.texture_coords[0].is_some() {
            if let Some(ref uvs) = mesh.texture_coords[0] {
                log!("Mesh {} has {} UV coordinates", mesh_idx, uvs.len());

                // Log first 5 UV coordinates for debugging
                for (i, uv) in uvs.iter().enumerate().take(5) {
                    log!("  UV[{}]: ({:.4}, {:.4})", i, uv.x, uv.y);
                }

                for uv in uvs {
                    // Flip V coordinate (1.0 - v) for Vulkan
                    fbx_data.tex_coords.push([uv.x, 1.0 - uv.y]);
                }
            }
        } else {
            log!("Mesh {} has no UV coordinates, using default [0.5, 0.5]", mesh_idx);
            // Fallback: use default UV coordinates
            for _ in 0..mesh.vertices.len() {
                fbx_data.tex_coords.push([0.5, 0.5]);
            }
        }

        // Extract indices (triangulated)
        for face in &mesh.faces {
            for &index in &face.0 {
                fbx_data.indices.push(index);
            }
        }

        // Extract bone/skinning information
        if !mesh.bones.is_empty() {
            log!("Mesh {} has {} bones", mesh_idx, mesh.bones.len());

            for bone in &mesh.bones {
                let bone_name = bone.name.clone();
                log!("  Bone: {} with {} weights", bone_name, bone.weights.len());

                // Convert bone offset matrix (this is the inverse bind pose)
                // Note: Do NOT transpose! Skeletal animation needs the raw offset matrix
                // because it compensates for the row-major/column-major difference internally
                let offset = &bone.offset_matrix;
                let inverse_bind_pose = Matrix4::new(
                    offset.a1, offset.b1, offset.c1, offset.d1,
                    offset.a2, offset.b2, offset.c2, offset.d2,
                    offset.a3, offset.b3, offset.c3, offset.d3,
                    offset.a4, offset.b4, offset.c4, offset.d4,
                );

                // Calculate transform_link (bind pose = inverse of offset matrix)
                let transform_link = inverse_bind_pose.invert().unwrap_or(Matrix4::identity());

                // Collect vertex weights
                let mut vertex_indices = Vec::new();
                let mut vertex_weights = Vec::new();

                for vertex_weight in &bone.weights {
                    vertex_indices.push(vertex_weight.vertex_id as usize);
                    vertex_weights.push(vertex_weight.weight);
                }

                if !vertex_indices.is_empty() {
                    fbx_data.clusters.push(ClusterInfo {
                        bone_name,
                        transform: Matrix4::identity(),  // Mesh transform (identity for simplicity)
                        transform_link,                   // Bone bind pose
                        inverse_bind_pose,
                        vertex_indices,
                        vertex_weights,
                    });
                }
            }

            log!("Extracted {} clusters for mesh {}", fbx_data.clusters.len(), mesh_idx);
        }

        log!("Extracted {} positions, {} indices",
             fbx_data.positions.len(), fbx_data.indices.len());

        fbx_model.fbx_data.push(fbx_data);
    }

    // Process animations
    if !scene.animations.is_empty() {
        log!("Found {} animations", scene.animations.len());

        for (anim_idx, animation) in scene.animations.iter().enumerate() {
            log!("Animation {}: duration={}, ticks_per_second={}",
                 anim_idx, animation.duration, animation.ticks_per_second);

            let anim_name = if animation.name.is_empty() {
                format!("Animation_{}", anim_idx)
            } else {
                animation.name.clone()
            };

            let mut fbx_animation = FbxAnimation {
                name: anim_name,
                duration: (animation.duration / animation.ticks_per_second) as f32,
                bone_animations: HashMap::new(),
            };

            // First pass: identify bones with $AssimpFbx$ split channels
            let mut bones_with_assimp_channels = std::collections::HashSet::new();
            for channel in &animation.channels {
                if channel.name.contains("$AssimpFbx$") {
                    if let Some(idx) = channel.name.find("_$AssimpFbx$_") {
                        let base_name = &channel.name[..idx];
                        bones_with_assimp_channels.insert(base_name.to_string());
                    }
                }
            }

            // Process node animations
            for channel in &animation.channels {
                let channel_name = channel.name.clone();
                log!("  Channel: {} ({} position keys, {} rotation keys, {} scaling keys)",
                     channel_name,
                     channel.position_keys.len(),
                     channel.rotation_keys.len(),
                     channel.scaling_keys.len());

                // Handle Assimp FBX split channels (e.g., "b_Root_$AssimpFbx$_Translation")
                let (bone_name, channel_type) = if channel_name.contains("$AssimpFbx$") {
                    // Extract bone name and channel type
                    if let Some(idx) = channel_name.find("_$AssimpFbx$_") {
                        let base_name = &channel_name[..idx];
                        let suffix = &channel_name[idx + "_$AssimpFbx$_".len()..];
                        (base_name.to_string(), Some(suffix))
                    } else {
                        (channel_name.clone(), None)
                    }
                } else {
                    (channel_name.clone(), None)
                };

                // Get or create bone animation
                let bone_anim = fbx_animation.bone_animations.entry(bone_name.clone()).or_insert_with(|| {
                    BoneAnimation {
                        bone_name: bone_name.clone(),
                        translation_keys: Vec::new(),
                        rotation_keys: Vec::new(),
                        scale_keys: Vec::new(),
                    }
                });

                // For $AssimpFbx$ channels, only process the specific transformation type
                match channel_type {
                    Some("Translation") => {
                        // Only process position keys for Translation channel
                        let before_count = bone_anim.translation_keys.len();

                        // Debug: For specific bones, log first 10 position keys from the channel
                        if (bone_name == "b_Head" || bone_name == "B_Spine" || bone_name == "b_Neck_2") && channel.position_keys.len() > 5 {
                            log!("    {} Translation channel has {} position keys (showing first 10):", bone_name, channel.position_keys.len());
                            for (i, pos_key) in channel.position_keys.iter().take(10).enumerate() {
                                let time = (pos_key.time / animation.ticks_per_second) as f32;
                                log!("      [{:2}] t={:.4} pos=[{:.3}, {:.3}, {:.3}]",
                                     i, time, pos_key.value.x, pos_key.value.y, pos_key.value.z);
                            }
                        }

                        for pos_key in &channel.position_keys {
                            let time = (pos_key.time / animation.ticks_per_second) as f32;
                            bone_anim.translation_keys.push(KeyFrame {
                                time,
                                value: [pos_key.value.x, pos_key.value.y, pos_key.value.z],
                            });
                        }
                        log!("    Added {} translation keys to {}", bone_anim.translation_keys.len() - before_count, bone_name);
                    }
                    Some("Rotation") => {
                        // Only process rotation keys for Rotation channel
                        let before_count = bone_anim.rotation_keys.len();

                        // Debug: For specific bones, log first 10 rotation keys from the channel
                        if (bone_name == "b_Root" || bone_name == "b_Head" || bone_name == "B_Spine") && channel.rotation_keys.len() > 5 {
                            log!("    {} Rotation channel has {} rotation keys (showing first 10):", bone_name, channel.rotation_keys.len());
                            for (i, rot_key) in channel.rotation_keys.iter().take(10).enumerate() {
                                let time = (rot_key.time / animation.ticks_per_second) as f32;
                                let quat = &rot_key.value;
                                let euler = quat_to_euler(quat.x, quat.y, quat.z, quat.w);
                                log!("      [{:2}] raw_time={:.4} t={:.4} quat=[{:.3}, {:.3}, {:.3}, {:.3}] euler=[{:.3}, {:.3}, {:.3}]",
                                     i, rot_key.time, time, quat.x, quat.y, quat.z, quat.w, euler[0], euler[1], euler[2]);
                            }
                        }

                        for rot_key in &channel.rotation_keys {
                            let time = (rot_key.time / animation.ticks_per_second) as f32;
                            let quat = &rot_key.value;

                            // Calculate quaternion length
                            let quat_length = (quat.x * quat.x + quat.y * quat.y + quat.z * quat.z + quat.w * quat.w).sqrt();

                            // Skip quaternions that are too small (essentially zero rotation)
                            if quat_length < 0.01 {
                                if bone_name == "b_Root" && time < 0.5 {
                                    log!("    Skipping near-zero quaternion at t={:.4}: quat=[{:.3}, {:.3}, {:.3}, {:.3}] length={:.3}",
                                         time, quat.x, quat.y, quat.z, quat.w, quat_length);
                                }
                                continue;
                            }

                            // Normalize quaternion if it's not normalized
                            let normalized_quat = if (quat_length - 1.0).abs() > 0.01 {
                                // Normalize
                                Quaternion::new(
                                    quat.w / quat_length,
                                    quat.x / quat_length,
                                    quat.y / quat_length,
                                    quat.z / quat_length,
                                )
                            } else {
                                // Already normalized - cgmath Quaternion is (w, x, y, z)
                                Quaternion::new(quat.w, quat.x, quat.y, quat.z)
                            };

                            // クォータニオンのまま保存（オイラー角変換なし）
                            bone_anim.rotation_keys.push(KeyFrame {
                                time,
                                value: normalized_quat,
                            });
                        }
                        log!("    Added {} rotation keys to {}", bone_anim.rotation_keys.len() - before_count, bone_name);
                    }
                    Some("Scaling") => {
                        // Only process scaling keys for Scaling channel
                        let before_count = bone_anim.scale_keys.len();
                        for scale_key in &channel.scaling_keys {
                            let time = (scale_key.time / animation.ticks_per_second) as f32;
                            bone_anim.scale_keys.push(KeyFrame {
                                time,
                                value: [scale_key.value.x, scale_key.value.y, scale_key.value.z],
                            });
                        }
                        log!("    Added {} scale keys to {}", bone_anim.scale_keys.len() - before_count, bone_name);
                    }
                    None => {
                        // Normal channel with all transformation types
                        // Skip this channel if the bone has $AssimpFbx$ split channels
                        if bones_with_assimp_channels.contains(&bone_name) {
                            log!("  Skipping channel '{}' because it has $AssimpFbx$ split channels", channel_name);
                            continue;
                        }

                        // Debug: For specific bones, log first 10 position keys from the channel
                        if (bone_name == "b_Head" || bone_name == "B_Spine" || bone_name == "b_Neck_2") && channel.position_keys.len() > 5 {
                            log!("    {} Normal channel has {} position keys (showing first 10):", bone_name, channel.position_keys.len());
                            for (i, pos_key) in channel.position_keys.iter().take(10).enumerate() {
                                let time = (pos_key.time / animation.ticks_per_second) as f32;
                                log!("      [{:2}] t={:.4} pos=[{:.3}, {:.3}, {:.3}]",
                                     i, time, pos_key.value.x, pos_key.value.y, pos_key.value.z);
                            }
                        }

                        for pos_key in &channel.position_keys {
                            let time = (pos_key.time / animation.ticks_per_second) as f32;
                            bone_anim.translation_keys.push(KeyFrame {
                                time,
                                value: [pos_key.value.x, pos_key.value.y, pos_key.value.z],
                            });
                        }

                        // Debug: For specific bones, log first 10 rotation keys from the channel
                        if (bone_name == "b_Head" || bone_name == "B_Spine") && channel.rotation_keys.len() > 5 {
                            log!("    {} Normal channel has {} rotation keys (showing first 10):", bone_name, channel.rotation_keys.len());
                            for (i, rot_key) in channel.rotation_keys.iter().take(10).enumerate() {
                                let time = (rot_key.time / animation.ticks_per_second) as f32;
                                let quat = &rot_key.value;
                                let euler = quat_to_euler(quat.x, quat.y, quat.z, quat.w);
                                log!("      [{:2}] raw_time={:.4} t={:.4} quat=[{:.3}, {:.3}, {:.3}, {:.3}] euler=[{:.3}, {:.3}, {:.3}]",
                                     i, rot_key.time, time, quat.x, quat.y, quat.z, quat.w, euler[0], euler[1], euler[2]);
                            }
                        }

                        for rot_key in &channel.rotation_keys {
                            let time = (rot_key.time / animation.ticks_per_second) as f32;
                            let quat = &rot_key.value;

                            // Calculate quaternion length
                            let quat_length = (quat.x * quat.x + quat.y * quat.y + quat.z * quat.z + quat.w * quat.w).sqrt();

                            // Skip quaternions that are too small (essentially zero rotation)
                            if quat_length < 0.01 {
                                continue;
                            }

                            // Normalize quaternion if it's not normalized
                            let normalized_quat = if (quat_length - 1.0).abs() > 0.01 {
                                // Normalize
                                Quaternion::new(
                                    quat.w / quat_length,
                                    quat.x / quat_length,
                                    quat.y / quat_length,
                                    quat.z / quat_length,
                                )
                            } else {
                                // Already normalized - cgmath Quaternion is (w, x, y, z)
                                Quaternion::new(quat.w, quat.x, quat.y, quat.z)
                            };

                            // クォータニオンのまま保存（オイラー角変換なし）
                            bone_anim.rotation_keys.push(KeyFrame {
                                time,
                                value: normalized_quat,
                            });
                        }

                        for scale_key in &channel.scaling_keys {
                            let time = (scale_key.time / animation.ticks_per_second) as f32;
                            bone_anim.scale_keys.push(KeyFrame {
                                time,
                                value: [scale_key.value.x, scale_key.value.y, scale_key.value.z],
                            });
                        }
                    }
                    Some(other) => {
                        log!("Warning: Unknown $AssimpFbx$ channel type '{}' in {}", other, channel_name);
                    }
                }
            }

            // Merge duplicate rotation keyframes (same time) - FBX often has quaternion components as separate keys
            for bone_anim in fbx_animation.bone_animations.values_mut() {
                if bone_anim.rotation_keys.len() > 1 {
                    let original_count = bone_anim.rotation_keys.len();
                    bone_anim.rotation_keys = merge_duplicate_rotation_keys(&bone_anim.rotation_keys);
                    if bone_anim.rotation_keys.len() != original_count && (bone_anim.bone_name == "b_Head" || bone_anim.bone_name == "b_Root") {
                        log!("  Merged {} rotation keys for {} (was {}, now {})",
                             original_count - bone_anim.rotation_keys.len(),
                             bone_anim.bone_name,
                             original_count,
                             bone_anim.rotation_keys.len());
                    }
                }
            }

            // Debug: Log bone animation info for key bones
            for bone_name in &["b_Root", "b_Head", "B_Tail_0", "B_Tail_1", "B_Spine", "b_Neck_0"] {
                if let Some(bone_anim) = fbx_animation.bone_animations.get(*bone_name) {
                    log!("DEBUG: {} animation - {} translation keys, {} rotation keys, {} scale keys",
                         bone_name,
                         bone_anim.translation_keys.len(),
                         bone_anim.rotation_keys.len(),
                         bone_anim.scale_keys.len());

                    // Log first and last keyframe values
                    if !bone_anim.translation_keys.is_empty() {
                        let first = &bone_anim.translation_keys[0];
                        let last = &bone_anim.translation_keys[bone_anim.translation_keys.len() - 1];
                        log!("  Translation: first t={:.4} v=[{:.3}, {:.3}, {:.3}], last t={:.4} v=[{:.3}, {:.3}, {:.3}]",
                             first.time, first.value[0], first.value[1], first.value[2],
                             last.time, last.value[0], last.value[1], last.value[2]);
                    }
                    if !bone_anim.rotation_keys.is_empty() {
                        let first = &bone_anim.rotation_keys[0];
                        let last = &bone_anim.rotation_keys[bone_anim.rotation_keys.len() - 1];
                        log!("  Rotation: first t={:.4} quat=[{:.3}, {:.3}, {:.3}, {:.3}], last t={:.4} quat=[{:.3}, {:.3}, {:.3}, {:.3}]",
                             first.time, first.value.s, first.value.v.x, first.value.v.y, first.value.v.z,
                             last.time, last.value.s, last.value.v.x, last.value.v.y, last.value.v.z);

                        // Log first 10 rotation keys after merge for b_Head
                        if *bone_name == "b_Head" {
                            log!("  b_Head rotation keys after merge (showing first 10):");
                            for (i, key) in bone_anim.rotation_keys.iter().take(10).enumerate() {
                                let quat = &key.value;
                                log!("    [{:2}] t={:.4} quat=[{:.3}, {:.3}, {:.3}, {:.3}]",
                                     i, key.time, quat.s, quat.v.x, quat.v.y, quat.v.z);
                            }
                        }

                        // For b_Root, log all rotation keyframes to investigate the 90-degree rotation issue
                        if *bone_name == "b_Root" && bone_anim.rotation_keys.len() > 5 {
                            log!("  b_Root rotation keyframes (showing first 10):");
                            for (i, key) in bone_anim.rotation_keys.iter().take(10).enumerate() {
                                let quat = &key.value;
                                log!("    [{:2}] t={:.4} quat=[{:.3}, {:.3}, {:.3}, {:.3}]",
                                     i, key.time, quat.s, quat.v.x, quat.v.y, quat.v.z);
                            }
                        }
                    }
                }
            }

            // Fix duration: Find the minimum last keyframe time across all bones and all keyframe types
            // This ensures the animation loops smoothly without jumps
            // However, skip adjustment if all keyframes are at time=0 (static pose)
            let mut actual_duration = fbx_animation.duration;
            let mut has_animated_keys = false;

            for (bone_name, bone_anim) in &fbx_animation.bone_animations {
                // For each bone, find the minimum last keyframe time across all keyframe types
                // This ensures all keyframe types for this bone have data throughout
                let mut bone_min_time = f32::MAX;
                let mut has_keys = false;

                if let Some(last_trans) = bone_anim.translation_keys.last() {
                    bone_min_time = bone_min_time.min(last_trans.time);
                    has_keys = true;
                    // Check if this bone has animation (more than 1 keyframe or keyframe at time > 0)
                    if bone_anim.translation_keys.len() > 1 || last_trans.time > 0.0 {
                        has_animated_keys = true;
                    }
                }
                if let Some(last_rot) = bone_anim.rotation_keys.last() {
                    bone_min_time = bone_min_time.min(last_rot.time);
                    has_keys = true;
                    if bone_anim.rotation_keys.len() > 1 || last_rot.time > 0.0 {
                        has_animated_keys = true;
                    }
                }
                if let Some(last_scale) = bone_anim.scale_keys.last() {
                    bone_min_time = bone_min_time.min(last_scale.time);
                    has_keys = true;
                    if bone_anim.scale_keys.len() > 1 || last_scale.time > 0.0 {
                        has_animated_keys = true;
                    }
                }

                // The actual duration should be the minimum across all bones
                // This ensures ALL bones have ALL keyframe types throughout the entire duration
                // But only if bone_min_time > 0 (not a static pose)
                if has_keys && bone_min_time > 0.0 && bone_min_time < actual_duration {
                    log!("Adjusting duration based on bone '{}': {:.4} -> {:.4}", bone_name, fbx_animation.duration, bone_min_time);
                    actual_duration = bone_min_time;
                }
            }

            // Only adjust duration if there are actually animated keyframes
            if has_animated_keys && actual_duration != fbx_animation.duration {
                log!("Animation duration adjusted from {:.4}s to {:.4}s to prevent loop jumps",
                     fbx_animation.duration, actual_duration);
                fbx_animation.duration = actual_duration;
            } else if !has_animated_keys {
                log!("Animation appears to be a static pose (all keyframes at time=0), keeping original duration={:.4}s",
                     fbx_animation.duration);
            }

            fbx_model.animations.push(fbx_animation);
        }
    }

    // Build bone hierarchy and get mesh-to-node mapping
    let mesh_to_node = build_bone_hierarchy_from_scene(&scene, &mut fbx_model);

    // Check which nodes have animations
    let animated_nodes: std::collections::HashSet<String> = if !fbx_model.animations.is_empty() {
        fbx_model.animations[0].bone_animations.keys().cloned().collect()
    } else {
        std::collections::HashSet::new()
    };

    // Debug: Log animated nodes
    log!("Animated nodes:");
    for node_name in &animated_nodes {
        log!("  - {}", node_name);
    }

    // Apply initial node transform to meshes OR set parent node for animation
    log!("Setting up meshes for hierarchy animation...");
    for (mesh_idx, fbx_data) in fbx_model.fbx_data.iter_mut().enumerate() {
        // Only process meshes without clusters (hierarchy animation)
        if fbx_data.clusters.is_empty() {
            if let Some(node_name) = mesh_to_node.get(&mesh_idx) {
                log!("  Mesh {} belongs to node: {}", mesh_idx, node_name);

                // Check if this node or any ancestor has animation
                let mut has_animated_ancestor = false;
                let mut current = Some(node_name.clone());
                log!("    Checking node hierarchy for animations:");
                while let Some(ref node) = current {
                    log!("      Checking node: {}", node);
                    if animated_nodes.contains(node) {
                        has_animated_ancestor = true;
                        log!("        -> Found! This node has animation");
                        break;
                    }
                    current = fbx_model.nodes.get(node).and_then(|n| n.parent.clone());
                }
                if !has_animated_ancestor {
                    log!("      -> No animated ancestor found");
                }

                if has_animated_ancestor {
                    // This mesh will be animated, set parent_node for animation updates
                    fbx_data.parent_node = Some(node_name.clone());
                    log!("    Mesh {} will be animated (has animated ancestor)", mesh_idx);
                } else {
                    // This mesh is static, apply transform once at load time
                    let global_transform = compute_global_node_transform_for_hierarchy(&fbx_model.nodes, node_name);

                    for i in 0..fbx_data.positions.len() {
                        let local_pos = Point3::new(
                            fbx_data.local_positions[i].x,
                            fbx_data.local_positions[i].y,
                            fbx_data.local_positions[i].z,
                        );
                        let world_pos = global_transform.transform_point(local_pos);
                        fbx_data.positions[i] = Vector3::new(world_pos.x, world_pos.y, world_pos.z);
                    }
                    log!("    Mesh {} is static, applied transform at load time (first vertex: {:?})",
                         mesh_idx, fbx_data.positions.get(0));
                }
            } else {
                log!("  Mesh {} has no associated node", mesh_idx);
            }
        }
    }

    log!("=== FBX loading complete: {} meshes, {} animations ===",
         fbx_model.fbx_data.len(), fbx_model.animations.len());

    Ok(fbx_model)
}

/// Build bone hierarchy from russimp scene and return mesh-to-node mapping
fn build_bone_hierarchy_from_scene(scene: &Scene, fbx_model: &mut FbxModel) -> HashMap<usize, String> {
    fn traverse_node(
        node: &russimp::node::Node,
        nodes: &mut HashMap<String, BoneNode>,
        parent: Option<String>,
    ) {
        let raw_node_name = node.name.clone();

        // Note: Suffix stripping is disabled for now because it was incorrectly removing
        // original bone name suffixes like "_15" in "B_Hair_15"
        // TODO: Implement smarter suffix detection if needed for stickman_bin.fbx
        let node_name = raw_node_name;

        // Skip $AssimpFbx$ nodes completely - don't add them to hierarchy
        if node_name.contains("$AssimpFbx$") {
            // Process children with the current parent (skip this node)
            for child in node.children.borrow().iter() {
                traverse_node(child, nodes, parent.clone());
            }
            return;
        }

        // Convert russimp matrix to cgmath Matrix4
        // Russimp (Assimp) uses row-major matrices: [a1, a2, a3, a4] is row 0
        // cgmath Matrix4::new() expects column-major: each 4 arguments form a column
        // To transpose: column 0 of cgmath = [a1, b1, c1, d1] (first element of each row)
        let transform = node.transformation;
        let local_transform = Matrix4::new(
            transform.a1, transform.b1, transform.c1, transform.d1,  // Column 0 (a1=row0col0, b1=row1col0, c1=row2col0, d1=row3col0)
            transform.a2, transform.b2, transform.c2, transform.d2,  // Column 1 (a2=row0col1, b2=row1col1, c2=row2col1, d2=row3col1)
            transform.a3, transform.b3, transform.c3, transform.d3,  // Column 2
            transform.a4, transform.b4, transform.c4, transform.d4,  // Column 3
        );

        // Use identity default values - local_transform already contains the full transform
        // These defaults are only used when there are no animation keys for a specific component
        let default_translation = [0.0, 0.0, 0.0];
        let default_rotation = Quaternion::new(1.0, 0.0, 0.0, 0.0);  // 単位クォータニオン
        let default_scaling = [1.0, 1.0, 1.0];

        let bone_node = BoneNode {
            name: node_name.clone(),
            parent: parent.clone(),
            local_transform,
            default_translation,
            default_rotation,
            default_scaling,
        };

        nodes.insert(node_name.clone(), bone_node);

        // Recursively process children
        for child in node.children.borrow().iter() {
            traverse_node(child, nodes, Some(node_name.clone()));
        }
    }

    // Build mesh-to-node mapping (which node each mesh belongs to)
    fn build_mesh_node_mapping(
        node: &russimp::node::Node,
        mesh_to_node: &mut HashMap<usize, String>,
        parent_transform: Matrix4<f32>,
    ) {
        let raw_node_name = node.name.clone();

        // Note: Suffix stripping disabled (same as traverse_node)
        let node_name = raw_node_name;

        // Convert russimp matrix to cgmath Matrix4
        // Russimp (Assimp) uses row-major matrices: [a1, a2, a3, a4] is row 0
        // cgmath Matrix4::new() expects column-major: each 4 arguments form a column
        // To transpose: column 0 of cgmath = [a1, b1, c1, d1] (first element of each row)
        let transform = node.transformation;
        let local_transform = Matrix4::new(
            transform.a1, transform.b1, transform.c1, transform.d1,  // Column 0 (a1=row0col0, b1=row1col0, c1=row2col0, d1=row3col0)
            transform.a2, transform.b2, transform.c2, transform.d2,  // Column 1 (a2=row0col1, b2=row1col1, c2=row2col1, d2=row3col1)
            transform.a3, transform.b3, transform.c3, transform.d3,  // Column 2
            transform.a4, transform.b4, transform.c4, transform.d4,  // Column 3
        );

        // Calculate global transform for this node
        let global_transform = parent_transform * local_transform;

        // Map all meshes in this node to this node name
        for &mesh_idx in &node.meshes {
            mesh_to_node.insert(mesh_idx as usize, node_name.clone());
            log!("  Mesh {} belongs to node: {}", mesh_idx, node_name);
        }

        // Recursively process children
        for child in node.children.borrow().iter() {
            build_mesh_node_mapping(child, mesh_to_node, global_transform);
        }
    }

    if let Some(root) = &scene.root {
        traverse_node(root, &mut fbx_model.nodes, None);
        log!("Built bone hierarchy with {} nodes", fbx_model.nodes.len());

        // Debug: Log all nodes in the hierarchy
        log!("=== All Nodes in Hierarchy ===");
        for (name, node) in &fbx_model.nodes {
            log!("Node: {} (parent: {:?})", name, node.parent);
            log!("  local_transform (row-major display):");
            log!("    [{:.6}, {:.6}, {:.6}, {:.6}]", node.local_transform[0][0], node.local_transform[1][0], node.local_transform[2][0], node.local_transform[3][0]);
            log!("    [{:.6}, {:.6}, {:.6}, {:.6}]", node.local_transform[0][1], node.local_transform[1][1], node.local_transform[2][1], node.local_transform[3][1]);
            log!("    [{:.6}, {:.6}, {:.6}, {:.6}]", node.local_transform[0][2], node.local_transform[1][2], node.local_transform[2][2], node.local_transform[3][2]);
            log!("    [{:.6}, {:.6}, {:.6}, {:.6}]", node.local_transform[0][3], node.local_transform[1][3], node.local_transform[2][3], node.local_transform[3][3]);
        }
        log!("=== End of Nodes ===");

        // Debug: Log $AssimpFbx$ nodes
        for (name, node) in &fbx_model.nodes {
            if name.contains("$AssimpFbx$") && name.contains("b_Root") {
                log!("DEBUG: {} - parent: {:?}", name, node.parent);
                log!("DEBUG: {} - local_transform: {:?}", name, node.local_transform);
            }
        }

        if let Some(b_root) = fbx_model.nodes.get("b_Root") {
            log!("DEBUG: Bone b_Root - parent: {:?}", b_root.parent);
            log!("DEBUG: Bone b_Root - local_transform: {:?}", b_root.local_transform);
        }
        if let Some(b_head) = fbx_model.nodes.get("b_Head") {
            log!("DEBUG: Bone b_Head - parent: {:?}", b_head.parent);
        }
        if let Some(b_spine) = fbx_model.nodes.get("B_Spine") {
            log!("DEBUG: Bone B_Spine - parent: {:?}", b_spine.parent);
        }

        // Build mesh-to-node mapping
        let mut mesh_to_node = HashMap::new();
        log!("Building mesh-to-node mapping...");
        build_mesh_node_mapping(root, &mut mesh_to_node, Matrix4::identity());
        log!("Built mesh-to-node mapping with {} meshes", mesh_to_node.len());

        mesh_to_node
    } else {
        HashMap::new()
    }
}

/// Compute global transform for a node by traversing up the hierarchy
fn compute_global_node_transform(nodes: &HashMap<String, BoneNode>, node_name: &str) -> Matrix4<f32> {
    let mut transform = Matrix4::identity();
    let mut current_name = Some(node_name.to_string());

    // Traverse up the hierarchy and accumulate transforms
    while let Some(name) = current_name {
        if let Some(node) = nodes.get(&name) {
            transform = node.local_transform * transform;
            current_name = node.parent.clone();
        } else {
            break;
        }
    }

    transform
}

/// Compute global transform for hierarchy animation (with transpose fix for russimp matrices)
///
/// Russimp returns node.transformation in row-major format, but cgmath expects column-major.
/// This function applies the necessary transpose to correct the transformation for hierarchy animations.
/// Note: This is only needed for hierarchy animation. Skeletal animation doesn't need this because
/// inverse_bind_pose compensates for the transpose issue.
fn compute_global_node_transform_for_hierarchy(nodes: &HashMap<String, BoneNode>, node_name: &str) -> Matrix4<f32> {
    let transform = compute_global_node_transform(nodes, node_name);
    // Transpose to fix row-major to column-major conversion
    transform.transpose()
}

/// Convert quaternion to Euler angles (XYZ order, in radians, then converted to degrees)
fn quat_to_euler(x: f32, y: f32, z: f32, w: f32) -> [f32; 3] {
    // Convert to Euler angles (XYZ order)
    let sinr_cosp = 2.0 * (w * x + y * z);
    let cosr_cosp = 1.0 - 2.0 * (x * x + y * y);
    let roll = sinr_cosp.atan2(cosr_cosp);

    let sinp = 2.0 * (w * y - z * x);
    let pitch = if sinp.abs() >= 1.0 {
        std::f32::consts::FRAC_PI_2.copysign(sinp)
    } else {
        sinp.asin()
    };

    let siny_cosp = 2.0 * (w * z + x * y);
    let cosy_cosp = 1.0 - 2.0 * (y * y + z * z);
    let yaw = siny_cosp.atan2(cosy_cosp);

    // Convert radians to degrees (FBX uses degrees)
    [
        roll.to_degrees(),
        pitch.to_degrees(),
        yaw.to_degrees(),
    ]
}
