use crate::app::{App, AppData};
use crate::vulkanr::data as vulkan_data;
use crate::vulkanr::data::*;
use crate::math::*;

use anyhow::Result;

impl App {
    pub(crate) unsafe fn create_grid_data(
        data: &mut AppData,
        index: i32,
        color: Vec4,
        tex_coord: Vec2,
    ) -> Result<()> {
        let grid_count = 1000;
        let grid_extent = 10000.0;
        let grid_spacing = 1.0;

        for i in 0..grid_count {
            let mut pos1 = Vec3::new(0.0, 0.0, 0.0);
            if index == 0 {
                pos1.x = grid_extent;
                pos1.z = i as f32 * grid_spacing;
            } else if index == 1 {
                pos1.z = i as f32 * grid_spacing;
                pos1.y = grid_extent;
            } else if index == 2 {
                pos1.x = i as f32 * grid_spacing;
                pos1.z = grid_extent;
            }
            let mut pos2 = Vec3::new(0.0, 0.0, 0.0);
            if index == 0 {
                pos2.x = -grid_extent;
                pos2.z = pos1.z;
            } else if index == 1 {
                pos2.z = pos1.z;
                pos2.y = -grid_extent;
            } else if index == 2 {
                pos2.x = pos1.x;
                pos2.z = -grid_extent;
            }
            let vertex1 = vulkan_data::Vertex::new(pos1, color, tex_coord);
            let vertex2 = vulkan_data::Vertex::new(pos2, color, tex_coord);
            let vertex3 = vulkan_data::Vertex::new(-pos1, color, tex_coord);
            let vertex4 = vulkan_data::Vertex::new(-pos2, color, tex_coord);
            data.grid.vertices.push(vertex1);
            data.grid.indices.push(data.grid.indices.len() as u32);
            data.grid.vertices.push(vertex2);
            data.grid.indices.push(data.grid.indices.len() as u32);
            data.grid.vertices.push(vertex3);
            data.grid.indices.push(data.grid.indices.len() as u32);
            data.grid.vertices.push(vertex4);
            data.grid.indices.push(data.grid.indices.len() as u32);
        }

        Ok(())
    }
}
