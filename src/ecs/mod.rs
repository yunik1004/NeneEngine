//! Entity Component System (ECS).
//!
//! Separates game data (components) from game logic (systems) for better
//! cache performance, modularity, and hot-reload friendliness.
//!
//! # Core concepts
//!
//! - [`Entity`] — a lightweight ID handle (index + generation counter)
//! - [`Component`] — any `Send + Sync + 'static` type; no trait impl needed
//! - [`World`] — holds all entities and their component data
//! - [`Bundle`] — a tuple of components spawned together
//!
//! # Example
//!
//! ```
//! use nene::ecs::World;
//!
//! struct Position { x: f32, y: f32 }
//! struct Velocity  { x: f32, y: f32 }
//! struct Health(f32);
//!
//! let mut world = World::new();
//!
//! // Spawn entities with component bundles
//! let player = world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 0.0 }, Health(100.0)));
//! let _enemy = world.spawn((Position { x: 5.0, y: 0.0 }, Health(30.0)));
//!
//! // Add / remove components at runtime
//! world.insert(player, Health(80.0));
//! let _hp: Option<&Health> = world.get(player);
//!
//! // Single-component iteration
//! for (_e, hp) in world.query::<Health>() {
//!     let _ = hp.0;
//! }
//!
//! // Multi-component iteration (closure-based)
//! world.view_mut(|_e, pos: &mut Position, vel: &Velocity| {
//!     pos.x += vel.x;
//!     pos.y += vel.y;
//! });
//!
//! // Despawn
//! world.despawn(player);
//! assert!(!world.is_alive(player));
//! ```

mod query;
mod storage;
mod world;

pub use query::{FilteredIter, QueryBuilder, QueryBuilderMut};
pub use storage::{Bundle, Component, SparseSet};
pub use world::{Entity, World};
