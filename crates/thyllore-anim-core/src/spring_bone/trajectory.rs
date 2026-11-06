use std::collections::{BTreeMap, HashMap};

use cgmath::Vector3;

#[derive(Clone, Debug, PartialEq)]
pub struct SpringTailSample {
    pub frame: usize,
    pub chain: String,
    pub bone: String,
    pub position: Vector3<f32>,
}

pub fn format_tail_csv(samples: &[SpringTailSample]) -> String {
    let mut lines = vec!["frame,chain,bone,x,y,z".to_string()];
    for s in samples {
        lines.push(format!(
            "{},{},{},{},{},{}",
            s.frame, s.chain, s.bone, s.position.x, s.position.y, s.position.z
        ));
    }
    lines.join("\n") + "\n"
}

pub fn parse_tail_csv(text: &str) -> anyhow::Result<Vec<SpringTailSample>> {
    let mut samples = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if i == 0 {
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split(',').collect();
        if parts.len() != 6 {
            return Err(anyhow::anyhow!(
                "line {}: expected 6 columns, got {}",
                i + 1,
                parts.len()
            ));
        }
        let frame: usize = parts[0]
            .parse()
            .map_err(|e| anyhow::anyhow!("line {}: invalid frame: {}", i + 1, e))?;
        let chain = parts[1].to_string();
        let bone = parts[2].to_string();
        let x: f32 = parts[3]
            .parse()
            .map_err(|e| anyhow::anyhow!("line {}: invalid x: {}", i + 1, e))?;
        let y: f32 = parts[4]
            .parse()
            .map_err(|e| anyhow::anyhow!("line {}: invalid y: {}", i + 1, e))?;
        let z: f32 = parts[5]
            .parse()
            .map_err(|e| anyhow::anyhow!("line {}: invalid z: {}", i + 1, e))?;
        samples.push(SpringTailSample {
            frame,
            chain,
            bone,
            position: Vector3::new(x, y, z),
        });
    }
    Ok(samples)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChainTrajectoryError {
    pub chain: String,
    pub rms: f32,
    pub max: f32,
    pub max_frame: usize,
    pub sample_count: usize,
}

struct ChainErrorAccumulator {
    sum_sq: f32,
    max_sq: f32,
    max_frame: usize,
    count: usize,
}

pub fn compare_tail_trajectories(
    reference: &[SpringTailSample],
    candidate: &[SpringTailSample],
) -> Vec<ChainTrajectoryError> {
    let mut lookup: HashMap<(usize, &str, &str), Vector3<f32>> = HashMap::new();
    for entry in candidate {
        lookup.insert((entry.frame, &entry.chain, &entry.bone), entry.position);
    }

    let mut map: BTreeMap<String, ChainErrorAccumulator> = BTreeMap::new();

    for ref_s in reference {
        let key: (usize, &str, &str) = (ref_s.frame, ref_s.chain.as_str(), ref_s.bone.as_str());
        if let Some(cand_pos) = lookup.get(&key) {
            let dx = cand_pos.x - ref_s.position.x;
            let dy = cand_pos.y - ref_s.position.y;
            let dz = cand_pos.z - ref_s.position.z;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            let entry = map
                .entry(ref_s.chain.clone())
                .or_insert(ChainErrorAccumulator {
                    sum_sq: 0.0,
                    max_sq: 0.0,
                    max_frame: 0,
                    count: 0,
                });
            entry.sum_sq += dist_sq;
            entry.count += 1;
            if dist_sq > entry.max_sq {
                entry.max_sq = dist_sq;
                entry.max_frame = ref_s.frame;
            }
        }
    }

    map.into_iter()
        .map(|(chain, acc)| ChainTrajectoryError {
            chain,
            rms: (acc.sum_sq / acc.count as f32).sqrt(),
            max: acc.max_sq.sqrt(),
            max_frame: acc.max_frame,
            sample_count: acc.count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use cgmath::Vector3;

    use super::*;

    fn make_sample(
        frame: usize,
        chain: &str,
        bone: &str,
        x: f32,
        y: f32,
        z: f32,
    ) -> SpringTailSample {
        SpringTailSample {
            frame,
            chain: chain.to_string(),
            bone: bone.to_string(),
            position: Vector3::new(x, y, z),
        }
    }

    #[test]
    fn test_csv_roundtrip() {
        let samples = vec![
            make_sample(0, "tail", "tail_02", 1.0, 2.0, 3.0),
            make_sample(1, "tail", "tail_02", 1.1, 2.1, 3.1),
        ];
        let csv = format_tail_csv(&samples);
        let parsed = parse_tail_csv(&csv).unwrap();
        assert_eq!(samples, parsed);
    }

    #[test]
    fn test_parse_bad_row() {
        let csv = "frame,chain,bone,x,y,z\n0,tail,tail_02,1.0,2.0\n";
        let err = parse_tail_csv(csv).unwrap_err();
        assert!(err.to_string().contains("line 2"));
    }

    #[test]
    fn test_identical_trajectories() {
        let samples = vec![
            make_sample(0, "tail", "tail_02", 1.0, 2.0, 3.0),
            make_sample(1, "tail", "tail_02", 1.1, 2.1, 3.1),
        ];
        let errors = compare_tail_trajectories(&samples, &samples);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].chain, "tail");
        assert!((errors[0].rms - 0.0).abs() < 1e-6);
        assert!((errors[0].max - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_single_bone_offset() {
        let reference = vec![make_sample(0, "tail", "tail_02", 0.0, 0.0, 0.0)];
        let candidate = vec![make_sample(0, "tail", "tail_02", 0.3, 0.0, 0.0)];
        let errors = compare_tail_trajectories(&reference, &candidate);
        assert_eq!(errors.len(), 1);
        assert!((errors[0].max - 0.3).abs() < 1e-6);
        assert_eq!(errors[0].max_frame, 0);
        assert!((errors[0].rms - (0.09_f32).sqrt()).abs() < 1e-6);
    }
}
