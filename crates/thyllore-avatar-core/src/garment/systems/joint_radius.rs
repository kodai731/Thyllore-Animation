use cgmath::{InnerSpace, Vector3};

use crate::garment::components::chain_gen::{GeneratedChain, InfluenceTarget, VertexInfluences};

/// Per chain and joint, the median distance of the vertices the joint dominates from its bone segment.
pub fn joint_half_thicknesses(
    positions: &[Vector3<f32>],
    chains: &[GeneratedChain],
    influences: &[VertexInfluences],
) -> Vec<Vec<f32>> {
    let mut distances: Vec<Vec<Vec<f32>>> = chains
        .iter()
        .map(|chain| vec![Vec::new(); chain.joints.len()])
        .collect();

    for vertex_influences in influences {
        let Some((chain, joint)) = dominant_joint(vertex_influences) else {
            continue;
        };
        let (head, tail) = joint_segment(&chains[chain], joint);
        let distance =
            distance_to_segment(positions[vertex_influences.vertex as usize], head, tail);
        distances[chain][joint].push(distance);
    }

    distances
        .into_iter()
        .map(|chain| chain.into_iter().map(median_or_zero).collect())
        .collect()
}

fn dominant_joint(vertex_influences: &VertexInfluences) -> Option<(usize, usize)> {
    let (target, _) = vertex_influences
        .influences
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    match target {
        InfluenceTarget::Root => None,
        InfluenceTarget::Joint { chain, joint } => Some((*chain, *joint)),
    }
}

fn joint_segment(chain: &GeneratedChain, joint: usize) -> (Vector3<f32>, Vector3<f32>) {
    let head = chain.joints[joint].head;
    let tail = chain
        .joints
        .get(joint + 1)
        .map_or(chain.tail, |next| next.head);
    (head, tail)
}

fn distance_to_segment(point: Vector3<f32>, a: Vector3<f32>, b: Vector3<f32>) -> f32 {
    let ab = b - a;
    let len_sq = ab.dot(ab);
    if len_sq < 1e-12 {
        return (point - a).magnitude();
    }
    let t = ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    (point - (a + ab * t)).magnitude()
}

fn median_or_zero(mut values: Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::garment::components::chain_gen::GeneratedJoint;

    fn chain_along_y(heads: &[f32]) -> GeneratedChain {
        GeneratedChain {
            column: 0,
            joints: heads
                .iter()
                .enumerate()
                .map(|(index, &y)| GeneratedJoint {
                    name: format!("j{index}"),
                    head: Vector3::new(0.0, y, 0.0),
                })
                .collect(),
            tail: Vector3::new(0.0, heads[heads.len() - 1] - 1.0, 0.0),
            vertices: Vec::new(),
        }
    }

    fn influence(vertex: u32, joint: usize, weight: f32) -> VertexInfluences {
        VertexInfluences {
            vertex,
            influences: vec![
                (InfluenceTarget::Joint { chain: 0, joint }, weight),
                (InfluenceTarget::Root, 1.0 - weight),
            ],
        }
    }

    #[test]
    fn half_thickness_is_the_median_offset_from_the_bone() {
        let chain = chain_along_y(&[2.0, 1.0]);
        let positions = vec![
            Vector3::new(0.02, 1.5, 0.0),
            Vector3::new(-0.02, 1.7, 0.0),
            Vector3::new(0.05, 1.2, 0.0),
            Vector3::new(0.01, 0.5, 0.0),
        ];
        let influences = vec![
            influence(0, 0, 0.9),
            influence(1, 0, 0.9),
            influence(2, 0, 0.9),
            influence(3, 1, 0.9),
        ];

        let radii = joint_half_thicknesses(&positions, &[chain], &influences);
        assert!((radii[0][0] - 0.02).abs() < 1e-6);
        assert!((radii[0][1] - 0.01).abs() < 1e-6);
    }

    #[test]
    fn joints_without_dominated_vertices_get_zero() {
        let chain = chain_along_y(&[2.0, 1.0]);
        let positions = vec![Vector3::new(0.03, 1.5, 0.0)];
        let influences = vec![influence(0, 0, 0.3)];

        let radii = joint_half_thicknesses(&positions, &[chain], &influences);
        assert_eq!(radii, vec![vec![0.0, 0.0]]);
    }
}
