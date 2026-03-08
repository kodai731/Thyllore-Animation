#[derive(Clone, Debug)]
pub enum DeferredAction {
    LoadModel { path: String },
    TakeScreenshot,
    DebugShadowInfo,
    DebugBillboardDepth,
    DumpDebugInfo,
    DumpAnimationDebug,
}

