use std::sync::atomic::{AtomicBool, Ordering};

pub struct FbxDebugFlags {
    pub animation: AtomicBool,
    pub hierarchy: AtomicBool,
    pub skinning: AtomicBool,
    pub transform: AtomicBool,
}

impl FbxDebugFlags {
    pub fn animation_enabled(&self) -> bool {
        self.animation.load(Ordering::Relaxed)
    }

    pub fn hierarchy_enabled(&self) -> bool {
        self.hierarchy.load(Ordering::Relaxed)
    }

    pub fn skinning_enabled(&self) -> bool {
        self.skinning.load(Ordering::Relaxed)
    }

    pub fn transform_enabled(&self) -> bool {
        self.transform.load(Ordering::Relaxed)
    }

    pub fn set_animation(&self, value: bool) {
        self.animation.store(value, Ordering::Relaxed);
    }

    pub fn set_hierarchy(&self, value: bool) {
        self.hierarchy.store(value, Ordering::Relaxed);
    }

    pub fn set_skinning(&self, value: bool) {
        self.skinning.store(value, Ordering::Relaxed);
    }

    pub fn set_transform(&self, value: bool) {
        self.transform.store(value, Ordering::Relaxed);
    }
}

pub static FBX_DEBUG: FbxDebugFlags = FbxDebugFlags {
    animation: AtomicBool::new(false),
    hierarchy: AtomicBool::new(false),
    skinning: AtomicBool::new(false),
    transform: AtomicBool::new(false),
};
