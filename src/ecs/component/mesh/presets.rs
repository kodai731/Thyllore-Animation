use super::{VertexAttribute, VertexAttributeId, VertexFormat};

pub const POSITION: VertexAttribute = VertexAttribute::new(
    VertexAttributeId::Position,
    VertexFormat::Float32x3,
    0,
);

pub const NORMAL: VertexAttribute = VertexAttribute::new(
    VertexAttributeId::Normal,
    VertexFormat::Float32x3,
    1,
);

pub const TANGENT: VertexAttribute = VertexAttribute::new(
    VertexAttributeId::Tangent,
    VertexFormat::Float32x4,
    2,
);

pub const COLOR: VertexAttribute = VertexAttribute::new(
    VertexAttributeId::Color,
    VertexFormat::Float32x3,
    3,
);

pub const TEX_COORD_0: VertexAttribute = VertexAttribute::new(
    VertexAttributeId::TexCoord0,
    VertexFormat::Float32x2,
    4,
);

pub const TEX_COORD_1: VertexAttribute = VertexAttribute::new(
    VertexAttributeId::TexCoord1,
    VertexFormat::Float32x2,
    5,
);
