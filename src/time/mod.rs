/// Per-frame timing information.
///
/// Passed to the `update` callback each frame. All durations are in seconds.
#[derive(Debug, Clone, Copy)]
pub struct Time {
    /// Seconds elapsed since the previous frame.
    pub delta: f32,
    /// Total seconds elapsed since the application started.
    pub elapsed: f64,
    /// Frames rendered since start.
    pub frame: u64,
}

impl Time {
    /// Frames per second derived from `delta` (clamped to avoid division by zero).
    pub fn fps(&self) -> f32 {
        if self.delta > 0.0 {
            1.0 / self.delta
        } else {
            0.0
        }
    }
}
