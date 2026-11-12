#![cfg(feature = "text-to-motion")]

use std::sync::OnceLock;

use crate::asset::AssetStorage;
use crate::ecs::events::UiCommand;
use crate::ecs::resource::GrpcServerProcess;
use crate::ecs::world::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

const TRAINING_REPO_ENV: &str = "ANIMATION_TRAINING_PATH";
const TRAINING_REPO_FALLBACK: &str = "../AnimationModelTraining";
const SERVER_CONFIG_PATH: &str = "configs/server.yaml";

#[derive(Clone, Copy, Debug)]
pub enum GrpcServerEvent {
    Start,
    Stop,
}

impl UiCommand for GrpcServerEvent {
    fn apply(self: Box<Self>, world: &mut World, _: &mut AssetStorage, _: &GraphicsResources) {
        let Some(mut server) = world.get_resource_mut::<GrpcServerProcess>() else {
            return;
        };

        match *self {
            GrpcServerEvent::Start => start_grpc_server(&mut server),
            GrpcServerEvent::Stop => server.stop(),
        }
    }
}

fn start_grpc_server(server: &mut GrpcServerProcess) {
    if let Err(error) = server.start(training_repo_path(), SERVER_CONFIG_PATH) {
        log_error!("Failed to start gRPC server: {}", error);
        server.last_error = Some(error);
    }
}

fn training_repo_path() -> &'static str {
    static TRAINING_REPO_PATH: OnceLock<String> = OnceLock::new();
    TRAINING_REPO_PATH.get_or_init(resolve_training_repo_path)
}

fn resolve_training_repo_path() -> String {
    std::env::var(TRAINING_REPO_ENV)
        .ok()
        .or_else(query_wsl_training_repo_path)
        .unwrap_or_else(|| TRAINING_REPO_FALLBACK.to_string())
}

fn query_wsl_training_repo_path() -> Option<String> {
    let output = std::process::Command::new("wsl")
        .args([
            "--",
            "bash",
            "-l",
            "-c",
            "echo $HOME/Projects/AnimationModelTraining",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    let wsl_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!wsl_path.is_empty()).then_some(wsl_path)
}
