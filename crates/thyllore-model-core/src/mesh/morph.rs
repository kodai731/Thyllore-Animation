#[derive(Clone, Debug)]
pub struct SparseDelta {
    pub vertex_index: u32,
    pub delta: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct MorphChannel {
    pub name: String,
    pub default_weight: f32,
    pub position_deltas: Vec<SparseDelta>,
    pub normal_deltas: Vec<SparseDelta>,
}

#[derive(Clone, Debug, Default)]
pub struct MeshMorph {
    pub source_mesh: String,
    pub channels: Vec<MorphChannel>,
}

impl MeshMorph {
    pub fn channel_index(&self, name: &str) -> Option<usize> {
        self.channels.iter().position(|c| c.name == name)
    }

    pub fn channel_names(&self) -> Vec<String> {
        self.channels.iter().map(|c| c.name.clone()).collect()
    }

    pub fn default_weights(&self) -> Vec<f32> {
        self.channels.iter().map(|c| c.default_weight).collect()
    }
}

pub fn accumulate_morph_positions(
    base: &[[f32; 3]],
    morph: &MeshMorph,
    weights: &[f32],
) -> Vec<[f32; 3]> {
    let mut output = base.to_vec();
    for (i, channel) in morph.channels.iter().enumerate() {
        let weight = weights.get(i).copied().unwrap_or(0.0);
        if weight.abs() <= 1e-6 {
            continue;
        }
        for delta in &channel.position_deltas {
            let idx = delta.vertex_index as usize;
            if idx < output.len() {
                output[idx][0] += delta.delta[0] * weight;
                output[idx][1] += delta.delta[1] * weight;
                output[idx][2] += delta.delta[2] * weight;
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_positions() -> [[f32; 3]; 8] {
        [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ]
    }

    fn smile_morph() -> MeshMorph {
        MeshMorph {
            source_mesh: String::new(),
            channels: vec![
                MorphChannel {
                    name: "smile".to_string(),
                    default_weight: 0.0,
                    position_deltas: vec![
                        SparseDelta {
                            vertex_index: 1,
                            delta: [0.0, -0.5, 0.0],
                        },
                        SparseDelta {
                            vertex_index: 3,
                            delta: [0.0, -0.2, 0.0],
                        },
                    ],
                    normal_deltas: Vec::new(),
                },
                MorphChannel {
                    name: "blink".to_string(),
                    default_weight: 0.0,
                    position_deltas: vec![SparseDelta {
                        vertex_index: 6,
                        delta: [0.0, -0.3, 0.0],
                    }],
                    normal_deltas: Vec::new(),
                },
            ],
        }
    }

    fn almost_eq(a: &[f32; 3], b: &[f32; 3]) {
        assert!(
            (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4 && (a[2] - b[2]).abs() < 1e-4,
            "expected {:?}, got {:?}",
            b,
            a
        );
    }

    #[test]
    fn test_weights_all_smile() {
        let base = cube_positions();
        let morph = smile_morph();
        let result = accumulate_morph_positions(&base, &morph, &[1.0, 0.0]);

        assert_eq!(result.len(), 8);
        almost_eq(&result[1], &[1.0, -1.5, -1.0]);
        almost_eq(&result[3], &[1.0, 0.8, -1.0]);
        for (i, &expected) in base.iter().enumerate() {
            if i != 1 && i != 3 {
                almost_eq(&result[i], &expected);
            }
        }
    }

    #[test]
    fn test_weights_all_blink() {
        let base = cube_positions();
        let morph = smile_morph();
        let result = accumulate_morph_positions(&base, &morph, &[0.0, 1.0]);

        assert_eq!(result.len(), 8);
        almost_eq(&result[6], &[-1.0, 0.7, 1.0]);
        for (i, &expected) in base.iter().enumerate() {
            if i != 6 {
                almost_eq(&result[i], &expected);
            }
        }
    }

    #[test]
    fn test_weights_half_half() {
        let base = cube_positions();
        let morph = smile_morph();
        let result = accumulate_morph_positions(&base, &morph, &[0.5, 0.5]);

        assert_eq!(result.len(), 8);
        almost_eq(&result[1], &[1.0, -1.25, -1.0]);
        almost_eq(&result[3], &[1.0, 0.9, -1.0]);
        almost_eq(&result[6], &[-1.0, 0.85, 1.0]);
        for (i, &expected) in base.iter().enumerate() {
            if i != 1 && i != 3 && i != 6 {
                almost_eq(&result[i], &expected);
            }
        }
    }

    #[test]
    fn test_idempotent() {
        let base = cube_positions();
        let morph = smile_morph();
        let weights = [0.5, 0.5];
        let result1 = accumulate_morph_positions(&base, &morph, &weights);
        let result2 = accumulate_morph_positions(&base, &morph, &weights);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_channel_index() {
        let morph = smile_morph();
        assert_eq!(morph.channel_index("smile"), Some(0));
        assert_eq!(morph.channel_index("blink"), Some(1));
        assert_eq!(morph.channel_index("frown"), None);
    }

    #[test]
    fn test_empty_weights() {
        let base = cube_positions();
        let morph = smile_morph();
        let result = accumulate_morph_positions(&base, &morph, &[]);
        assert_eq!(result, base.to_vec());
    }
}
