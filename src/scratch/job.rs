use crate::asset::AssetStorage;
use crate::ecs::World;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;

pub struct ScratchContext<'a> {
    pub world: &'a mut World,
    pub assets: &'a mut AssetStorage,
    pub graphics: &'a GraphicsResources,
}

pub enum ScratchStep {
    Continue,
    Done(serde_json::Value),
}

pub trait ScratchJob: std::fmt::Debug {
    fn step(&mut self, ctx: &mut ScratchContext) -> anyhow::Result<ScratchStep>;
}

pub type ScratchEntryFn =
    fn(name: &str, args: serde_json::Value) -> anyhow::Result<Box<dyn ScratchJob>>;

pub const SCRATCH_ENTRY_SYMBOL: &[u8] = b"thyllore_scratch_entry";
