use crate::ecs::resource::{FbxModelCache, GltfModelCache};
use crate::ecs::world::World;
use crate::loader::fbx::FbxModel;

pub(super) fn insert_model_caches(
    world: &mut World,
    model_name: &str,
    fbx_model: Option<FbxModel>,
) {
    if let Some(fbx) = fbx_model {
        let needs_coord_conversion = fbx.fbx_data.iter().any(|d| !d.clusters.is_empty());
        world.insert_resource(FbxModelCache::new(
            fbx,
            model_name.to_string(),
            needs_coord_conversion,
        ));
        world.insert_resource(GltfModelCache::empty());
        return;
    }

    world.insert_resource(FbxModelCache::empty());
    if is_gltf_path(model_name) {
        world.insert_resource(GltfModelCache::new(model_name.to_string()));
    } else {
        world.insert_resource(GltfModelCache::empty());
    }
}

pub(super) fn is_gltf_path(model_name: &str) -> bool {
    let path_lower = model_name.to_lowercase();
    path_lower.ends_with(".gltf") || path_lower.ends_with(".glb")
}
