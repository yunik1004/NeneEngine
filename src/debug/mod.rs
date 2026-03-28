//! Immediate-mode debug drawing — lines, spheres, boxes, rays, axes.
//!
//! # Usage
//! ```no_run
//! # use nene::debug::{DebugDraw, color};
//! # use nene::math::{Mat4, Vec3};
//! # use nene::renderer::Context;
//! # fn demo(ctx: &mut Context, view_proj: Mat4, mut debug: DebugDraw) {
//! debug.line(Vec3::ZERO, Vec3::X, color::RED);
//! debug.sphere(Vec3::new(2.0, 1.0, 0.0), 0.5, color::GREEN);
//! debug.aabb(Vec3::splat(-1.0), Vec3::splat(1.0), color::YELLOW);
//! debug.axes(Vec3::ZERO, 1.0);
//! debug.flush(ctx, view_proj);
//! # }
//! ```

mod draw;
mod profiler;

pub use draw::{color, DebugBuffer, DebugDraw, DebugVertex, MAX_DEBUG_VERTS};
pub use profiler::{Profiler, ScopeGuard, PROFILE_HISTORY};
