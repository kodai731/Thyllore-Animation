#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightningDebugView {
    #[default]
    Off,
    Coverage,
    SegmentHits,
    CoreCoverage,
}

impl LightningDebugView {
    pub const ALL: [LightningDebugView; 4] = [
        LightningDebugView::Off,
        LightningDebugView::Coverage,
        LightningDebugView::SegmentHits,
        LightningDebugView::CoreCoverage,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LightningDebugView::Off => "Off",
            LightningDebugView::Coverage => "Coverage",
            LightningDebugView::SegmentHits => "Segment Hits",
            LightningDebugView::CoreCoverage => "Core Coverage",
        }
    }

    pub fn as_shader_value(self) -> i32 {
        match self {
            LightningDebugView::Off => 0,
            LightningDebugView::Coverage => 1,
            LightningDebugView::SegmentHits => 2,
            LightningDebugView::CoreCoverage => 3,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(LightningDebugView::Off),
            "coverage" => Some(LightningDebugView::Coverage),
            "segment-hits" | "hits" => Some(LightningDebugView::SegmentHits),
            "core-coverage" | "core" => Some(LightningDebugView::CoreCoverage),
            _ => None,
        }
    }
}

impl std::str::FromStr for LightningDebugView {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value).ok_or_else(|| {
            format!("invalid lightning debug view '{value}': expected off|coverage|hits|core")
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightningShadingMode {
    #[default]
    ClosedForm,
    ReferenceQuadrature,
}

impl LightningShadingMode {
    pub const ALL: [LightningShadingMode; 2] = [
        LightningShadingMode::ClosedForm,
        LightningShadingMode::ReferenceQuadrature,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LightningShadingMode::ClosedForm => "Closed Form",
            LightningShadingMode::ReferenceQuadrature => "Reference Quadrature",
        }
    }

    pub fn as_shader_value(self) -> i32 {
        match self {
            LightningShadingMode::ClosedForm => 0,
            LightningShadingMode::ReferenceQuadrature => 1,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "closed" | "closed-form" => Some(LightningShadingMode::ClosedForm),
            "reference" | "quadrature" => Some(LightningShadingMode::ReferenceQuadrature),
            _ => None,
        }
    }
}

impl std::str::FromStr for LightningShadingMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
            .ok_or_else(|| format!("invalid lightning mode '{value}': expected closed|reference"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightningRenderSettings {
    pub shading_mode: LightningShadingMode,
    pub reference_step_count: u32,
    pub debug_view: LightningDebugView,
    pub batch_fixed_time: Option<f32>,
}

impl Default for LightningRenderSettings {
    fn default() -> Self {
        Self {
            shading_mode: LightningShadingMode::ClosedForm,
            reference_step_count: 1024,
            debug_view: LightningDebugView::Off,
            batch_fixed_time: None,
        }
    }
}

/// Instance slots of the lightning UBO and of the segment UBO.
pub const LIGHTNING_MAX_INSTANCES: usize = 4;
