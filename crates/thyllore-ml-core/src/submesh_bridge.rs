use cgmath::Vector3;

use thyllore_avatar_core::cloth_setup::components::hidden_mask::{HiddenMaskParams, MaskMesh};
use thyllore_avatar_core::cloth_setup::components::submesh_role::{
    guess_submesh_role as core_guess_submesh_role, SubmeshKey,
};
use thyllore_avatar_core::cloth_setup::systems::hidden_mask::{
    find_hidden_triangle_indices, merge_mask_meshes,
};

#[derive(serde::Deserialize)]
pub struct MaskMeshInput {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

fn to_mask_mesh(input: &MaskMeshInput) -> MaskMesh {
    MaskMesh {
        positions: input.positions.iter().map(|p| Vector3::from(*p)).collect(),
        normals: input.normals.iter().map(|n| Vector3::from(*n)).collect(),
        triangles: input.triangles.clone(),
    }
}

pub fn guess_submesh_role(mesh: &str, material: &str) -> String {
    let key = SubmeshKey {
        mesh: mesh.to_string(),
        material: material.to_string(),
    };
    let role = core_guess_submesh_role(&key);
    match serde_json::to_value(role).expect("SubmeshRole serializes to JSON") {
        serde_json::Value::String(name) => name,
        _ => panic!("SubmeshRole is serialized as a string"),
    }
}

pub fn find_hidden_triangles(
    body: &MaskMeshInput,
    garments: &[MaskMeshInput],
    params: &HiddenMaskParams,
) -> Vec<u32> {
    let body_mesh = to_mask_mesh(body);
    let garment_meshes: Vec<MaskMesh> = garments.iter().map(to_mask_mesh).collect();
    let merged_garment = merge_mask_meshes(&garment_meshes);
    find_hidden_triangle_indices(&body_mesh, &merged_garment, params)
}

pub fn hidden_mask_defaults() -> HiddenMaskParams {
    HiddenMaskParams::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submesh_bridge_guess_hair() {
        let role = guess_submesh_role("woman 1", "Hair");
        assert_eq!(role, "hair");
    }

    #[test]
    fn submesh_bridge_guess_skin() {
        let role = guess_submesh_role("woman 1", "Body");
        assert_eq!(role, "skin");
    }

    #[test]
    fn submesh_bridge_find_hidden_triangles_covered() {
        let body = MaskMeshInput {
            positions: vec![
                [-0.5, 0.0, -0.5],
                [0.5, 0.0, -0.5],
                [0.5, 0.0, 0.5],
                [-0.5, 0.0, 0.5],
            ],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        };

        let garments = vec![MaskMeshInput {
            positions: vec![
                [-1.0, 0.05, -1.0],
                [1.0, 0.05, -1.0],
                [1.0, 0.05, 1.0],
                [-1.0, 0.05, 1.0],
            ],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        }];

        let params = HiddenMaskParams {
            cover_distance: 0.1,
            cone_angle_deg: 30.0,
            ray_count: 5,
            hidden_ratio: 0.6,
        };

        let hidden = find_hidden_triangles(&body, &garments, &params);
        assert_eq!(hidden, vec![0, 1], "both body triangles should be hidden");
    }

    #[test]
    fn submesh_bridge_find_hidden_triangles_empty_garments() {
        let body = MaskMeshInput {
            positions: vec![[-0.5, 0.0, -0.5], [0.5, 0.0, -0.5], [0.5, 0.0, 0.5]],
            normals: vec![[0.0, 1.0, 0.0]; 3],
            triangles: vec![[0, 1, 2]],
        };

        let params = HiddenMaskParams {
            cover_distance: 0.1,
            cone_angle_deg: 30.0,
            ray_count: 5,
            hidden_ratio: 0.6,
        };

        let hidden = find_hidden_triangles(&body, &[], &params);
        assert!(
            hidden.is_empty(),
            "no triangles should be hidden with empty garments"
        );
    }
}
