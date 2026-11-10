use anyhow::Result;

use crate::app::App;
use crate::asset::AssetStorage;
use crate::ecs::{GpuSceneContext, World};
use crate::vulkanr::context::{CommandState, SwapchainState};
use crate::vulkanr::swapchain::RRSwapchain;

impl App {
    /// Waits for the GPU, borrows the scene parts and runs one rebuild (model load, mesh spawn).
    pub unsafe fn rebuild_scene<T>(
        &mut self,
        rebuild: impl FnOnce(
            &mut GpuSceneContext,
            &RRSwapchain,
            &mut World,
            &mut AssetStorage,
        ) -> Result<T>,
    ) -> Result<T> {
        self.rrdevice.wait_idle()?;
        let command_pool = self.resource::<CommandState>().pool.clone();
        let swapchain = self.resource::<SwapchainState>().swapchain.clone();

        let (mut gpu, world, assets) =
            self.data
                .scene_parts(&self.instance, &self.rrdevice, &command_pool);
        rebuild(&mut gpu, &swapchain, world, assets)
    }
}
