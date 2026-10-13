use cgmath::SquareMatrix;

use crate::asset::{AssetStorage, NodeAsset};
use crate::ecs::resource::NodeAssets;
use crate::ecs::world::World;
use crate::loader::ModelLoadResult;
use crate::vulkanr::resource::graphics_resource::NodeData;

pub(super) fn replace_node_assets(
    world: &mut World,
    assets: &mut AssetStorage,
    load_result: &ModelLoadResult,
) {
    for node in &load_result.nodes {
        assets.add_node(NodeAsset {
            id: node.index as u64,
            name: node.name.clone(),
            parent_id: node.parent_index.map(|i| i as u64),
            local_transform: node.local_transform,
        });
    }

    let nodes: Vec<NodeData> = load_result
        .nodes
        .iter()
        .map(|n| NodeData {
            index: n.index,
            name: n.name.clone(),
            parent_index: n.parent_index,
            local_transform: n.local_transform,
            global_transform: cgmath::Matrix4::identity(),
        })
        .collect();
    log!("Loaded {} nodes into NodeAssets", nodes.len());
    world.resource_mut::<NodeAssets>().nodes = nodes;
}
