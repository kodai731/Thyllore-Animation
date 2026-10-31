use crate::ecs::component::CapsuleFitParams;
use crate::ecs::component::MorphWeights;
use crate::ecs::systems::batch_run_systems::BatchAction;
use crate::ecs::systems::phases::event_dispatch::morph::MorphEvent;
use crate::ecs::world::World;
use thyllore_avatar_core::garment::components::submesh_role::{
    SubmeshKey, SubmeshRole, SubmeshSelector,
};

use super::chain_gen::*;
use super::model::*;
use super::morph::*;
use super::spring::*;

use crate::ecs::events::UiCommandQueue;

#[test]
fn test_morph_weight_parses_channel_names_with_dots_and_colons() {
    let action = morph_weight_parse("morph_weight=vrc.v_aa:0.25")
        .expect("prefix matches")
        .expect("spec is valid");

    assert_eq!(
        format!("{:?}", action),
        "SetMorphWeight { channel: \"vrc.v_aa\", weight: 0.25 }"
    );
}

#[test]
fn test_morph_weight_rejects_malformed_specs() {
    assert!(morph_weight_parse("key_morph_weights").is_none());
    for spec in [
        "morph_weight=smile",
        "morph_weight=:0.5",
        "morph_weight=smile:x",
    ] {
        assert!(
            morph_weight_parse(spec).expect("prefix matches").is_err(),
            "{spec} must be rejected"
        );
    }
}

#[test]
fn test_morph_weight_reaches_every_morph_entity_by_channel_name() {
    let mut world = World::new();
    world.insert_resource(UiCommandQueue::default());
    let first = world.entity().with_name("first").build();
    let second = world.entity().with_name("second").build();
    world.insert_component(first, MorphWeights { weights: vec![0.0] });
    world.insert_component(second, MorphWeights { weights: vec![0.0] });

    SetMorphWeight {
        channel: "smile".to_string(),
        weight: 0.5,
    }
    .apply(&mut world);

    let sent: Vec<String> = world
        .resource_mut::<UiCommandQueue>()
        .drain()
        .map(|command| format!("{:?}", command))
        .collect();
    let expected: Vec<String> = [first, second]
        .into_iter()
        .map(|entity| {
            format!(
                "{:?}",
                MorphEvent::SetMorphWeight {
                    entity,
                    channel: "smile".to_string(),
                    weight: 0.5,
                }
            )
        })
        .collect();
    assert_eq!(sent, expected);
}

#[test]
fn test_named_actions_reject_empty_values() {
    assert!(capture_expression_parse("capture_expression=")
        .expect("prefix matches")
        .is_err());
    assert!(add_spring_chains_under_parse("add_spring_chains_under= ")
        .expect("prefix matches")
        .is_err());
    assert!(
        add_spring_chains_under_parse("add_spring_chains_under=Head,min_joints=0")
            .expect("prefix matches")
            .is_err()
    );
    assert!(
        add_spring_chains_under_parse("add_spring_chains_under=Head,min_joints=3")
            .expect("prefix matches")
            .is_ok()
    );
}

#[test]
fn test_fit_body_colliders_parses_correct_format() {
    let action = fit_body_colliders_parse("fit_body_colliders=0.5,0.9,0.85,8")
        .expect("prefix matches")
        .expect("spec is valid");

    assert_eq!(
        format!("{:?}", action),
        "FitBodyColliders { params: CapsuleFitParams { min_weight: 0.5, radius_percentile: 0.9, radius_scale: 0.85, min_vertex_count: 8 } }"
    );
}

#[test]
fn test_fit_body_colliders_rejects_malformed_specs() {
    assert!(fit_body_colliders_parse("fit_body_colliders=0.5,0.9,0.85")
        .expect("prefix matches")
        .is_err());
    assert!(fit_body_colliders_parse("fit_body_colliders=0.5,0.9")
        .expect("prefix matches")
        .is_err());
    assert!(fit_body_colliders_parse("fit_body_colliders=a,0.9,0.85,8")
        .expect("prefix matches")
        .is_err());
}

#[test]
fn test_dump_spring_tails_parses_correct_format() {
    let action = dump_spring_tails_parse("dump_spring_tails=output.csv,60.0")
        .expect("prefix matches")
        .expect("spec is valid");

    assert_eq!(action.name(), "dump_spring_tails");
}

#[test]
fn test_dump_spring_tails_rejects_malformed_specs() {
    assert!(dump_spring_tails_parse("dump_spring_tails=output.csv")
        .expect("prefix matches")
        .is_err());
    assert!(dump_spring_tails_parse("dump_spring_tails=output.csv,abc")
        .expect("prefix matches")
        .is_err());
    assert!(dump_spring_tails_parse("dump_spring_tails=output.csv,0")
        .expect("prefix matches")
        .is_err());
    assert!(dump_spring_tails_parse("dump_spring_tails=output.csv,-1")
        .expect("prefix matches")
        .is_err());
}

#[test]
fn test_submesh_role_parses_submesh_and_whole_mesh() {
    let submesh = submesh_role_spec_parse("hair:Body/Hair_mat").expect("spec is valid");
    assert_eq!(submesh.role, SubmeshRole::Hair);
    assert_eq!(
        submesh.target,
        SubmeshSelector::Submesh(SubmeshKey {
            mesh: "Body".to_string(),
            material: "Hair_mat".to_string(),
        })
    );

    let whole_mesh = submesh_role_spec_parse("garment:Cloth_skirt").expect("spec is valid");
    assert_eq!(whole_mesh.role, SubmeshRole::Garment);
    assert_eq!(
        whole_mesh.target,
        SubmeshSelector::Mesh("Cloth_skirt".to_string())
    );
}

#[test]
fn test_submesh_role_rejects_malformed_specs() {
    assert!(submesh_role_parse("submesh_role=fur:Body/x")
        .expect("prefix matches")
        .is_err());
    assert!(submesh_role_parse("submesh_role=skin")
        .expect("prefix matches")
        .is_err());
}

#[test]
fn test_fit_spring_params_parses_correct_format() {
    let action: Box<dyn BatchAction> =
        fit_spring_params_parse("fit_spring_params=reference.csv,30.0,1.0,100,0.001")
            .expect("prefix matches")
            .expect("spec is valid");

    assert_eq!(action.name(), "fit_spring_params");
}

#[test]
fn test_fit_spring_params_rejects_insufficient_elements() {
    assert!(
        fit_spring_params_parse("fit_spring_params=reference.csv,30.0")
            .expect("prefix matches")
            .is_err()
    );
}

#[test]
fn test_hide_covered_skin_parses_correct_format() {
    let result = hide_covered_skin_parse("hide_covered_skin=0.5,45.0,8,0.5");
    assert!(result.is_some());
    let action: Box<dyn BatchAction> = result.unwrap().unwrap();
    assert_eq!(action.name(), "hide_covered_skin");
}

#[test]
fn test_hide_covered_skin_rejects_out_of_range() {
    assert!(hide_covered_skin_parse("hide_covered_skin=0.0,45.0,8,0.5")
        .expect("prefix matches")
        .is_err());
    assert!(hide_covered_skin_parse("hide_covered_skin=0.5,91.0,8,0.5")
        .expect("prefix matches")
        .is_err());
    assert!(hide_covered_skin_parse("hide_covered_skin=0.5,45.0,0,0.5")
        .expect("prefix matches")
        .is_err());
    assert!(hide_covered_skin_parse("hide_covered_skin=0.5,45.0,8,1.5")
        .expect("prefix matches")
        .is_err());
}

#[test]
fn test_generate_chain_bones_parses_submesh_root_and_settings() {
    let action = generate_chain_bones_parse(
        "generate_chain_bones=garment/cloth,Spine,joint_count=3,column_count=2",
    )
    .expect("prefix matches")
    .expect("spec is valid");
    assert_eq!(action.name(), "generate_chain_bones");

    let defaults_only = generate_chain_bones_parse("generate_chain_bones=garment/cloth,Spine")
        .expect("prefix matches");
    assert!(defaults_only.is_ok());

    let whole_mesh =
        generate_chain_bones_parse("generate_chain_bones=garment,Spine").expect("prefix matches");
    assert!(whole_mesh.is_ok());
}

#[test]
fn test_generate_chain_bones_rejects_malformed_specs() {
    for spec in [
        "generate_chain_bones=garment/cloth,,joint_count=3",
        "generate_chain_bones=garment/cloth,spine,joint_count=0",
        "generate_chain_bones=garment/cloth,spine,joints=3",
        "generate_chain_bones=,spine",
    ] {
        assert!(
            generate_chain_bones_parse(spec)
                .expect("prefix matches")
                .is_err(),
            "{spec}"
        );
    }
}

#[test]
fn test_dump_skinned_vertices_parses_correct_format() {
    let action = dump_skinned_vertices_parse("dump_skinned_vertices=output.csv,60.0,Body/Skin")
        .expect("prefix matches")
        .expect("spec is valid");

    assert_eq!(action.name(), "dump_skinned_vertices");
}

#[test]
fn test_dump_skinned_vertices_rejects_malformed_specs() {
    assert!(
        dump_skinned_vertices_parse("dump_skinned_vertices=output.csv,60.0")
            .expect("prefix matches")
            .is_err()
    );
    assert!(
        dump_skinned_vertices_parse("dump_skinned_vertices=output.csv,abc,Body/Skin")
            .expect("prefix matches")
            .is_err()
    );
    assert!(
        dump_skinned_vertices_parse("dump_skinned_vertices=output.csv,0,Body/Skin")
            .expect("prefix matches")
            .is_err()
    );
    assert!(
        dump_skinned_vertices_parse("dump_skinned_vertices=output.csv,60.0,Bare")
            .expect("prefix matches")
            .is_err()
    );
}
