use serde::Deserialize;
use thyllore_anim_core::{Interpolation, Keyframe, MorphWeightChannel};
use thyllore_model_core::{MeshMorph, MorphChannel, SparseDelta};

#[derive(Deserialize, Default)]
struct MeshExtras {
    #[serde(rename = "targetNames", default)]
    target_names: Vec<String>,
}

pub(super) struct MorphNodeInfo {
    pub source_mesh: String,
    pub target_names: Vec<String>,
}

pub(super) fn morph_source_name(node: &gltf::Node) -> String {
    node.name()
        .map(str::to_string)
        .unwrap_or_else(|| format!("node_{}", node.index()))
}

pub(super) fn read_target_names(mesh: &gltf::Mesh, target_count: usize) -> Vec<String> {
    let named = mesh
        .extras()
        .as_deref()
        .and_then(|raw| serde_json::from_str::<MeshExtras>(raw.get()).ok())
        .unwrap_or_default()
        .target_names;

    (0..target_count)
        .map(|index| {
            named
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("target_{index}"))
        })
        .collect()
}

pub(super) fn read_mesh_morph<'a, 's, F>(
    reader: &gltf::mesh::Reader<'a, 's, F>,
    target_names: &[String],
    source_mesh: &str,
) -> MeshMorph
where
    F: Clone + Fn(gltf::Buffer<'a>) -> Option<&'s [u8]>,
{
    let channels = reader
        .read_morph_targets()
        .enumerate()
        .map(|(index, (positions, normals, _tangents))| MorphChannel {
            name: target_names
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("target_{index}")),
            position_deltas: sparse_deltas(positions),
            normal_deltas: sparse_deltas(normals),
        })
        .collect();

    MeshMorph {
        source_mesh: source_mesh.to_string(),
        channels,
    }
}

fn sparse_deltas(dense: Option<impl Iterator<Item = [f32; 3]>>) -> Vec<SparseDelta> {
    dense
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, delta)| *delta != [0.0, 0.0, 0.0])
        .map(|(vertex_index, delta)| SparseDelta {
            vertex_index: vertex_index as u32,
            delta,
        })
        .collect()
}

pub(super) fn convert_weight_animation_to_channels(
    key_frames: &[f32],
    weights: &[f32],
    interpolation: Interpolation,
    node_info: &MorphNodeInfo,
) -> Vec<MorphWeightChannel> {
    let target_count = node_info.target_names.len();
    if target_count == 0 {
        return Vec::new();
    }
    let is_cubic = interpolation == Interpolation::CubicSpline;
    let stride = if is_cubic {
        target_count * 3
    } else {
        target_count
    };
    let key_count = key_frames.len().min(weights.len() / stride);

    node_info
        .target_names
        .iter()
        .enumerate()
        .map(|(target, name)| MorphWeightChannel {
            source_mesh: node_info.source_mesh.clone(),
            channel: name.clone(),
            keyframes: (0..key_count)
                .map(|key| {
                    let base = key * stride;
                    if is_cubic {
                        let mut keyframe = Keyframe::with_interpolation(
                            key_frames[key],
                            weights[base + target_count + target],
                            interpolation,
                        );
                        keyframe.in_tangent = Some(weights[base + target]);
                        keyframe.out_tangent = Some(weights[base + target_count * 2 + target]);
                        keyframe
                    } else {
                        Keyframe::with_interpolation(
                            key_frames[key],
                            weights[base + target],
                            interpolation,
                        )
                    }
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_info(names: &[&str]) -> MorphNodeInfo {
        MorphNodeInfo {
            source_mesh: "face".to_string(),
            target_names: names.iter().map(|n| n.to_string()).collect(),
        }
    }

    #[test]
    fn sparse_deltas_drop_zero_vectors() {
        let dense = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
        ];

        let deltas = sparse_deltas(Some(dense.into_iter()));

        assert_eq!(deltas.len(), 2);
        assert_eq!(deltas[0].vertex_index, 1);
        assert_eq!(deltas[1].vertex_index, 3);
        assert_eq!(deltas[1].delta, [0.0, 2.0, 0.0]);
    }

    #[test]
    fn linear_weights_split_per_target() {
        let channels = convert_weight_animation_to_channels(
            &[0.0, 1.0],
            &[0.1, 0.2, 0.3, 0.4],
            Interpolation::Linear,
            &node_info(&["smile", "blink"]),
        );

        assert_eq!(channels.len(), 2);
        assert_eq!(channels[0].channel, "smile");
        let smile: Vec<f32> = channels[0].keyframes.iter().map(|k| k.value).collect();
        let blink: Vec<f32> = channels[1].keyframes.iter().map(|k| k.value).collect();
        assert_eq!(smile, vec![0.1, 0.3]);
        assert_eq!(blink, vec![0.2, 0.4]);
    }

    #[test]
    fn cubic_weights_take_middle_value_and_tangents() {
        let channels = convert_weight_animation_to_channels(
            &[0.0],
            &[-1.0, 0.5, 2.0],
            Interpolation::CubicSpline,
            &node_info(&["smile"]),
        );

        let keyframe = &channels[0].keyframes[0];
        assert_eq!(keyframe.value, 0.5);
        assert_eq!(keyframe.in_tangent, Some(-1.0));
        assert_eq!(keyframe.out_tangent, Some(2.0));
    }

    #[test]
    fn missing_target_names_fall_back_to_index() {
        let info = node_info(&[]);
        assert!(
            convert_weight_animation_to_channels(&[0.0], &[1.0], Interpolation::Linear, &info)
                .is_empty()
        );
    }
}
