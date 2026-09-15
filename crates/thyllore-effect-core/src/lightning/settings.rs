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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LightningRenderSettings {
    pub debug_view: LightningDebugView,
}
