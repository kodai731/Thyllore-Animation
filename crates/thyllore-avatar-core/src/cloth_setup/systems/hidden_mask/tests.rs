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
            (0..5).map(move |iz| Vector3::new(-1.0 + ix as f32 * 0.5, 0.0, -1.0 + iz as f32 * 0.5))
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

#[test]
fn test_order_visible_triangles_first_empty() {
    let indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
    let (reordered, visible_count) = order_visible_triangles_first(&indices, &[]);
    assert_eq!(reordered, indices);
    assert_eq!(visible_count, 9);
}

#[test]
fn test_order_visible_triangles_first_one_hidden() {
    let indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
    let (reordered, visible_count) = order_visible_triangles_first(&indices, &[1]);
    assert_eq!(reordered, [0, 1, 2, 6, 7, 8, 3, 4, 5]);
    assert_eq!(visible_count, 6);
}

#[test]
fn test_order_visible_triangles_first_out_of_bounds_and_duplicates() {
    let indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8];
    let (reordered, visible_count) = order_visible_triangles_first(&indices, &[1, 1, 99]);
    assert_eq!(reordered, [0, 1, 2, 6, 7, 8, 3, 4, 5]);
    assert_eq!(visible_count, 6);
}
