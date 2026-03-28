//! CPU particle simulation + GPU billboard rendering.
//!
//! # Quick start
//! ```no_run
//! use nene::particle::{EmitterConfig, ParticleSystem};
//! use nene::math::Vec3;
//!
//! // In init:
//! // let mut fire = ParticleSystem::new(&ctx, EmitterConfig::fire());
//!
//! // In update (per frame):
//! // let view_proj = camera.view_proj(aspect);
//! // let cam_right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
//! // let cam_up    = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);
//! // fire.update(time.delta, emitter_pos, view_proj, cam_right, cam_up, &ctx);
//!
//! // In render:
//! // fire.draw(&mut pass);
//! ```

mod emitter;
mod pool;
mod system;

pub use emitter::{EmitterConfig, ParticleInstance};
pub use pool::ParticlePool;
pub use system::{ParticleSystem, MAX_PARTICLES};
