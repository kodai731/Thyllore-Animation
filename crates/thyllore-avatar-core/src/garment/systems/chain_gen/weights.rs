use std::collections::BTreeSet;

use cgmath::{InnerSpace, Vector3};
use thyllore_math_core::project_point_to_segment;

use super::frame::collect_target_vertices;
use crate::garment::components::chain_gen::{
    ChainGenInput, ChainGenParams, GeneratedChain, InfluenceTarget, VertexInfluences,
};

pub(super) const MAX_INFLUENCES_PER_VERTEX: usize = 4;

/// Arc length at every joint head of a chain and its total length to the tail.
struct ChainArc {
    arc: Vec<f32>,
    total_length: f32,
}

impl ChainArc {
    fn new(chain: &GeneratedChain) -> Self {
        let mut arc = vec![0.0];
        let mut current = chain.joints[0].head;
        for joint in &chain.joints[1..] {
            arc.push(arc.last().copied().unwrap() + (joint.head - current).magnitude());
            current = joint.head;
        }
        let total_length = arc.last().copied().unwrap() + (chain.tail - current).magnitude();
        Self { arc, total_length }
    }

    fn first_joint_length(&self) -> f32 {
        match self.arc.get(1) {
            Some(second) => second - self.arc[0],
            None => self.total_length,
        }
    }
}

struct PolylineHit {
    distance_squared: f32,
    arc_position: f32,
}

fn project_to_polyline(point: Vector3<f32>, chain: &GeneratedChain, arc: &ChainArc) -> PolylineHit {
    let nearest = chain
        .joints
        .iter()
        .enumerate()
        .map(|(i, joint)| {
            let (next_pos, next_arc) = match chain.joints.get(i + 1) {
                Some(next_joint) => (next_joint.head, arc.arc[i + 1]),
                None => (chain.tail, arc.total_length),
            };
            let (t, projected) = project_point_to_segment(point, joint.head, next_pos);
            PolylineHit {
                distance_squared: (point - projected).magnitude2(),
                arc_position: arc.arc[i] + t * (next_arc - arc.arc[i]),
            }
        })
        .min_by(|a, b| a.distance_squared.total_cmp(&b.distance_squared));

    match nearest {
        Some(hit) => PolylineHit {
            arc_position: hit.arc_position.clamp(0.0, arc.total_length),
            ..hit
        },
        None => PolylineHit {
            distance_squared: 0.0,
            arc_position: 0.0,
        },
    }
}

fn joint_weights_at_arc(s: f32, arc: &ChainArc) -> Vec<(usize, f32)> {
    let last_joint_index = arc.arc.len() - 1;
    if s >= arc.total_length {
        return vec![(last_joint_index, 1.0)];
    }

    for (i, &arc_i) in arc.arc.iter().enumerate() {
        if i == last_joint_index {
            return vec![(i, 1.0)];
        }
        let arc_next = arc.arc[i + 1];
        if s < arc_next {
            let segment_length = arc_next - arc_i;
            if segment_length < 1e-10 {
                return vec![(i, 1.0)];
            }
            let f = (s - arc_i) / segment_length;
            return vec![(i, 1.0 - f), (i + 1, f)];
        }
    }

    vec![(last_joint_index, 1.0)]
}

fn chain_influences(
    chain_index: usize,
    arc_position: f32,
    arc: &ChainArc,
    scale: f32,
) -> impl Iterator<Item = (InfluenceTarget, f32)> {
    joint_weights_at_arc(arc_position, arc)
        .into_iter()
        .map(move |(joint, weight)| {
            (
                InfluenceTarget::Joint {
                    chain: chain_index,
                    joint,
                },
                weight * scale,
            )
        })
}

fn find_nearest_other_chain(
    point: Vector3<f32>,
    own_chain: usize,
    chains: &[GeneratedChain],
    arcs: &[ChainArc],
) -> Option<(usize, PolylineHit)> {
    chains
        .iter()
        .zip(arcs)
        .enumerate()
        .filter(|(index, _)| *index != own_chain)
        .map(|(index, (chain, arc))| (index, project_to_polyline(point, chain, arc)))
        .min_by(|a, b| a.1.distance_squared.total_cmp(&b.1.distance_squared))
}

/// Share of the neighbouring column: 0 beyond `width` from the boundary, 0.5 at equal distance.
fn column_blend_share(own_distance: f32, other_distance: f32, width: f32) -> f32 {
    ((own_distance - other_distance + width) / (2.0 * width)).clamp(0.0, 1.0)
}

fn apply_root_blend(influences: &mut Vec<(InfluenceTarget, f32)>, arc_position: f32, width: f32) {
    if arc_position >= width {
        return;
    }
    let chain_share = arc_position / width;
    for influence in influences.iter_mut() {
        influence.1 *= chain_share;
    }
    influences.push((InfluenceTarget::Root, 1.0 - chain_share));
}

fn normalize_influences(influences: &mut Vec<(InfluenceTarget, f32)>) {
    influences.retain(|(_, weight)| *weight > 0.0);
    influences.sort_by(|a, b| b.1.total_cmp(&a.1));
    influences.truncate(MAX_INFLUENCES_PER_VERTEX);

    let sum: f32 = influences.iter().map(|(_, w)| w).sum();
    if sum > 1e-8 {
        for influence in influences.iter_mut() {
            influence.1 /= sum;
        }
    }
}

fn vertex_influences(
    vertex: u32,
    chain_index: usize,
    input: &ChainGenInput,
    params: &ChainGenParams,
    chains: &[GeneratedChain],
    arcs: &[ChainArc],
) -> VertexInfluences {
    let point = input.positions[vertex as usize];
    let own = project_to_polyline(point, &chains[chain_index], &arcs[chain_index]);

    let blend_width = params.settings.column_blend_width;
    let neighbour = (blend_width > 0.0)
        .then(|| find_nearest_other_chain(point, chain_index, chains, arcs))
        .flatten()
        .map(|(other_index, hit)| {
            let share = column_blend_share(
                own.distance_squared.sqrt(),
                hit.distance_squared.sqrt(),
                blend_width,
            );
            (other_index, hit, share)
        })
        .filter(|(_, _, share)| *share > 0.0);
    let own_share = neighbour.as_ref().map_or(1.0, |(_, _, share)| 1.0 - share);

    let mut influences: Vec<(InfluenceTarget, f32)> =
        chain_influences(chain_index, own.arc_position, &arcs[chain_index], own_share).collect();
    if let Some((other_index, hit, share)) = neighbour {
        influences.extend(chain_influences(
            other_index,
            hit.arc_position,
            &arcs[other_index],
            share,
        ));
    }

    let root_blend_width = match params.settings.root_blend_width {
        width if width > 0.0 => width,
        _ => arcs[chain_index].first_joint_length(),
    };
    apply_root_blend(&mut influences, own.arc_position, root_blend_width);
    normalize_influences(&mut influences);

    VertexInfluences { vertex, influences }
}

pub fn compute_chain_weights(
    input: &ChainGenInput,
    params: &ChainGenParams,
    chains: &[GeneratedChain],
) -> Vec<VertexInfluences> {
    let arcs: Vec<ChainArc> = chains.iter().map(ChainArc::new).collect();

    let mut result: Vec<VertexInfluences> = chains
        .iter()
        .enumerate()
        .flat_map(|(chain_index, chain)| {
            chain
                .vertices
                .iter()
                .map(move |&vertex| (chain_index, vertex))
        })
        .map(|(chain_index, vertex)| {
            vertex_influences(vertex, chain_index, input, params, chains, &arcs)
        })
        .collect();

    let chained: BTreeSet<u32> = chains
        .iter()
        .flat_map(|chain| chain.vertices.iter().copied())
        .collect();
    result.extend(
        collect_target_vertices(input)
            .into_iter()
            .filter(|vertex| !chained.contains(vertex))
            .map(|vertex| VertexInfluences {
                vertex,
                influences: vec![(InfluenceTarget::Root, 1.0)],
            }),
    );

    result.sort_by_key(|a| a.vertex);
    result
}
