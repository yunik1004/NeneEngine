/// Per-frame timing information.
#[derive(Debug, Clone, Copy)]
pub struct Time {
    pub delta: f32,
    pub elapsed: f64,
    pub frame: u64,
}

impl Time {
    pub fn fps(&self) -> f32 {
        if self.delta > 0.0 { 1.0 / self.delta } else { 0.0 }
    }
}

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
    pub delta: f32,
    pub step: u32,
    pub tick: u64,
}
