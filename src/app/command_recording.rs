use anyhow::Result;

use super::App;
use crate::app::build_pass_context;
use crate::ecs::resource::GpuPassTimings;
use crate::hooks::pass::{TargetUse, TransientRequest};
use crate::vulkanr::renderer::deferred;
use thyllore_vulkan_core::types::CommandBuffer;

impl App {
    pub unsafe fn record_command_buffer(
        &mut self,
        image_index: usize,
        draw_data: &imgui::DrawData,
        frame_slot: usize,
    ) -> Result<()> {
        let command_buffer = self
            .resource::<crate::vulkanr::context::CommandState>()
            .buffers
            .command_buffers[image_index];

        thyllore_vulkan_core::begin_frame_command_buffer(&self.rrdevice, command_buffer)?;

        // Collect GPU timestamp profiler results from previous frame and write to ECS resource.
        // Derive next_frame from the current GpuPassTimings resource (frame + 1), defaulting to 1
        // if the resource is absent. The borrow must be dropped before insert_resource.
        let next_frame = self
            .data
            .ecs_world
            .get_resource::<GpuPassTimings>()
            .map(|t| t.frame + 1)
            .unwrap_or(1);

        if let Some(mut passes) = self
            .gpu_timestamp_profiler
            .collect(&self.rrdevice, image_index)
        {
            let frame_total_ms = passes
                .iter()
                .position(|(label, _)| label == "frame")
                .map(|i| passes.remove(i).1);
            self.data.ecs_world.insert_resource(GpuPassTimings {
                frame: next_frame,
                passes,
                frame_total_ms,
            });
        }

        self.gpu_timestamp_profiler
            .begin_frame(&self.rrdevice, command_buffer, image_index);

        let use_gbuffer =
            self.data.raytracing.is_available() && self.data.viewport.offscreen.is_some();

        if use_gbuffer {
            self.gpu_timestamp_profiler.begin_scope(
                &self.rrdevice,
                command_buffer,
                image_index,
                "gbuffer".to_string(),
            );
            deferred::record_gbuffer_pass(self, command_buffer, image_index)?;
            self.gpu_timestamp_profiler
                .end_scope(&self.rrdevice, command_buffer, image_index);

            self.gpu_timestamp_profiler.begin_scope(
                &self.rrdevice,
                command_buffer,
                image_index,
                "object_id_copy".to_string(),
            );
            self.record_object_id_copy(command_buffer);
            self.gpu_timestamp_profiler
                .end_scope(&self.rrdevice, command_buffer, image_index);

            if self.data.raytracing.has_valid_tlas() {
                self.gpu_timestamp_profiler.begin_scope(
                    &self.rrdevice,
                    command_buffer,
                    image_index,
                    "ray_query".to_string(),
                );
                deferred::record_ray_query_pass(self, command_buffer)?;
                self.gpu_timestamp_profiler
                    .end_scope(&self.rrdevice, command_buffer, image_index);
            } else {
                self.prepare_empty_shadow_mask(command_buffer);
            }

            let has_hdr_pipeline = self.data.viewport.hdr_buffer.is_some()
                && self.data.raytracing.tonemap_pipeline.is_some();

            if has_hdr_pipeline {
                self.record_pass_graph(command_buffer, image_index, frame_slot)?;
            } else {
                self.gpu_timestamp_profiler.begin_scope(
                    &self.rrdevice,
                    command_buffer,
                    image_index,
                    "composite_offscreen".to_string(),
                );
                deferred::record_composite_to_offscreen(self, command_buffer, image_index)?;
                self.gpu_timestamp_profiler
                    .end_scope(&self.rrdevice, command_buffer, image_index);
            }

            self.gpu_timestamp_profiler.begin_scope(
                &self.rrdevice,
                command_buffer,
                image_index,
                "imgui".to_string(),
            );
            self.begin_main_render_pass(command_buffer, image_index);
            self.record_imgui_rendering(command_buffer, draw_data)?;
            thyllore_vulkan_core::end_render_pass(&self.rrdevice, command_buffer);
            self.gpu_timestamp_profiler
                .end_scope(&self.rrdevice, command_buffer, image_index);
        } else {
            if let Some(ref offscreen) = self.data.viewport.offscreen {
                self.begin_offscreen_render_pass(command_buffer, offscreen);
                self.gpu_timestamp_profiler.begin_scope(
                    &self.rrdevice,
                    command_buffer,
                    image_index,
                    "offscreen_3d".to_string(),
                );
                self.record_3d_rendering_to_offscreen(command_buffer, image_index, offscreen)?;
                self.gpu_timestamp_profiler
                    .end_scope(&self.rrdevice, command_buffer, image_index);
                thyllore_vulkan_core::end_render_pass(&self.rrdevice, command_buffer);
            }

            self.gpu_timestamp_profiler.begin_scope(
                &self.rrdevice,
                command_buffer,
                image_index,
                "imgui".to_string(),
            );
            self.begin_main_render_pass(command_buffer, image_index);
            self.record_imgui_rendering(command_buffer, draw_data)?;
            thyllore_vulkan_core::end_render_pass(&self.rrdevice, command_buffer);
            self.gpu_timestamp_profiler
                .end_scope(&self.rrdevice, command_buffer, image_index);
        }

        self.gpu_timestamp_profiler
            .end_frame(&self.rrdevice, command_buffer, image_index);

        thyllore_vulkan_core::end_frame_command_buffer(&self.rrdevice, command_buffer)?;

        Ok(())
    }

    unsafe fn record_pass_graph(
        &mut self,
        command_buffer: CommandBuffer,
        image_index: usize,
        frame_slot: usize,
    ) -> Result<()> {
        let nodes = self.data.pass_graph.nodes();
        let (node_uses, transient_requests) = {
            let ctx = build_pass_context(&self.rrdevice, &mut self.data);
            let node_uses: Vec<Vec<TargetUse>> = nodes
                .iter()
                .map(|node| {
                    node.reads(&ctx)
                        .into_iter()
                        .chain(node.writes(&ctx))
                        .collect()
                })
                .collect();
            let transient_requests: Vec<TransientRequest> = nodes
                .iter()
                .flat_map(|node| node.transients(&ctx))
                .collect();
            (node_uses, transient_requests)
        };
        self.assign_frame_transients(&nodes, &transient_requests, &node_uses)?;

        {
            let mut ctx = build_pass_context(&self.rrdevice, &mut self.data);
            for node in &nodes {
                node.prepare(&mut ctx, frame_slot)?;
            }
        }

        let mut transients_seen = std::collections::HashSet::new();
        for (node, uses) in nodes.iter().zip(&node_uses) {
            let barriers = self.collect_pass_barriers(uses, &mut transients_seen)?;
            thyllore_vulkan_core::record_pending_barriers(
                &self.rrdevice,
                command_buffer,
                &barriers,
            );
            self.gpu_timestamp_profiler.begin_scope(
                &self.rrdevice,
                command_buffer,
                image_index,
                node.name().to_string(),
            );
            let ctx = build_pass_context(&self.rrdevice, &mut self.data);
            node.record(&ctx, command_buffer, image_index, frame_slot)?;
            self.gpu_timestamp_profiler
                .end_scope(&self.rrdevice, command_buffer, image_index);
        }
        Ok(())
    }

    unsafe fn prepare_empty_shadow_mask(&self, command_buffer: CommandBuffer) {
        let Some(ref gbuffer) = self.data.raytracing.gbuffer else {
            return;
        };

        thyllore_vulkan_core::clear_shadow_mask_to_lit(
            &self.rrdevice,
            command_buffer,
            gbuffer.shadow_mask_image(),
        );
    }

    unsafe fn record_object_id_copy(&mut self, command_buffer: CommandBuffer) {
        use crate::ecs::resource::ObjectIdReadback;

        if !self.data.ecs_world.contains_resource::<ObjectIdReadback>() {
            return;
        }

        let readback = self.data.ecs_world.resource::<ObjectIdReadback>();
        if readback.copy_in_flight {
            drop(readback);
            return;
        }
        let Some((px, py)) = readback.pending_pixel else {
            drop(readback);
            return;
        };
        drop(readback);

        let Some(ref gbuffer) = self.data.raytracing.gbuffer else {
            return;
        };

        thyllore_vulkan_core::record_picked_texel_copy(
            &self.rrdevice,
            command_buffer,
            gbuffer.object_id_image(),
            gbuffer.position_image(),
            gbuffer.readback_staging_buffer(),
            (px, py),
        );

        let mut readback = self.data.ecs_world.resource_mut::<ObjectIdReadback>();
        readback.pending_pixel = None;
        readback.copy_in_flight = true;
    }
}
