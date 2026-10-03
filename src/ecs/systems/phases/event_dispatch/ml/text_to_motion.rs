#![cfg(feature = "text-to-motion")]

#[cfg(feature = "auto-rig")]
use super::auto_rig::{
    apply_mesh_response, apply_motion_response, apply_rig_response, handle_mesh_server_status,
    handle_rigging_server_status, ServerReadiness,
};

pub fn drain_grpc_responses(
    world: &mut crate::ecs::world::World,
    assets: &mut crate::asset::AssetStorage,
) {
    use crate::grpc::{GrpcResponse, GrpcThreadHandle};

    let handle = match world.get_resource::<GrpcThreadHandle>() {
        Some(h) => h,
        None => return,
    };

    let mut responses = Vec::new();
    while let Some(response) = handle.try_recv() {
        responses.push(response);
    }
    drop(handle);

    for response in responses {
        match response {
            #[cfg(feature = "auto-rig")]
            GrpcResponse::MotionGenerated {
                curves,
                generation_time_ms,
                model_used,
            } => {
                apply_motion_response(world, assets, curves, generation_time_ms, model_used);
            }

            #[cfg(not(feature = "auto-rig"))]
            GrpcResponse::MotionGenerated { .. } => {
                log_warn!("MotionGenerated received but auto-rig feature is disabled");
            }

            #[cfg(feature = "auto-rig")]
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
                #[cfg(feature = "auto-rig")]
                {
                    use crate::ecs::resource::TextToAnimationState;
                    if let Some(mut state) = world.get_resource_mut::<TextToAnimationState>() {
                        state.server_ready = ready;
                    }
                }
                #[cfg(not(feature = "auto-rig"))]
                {
                    let _ = ready;
                }
            }

            #[cfg(feature = "auto-rig")]
            GrpcResponse::MeshServerStatus { ready } => {
                handle_mesh_server_status(world, ServerReadiness::from_flag(ready));
            }

            #[cfg(feature = "auto-rig")]
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

            #[cfg(feature = "auto-rig")]
            GrpcResponse::RiggingServerStatus { ready, .. } => {
                handle_rigging_server_status(world, ServerReadiness::from_flag(ready));
            }

            GrpcResponse::Error { message } => {
                route_grpc_error(world, &message);
            }
        }
    }
}

fn route_grpc_error(world: &mut crate::ecs::world::World, message: &str) {
    #[cfg(feature = "auto-rig")]
    {
        use crate::ecs::resource::{TextToAnimationState, TextToAnimationStatus};
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
    }

    #[cfg(feature = "auto-rig")]
    {
        use crate::ecs::resource::{TextToMeshState, TextToMeshStatus};
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
    }

    #[cfg(feature = "auto-rig")]
    {
        use crate::ecs::resource::{AutoRigState, AutoRigStatus};
        if let Some(mut state) = world.get_resource_mut::<AutoRigState>() {
            if state.status == AutoRigStatus::Rigging
                || state.status == AutoRigStatus::WaitingForServer
            {
                log_error!("AutoRig: error - {}", message);
                state.status = AutoRigStatus::Error;
                state.error_message = Some(message.to_string());
                state.source_glb_data = None;
                return;
            }
        }
    }

    log_warn!("gRPC error with no active request: {}", message);
}
