use std::collections::BTreeSet;
use std::f32::consts::{PI, TAU};

use cgmath::{InnerSpace, Vector3};

use super::*;
use crate::cloth_setup::components::chain_gen::{
    joint_name, ChainGenInput, ChainGenParams, ChainGenSettings, ChainRole, GeneratedChain,
    InfluenceTarget, VertexInfluences,
};
use crate::cloth_setup::components::submesh_role::SubmeshSelector;
use thyllore_model_core::INFLUENCES_PER_VERTEX;

fn build_straight_strand() -> (Vec<Vector3<f32>>, Vec<[u32; 3]>) {
    let positions = (1..=5)
        .map(|step| Vector3::new(0.0, -0.1 * step as f32, 0.0))
        .collect();
    (positions, vec![[0, 1, 2], [2, 3, 4]])
}

fn build_params(role: ChainRole, joint_count: u32, column_count: u32) -> ChainGenParams {
    ChainGenParams {
        role,
        target: SubmeshSelector::Mesh("test".to_string()),
        settings: ChainGenSettings {
            joint_count,
            column_count,
            ..ChainGenSettings::default()
        },
    }
}

fn build_hair_params(joint_count: u32) -> ChainGenParams {
    build_params(ChainRole::Hair, joint_count, 1)
}

fn head_attachment() -> Vec<Vector3<f32>> {
    (0..8)
        .map(|i| {
            let angle = TAU * i as f32 / 8.0;
            Vector3::new(0.2 * angle.cos(), 0.1, 0.2 * angle.sin())
        })
        .collect()
}

fn build_cap_with_two_cards() -> (Vec<Vector3<f32>>, Vec<[u32; 3]>) {
    let mut positions = Vec::new();
    let mut triangles = Vec::new();

    let cap_ring = |y: f32, radius: f32| -> Vec<Vector3<f32>> {
        (0..12)
            .map(|i| {
                let angle = TAU * i as f32 / 12.0;
                Vector3::new(radius * angle.cos(), y, radius * angle.sin())
            })
            .collect()
    };
    for (upper, lower) in [(0.25f32, 0.1), (0.15, 0.2)]
        .iter()
        .zip([(0.15f32, 0.2), (0.05, 0.22)])
    {
        let base = positions.len() as u32;
        positions.extend(cap_ring(upper.0, upper.1));
        positions.extend(cap_ring(lower.0, lower.1));
        for i in 0..12u32 {
            let next = (i + 1) % 12;
            triangles.push([base + i, base + next, base + 12 + i]);
            triangles.push([base + next, base + 12 + next, base + 12 + i]);
        }
    }

    for angle in [PI * 0.75, -PI * 0.75] {
        let base = positions.len() as u32;
        let (sin, cos) = angle.sin_cos();
        for step in 0..5 {
            let y = 0.05 - 0.08 * step as f32;
            positions.push(Vector3::new(0.21 * cos, y, 0.21 * sin));
            positions.push(Vector3::new(
                0.21 * cos + 0.01 * sin,
                y,
                0.21 * sin - 0.01 * cos,
            ));
        }
        for step in 0..4u32 {
            let a = base + 2 * step;
            triangles.push([a, a + 1, a + 2]);
            triangles.push([a + 1, a + 3, a + 2]);
        }
    }
    (positions, triangles)
}

#[test]
fn straight_strand_places_heads_at_equal_spacing() {
    let head = head_attachment();
    let (positions, triangles) = build_straight_strand();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };

    let params = build_hair_params(4);
    let chains = place_chains(&input, &params).unwrap();

    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].vertices, vec![0, 1, 2, 3, 4]);
    assert_eq!(chains[0].joints.len(), 4);
    assert_eq!(chains[0].joints[0].name, joint_name(&params, 0, 0));
    for (joint_index, joint) in chains[0].joints.iter().enumerate() {
        let expected = Vector3::new(0.0, -0.1 - 0.1 * joint_index as f32, 0.0);
        assert!(
            (joint.head - expected).magnitude() < 1e-4,
            "joint {joint_index}: {:?}",
            joint.head
        );
    }
    assert!((chains[0].tail - Vector3::new(0.0, -0.5, 0.0)).magnitude() < 1e-4);
}

#[test]
fn zero_joint_count_is_rejected() {
    let head = head_attachment();
    let (positions, triangles) = build_straight_strand();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };

    assert!(place_chains(&input, &build_hair_params(0)).is_err());
}

#[test]
fn garment_column_split_by_azimuth() {
    let mut positions: Vec<Vector3<f32>> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let ring_count = 2;
    let samples_per_ring = 24;
    let radius = 0.3;
    let y_offsets = [-0.2f32, -0.6];

    for &y in &y_offsets {
        for i in 0..samples_per_ring {
            let angle = TAU * i as f32 / samples_per_ring as f32;
            positions.push(Vector3::new(radius * angle.cos(), y, radius * angle.sin()));
        }
    }

    for ring in 0..ring_count {
        let base = (ring * samples_per_ring) as u32;
        for i in 0..samples_per_ring {
            let a = base + i as u32;
            let b = base + (i + 1) as u32 % samples_per_ring as u32;
            if ring < ring_count - 1 {
                let c = base + samples_per_ring as u32 + i as u32;
                let d = base + samples_per_ring as u32 + (i + 1) as u32 % samples_per_ring as u32;
                triangles.push([a, b, c]);
                triangles.push([b, d, c]);
            }
        }
    }

    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &[],
        tex_coords: &[],
    };
    let params = build_params(ChainRole::Garment, 2, 8);

    let chains = place_chains(&input, &params).unwrap();
    assert_eq!(chains.len(), 8);

    let mut angles: Vec<f32> = chains
        .iter()
        .map(|chain| {
            let head = chain.joints[0].head;
            head.x.atan2(head.z)
        })
        .collect();
    angles.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let wrapped_first = angles[0] + TAU;
    let expected_gap = TAU / 8.0;
    for (index, next) in angles.iter().skip(1).chain([&wrapped_first]).enumerate() {
        let gap = next - angles[index];
        assert!(
            (gap - expected_gap).abs() < 1f32.to_radians(),
            "azimuth gap {gap:.4} vs expected {expected_gap:.4}"
        );
    }
}

#[test]
fn hair_azimuth_mode_detection() {
    let head = head_attachment();
    let mut positions: Vec<Vector3<f32>> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();

    let mode_centers = [0.5f32, 3.5];
    let count_per_mode = 6;

    for &center in &mode_centers {
        for i in 0..count_per_mode {
            let angle = center + (i as f32 - 2.5) * 0.05;
            let y = -0.1 * (i as f32 + 1.0);
            positions.push(Vector3::new(angle.cos() * 0.3, y, angle.sin() * 0.3));
        }
    }

    for mode in 0..mode_centers.len() {
        let base = (mode * count_per_mode) as u32;
        for i in 0..count_per_mode - 2 {
            triangles.push([base + i as u32, base + i as u32 + 1, base + i as u32 + 2]);
        }
    }

    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };
    let params = build_params(ChainRole::Hair, 2, 0);

    let chains = place_chains(&input, &params).unwrap();
    assert_eq!(chains.len(), 2);
}

#[test]
fn weights_sum_to_one() {
    let head = head_attachment();
    let (positions, triangles) = build_straight_strand();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };

    let chains = place_chains(&input, &build_hair_params(4)).unwrap();
    let weights = compute_chain_weights(&input, &build_hair_params(4), &chains);

    for entry in &weights {
        let sum: f32 = entry.influences.iter().map(|(_, w)| w).sum();
        assert!(
            (sum - 1.0).abs() < 1e-5,
            "vertex {}: weight sum = {:.6}",
            entry.vertex,
            sum
        );
    }
}

#[test]
fn midpoint_vertex_has_equal_joint_weights() {
    let head = head_attachment();
    let (mut positions, triangles) = build_straight_strand();
    let mut chains = place_chains(
        &ChainGenInput {
            positions: &positions,
            triangles: &triangles,
            root_position: Vector3::new(0.0, 0.0, 0.0),
            attachment_positions: &head,
            tex_coords: &[],
        },
        &build_hair_params(4),
    )
    .unwrap();

    let midpoint_vertex = positions.len() as u32;
    positions.push(Vector3::new(0.0, -0.25, 0.0));
    chains[0].vertices.push(midpoint_vertex);
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };
    let weights = compute_chain_weights(&input, &build_hair_params(4), &chains);

    let midpoint_entry = weights
        .iter()
        .find(|e| e.vertex == midpoint_vertex)
        .expect("midpoint vertex not found");
    let mut joint1_weight = 0.0f32;
    let mut joint2_weight = 0.0f32;
    for (target, w) in &midpoint_entry.influences {
        match *target {
            InfluenceTarget::Joint { chain: _, joint } => match joint {
                1 => joint1_weight = *w,
                2 => joint2_weight = *w,
                _ => {}
            },
            InfluenceTarget::Root => {}
        }
    }
    assert!(
        (joint1_weight - 0.5).abs() < 1e-4,
        "joint1 weight = {:.6}, expected ~0.5",
        joint1_weight
    );
    assert!(
        (joint2_weight - 0.5).abs() < 1e-4,
        "joint2 weight = {:.6}, expected ~0.5",
        joint2_weight
    );
}

#[test]
fn first_vertex_has_full_root_weight() {
    let head = head_attachment();
    let (positions, triangles) = build_straight_strand();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };

    let chains = place_chains(&input, &build_hair_params(4)).unwrap();
    let weights = compute_chain_weights(&input, &build_hair_params(4), &chains);

    let first_entry = weights
        .iter()
        .find(|e| e.vertex == 0)
        .expect("vertex 0 not found");
    let mut root_weight = 0.0f32;
    for (target, w) in &first_entry.influences {
        if let InfluenceTarget::Root = target {
            root_weight = *w;
        }
    }
    assert!(
        (root_weight - 1.0).abs() < 1e-5,
        "vertex 0 root weight = {:.6}, expected 1.0",
        root_weight
    );
}

#[test]
fn hair_joints_y_monotonically_decreasing() {
    let head = head_attachment();
    let positions: Vec<Vector3<f32>> = (0..6)
        .map(|i| Vector3::new(0.0, 0.2 - 0.1 * i as f32, 0.0))
        .collect();
    let triangles = vec![[0u32, 1, 2], [2, 3, 4], [4, 5, 0]];
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };

    let chains = place_chains(&input, &build_hair_params(3)).unwrap();
    assert_eq!(chains.len(), 1);
    let joints = &chains[0].joints;

    for i in 1..joints.len() {
        assert!(
            joints[i].head.y < joints[i - 1].head.y,
            "joint[{}].head.y ({:.4}) >= joint[{}].head.y ({:.4})",
            i,
            joints[i].head.y,
            i - 1,
            joints[i - 1].head.y
        );
    }
}

#[test]
fn reversed_vertex_order_same_joints() {
    let head = head_attachment();
    let positions: Vec<Vector3<f32>> = (0..6)
        .map(|i| Vector3::new(0.0, 0.2 - 0.1 * i as f32, 0.0))
        .collect();
    let triangles = vec![[0u32, 1, 2], [2, 3, 4], [4, 5, 0]];
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };

    let chains = place_chains(&input, &build_hair_params(2)).unwrap();
    let original_joints: Vec<Vector3<f32>> = chains[0].joints.iter().map(|j| j.head).collect();

    let reversed_positions: Vec<Vector3<f32>> = positions.iter().rev().cloned().collect();
    let reversed_triangles = vec![[5u32, 4, 3], [3, 2, 0], [0, 1, 5]];
    let reversed_input = ChainGenInput {
        positions: &reversed_positions,
        triangles: &reversed_triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };

    let reversed_chains = place_chains(&reversed_input, &build_hair_params(2)).unwrap();
    let reversed_joints: Vec<Vector3<f32>> =
        reversed_chains[0].joints.iter().map(|j| j.head).collect();

    assert_eq!(original_joints.len(), reversed_joints.len());
    for (i, (a, b)) in original_joints.iter().zip(&reversed_joints).enumerate() {
        let diff = (a.x - b.x).hypot(a.y - b.y).hypot(a.z - b.z);
        assert!(
            diff < 1e-5,
            "joint[{}] head differs: original={:?}, reversed={:?}, diff={:.6}",
            i,
            a,
            b,
            diff
        );
    }
}

fn build_two_strands() -> (Vec<Vector3<f32>>, Vec<[u32; 3]>) {
    let mut positions = Vec::new();
    let mut triangles = Vec::new();
    for (strand, x) in [(0u32, -0.05f32), (1, 0.05)] {
        let base = positions.len() as u32;
        for step in 0..4 {
            positions.push(Vector3::new(x, -0.1 * step as f32 - 0.1, 0.0));
            positions.push(Vector3::new(x, -0.1 * step as f32 - 0.1, 0.02));
        }
        for step in 0..3u32 {
            let a = base + 2 * step;
            triangles.push([a, a + 1, a + 2]);
            triangles.push([a + 1, a + 3, a + 2]);
        }
        let _ = strand;
    }
    (positions, triangles)
}

fn total_weight_on_chain(entry: &VertexInfluences, chain: usize) -> f32 {
    entry
        .influences
        .iter()
        .filter(
            |(target, _)| matches!(target, InfluenceTarget::Joint { chain: c, .. } if *c == chain),
        )
        .map(|(_, weight)| weight)
        .sum()
}

#[test]
fn zero_column_blend_keeps_every_vertex_on_its_own_column() {
    let (positions, triangles) = build_two_strands();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &[],
        tex_coords: &[],
    };
    let params = build_params(ChainRole::Garment, 3, 2);

    let chains = place_chains(&input, &params).unwrap();
    let weights = compute_chain_weights(&input, &params, &chains);

    assert_eq!(chains.len(), 2);
    for entry in &weights {
        let chain_count = (0..2)
            .filter(|&chain| total_weight_on_chain(entry, chain) > 0.0)
            .count();
        assert!(chain_count <= 1, "vertex {} spans columns", entry.vertex);
    }
}

#[test]
fn column_blend_shares_boundary_vertices_between_neighbouring_columns() {
    let (mut positions, triangles) = build_two_strands();
    let input_for_chains = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &[],
        tex_coords: &[],
    };
    let mut params = build_params(ChainRole::Garment, 3, 2);
    let mut chains = place_chains(&input_for_chains, &params).unwrap();

    let boundary_vertex = positions.len() as u32;
    positions.push(Vector3::new(0.0, -0.25, 0.01));
    chains[0].vertices.push(boundary_vertex);
    params.settings.column_blend_width = 0.1;
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &[],
        tex_coords: &[],
    };

    let weights = compute_chain_weights(&input, &params, &chains);

    let entry = weights
        .iter()
        .find(|e| e.vertex == boundary_vertex)
        .unwrap();
    let own = total_weight_on_chain(entry, 0);
    let other = total_weight_on_chain(entry, 1);
    assert!((own - other).abs() < 1e-4, "own {own} other {other}");
    assert!(entry.influences.len() <= INFLUENCES_PER_VERTEX);
    let sum: f32 = entry.influences.iter().map(|(_, w)| w).sum();
    assert!((sum - 1.0).abs() < 1e-5);

    let far_vertex = weights.iter().find(|e| e.vertex == 0).unwrap();
    assert_eq!(total_weight_on_chain(far_vertex, 1), 0.0);
}

#[test]
fn cap_stays_on_root_and_each_card_is_one_column() {
    let (positions, triangles) = build_cap_with_two_cards();
    let head = head_attachment();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };
    let params = build_params(ChainRole::Hair, 3, 0);

    let chains = place_chains(&input, &params).unwrap();
    let weights = compute_chain_weights(&input, &params, &chains);

    assert_eq!(chains.len(), 2, "one column per card");
    let card_start = 48u32;
    for chain in &chains {
        assert!(
            chain.vertices.iter().all(|&v| v >= card_start),
            "cap vertex in a chain"
        );
        let first_card = (chain.vertices[0] - card_start) / 10;
        assert!(
            chain
                .vertices
                .iter()
                .all(|&v| (v - card_start) / 10 == first_card),
            "card split across columns: {:?}",
            chain.vertices
        );
        for joint in &chain.joints {
            assert!(
                joint.head.y < 0.1,
                "joint above the root band: {:?}",
                joint.head
            );
        }
    }
    for entry in weights.iter().filter(|e| e.vertex < card_start) {
        assert_eq!(entry.influences, vec![(InfluenceTarget::Root, 1.0)]);
    }
}

#[test]
fn root_band_offset_moves_the_band_down() {
    let (positions, triangles) = build_cap_with_two_cards();
    let head = head_attachment();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };
    let mut params = build_params(ChainRole::Hair, 3, 2);
    let baseline = place_chains(&input, &params).unwrap();
    params.settings.azimuth.root_band_offset = 0.2;

    let shifted = place_chains(&input, &params).unwrap();

    let strand_count =
        |chains: &[GeneratedChain]| -> usize { chains.iter().map(|c| c.vertices.len()).sum() };
    assert!(strand_count(&shifted) < strand_count(&baseline));
    assert!(shifted[0].joints[0].head.y < baseline[0].joints[0].head.y);
}

#[test]
fn root_band_without_attachment_uses_the_widest_slice_of_the_target() {
    let (positions, triangles) = build_cap_with_two_cards();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &[],
        tex_coords: &[],
    };

    let chains = place_chains(&input, &build_params(ChainRole::Hair, 3, 2)).unwrap();

    assert_eq!(chains.len(), 2);
    assert!(chains.iter().all(|c| c.vertices.iter().all(|&v| v >= 48)));
}

#[test]
fn component_labels_separate_the_cap_from_each_card() {
    let (positions, triangles) = build_cap_with_two_cards();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &[],
        tex_coords: &[],
    };

    let labels = component_labels(&input);

    assert_eq!(labels.len(), positions.len());
    let distinct: BTreeSet<usize> = labels.iter().copied().collect();
    assert_eq!(distinct.len(), 4, "two cap rings and two cards");
    assert!(labels[48..58].iter().all(|&l| l == 48));
    assert!(labels[58..68].iter().all(|&l| l == 58));
}

#[test]
fn hair_root_band_reports_the_equator_and_the_offset_band() {
    let (positions, triangles) = build_cap_with_two_cards();
    let head = head_attachment();
    let input = ChainGenInput {
        positions: &positions,
        triangles: &triangles,
        root_position: Vector3::new(0.0, 0.0, 0.0),
        attachment_positions: &head,
        tex_coords: &[],
    };
    let mut params = build_params(ChainRole::Hair, 3, 2);
    params.settings.azimuth.root_band_offset = 0.05;

    let band = hair_root_band(&input, &params).unwrap();

    assert!((band.progress - band.equator_progress - 0.05).abs() < 1e-6);
    assert!(
        (band.center.y - 0.1).abs() < 1e-6,
        "ring height {:?}",
        band.center
    );
    assert!(hair_root_band(&input, &build_params(ChainRole::Garment, 3, 2)).is_none());
}
