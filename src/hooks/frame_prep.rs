use crate::ecs::FrameContext;

/// Advance steps an effect's per-frame state (time, attachment, trails); Accumulate runs after the
/// GPU timings of the previous frame are written and records history for the next one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FramePrepStage {
    Advance,
    Accumulate,
}

impl FramePrepStage {
    pub fn label(self) -> &'static str {
        match self {
            FramePrepStage::Advance => "advance",
            FramePrepStage::Accumulate => "accumulate",
        }
    }
}

pub type FramePrepRunFn = fn(&mut FrameContext);

#[derive(Clone, Copy)]
pub struct FramePrepHook {
    pub name: &'static str,
    pub stage: FramePrepStage,
    pub run: FramePrepRunFn,
}

impl FramePrepHook {
    pub fn timing_key(&self) -> String {
        format!("{}_{}", self.name, self.stage.label())
    }
}

inventory::collect!(FramePrepHook);

/// Registers a function that runs every frame inside the render prep phase, at the given stage.
#[macro_export]
macro_rules! frame_prep_hook {
    ($name:literal, $stage:ident, $run:path) => {
        inventory::submit! {
            $crate::hooks::frame_prep::FramePrepHook {
                name: $name,
                stage: $crate::hooks::frame_prep::FramePrepStage::$stage,
                run: $run,
            }
        }
    };
}

/// Every frame prep hook submitted at link time, sorted by stage then name.
pub struct FramePrepHooks {
    entries: Vec<FramePrepHook>,
}

impl FramePrepHooks {
    pub fn collect() -> anyhow::Result<Self> {
        let mut entries: Vec<FramePrepHook> = inventory::iter::<FramePrepHook>
            .into_iter()
            .copied()
            .collect();
        entries.sort_by_key(|hook| (hook.stage, hook.name));
        for pair in entries.windows(2) {
            anyhow::ensure!(
                (pair[0].stage, pair[0].name) != (pair[1].stage, pair[1].name),
                "frame prep hook {} registered twice at stage {:?}",
                pair[0].name,
                pair[0].stage
            );
        }
        Ok(Self { entries })
    }

    pub fn at_stage(&self, stage: FramePrepStage) -> Vec<FramePrepHook> {
        self.entries
            .iter()
            .filter(|hook| hook.stage == stage)
            .copied()
            .collect()
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.entries.iter().map(|hook| hook.name).collect()
    }
}

impl std::fmt::Debug for FramePrepHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FramePrepHooks")
            .field("names", &self.names())
            .finish()
    }
}
