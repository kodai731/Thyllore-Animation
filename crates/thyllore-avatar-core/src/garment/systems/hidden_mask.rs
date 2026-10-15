use cgmath::{InnerSpace, Vector3};

use crate::garment::components::hidden_mask::{HiddenMaskParams, MaskMesh};
use thyllore_math_core::TriangleBvh;

fn cone_directions(normal: Vector3<f32>, cone_angle_deg: f32, count: usize) -> Vec<Vector3<f32>> {
    let mut directions = Vec::with_capacity(count);
    if count == 0 {
        return directions;
    }
    directions.push(normal);
    if count == 1 {
        return directions;
    }
    let reference_axis = if normal.y.abs() < 0.9 {
        Vector3::unit_y()
    } else {
        Vector3::unit_x()
    };
    let tangent = (reference_axis - normal * normal.dot(reference_axis)).normalize();
    let bitangent = normal.cross(tangent).normalize();
    let cone_rad = cone_angle_deg.to_radians();
    for i in 1..count {
        let azimuth = 2.0 * std::f32::consts::PI * (i as f32) / (count - 1) as f32;
        let cos_a = azimuth.cos();
        let sin_a = azimuth.sin();
        let tangent_cone = tangent * cos_a + bitangent * sin_a;
        let dir = (normal * cone_rad.cos() + tangent_cone * cone_rad.sin()).normalize();
        directions.push(dir);
    }
    directions
}

pub fn find_hidden_body_vertices(
    body_positions: &[Vector3<f32>],
    body_normals: &[Vector3<f32>],
    garment: &TriangleBvh,
    params: &HiddenMaskParams,
) -> Vec<bool> {
    let mut hidden = Vec::with_capacity(body_positions.len());
    for i in 0..body_positions.len() {
        let origin = body_positions[i] + body_normals[i] * 1e-4;
        let dirs = cone_directions(body_normals[i], params.cone_angle_deg, params.ray_count);
        let hit_count = dirs
            .iter()
            .filter(|&&dir| {
                garment
                    .raycast(origin, dir, params.cover_distance)
                    .is_some()
            })
            .count();
        let ratio = if dirs.is_empty() {
            0.0
        } else {
            hit_count as f32 / dirs.len() as f32
        };
        hidden.push(ratio >= params.hidden_ratio);
    }
    hidden
}

pub fn find_hidden_body_triangles(triangles: &[[u32; 3]], hidden_vertices: &[bool]) -> Vec<bool> {
    triangles
        .iter()
        .map(|tri| {
            hidden_vertices[tri[0] as usize]
                && hidden_vertices[tri[1] as usize]
                && hidden_vertices[tri[2] as usize]
        })
        .collect()
}

pub fn find_hidden_triangle_indices(
    body: &MaskMesh,
    garments: &MaskMesh,
    params: &HiddenMaskParams,
) -> Vec<u32> {
    if garments.positions.is_empty() {
        return Vec::new();
    }
    let garment_bvh = TriangleBvh::build(&garments.positions, &garments.triangles);
    let hidden_vertices =
        find_hidden_body_vertices(&body.positions, &body.normals, &garment_bvh, params);
    body.triangles
        .iter()
        .enumerate()
        .filter(|(_, tri)| {
            hidden_vertices[tri[0] as usize]
                && hidden_vertices[tri[1] as usize]
                && hidden_vertices[tri[2] as usize]
        })
        .map(|(i, _)| i as u32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_garment_plate(y: f32, x_min: f32, x_max: f32, z_min: f32, z_max: f32) -> TriangleBvh {
        let positions: Vec<Vector3<f32>> = vec![
            Vector3::new(x_min, y, z_min),
            Vector3::new(x_max, y, z_min),
            Vector3::new(x_max, y, z_max),
            Vector3::new(x_min, y, z_max),
        ];
        let triangles: [[u32; 3]; 2] = [[0, 1, 2], [0, 2, 3]];
        TriangleBvh::build(&positions, &triangles)
    }

    #[test]
    fn test_partial_coverage() {
        let positions: Vec<Vector3<f32>> = (0..5)
            .flat_map(|ix| {
                (0..5).map(move |iz| {
                    Vector3::new(-1.0 + ix as f32 * 0.5, 0.0, -1.0 + iz as f32 * 0.5)
                })
            })
            .collect();
        let normals: Vec<Vector3<f32>> = vec![Vector3::new(0.0, 1.0, 0.0); positions.len()];
        let garment = make_garment_plate(0.05, -1.5, 0.0, -1.5, 1.5);
        let params = HiddenMaskParams {
            cover_distance: 0.1,
            cone_angle_deg: 30.0,
            ray_count: 5,
            hidden_ratio: 0.6,
        };
        let hidden = find_hidden_body_vertices(&positions, &normals, &garment, &params);
        for (position, is_hidden) in positions.iter().zip(&hidden) {
            if position.x < -0.2 {
                assert!(*is_hidden, "{position:?} should be hidden");
            }
            if position.x > 0.2 {
                assert!(!*is_hidden, "{position:?} should not be hidden");
            }
        }
    }

    #[test]
    fn test_out_of_range() {
        let positions: Vec<Vector3<f32>> = vec![Vector3::new(0.0, 0.0, 0.0)];
        let normals: Vec<Vector3<f32>> = vec![Vector3::new(0.0, 1.0, 0.0)];
        let garment = make_garment_plate(0.5, -1.0, 1.0, -1.0, 1.0);
        let params = HiddenMaskParams {
            cover_distance: 0.1,
            cone_angle_deg: 30.0,
            ray_count: 5,
            hidden_ratio: 0.6,
        };
        let hidden = find_hidden_body_vertices(&positions, &normals, &garment, &params);
        assert!(!hidden[0], "garment is out of cover_distance range");
    }

    #[test]
    fn test_cone_directions_unit_length() {
        let normal = Vector3::new(0.0, 1.0, 0.0);
        let dirs = cone_directions(normal, 30.0, 5);
        assert_eq!(dirs.len(), 5);
        for dir in &dirs {
            let len_sq = dir.dot(*dir);
            assert!(
                (len_sq - 1.0).abs() < 1e-5,
                "direction {:?} has length squared {}",
                dir,
                len_sq
            );
        }
        for dir in &dirs {
            let dot = normal.dot(*dir);
            let cos_30 = 30.0_f32.to_radians().cos();
            assert!(
                (dot - 1.0).abs() < 1e-5 || (dot - cos_30).abs() < 1e-4,
                "direction {:?} dot with normal = {}, expected ~1 or ~{}",
                dir,
                dot,
                cos_30
            );
        }
    }

    #[test]
    fn test_hidden_triangles_all_three() {
        let triangles: [[u32; 3]; 4] = [[0, 1, 2], [0, 2, 3], [0, 3, 4], [1, 2, 5]];
        let hidden_vertices = [true, true, true, false, true, true];
        let hidden_triangles = find_hidden_body_triangles(&triangles, &hidden_vertices);
        assert!(hidden_triangles[0], "triangle [0,1,2] all hidden");
        assert!(!hidden_triangles[1], "triangle [0,2,3] vertex 3 not hidden");
        assert!(!hidden_triangles[2], "triangle [0,3,4] vertex 3 not hidden");
        assert!(hidden_triangles[3], "triangle [1,2,5] all hidden");
    }

    #[test]
    fn test_find_hidden_triangle_indices_covered_and_not_covered() {
        let body = MaskMesh {
            positions: vec![
                Vector3::new(-0.5, 0.0, -0.5),
                Vector3::new(0.5, 0.0, -0.5),
                Vector3::new(0.5, 0.0, 0.5),
                Vector3::new(-0.5, 0.0, 0.5),
            ],
            normals: vec![Vector3::new(0.0, 1.0, 0.0); 4],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        };
        let params = HiddenMaskParams {
            cover_distance: 0.1,
            cone_angle_deg: 30.0,
            ray_count: 5,
            hidden_ratio: 0.6,
        };
        let garment_covering = MaskMesh {
            positions: vec![
                Vector3::new(-1.0, 0.05, -1.0),
                Vector3::new(1.0, 0.05, -1.0),
                Vector3::new(1.0, 0.05, 1.0),
                Vector3::new(-1.0, 0.05, 1.0),
            ],
            normals: vec![Vector3::new(0.0, 1.0, 0.0); 4],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        };
        let hidden = find_hidden_triangle_indices(&body, &garment_covering, &params);
        assert_eq!(hidden, vec![0, 1], "both body triangles should be hidden");

        let garment_far = MaskMesh {
            positions: vec![
                Vector3::new(-1.0, 1.0, -1.0),
                Vector3::new(1.0, 1.0, -1.0),
                Vector3::new(1.0, 1.0, 1.0),
                Vector3::new(-1.0, 1.0, 1.0),
            ],
            normals: vec![Vector3::new(0.0, 1.0, 0.0); 4],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        };
        let hidden = find_hidden_triangle_indices(&body, &garment_far, &params);
        assert!(
            hidden.is_empty(),
            "no body triangles should be hidden when garment is far"
        );
    }
}
