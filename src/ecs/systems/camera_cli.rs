use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::asset::AssetStorage;
use crate::ecs::resource::ActiveCamera;
use crate::ecs::systems::CameraShotMotion;
use crate::ecs::world::World;
use crate::hooks::bootstrap::BootstrapOverrides;

#[derive(Args, Debug)]
pub struct CameraOverrides {
    #[arg(long = "batch-active-camera")]
    pub active_camera: Option<String>,
    #[arg(long = "batch-export-camera")]
    pub export_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct PendingActiveCamera(pub String);

#[derive(Clone, Debug)]
pub struct CameraExportRequest(pub PathBuf);

impl BootstrapOverrides for CameraOverrides {
    const NAME: &'static str = "camera";

    fn apply(&self, world: &mut World, _assets: &mut AssetStorage) -> Result<()> {
        if !world.contains_resource::<CameraShotMotion>() {
            world.insert_resource(CameraShotMotion::default());
        }
        if !world.contains_resource::<ActiveCamera>() {
            world.insert_resource(ActiveCamera::default());
        }

        if let Some(name) = &self.active_camera {
            world.insert_resource(PendingActiveCamera(name.clone()));
        }
        if let Some(path) = &self.export_path {
            world.insert_resource(CameraExportRequest(path.clone()));
        }
        Ok(())
    }
}

crate::bootstrap_hook!(CameraOverrides);

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resolve_no_flags() {
        let overrides = CameraOverrides::resolve(&args(&["bin"])).unwrap();
        assert!(overrides.active_camera.is_none());
        assert!(overrides.export_path.is_none());
    }

    #[test]
    fn resolve_both_flags() {
        let overrides = CameraOverrides::resolve(&args(&[
            "bin",
            "--batch-active-camera",
            "Cam",
            "--batch-export-camera",
            "out.glb",
        ]))
        .unwrap();
        assert_eq!(overrides.active_camera.as_deref(), Some("Cam"));
        assert_eq!(
            overrides.export_path.as_deref(),
            Some(std::path::Path::new("out.glb"))
        );
    }

    #[test]
    fn apply_without_flags_inserts_defaults() {
        let mut world = World::new();
        let mut assets = AssetStorage::new();
        let overrides = CameraOverrides::resolve(&args(&["bin"])).unwrap();
        overrides.apply(&mut world, &mut assets).unwrap();
        assert!(world.contains_resource::<CameraShotMotion>());
        assert!(world.contains_resource::<ActiveCamera>());
        assert!(!world.contains_resource::<PendingActiveCamera>());
        assert!(!world.contains_resource::<CameraExportRequest>());
    }

    #[test]
    fn apply_with_flags_inserts_requests() {
        let mut world = World::new();
        let mut assets = AssetStorage::new();
        let overrides = CameraOverrides::resolve(&args(&[
            "bin",
            "--batch-active-camera",
            "Cam",
            "--batch-export-camera",
            "out.glb",
        ]))
        .unwrap();
        overrides.apply(&mut world, &mut assets).unwrap();
        assert_eq!(world.resource::<PendingActiveCamera>().0, "Cam");
        assert_eq!(
            world.resource::<CameraExportRequest>().0,
            PathBuf::from("out.glb")
        );
    }
}
