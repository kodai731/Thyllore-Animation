use crate::ecs::component::mesh::presets::{COLOR, POSITION};
use crate::ecs::component::mesh::{MeshData, PrimitiveTopology};
use crate::ecs::component::{ColorVertex, LineMesh, MeshScale};

pub fn create_grid_mesh_data() -> MeshData {
    let grid_count = 1000;
    let grid_extent = 10000.0;
    let grid_spacing = 1.0;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    add_grid_axis_data(
        &mut positions,
        &mut colors,
        &mut indices,
        0,
        grid_count,
        grid_extent,
        grid_spacing,
        [1.0, 0.0, 0.0],
    );
    add_grid_axis_data(
        &mut positions,
        &mut colors,
        &mut indices,
        1,
        grid_count,
        grid_extent,
        grid_spacing,
        [0.0, 1.0, 0.0],
    );
    add_grid_axis_data(
        &mut positions,
        &mut colors,
        &mut indices,
        2,
        grid_count,
        grid_extent,
        grid_spacing,
        [0.0, 0.0, 1.0],
    );

    MeshData::new(PrimitiveTopology::LineList)
        .with_inserted_attribute(POSITION, positions)
        .with_inserted_attribute(COLOR, colors)
        .with_indices(indices)
}

fn add_grid_axis_data(
    positions: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    axis: u32,
    count: u32,
    extent: f32,
    spacing: f32,
    color: [f32; 3],
) {
    for i in 0..count {
        let offset = i as f32 * spacing;

        let (pos1, pos2) = match axis {
            0 => ([extent, 0.0, offset], [-extent, 0.0, offset]),
            1 => ([0.0, extent, offset], [0.0, -extent, offset]),
            _ => ([offset, 0.0, extent], [offset, 0.0, -extent]),
        };

        let neg_pos1 = [-pos1[0], -pos1[1], -pos1[2]];
        let neg_pos2 = [-pos2[0], -pos2[1], -pos2[2]];

        add_line_vertex_data(positions, colors, indices, pos1, color);
        add_line_vertex_data(positions, colors, indices, pos2, color);
        add_line_vertex_data(positions, colors, indices, neg_pos1, color);
        add_line_vertex_data(positions, colors, indices, neg_pos2, color);
    }
}

fn add_line_vertex_data(
    positions: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    pos: [f32; 3],
    color: [f32; 3],
) {
    let index = positions.len() as u32;
    positions.push(pos);
    colors.push(color);
    indices.push(index);
}

#[deprecated(note = "Use create_grid_mesh_data() instead")]
pub fn create_grid_mesh() -> (LineMesh, u32) {
    let mut mesh = LineMesh::default();

    let grid_count = 1000;
    let grid_extent = 10000.0;
    let grid_spacing = 1.0;

    add_grid_axis(
        &mut mesh,
        0,
        grid_count,
        grid_extent,
        grid_spacing,
        [1.0, 0.0, 0.0],
    );
    add_grid_axis(
        &mut mesh,
        2,
        grid_count,
        grid_extent,
        grid_spacing,
        [0.0, 0.0, 1.0],
    );

    let xz_only_index_count = mesh.indices.len() as u32;

    add_grid_axis(
        &mut mesh,
        1,
        grid_count,
        grid_extent,
        grid_spacing,
        [0.0, 1.0, 0.0],
    );

    (mesh, xz_only_index_count)
}

#[allow(deprecated)]
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
                let p1 = [extent, 0.0, offset];
                let p2 = [-extent, 0.0, offset];
                (p1, p2)
            }
            1 => {
                let p1 = [0.0, extent, offset];
                let p2 = [0.0, -extent, offset];
                (p1, p2)
            }
            _ => {
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

#[allow(deprecated)]
fn add_line_vertices(mesh: &mut LineMesh, pos: [f32; 3], color: [f32; 3]) {
    let vertex = ColorVertex { pos, color };
    let index = mesh.vertices.len() as u32;
    mesh.vertices.push(vertex);
    mesh.indices.push(index);
}

pub fn create_default_grid_scale() -> MeshScale {
    MeshScale::new(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(deprecated)]
    #[test]
    fn test_grid_mesh_xz_only_index_count() {
        let (mesh, xz_only_index_count) = create_grid_mesh();
        let total_index_count = mesh.indices.len() as u32;

        assert!(xz_only_index_count > 0);
        assert!(xz_only_index_count < total_index_count);

        let y_axis_index_count = total_index_count - xz_only_index_count;
        let expected_per_axis = total_index_count / 3;
        assert_eq!(xz_only_index_count, expected_per_axis * 2);
        assert_eq!(y_axis_index_count, expected_per_axis);
    }

    #[allow(deprecated)]
    #[test]
    fn test_grid_mesh_xz_vertices_have_zero_y() {
        let (mesh, xz_only_index_count) = create_grid_mesh();

        for i in 0..xz_only_index_count as usize {
            let vertex = &mesh.vertices[mesh.indices[i] as usize];
            assert_eq!(
                vertex.pos[1], 0.0,
                "XZ grid vertex at index {} should have y=0, got y={}",
                i, vertex.pos[1]
            );
        }
    }

    #[allow(deprecated)]
    #[test]
    fn test_grid_mesh_y_axis_vertices_exist_after_xz() {
        let (mesh, xz_only_index_count) = create_grid_mesh();
        let total = mesh.indices.len();

        let mut has_nonzero_y = false;
        for i in xz_only_index_count as usize..total {
            let vertex = &mesh.vertices[mesh.indices[i] as usize];
            if vertex.pos[1] != 0.0 {
                has_nonzero_y = true;
                break;
            }
        }
        assert!(
            has_nonzero_y,
            "Y-axis grid should have vertices with non-zero Y"
        );
    }
}
