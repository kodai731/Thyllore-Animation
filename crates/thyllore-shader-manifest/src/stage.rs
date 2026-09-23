#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StageKind {
    Vertex,
    Fragment,
    Geometry,
    Compute,
    RayGeneration,
    Intersection,
    AnyHit,
    ClosestHit,
    Miss,
}

/// One row per stage: the Slang `[shader("..")]` attribute (also slangc's `-stage`), the word an
/// entry file may end with, and the suffix of its `.spv`.
struct StageNames {
    kind: StageKind,
    attribute: &'static str,
    file_word: &'static str,
    spirv_suffix: &'static str,
}

const STAGES: [StageNames; 9] = [
    StageNames {
        kind: StageKind::Vertex,
        attribute: "vertex",
        file_word: "Vertex",
        spirv_suffix: "Vert",
    },
    StageNames {
        kind: StageKind::Fragment,
        attribute: "fragment",
        file_word: "Fragment",
        spirv_suffix: "Frag",
    },
    StageNames {
        kind: StageKind::Geometry,
        attribute: "geometry",
        file_word: "Geometry",
        spirv_suffix: "Geom",
    },
    StageNames {
        kind: StageKind::Compute,
        attribute: "compute",
        file_word: "Compute",
        spirv_suffix: "Comp",
    },
    StageNames {
        kind: StageKind::RayGeneration,
        attribute: "raygeneration",
        file_word: "RayGen",
        spirv_suffix: "Rgen",
    },
    StageNames {
        kind: StageKind::Intersection,
        attribute: "intersection",
        file_word: "Intersection",
        spirv_suffix: "Rint",
    },
    StageNames {
        kind: StageKind::AnyHit,
        attribute: "anyhit",
        file_word: "AnyHit",
        spirv_suffix: "Rahit",
    },
    StageNames {
        kind: StageKind::ClosestHit,
        attribute: "closesthit",
        file_word: "ClosestHit",
        spirv_suffix: "Rchit",
    },
    StageNames {
        kind: StageKind::Miss,
        attribute: "miss",
        file_word: "Miss",
        spirv_suffix: "Rmiss",
    },
];

impl StageKind {
    pub const ALL: [StageKind; 9] = [
        StageKind::Vertex,
        StageKind::Fragment,
        StageKind::Geometry,
        StageKind::Compute,
        StageKind::RayGeneration,
        StageKind::Intersection,
        StageKind::AnyHit,
        StageKind::ClosestHit,
        StageKind::Miss,
    ];

    pub fn from_attribute(attribute: &str) -> Option<Self> {
        STAGES
            .iter()
            .find(|names| names.attribute == attribute)
            .map(|names| names.kind)
    }

    fn names(self) -> &'static StageNames {
        STAGES
            .iter()
            .find(|names| names.kind == self)
            .expect("every stage has a row in STAGES")
    }

    pub fn attribute(self) -> &'static str {
        self.names().attribute
    }

    pub fn file_word(self) -> &'static str {
        self.names().file_word
    }

    pub fn spirv_suffix(self) -> &'static str {
        self.names().spirv_suffix
    }

    pub fn reflect_variant(self) -> &'static str {
        match self {
            Self::Vertex => "Vertex",
            Self::Fragment => "Fragment",
            Self::Geometry => "Geometry",
            Self::Compute => "Compute",
            Self::RayGeneration => "RayGeneration",
            Self::Intersection => "Intersection",
            Self::AnyHit => "AnyHit",
            Self::ClosestHit => "ClosestHit",
            Self::Miss => "Miss",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_round_trips_through_the_table() {
        for stage in StageKind::ALL {
            assert_eq!(StageKind::from_attribute(stage.attribute()), Some(stage));
        }
        assert_eq!(StageKind::from_attribute("pixel"), None);
        assert_eq!(StageKind::RayGeneration.spirv_suffix(), "Rgen");
        assert_eq!(StageKind::RayGeneration.file_word(), "RayGen");
    }
}
