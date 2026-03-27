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

// ── FixedTime ─────────────────────────────────────────────────────────────────

/// Timing information for one fixed-timestep tick.
///
/// Passed to the `fixed_update` callback by
/// [`Window::run_with_fixed_update`](crate::window::Window::run_with_fixed_update).
///
/// Unlike [`Time`], `delta` is always the same constant value (1 / hz).
/// When a frame is slower than the fixed step the callback fires multiple times
/// per rendered frame; when a frame is faster it may fire zero times.
///
/// # Example
/// ```no_run
/// use nene::time::FixedTime;
///
/// // Inside a fixed_update closure:
/// fn tick(ft: &FixedTime) {
///     // ft.delta is always exactly 1/60 s when running at 60 Hz.
///     println!("tick {} — step {}  dt={:.4}", ft.tick, ft.step, ft.delta);
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct FixedTime {
    /// The constant step size in seconds (= 1 / hz).
    pub delta: f32,
    /// Which tick this is within the current frame (0-indexed).
    ///
    /// Useful for detecting multi-tick catch-up frames.
    pub step: u32,
    /// Total number of fixed ticks that have fired since startup.
    pub tick: u64,
}
