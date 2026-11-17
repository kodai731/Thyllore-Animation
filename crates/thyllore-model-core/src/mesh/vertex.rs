use std::hash::{Hash, Hasher};

use glam::{Vec2, Vec3, Vec4};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: [f32; 4],
    pub tex_coord: Vec2,
    pub normal: Vec3,
}

impl Vertex {
    pub fn new(pos: Vec3, color: Vec4, tex_coord: Vec2) -> Self {
        Self {
            pos,
            color: color.to_array(),
            tex_coord,
            normal: Vec3::new(0.0, 0.0, 1.0),
        }
    }

    pub fn new_with_normal(pos: Vec3, color: Vec4, tex_coord: Vec2, normal: Vec3) -> Self {
        Self {
            pos,
            color: color.to_array(),
            tex_coord,
            normal,
        }
    }
}

impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
            && self.color == other.color
            && self.tex_coord == other.tex_coord
            && self.normal == other.normal
    }
}

impl Eq for Vertex {}

impl Hash for Vertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pos[0].to_bits().hash(state);
        self.pos[1].to_bits().hash(state);
        self.pos[2].to_bits().hash(state);
        self.color[0].to_bits().hash(state);
        self.color[1].to_bits().hash(state);
        self.color[2].to_bits().hash(state);
        self.tex_coord[0].to_bits().hash(state);
        self.tex_coord[1].to_bits().hash(state);
        self.normal[0].to_bits().hash(state);
        self.normal[1].to_bits().hash(state);
        self.normal[2].to_bits().hash(state);
    }
}

/// The part of a vertex that deformation rewrites and ray tracing intersects.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct VertexGeometry {
    pub pos: Vec3,
    pub normal: Vec3,
}

/// The part of a vertex that deformation never touches.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct VertexAttributes {
    pub color: [f32; 4],
    pub tex_coord: Vec2,
}

impl Vertex {
    pub fn geometry(&self) -> VertexGeometry {
        VertexGeometry {
            pos: self.pos,
            normal: self.normal,
        }
    }

    pub fn attributes(&self) -> VertexAttributes {
        VertexAttributes {
            color: self.color,
            tex_coord: self.tex_coord,
        }
    }
}

pub fn write_vertex_geometry(vertices: &[Vertex], geometry: &mut [VertexGeometry]) {
    for (target, vertex) in geometry.iter_mut().zip(vertices) {
        *target = vertex.geometry();
    }
}

pub fn write_vertex_attributes(vertices: &[Vertex], attributes: &mut [VertexAttributes]) {
    for (target, vertex) in attributes.iter_mut().zip(vertices) {
        *target = vertex.attributes();
    }
}

#[derive(Clone, Debug, Default)]
pub struct VertexData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn vertex_is_packed_without_padding() {
        assert_eq!(size_of::<Vertex>(), 48);
        assert_eq!(offset_of!(Vertex, pos), 0);
        assert_eq!(offset_of!(Vertex, color), 12);
        assert_eq!(offset_of!(Vertex, tex_coord), 28);
        assert_eq!(offset_of!(Vertex, normal), 36);
    }

    #[test]
    fn vertex_streams_are_packed_without_padding() {
        assert_eq!(size_of::<VertexGeometry>(), 24);
        assert_eq!(offset_of!(VertexGeometry, pos), 0);
        assert_eq!(offset_of!(VertexGeometry, normal), 12);

        assert_eq!(size_of::<VertexAttributes>(), 24);
        assert_eq!(offset_of!(VertexAttributes, color), 0);
        assert_eq!(offset_of!(VertexAttributes, tex_coord), 16);
    }

    #[test]
    fn vertex_streams_carry_every_vertex_field() {
        let vertices = [Vertex::new_with_normal(
            Vec3::new(1.0, 2.0, 3.0),
            Vec4::new(0.1, 0.2, 0.3, 0.4),
            Vec2::new(0.5, 0.6),
            Vec3::new(0.0, 1.0, 0.0),
        )];

        let mut geometry = [VertexGeometry::default()];
        let mut attributes = [VertexAttributes::default()];
        write_vertex_geometry(&vertices, &mut geometry);
        write_vertex_attributes(&vertices, &mut attributes);

        assert_eq!(geometry[0].pos, vertices[0].pos);
        assert_eq!(geometry[0].normal, vertices[0].normal);
        assert_eq!(attributes[0].color, vertices[0].color);
        assert_eq!(attributes[0].tex_coord, vertices[0].tex_coord);
    }
}
