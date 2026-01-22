use crate::ecs::component::{GizmoVertex, LineMesh, MeshScale};

pub fn create_grid_mesh() -> LineMesh {
    let mut mesh = LineMesh::default();

    let grid_count = 1000;
    let grid_extent = 10000.0;
    let grid_spacing = 1.0;

    add_grid_axis(&mut mesh, 0, grid_count, grid_extent, grid_spacing, [1.0, 0.0, 0.0]);
    add_grid_axis(&mut mesh, 1, grid_count, grid_extent, grid_spacing, [0.0, 1.0, 0.0]);
    add_grid_axis(&mut mesh, 2, grid_count, grid_extent, grid_spacing, [0.0, 0.0, 1.0]);

    mesh
}

fn add_grid_axis(
    mesh: &mut LineMesh,
    axis: u32,
    count: u32,
    extent: f32,
    spacing: f32,
    color: [f32; 3],
) {
    for i in 0..count {
        let offset = i as f32 * spacing;

        let (pos1, pos2) = match axis {
            0 => {
                // X axis lines (red)
                let p1 = [extent, 0.0, offset];
                let p2 = [-extent, 0.0, offset];
                (p1, p2)
            }
            1 => {
                // Y axis lines (green)
                let p1 = [0.0, extent, offset];
                let p2 = [0.0, -extent, offset];
                (p1, p2)
            }
            _ => {
                // Z axis lines (blue)
                let p1 = [offset, 0.0, extent];
                let p2 = [offset, 0.0, -extent];
                (p1, p2)
            }
        };

        let neg_pos1 = [-pos1[0], -pos1[1], -pos1[2]];
        let neg_pos2 = [-pos2[0], -pos2[1], -pos2[2]];

        add_line_vertices(mesh, pos1, color);
        add_line_vertices(mesh, pos2, color);
        add_line_vertices(mesh, neg_pos1, color);
        add_line_vertices(mesh, neg_pos2, color);
    }
}

fn add_line_vertices(mesh: &mut LineMesh, pos: [f32; 3], color: [f32; 3]) {
    let vertex = GizmoVertex { pos, color };
    let index = mesh.vertices.len() as u32;
    mesh.vertices.push(vertex);
    mesh.indices.push(index);
}

pub fn create_default_grid_scale() -> MeshScale {
    MeshScale::new(1.0)
}
