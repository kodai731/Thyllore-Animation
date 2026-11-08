use glam::{Mat3, Mat4, Quat, Vec3};
use thyllore_math_core::{
    engine_to_blender_point, invert, is_well_conditioned, world_to_local_points, BLENDER_TO_ENGINE,
};

use crate::avatar_setup_bridge::BoneSpec;

/// A row-major 4x4 as the addon sends it; the engine stores matrices column-major.
pub type MatrixRows = [[f32; 4]; 4];

pub fn column_major(rows: &MatrixRows) -> Mat4 {
    Mat4::from_cols_array_2d(rows).transpose()
}

pub fn rows(matrix: Mat4) -> MatrixRows {
    matrix.transpose().to_cols_array_2d()
}

/// The armature's world frame with its axes turned to engine axes; its own local axes stay.
pub fn engine_frame(armature_world: &MatrixRows) -> Mat4 {
    BLENDER_TO_ENGINE * column_major(armature_world)
}

/// `engine_frame` with its three upper rows in metres: the root frame the spring preview propagates.
pub fn engine_frame_metres(armature_world: &MatrixRows, scale_length: f32) -> Mat4 {
    let mut frame = rows(engine_frame(armature_world));
    for row in frame.iter_mut().take(3) {
        for value in row.iter_mut() {
            *value *= scale_length;
        }
    }
    column_major(&frame)
}

/// The rotation of the armature's engine frame, its axes normalized.
pub fn engine_frame_rotation(armature_world: &MatrixRows) -> Quat {
    let frame = Mat3::from_mat4(engine_frame(armature_world));
    Quat::from_mat3(&Mat3::from_cols(
        frame.x_axis.normalize(),
        frame.y_axis.normalize(),
        frame.z_axis.normalize(),
    ))
}

/// Parent-relative locals of armature-space matrices (`Bone.matrix_local`, `PoseBone.matrix`);
/// a root bone's local is `root_frame` applied to its matrix.
pub fn parent_relative_locals(
    root_frame: Mat4,
    parents: &[Option<usize>],
    matrices: &[MatrixRows],
) -> anyhow::Result<Vec<Mat4>> {
    if matrices.len() != parents.len() {
        anyhow::bail!(
            "expected one matrix per bone ({}), got {}",
            parents.len(),
            matrices.len()
        );
    }
    let globals: Vec<Mat4> = matrices.iter().map(column_major).collect();
    parents
        .iter()
        .zip(&globals)
        .enumerate()
        .map(|(index, (parent, global))| match parent {
            None => Ok(root_frame * *global),
            Some(parent) => {
                let parent_global = globals.get(*parent).ok_or_else(|| {
                    anyhow::anyhow!("bone {index} names parent {parent} outside the bone list")
                })?;
                Ok(invert(parent_global, || format!("matrix of bone {parent}"))? * *global)
            }
        })
        .collect()
}

/// An engine offset in metres as a translation of the bone's `PoseBone.location`: Blender axes,
/// scene units, then into the armature's and the bone's own axes.
pub fn bone_local_offset(
    armature_world: &MatrixRows,
    bone_rest_matrix: &MatrixRows,
    offset_metres: Vec3,
    scale_length: f32,
) -> anyhow::Result<Vec3> {
    let world_delta = engine_to_blender_point(offset_metres) / scale_length;
    let armature_delta = invert3(armature_world, "armature world matrix")? * world_delta;
    Ok(invert3(bone_rest_matrix, "bone rest matrix")? * armature_delta)
}

/// `Bone.matrix_local`-space points of Blender world points.
pub fn armature_local_points(
    armature_world: &MatrixRows,
    world_points: &[[f32; 3]],
) -> anyhow::Result<Vec<[f32; 3]>> {
    world_to_local_points(&column_major(armature_world), world_points)
}

/// The bones of a Blender armature as the wheel's `BoneSpec`s: `rest_matrices` are the bones'
/// `matrix_local` rows, `parents` their indices in the same list.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoneSpecsRequest {
    pub names: Vec<String>,
    pub parents: Vec<Option<usize>>,
    pub armature_world: MatrixRows,
    pub rest_matrices: Vec<MatrixRows>,
    pub scale_length: f32,
}

/// The armature's world matrix carries the Z-up to Y-up conversion the FBX exporter and importer
/// place on it, so a bone's frame is converted on its output side only; conjugating it would rotate
/// the bone's own axes a second time.
pub fn bone_specs(request: &BoneSpecsRequest) -> anyhow::Result<Vec<BoneSpec>> {
    if request.rest_matrices.len() != request.names.len()
        || request.parents.len() != request.names.len()
    {
        anyhow::bail!(
            "expected one parent and one rest matrix per bone ({}), got {} and {}",
            request.names.len(),
            request.parents.len(),
            request.rest_matrices.len()
        );
    }
    let armature_world = column_major(&request.armature_world);
    Ok(request
        .names
        .iter()
        .zip(&request.parents)
        .zip(&request.rest_matrices)
        .map(|((name, parent), rest_matrix)| {
            let engine = BLENDER_TO_ENGINE * armature_world * column_major(rest_matrix);
            let translation_metres = engine.w_axis.truncate() * request.scale_length;
            let rest_frame = Mat3::from_mat4(engine) * request.scale_length;
            let mut bind_matrix = engine;
            bind_matrix.w_axis = translation_metres.extend(1.0);
            BoneSpec {
                name: name.clone(),
                parent: *parent,
                rest_position: translation_metres.into(),
                rest_frame: Some(rest_frame.transpose().to_cols_array_2d()),
                bind_matrix: Some(rows(bind_matrix)),
            }
        })
        .collect())
}

fn invert3(rows: &MatrixRows, what: &str) -> anyhow::Result<Mat3> {
    let matrix = column_major(rows);
    if !is_well_conditioned(matrix) {
        anyhow::bail!("{what} is not invertible");
    }
    Ok(Mat3::from_mat4(matrix).inverse())
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: MatrixRows = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    fn translation(x: f32, y: f32, z: f32) -> MatrixRows {
        rows(Mat4::from_translation(Vec3::new(x, y, z)))
    }

    #[test]
    fn child_local_is_its_matrix_relative_to_the_parent() {
        let locals = parent_relative_locals(
            Mat4::IDENTITY,
            &[None, Some(0)],
            &[translation(0.0, 0.0, 1.0), translation(0.0, 0.0, 1.5)],
        )
        .unwrap();
        assert!(locals[1].abs_diff_eq(Mat4::from_translation(Vec3::new(0.0, 0.0, 0.5)), 1e-6));
    }

    #[test]
    fn root_local_carries_the_root_frame() {
        let root_frame = engine_frame_metres(&IDENTITY, 0.01);
        let locals =
            parent_relative_locals(root_frame, &[None], &[translation(0.0, 0.0, 1.0)]).unwrap();
        let origin = locals[0].transform_point3(Vec3::ZERO);
        assert!(
            origin.abs_diff_eq(Vec3::new(0.0, 0.01, 0.0), 1e-6),
            "{origin:?}"
        );
    }

    #[test]
    fn bone_local_offset_undoes_the_axis_swap_and_the_unit() {
        let offset =
            bone_local_offset(&IDENTITY, &IDENTITY, Vec3::new(0.0, 0.02, 0.0), 0.01).unwrap();
        assert!(
            offset.abs_diff_eq(Vec3::new(0.0, 0.0, 2.0), 1e-6),
            "{offset:?}"
        );
    }

    #[test]
    fn armature_local_points_undo_the_armature_transform() {
        let points =
            armature_local_points(&translation(1.0, 0.0, 0.0), &[[1.5, 0.0, 0.0]]).unwrap();
        assert_eq!(points, vec![[0.5, 0.0, 0.0]]);
    }

    #[test]
    fn a_singular_armature_matrix_is_reported() {
        let flat = rows(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.0)));
        assert!(armature_local_points(&flat, &[]).is_err());
    }

    #[test]
    fn a_uniformly_small_armature_stays_invertible() {
        let tiny = rows(Mat4::from_scale(Vec3::splat(0.001)));
        let points = armature_local_points(&tiny, &[[0.001, 0.0, 0.0]]).unwrap();
        assert!(
            Vec3::from(points[0]).abs_diff_eq(Vec3::X, 1e-4),
            "{points:?}"
        );
        assert!(bone_local_offset(&tiny, &tiny, Vec3::Y, 1.0).is_ok());
    }

    fn rotation_x_90() -> MatrixRows {
        rows(Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2))
    }

    fn specs_of(
        armature_world: MatrixRows,
        rest_matrix: MatrixRows,
        scale_length: f32,
    ) -> BoneSpec {
        let mut specs = bone_specs(&BoneSpecsRequest {
            names: vec!["Hips".into()],
            parents: vec![None],
            armature_world,
            rest_matrices: vec![rest_matrix],
            scale_length,
        })
        .unwrap();
        specs.remove(0)
    }

    #[test]
    fn an_fbx_import_armature_keeps_the_bone_axes() {
        let spec = specs_of(rotation_x_90(), translation(0.0, 1.55, -0.03), 1.0);
        let bind = column_major(&spec.bind_matrix.unwrap());
        assert!(Mat3::from_mat4(bind).abs_diff_eq(Mat3::IDENTITY, 1e-6));
        assert!(Vec3::from(spec.rest_position).abs_diff_eq(Vec3::new(0.0, 1.55, -0.03), 1e-6));
    }

    #[test]
    fn a_native_armature_gets_the_axis_conversion_once() {
        let spec = specs_of(IDENTITY, translation(0.0, 0.0, 2.0), 1.0);
        let expected = BLENDER_TO_ENGINE * Mat4::from_translation(Vec3::new(0.0, 0.0, 2.0));
        assert!(column_major(&spec.bind_matrix.unwrap()).abs_diff_eq(expected, 1e-6));
        assert_eq!(spec.rest_position, [0.0, 2.0, 0.0]);
    }

    #[test]
    fn scale_length_scales_the_translation_and_the_rest_frame_only() {
        let spec = specs_of(IDENTITY, translation(0.0, 0.0, 2.0), 0.01);
        let bind = spec.bind_matrix.unwrap();
        assert!((bind[1][3] - 0.02).abs() < 1e-6);
        assert_eq!(bind[1][2], 1.0);
        assert_eq!(spec.rest_frame.unwrap()[1][2], 0.01);
    }
}
