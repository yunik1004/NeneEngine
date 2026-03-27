//! Frame profiler with named scopes and a UI overlay.
//!
//! # Quick start
//! ```no_run
//! use nene::profile::Profiler;
//!
//! let mut profiler = Profiler::new();
//!
//! // Each frame:
//! profiler.begin_frame();
//!
//! {
//!     let _s = profiler.scope("update");
//!     // ... update work ...
//! }
//! {
//!     let _s = profiler.scope("render");
//!     // ... render work ...
//! }
//!
//! profiler.end_frame();
//!
//! // Inside the UI pass:
//! // profiler.draw_overlay(&mut ui, 10.0, 10.0);
//! ```

use std::collections::VecDeque;
use std::time::Instant;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Number of past frames kept for rolling statistics.
pub const PROFILE_HISTORY: usize = 128;

// ── ScopeGuard ────────────────────────────────────────────────────────────────

/// RAII guard returned by [`Profiler::scope`].
///
/// The scope is recorded when this value is dropped.
pub struct ScopeGuard<'a> {
    profiler: &'a mut Profiler,
    name: &'static str,
    start: Instant,
}

impl Drop for ScopeGuard<'_> {
    fn drop(&mut self) {
        let elapsed_ms = self.start.elapsed().as_secs_f64() * 1000.0;
        if let Some(scope) = self
            .profiler
            .current_scopes
            .iter_mut()
            .find(|s| s.name == self.name)
        {
            scope.ms += elapsed_ms;
        } else {
            self.profiler.current_scopes.push(ScopeEntry {
                name: self.name,
                ms: elapsed_ms,
            });
        }
    }
}

// ── Internal types ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ScopeEntry {
    name: &'static str,
    ms: f64,
}

#[derive(Clone, Default)]
struct FrameRecord {
    frame_ms: f64,
    scopes: Vec<ScopeEntry>,
}

// ── Profiler ──────────────────────────────────────────────────────────────────

/// Tracks frame timing and named scopes. Draw the overlay with
/// [`draw_overlay`](Self::draw_overlay).
pub struct Profiler {
    frame_start: Option<Instant>,
    current_scopes: Vec<ScopeEntry>,
    history: VecDeque<FrameRecord>,

    // Rolling stats (updated each end_frame)
    fps: f32,
    frame_ms: f32,
    min_ms: f32,
    max_ms: f32,
    avg_ms: f32,
}

impl Profiler {
    /// Create a new profiler.
    pub fn new() -> Self {
        Self {
            frame_start: None,
            current_scopes: Vec::new(),
            history: VecDeque::with_capacity(PROFILE_HISTORY),
            fps: 0.0,
            frame_ms: 0.0,
            min_ms: 0.0,
            max_ms: 0.0,
            avg_ms: 0.0,
        }
    }

    // ── Frame lifecycle ───────────────────────────────────────────────────────

    /// Call at the very start of each frame (before update).
    pub fn begin_frame(&mut self) {
        self.frame_start = Some(Instant::now());
        self.current_scopes.clear();
    }

    /// Call at the end of each frame (after render). Updates rolling stats.
    pub fn end_frame(&mut self) {
        let frame_ms = self
            .frame_start
            .take()
            .map(|t| t.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        let record = FrameRecord {
            frame_ms,
            scopes: self.current_scopes.clone(),
        };

        if self.history.len() == PROFILE_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(record);

        self.recompute_stats();
    }

    /// Begin a named timing scope. The scope ends when the returned guard drops.
    ///
    /// ```no_run
    /// # use nene::profile::Profiler;
    /// # let mut p = Profiler::new();
    /// # p.begin_frame();
    /// {
    ///     let _s = p.scope("physics");
    ///     // ... physics update ...
    /// } // scope recorded here
    /// # p.end_frame();
    /// ```
    pub fn scope(&mut self, name: &'static str) -> ScopeGuard<'_> {
        ScopeGuard {
            profiler: self,
            name,
            start: Instant::now(),
        }
    }

    // ── Stats accessors ───────────────────────────────────────────────────────

    /// Current frame time in milliseconds.
    pub fn frame_ms(&self) -> f32 {
        self.frame_ms
    }

    /// Frames per second (1000 / avg_ms).
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Minimum frame time over the history window (ms).
    pub fn min_ms(&self) -> f32 {
        self.min_ms
    }

    /// Maximum frame time over the history window (ms).
    pub fn max_ms(&self) -> f32 {
        self.max_ms
    }

    /// Average frame time over the history window (ms).
    pub fn avg_ms(&self) -> f32 {
        self.avg_ms
    }

    /// Most recent recorded time for a named scope in milliseconds.
    /// Returns `0.0` if the scope was not recorded this frame.
    pub fn scope_ms(&self, name: &str) -> f32 {
        self.history
            .back()
            .and_then(|r| r.scopes.iter().find(|s| s.name == name))
            .map(|s| s.ms as f32)
            .unwrap_or(0.0)
    }

    /// Frame-time history as a slice of millisecond values, oldest first.
    pub fn frame_history(&self) -> impl Iterator<Item = f32> + '_ {
        self.history.iter().map(|r| r.frame_ms as f32)
    }

    // ── UI overlay ────────────────────────────────────────────────────────────

    /// Draw a stats panel into the given [`Ui`](crate::ui::Ui) context.
    ///
    /// Call inside `begin_frame` / `end_frame` of the `Ui`, after
    /// [`Profiler::end_frame`].
    pub fn draw_overlay(&self, ui: &mut crate::ui::Ui, x: f32, y: f32) {
        ui.begin_panel("Profiler", x, y, 200.0);

        ui.label("Frame time");
        ui.separator();
        ui.label_dim(&format!("fps      {:.0}", self.fps));
        ui.label_dim(&format!("cur      {:.2} ms", self.frame_ms));
        ui.label_dim(&format!("avg      {:.2} ms", self.avg_ms));
        ui.label_dim(&format!("min      {:.2} ms", self.min_ms));
        ui.label_dim(&format!("max      {:.2} ms", self.max_ms));

        if let Some(last) = self.history.back()
            && !last.scopes.is_empty()
        {
            ui.separator();
            ui.label("Scopes");
            ui.separator();
            for scope in &last.scopes {
                ui.label_dim(&format!("{:<10} {:.2} ms", scope.name, scope.ms));
            }
        }

        ui.end_panel();
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn recompute_stats(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let last = self.history.back().unwrap();
        self.frame_ms = last.frame_ms as f32;

        let frames: Vec<f32> = self.history.iter().map(|r| r.frame_ms as f32).collect();
        self.min_ms = frames.iter().cloned().fold(f32::MAX, f32::min);
        self.max_ms = frames.iter().cloned().fold(0.0_f32, f32::max);
        self.avg_ms = frames.iter().sum::<f32>() / frames.len() as f32;
        self.fps = if self.avg_ms > 0.0 {
            1000.0 / self.avg_ms
        } else {
            0.0
        };
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}
