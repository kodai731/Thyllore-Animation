use anyhow::{anyhow, Result};
use imgui::{TextureFeedback, TextureOp, TextureRequest};

use crate::app::App;
use crate::ecs::resource::ImguiTextures;
use crate::ecs::systems::{
    ensure_rgba32, imgui_texture_create, imgui_texture_destroy, imgui_texture_update, PixelRegion,
};
use crate::vulkanr::context::CommandState;

impl App {
    /// Answer every texture request of the frame Dear ImGui just rendered.
    pub unsafe fn apply_imgui_texture_requests(
        &mut self,
        requests: &[TextureRequest],
    ) -> Result<Vec<TextureFeedback>> {
        let layout = self
            .data
            .imgui
            .descriptor_set_layout
            .as_ref()
            .ok_or_else(|| anyhow!("ImGui descriptor set layout not initialized"))?;
        let sampler = self
            .data
            .imgui
            .sampler
            .ok_or_else(|| anyhow!("ImGui sampler not initialized"))?;
        let command_pool = self.resource::<CommandState>().pool.clone();

        let mut feedback = Vec::with_capacity(requests.len());
        for request in requests {
            let answer = match request.operation() {
                TextureOp::Create {
                    format,
                    width,
                    height,
                    row_pitch,
                    pixels,
                } => {
                    ensure_rgba32(*format)?;
                    let region = PixelRegion {
                        x: 0,
                        y: 0,
                        width: *width,
                        height: *height,
                        row_pitch: *row_pitch,
                        pixels,
                    };
                    let mut imgui_textures = self.resource_mut::<ImguiTextures>();
                    let texture_id = imgui_texture_create(
                        &self.instance,
                        &self.rrdevice,
                        &command_pool,
                        layout,
                        sampler,
                        &mut imgui_textures,
                        request.texture(),
                        region,
                    )?;
                    request.uploaded(texture_id)?
                }
                TextureOp::Update { format, rects, .. } => {
                    ensure_rgba32(*format)?;
                    let mut imgui_textures = self.resource_mut::<ImguiTextures>();
                    let texture_id = imgui_texture_update(
                        &self.instance,
                        &self.rrdevice,
                        &command_pool,
                        &mut imgui_textures,
                        request.texture(),
                        rects,
                    )?;
                    request.uploaded(texture_id)?
                }
                TextureOp::Destroy => {
                    let mut imgui_textures = self.resource_mut::<ImguiTextures>();
                    imgui_texture_destroy(&self.rrdevice, &mut imgui_textures, request.texture())?;
                    request.destroyed()?
                }
            };
            feedback.push(answer);
        }
        Ok(feedback)
    }
}
