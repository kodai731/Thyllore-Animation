#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrameStep {
    WallClock,
    Fixed { delta_seconds: f32 },
}

/// The engine's frame counter and how time advances between frames. A fixed step makes every
/// frame reproducible: the timeline and effects advance by the step, GPU readbacks are synced to
/// the previous frame, and wall-clock UI is skipped.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameClock {
    pub frame: u64,
    pub step: FrameStep,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::wall_clock()
    }
}

impl FrameClock {
    pub const BATCH_DELTA_SECONDS: f32 = 1.0 / 60.0;

    pub fn wall_clock() -> Self {
        Self {
            frame: 0,
            step: FrameStep::WallClock,
        }
    }

    pub fn fixed(delta_seconds: f32) -> Self {
        Self {
            frame: 0,
            step: FrameStep::Fixed { delta_seconds },
        }
    }

    pub fn advance(&mut self) {
        self.frame += 1;
    }

    pub fn is_fixed(&self) -> bool {
        matches!(self.step, FrameStep::Fixed { .. })
    }

    pub fn fixed_delta_seconds(&self) -> Option<f32> {
        match self.step {
            FrameStep::WallClock => None,
            FrameStep::Fixed { delta_seconds } => Some(delta_seconds),
        }
    }

    /// Elapsed time under a fixed step; `None` on the wall clock.
    pub fn fixed_time_seconds(&self) -> Option<f32> {
        self.fixed_delta_seconds()
            .map(|delta_seconds| self.frame as f32 * delta_seconds)
    }
}
