use std::path::Path;

use thyllore_anim_core::spring_bone::{
    apply_ramp, chains_in_scope, ChainRamp, RampEnd, RampScope, SpringJointParam,
};
use thyllore_animation::ecs::systems::skeleton_to_bone_inputs;
use thyllore_avatar_core::humanoid::components::skeleton_input::BoneInput;
use thyllore_avatar_core::vrchat::sidecar::read_sidecar_json;
use thyllore_avatar_core::vrchat::sidecar_spring::{
    build_sidecar_spring, spring_setup_from_sidecar,
};
use thyllore_importer_core::{load_fbx_to_graphics_resources, ModelLoadResult};

#[test]
#[ignore = "needs the local assets/models/fuse-woman-1/edited/woman-1-edit.* files"]
fn test_woman1_edit_ramp_sibling_chains() {
    let fbx_path = "assets/models/fuse-woman-1/edited/woman-1-edit.fbx";
    let sidecar_path = Path::new("assets/models/fuse-woman-1/edited/woman-1-edit.avatar.json");

    let (result, _fbx_model) = load_fbx_to_graphics_resources(fbx_path).expect("load fbx");
    let model = ModelLoadResult::from_fbx(result);
    let skeleton = &model.skeletons[0];

    let bones: Vec<BoneInput> = skeleton_to_bone_inputs(skeleton);

    let sidecar = read_sidecar_json(sidecar_path).expect("read sidecar");
    let setup = spring_setup_from_sidecar(&sidecar.spring, &bones);

    let scope_chains = chains_in_scope(
        &setup,
        skeleton,
        setup.chains[0].id,
        RampScope::SiblingChains,
    );
    assert_eq!(
        scope_chains.len(),
        setup.chains.len(),
        "sibling scope should cover all chains"
    );
    assert_eq!(scope_chains.len(), 6, "expected 6 chains");

    let mut ramp = ChainRamp::default();
    ramp.set_end(
        RampEnd::Root,
        SpringJointParam {
            stiffness: 0.8,
            drag_force: 0.6,
            ..Default::default()
        },
    );
    ramp.set_end(
        RampEnd::Tip,
        SpringJointParam {
            stiffness: 0.2,
            drag_force: 0.1,
            ..Default::default()
        },
    );

    let mut modified_setup = setup.clone();
    for chain_id in &scope_chains {
        let idx = modified_setup
            .chains
            .iter()
            .position(|c| c.id == *chain_id)
            .unwrap();
        let old_chain = modified_setup.chains[idx].clone();
        modified_setup.chains[idx] = apply_ramp(&old_chain, &ramp);
    }

    let ramped_sidecar = build_sidecar_spring(&modified_setup, &bones);

    for (i, chain) in ramped_sidecar.chains.iter().enumerate() {
        let orig_chain = &sidecar.spring.chains[i];

        let ramped_bones: Vec<_> = chain.joints.iter().map(|j| j.bone.as_str()).collect();
        let orig_bones: Vec<_> = orig_chain.joints.iter().map(|j| j.bone.as_str()).collect();
        assert_eq!(
            ramped_bones, orig_bones,
            "chain {} bone names should match",
            i
        );

        let expected_stiffness = [0.8, 0.6, 0.4, 0.2];
        assert_eq!(
            chain.joints.len(),
            expected_stiffness.len(),
            "chain {} should have 4 joints",
            i
        );
        for (j, (joint, &expected)) in chain
            .joints
            .iter()
            .zip(expected_stiffness.iter())
            .enumerate()
        {
            assert!(
                (joint.stiffness - expected).abs() < 1e-5,
                "chain {} joint {} stiffness: got {}, expected {}",
                i,
                j,
                joint.stiffness,
                expected
            );
        }
    }
}
