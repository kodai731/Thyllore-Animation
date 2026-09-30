use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use thyllore_avatar_core::expression::components::preset::{ExpressionLibrary, ExpressionPreset};
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::humanoid::systems::name_match::infer_mapping;
use thyllore_avatar_core::stats::components::stats::AvatarStats;
use thyllore_avatar_core::vrchat::sidecar::{
    build_sidecar, write_sidecar_json, SidecarInput, SidecarSpringChain,
};
use thyllore_exporter_core::systems::unity::curves::write_expression_anim;

#[derive(serde::Deserialize)]
struct RigBone {
    name: String,
    parent: Option<usize>,
    rest_position: [f32; 3],
}

#[derive(serde::Deserialize)]
struct Rig {
    bones: Vec<RigBone>,
    channels: Vec<String>,
    expression_mesh: String,
    spring_root: String,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        bail!("usage: unity-verify-fixtures <rig.json> <out_dir>");
    }
    let rig_path = Path::new(&args[1]);
    let out_dir = PathBuf::from(&args[2]);

    let rig: Rig = serde_json::from_str(&std::fs::read_to_string(rig_path)?)
        .with_context(|| format!("parse {}", rig_path.display()))?;
    let bones = to_bone_inputs(&rig);
    let (mapping, unresolved) = infer_mapping(&bones);
    println!(
        "humanoid: {} roles mapped, {} unresolved",
        mapping.by_role.len(),
        unresolved.len()
    );

    let happy_weights = happy_expression_weights();
    let library = ExpressionLibrary {
        presets: vec![ExpressionPreset {
            name: "happy".to_string(),
            weights: happy_weights.clone(),
        }],
    };
    let spring_chains = vec![SidecarSpringChain {
        root: rig.spring_root.clone(),
        stiffness: 0.7,
        gravity: 0.2,
        drag: 0.4,
        colliders: vec![],
    }];
    let stats = AvatarStats {
        triangles: 12,
        bones: bones.len() as u32,
        materials: 1,
        skinned_meshes: 1,
        meshes: 1,
        morph_meshes: 1,
        spring_chains: 1,
        spring_transforms: 2,
        ..AvatarStats::default()
    };

    let sidecar = build_sidecar(SidecarInput {
        mapping: &mapping,
        bones: &bones,
        channel_names: &rig.channels,
        expression_mesh_name: Some(&rig.expression_mesh),
        library: &library,
        spring_chains: &spring_chains,
        stats: &stats,
    });
    let sidecar_path = out_dir.join("synthetic_avatar.avatar.json");
    write_sidecar_json(&sidecar_path, &sidecar)?;
    println!("wrote {}", sidecar_path.display());

    let anim_path = out_dir.join("happy.anim");
    let anim = write_expression_anim("happy", &rig.expression_mesh, &happy_weights);
    std::fs::write(&anim_path, anim)?;
    println!("wrote {}", anim_path.display());
    Ok(())
}

fn to_bone_inputs(rig: &Rig) -> Vec<BoneInput> {
    rig.bones
        .iter()
        .map(|bone| BoneInput {
            name: bone.name.clone(),
            parent: bone.parent,
            rest_position: bone.rest_position,
        })
        .collect()
}

fn happy_expression_weights() -> BTreeMap<String, f32> {
    let mut weights = BTreeMap::new();
    weights.insert("mouth_smile".to_string(), 1.0);
    weights.insert("eye_blink".to_string(), 0.3);
    weights
}
