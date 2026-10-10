#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DebugPrimitiveKind {
    Cube,
    Sphere,
    Floor,
}

impl DebugPrimitiveKind {
    pub const ALL: [DebugPrimitiveKind; 3] = [
        DebugPrimitiveKind::Cube,
        DebugPrimitiveKind::Sphere,
        DebugPrimitiveKind::Floor,
    ];

    /// Stable name persisted in scene files.
    pub fn name(self) -> &'static str {
        match self {
            DebugPrimitiveKind::Cube => "cube",
            DebugPrimitiveKind::Sphere => "sphere",
            DebugPrimitiveKind::Floor => "floor",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }
}
