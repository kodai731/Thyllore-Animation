#[derive(Clone, Debug)]
pub enum DeferredAction {
    LoadModel {
        path: String,
    },
    TakeScreenshot,
    #[cfg(debug_assertions)]
    DebugShadowInfo,
    #[cfg(debug_assertions)]
    DebugBillboardDepth,
    DumpDebugInfo,
    DumpAnimationDebug,
}
