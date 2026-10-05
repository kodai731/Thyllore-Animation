#![cfg(test)]
use std::fs;
use std::io::Write;

mod support;

use cgmath::{Matrix3, SquareMatrix};

use support::fbx_ascii::write_rig_fbx;
use support::rig_convention::rig_convention;
use support::rig_names::CONVENTIONS;
use support::rig_nodes::build_rig_nodes;

#[test]
fn test_skin_roundtrip_all_conventions() {
    for convention_id in &CONVENTIONS {
        let convention = rig_convention(convention_id);
        let fbx_text = write_rig_fbx(&convention);

        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!("skin_{}.fbx", convention_id));
        let mut file = fs::File::create(&temp_path).unwrap();
        file.write_all(fbx_text.as_bytes()).unwrap();

        let fbx_model =
            thyllore_importer_core::fbx::fbx::load_fbx_with_ufbx(temp_path.to_str().unwrap())
                .unwrap();

        fs::remove_file(&temp_path).ok();

        assert_eq!(
            fbx_model.fbx_data.len(),
            1,
            "convention {}: expected 1 mesh, got {}",
            convention_id,
            fbx_model.fbx_data.len()
        );

        let fbx_data = &fbx_model.fbx_data[0];

        assert_eq!(
            fbx_data.positions.len(),
            152,
            "convention {}: expected 152 vertices, got {}",
            convention_id,
            fbx_data.positions.len()
        );

        assert_eq!(
            fbx_data.clusters.len(),
            19,
            "convention {}: expected 19 clusters, got {}",
            convention_id,
            fbx_data.clusters.len()
        );

        let nodes = build_rig_nodes(&convention);

        for (ci, cluster) in fbx_data.clusters.iter().enumerate() {
            let bone_node = nodes
                .iter()
                .find(|n| n.name == cluster.bone_name)
                .unwrap_or_else(|| {
                    panic!(
                        "convention {}: cluster {} bone_name \"{}\" not found in rig nodes",
                        convention_id, ci, cluster.bone_name
                    )
                });

            assert_eq!(
                cluster.vertex_indices.len(),
                8,
                "convention {}: cluster {} has {} indices, expected 8",
                convention_id,
                ci,
                cluster.vertex_indices.len()
            );

            for w in &cluster.vertex_weights {
                assert!(
                    (w - 1.0).abs() < 1e-6,
                    "convention {}: cluster {} weight {:.6} != 1.0",
                    convention_id,
                    ci,
                    w
                );
            }

            let bind_world = cluster.inverse_bind_pose.invert().unwrap();

            let scale = convention.unit_scale_factor / 100.0;
            let expected_pos_x = (bone_node.world_position.x * scale) as f32;
            let expected_pos_y = (bone_node.world_position.y * scale) as f32;
            let expected_pos_z = (bone_node.world_position.z * scale) as f32;
            let expected_rotation = Matrix3::from(bone_node.world_rotation);

            assert!(
                (bind_world[3][0] - expected_pos_x).abs() < 1e-4,
                "convention {}: cluster {} inverse_bind_pose^-1 translation x {:.6} != world {:.6}",
                convention_id,
                ci,
                bind_world[3][0],
                expected_pos_x
            );
            assert!(
                (bind_world[3][1] - expected_pos_y).abs() < 1e-4,
                "convention {}: cluster {} inverse_bind_pose^-1 translation y {:.6} != world {:.6}",
                convention_id,
                ci,
                bind_world[3][1],
                expected_pos_y
            );
            assert!(
                (bind_world[3][2] - expected_pos_z).abs() < 1e-4,
                "convention {}: cluster {} inverse_bind_pose^-1 translation z {:.6} != world {:.6}",
                convention_id,
                ci,
                bind_world[3][2],
                expected_pos_z
            );

            for c in 0..3 {
                for r in 0..3 {
                    let expected = expected_rotation[c][r] as f32;
                    assert!(
                        (bind_world[c][r] - expected).abs() < 1e-4,
                        "convention {}: cluster {} inverse_bind_pose^-1 rotation [{},{}] {:.6} != world {:.6}",
                        convention_id, ci, c, r, bind_world[c][r], expected
                    );
                }
            }
        }
    }
}
