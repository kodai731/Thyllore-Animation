use crate::app::{App, AppData};
use rust_rendering::vulkanr::data as vulkan_data;
use rust_rendering::vulkanr::data::*;
use rust_rendering::math::math::*;

use anyhow::Result;

impl App {
    pub(crate) unsafe fn create_grid_data(
        data: &mut AppData,
        index: i32,
        color: Vec4,
        tex_coord: Vec2,
    ) -> Result<()> {
        for i in 0..100 {
            let mut pos1 = Vec3::new(0.0, 0.0, 0.0);
            if index == 0 {
                pos1.x = 100.0;
                pos1.z = i as f32 * 0.1;
            } else if index == 1 {
                pos1.z = i as f32 * 0.1;
                pos1.y = 100.0;
            } else if index == 2 {
                pos1.x = i as f32 * 0.1;
                pos1.z = 100.0;
            }
            let mut pos2 = Vec3::new(0.0, 0.0, 0.0);
            if index == 0 {
                pos2.x = -100.0;
                pos2.z = pos1.z;
            } else if index == 1 {
                pos2.z = pos1.z;
                pos2.y = -100.0;
            } else if index == 2 {
                pos2.x = pos1.x;
                pos2.z = -100.0;
            }
            let vertex1 = vulkan_data::Vertex::new(pos1, color, tex_coord);
            let vertex2 = vulkan_data::Vertex::new(pos2, color, tex_coord);
            let vertex3 = vulkan_data::Vertex::new(-pos1, color, tex_coord);
            let vertex4 = vulkan_data::Vertex::new(-pos2, color, tex_coord);
            data.grid_vertices.push(vertex1);
            data.grid_indices.push(data.grid_indices.len() as u32);
            data.grid_vertices.push(vertex2);
            data.grid_indices.push(data.grid_indices.len() as u32);
            data.grid_vertices.push(vertex3);
            data.grid_indices.push(data.grid_indices.len() as u32);
            data.grid_vertices.push(vertex4);
            data.grid_indices.push(data.grid_indices.len() as u32);
        }

        Ok(())
    }
}
