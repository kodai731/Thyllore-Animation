#![cfg(feature = "text-to-motion")]

#[cfg(feature = "auto-rig")]
use super::auto_rig::{
    apply_mesh_response, apply_motion_response, apply_rig_response, handle_mesh_server_status,
    handle_rigging_server_status, ServerReadiness,
};
use crate::asset::AssetStorage;
use crate::ecs::world::World;
use thyllore_grpc_client::{GrpcResponse, GrpcThreadHandle};

fn take_grpc_responses(world: &World) -> Vec<GrpcResponse> {
    let Some(handle) = world.get_resource::<GrpcThreadHandle>() else {
        return Vec::new();
    };

    let responses = std::iter::from_fn(|| handle.try_recv()).collect();
    responses
}

fn warn_unclaimed_grpc_error(message: &str) {
    log_warn!("gRPC error with no active request: {}", message);
}

#[cfg(feature = "auto-rig")]
fn drain_grpc_responses(world: &mut World, assets: &mut AssetStorage) {
    for response in take_grpc_responses(world) {
        match response {
            GrpcResponse::MotionGenerated {
                curves,
                generation_time_ms,
                model_used,
            } => {
                apply_motion_response(world, assets, curves, generation_time_ms, model_used);
            }

            GrpcResponse::MeshGenerated {
                glb_data,
                vertex_count,
                face_count,
                generation_time_ms,
                intermediate_image_png,
            } => {
                apply_mesh_response(
                    world,
                    glb_data,
                    vertex_count,
                    face_count,
                    generation_time_ms,
                    intermediate_image_png,
                );
            }

            GrpcResponse::ServerStatus { ready, .. } => {
                use crate::ecs::resource::TextToAnimationState;
                if let Some(mut state) = world.get_resource_mut::<TextToAnimationState>() {
                    state.server_ready = ready;
                }
            }

            GrpcResponse::MeshServerStatus { ready } => {
                handle_mesh_server_status(world, ServerReadiness::from_flag(ready));
            }

            GrpcResponse::RigGenerated {
                rigged_glb_data,
                joint_count,
                bone_count,
                generation_time_ms,
                ..
            } => {
                apply_rig_response(
                    world,
                    rigged_glb_data,
                    joint_count,
                    bone_count,
                    generation_time_ms,
                );
            }

            GrpcResponse::RiggingServerStatus { ready, .. } => {
                handle_rigging_server_status(world, ServerReadiness::from_flag(ready));
            }

            GrpcResponse::Error { message } => {
                route_grpc_error(world, &message);
            }
        }
    }
}

#[cfg(not(feature = "auto-rig"))]
fn drain_grpc_responses(world: &mut World, _assets: &mut AssetStorage) {
    for response in take_grpc_responses(world) {
        match response {
            GrpcResponse::MotionGenerated { .. } => {
                log_warn!("MotionGenerated received but auto-rig feature is disabled");
            }
            GrpcResponse::ServerStatus { .. } => {}
            GrpcResponse::Error { message } => warn_unclaimed_grpc_error(&message),
        }
    }
}

#[cfg(feature = "auto-rig")]
fn route_grpc_error(world: &mut World, message: &str) {
    use crate::ecs::resource::{
        AutoRigState, AutoRigStatus, TextToAnimationState, TextToAnimationStatus, TextToMeshState,
        TextToMeshStatus,
    };
    use crate::ecs::systems::text_to_animation_mark_error;

    if let Some(mut state) = world.get_resource_mut::<TextToAnimationState>() {
        let active = matches!(
            state.status,
            TextToAnimationStatus::AutoRigging
                | TextToAnimationStatus::AutoRigApplying
                | TextToAnimationStatus::GeneratingMotion
                | TextToAnimationStatus::ApplyingClip
        );
        if active {
            log_error!("TextToAnimation: error - {}", message);
            text_to_animation_mark_error(&mut state, message.to_string());
            return;
        }
    }

    if let Some(mut state) = world.get_resource_mut::<TextToMeshState>() {
        if state.status == TextToMeshStatus::Generating
            || state.status == TextToMeshStatus::WaitingForServer
        {
            log_error!("TextToMesh: error - {}", message);
            state.status = TextToMeshStatus::Error;
            state.error_message = Some(message.to_string());
            state.pending_request = None;
            return;
        }
    }

    if let Some(mut state) = world.get_resource_mut::<AutoRigState>() {
        if state.status == AutoRigStatus::Rigging || state.status == AutoRigStatus::WaitingForServer
        {
            log_error!("AutoRig: error - {}", message);
            state.status = AutoRigStatus::Error;
            state.error_message = Some(message.to_string());
            state.source_glb_data = None;
            return;
        }
    }

    warn_unclaimed_grpc_error(message);
}

crate::dispatch_prep_hook!("grpc_responses", drain_grpc_responses);
