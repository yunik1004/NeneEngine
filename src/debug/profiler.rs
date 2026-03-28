/// Number of past frames kept for rolling statistics.
pub const PROFILE_HISTORY: usize = 128;

// ── ScopeGuard ────────────────────────────────────────────────────────────────

/// RAII guard returned by [`Profiler::scope`]. No-op in release builds.
pub struct ScopeGuard<'a> {
    #[cfg(debug_assertions)]
    profiler: &'a mut Profiler,
    #[cfg(debug_assertions)]
    name: &'static str,
    #[cfg(debug_assertions)]
    start: std::time::Instant,
    #[cfg(not(debug_assertions))]
    _phantom: std::marker::PhantomData<&'a ()>,
}

#[cfg(debug_assertions)]
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

// ── Internal types (debug only) ───────────────────────────────────────────────

#[cfg(debug_assertions)]
#[derive(Clone)]
pub(crate) struct ScopeEntry {
    pub(crate) name: &'static str,
    pub(crate) ms: f64,
}

#[cfg(debug_assertions)]
#[derive(Clone, Default)]
struct FrameRecord {
    frame_ms: f64,
    scopes: Vec<ScopeEntry>,
}

// ── Profiler ──────────────────────────────────────────────────────────────────

/// Frame profiler. All methods are no-ops in release builds — zero overhead.
pub struct Profiler {
    #[cfg(debug_assertions)]
    frame_start: Option<std::time::Instant>,
    #[cfg(debug_assertions)]
    pub(crate) current_scopes: Vec<ScopeEntry>,
    #[cfg(debug_assertions)]
    history: std::collections::VecDeque<FrameRecord>,
    #[cfg(debug_assertions)]
    fps: f32,
    #[cfg(debug_assertions)]
    frame_ms: f32,
    #[cfg(debug_assertions)]
    min_ms: f32,
    #[cfg(debug_assertions)]
    max_ms: f32,
    #[cfg(debug_assertions)]
    avg_ms: f32,
}

impl Profiler {
    pub fn new() -> Self {
        #[cfg(debug_assertions)]
        return Self {
            frame_start: None,
            current_scopes: Vec::new(),
            history: std::collections::VecDeque::with_capacity(PROFILE_HISTORY),
            fps: 0.0,
            frame_ms: 0.0,
            min_ms: 0.0,
            max_ms: 0.0,
            avg_ms: 0.0,
        };
        #[cfg(not(debug_assertions))]
        Self {}
    }

    pub fn begin_frame(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.frame_start = Some(std::time::Instant::now());
            self.current_scopes.clear();
        }
    }

    pub fn end_frame(&mut self) {
        #[cfg(debug_assertions)]
        {
            let frame_ms = self
                .frame_start
                .take()
                .map(|t| {
                    // Clamp to at least 1 ns so fps is always computable.
                    t.elapsed().as_nanos().max(1) as f64 * 1e-6
                })
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
    }

    /// Begin a named timing scope. No-op in release builds.
    pub fn scope(&mut self, name: &'static str) -> ScopeGuard<'_> {
        #[cfg(debug_assertions)]
        return ScopeGuard {
            profiler: self,
            name,
            start: std::time::Instant::now(),
        };
        #[cfg(not(debug_assertions))]
        {
            let _ = name;
            ScopeGuard {
                _phantom: std::marker::PhantomData,
            }
        }
    }

    pub fn frame_ms(&self) -> f32 {
        #[cfg(debug_assertions)]
        return self.frame_ms;
        #[cfg(not(debug_assertions))]
        0.0
    }

    pub fn fps(&self) -> f32 {
        #[cfg(debug_assertions)]
        return self.fps;
        #[cfg(not(debug_assertions))]
        0.0
    }

    pub fn min_ms(&self) -> f32 {
        #[cfg(debug_assertions)]
        return self.min_ms;
        #[cfg(not(debug_assertions))]
        0.0
    }

    pub fn max_ms(&self) -> f32 {
        #[cfg(debug_assertions)]
        return self.max_ms;
        #[cfg(not(debug_assertions))]
        0.0
    }

    pub fn avg_ms(&self) -> f32 {
        #[cfg(debug_assertions)]
        return self.avg_ms;
        #[cfg(not(debug_assertions))]
        0.0
    }

    pub fn scope_ms(&self, name: &str) -> f32 {
        #[cfg(debug_assertions)]
        return self
            .history
            .back()
            .and_then(|r| r.scopes.iter().find(|s| s.name == name))
            .map(|s| s.ms as f32)
            .unwrap_or(0.0);
        #[cfg(not(debug_assertions))]
        {
            let _ = name;
            0.0
        }
    }

    pub fn frame_history(&self) -> impl Iterator<Item = f32> + '_ {
        #[cfg(debug_assertions)]
        return self.history.iter().map(|r| r.frame_ms as f32);
        #[cfg(not(debug_assertions))]
        return std::iter::empty();
    }

    pub fn draw_overlay(&self, ui: &mut crate::ui::Ui, x: f32, y: f32) {
        #[cfg(debug_assertions)]
        {
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
        #[cfg(not(debug_assertions))]
        {
            let _ = (ui, x, y);
        }
    }

    #[cfg(debug_assertions)]
    fn recompute_stats(&mut self) {
        let Some(last) = self.history.back() else {
            return;
        };
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
