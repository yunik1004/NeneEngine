pub mod ai;
pub mod animation;
pub mod app;
pub mod asset;
pub mod audio;
pub mod camera;
pub mod debug;
pub mod ecs;
pub mod event;
pub mod input;
pub mod locale;
pub mod math;
pub mod mesh;
pub mod net;
pub mod pak;
pub mod particle;
pub mod persist;
pub mod physics;
pub mod renderer;
pub mod scene;
pub mod sprite;
pub mod text;
pub mod tilemap;
pub mod time;
pub mod ui;

/// Embed the asset archive produced by `nene-build` into the binary and
/// register it so that [`asset::Assets::new`] mounts it automatically.
///
/// Place this call **once**, at the top of `main()`, before `run::<App>()`:
///
/// ```rust,ignore
/// fn main() {
///     nene::embed_assets!();
///     nene::run::<MyGame>();
/// }
/// ```
///
/// When no `build.rs` / `nene-build` is present the macro expands to nothing.
#[macro_export]
macro_rules! embed_assets {
    () => {
        #[cfg(nene_has_pak)]
        $crate::pak::register_embedded_pak(
            include_bytes!(env!("NENE_ASSETS_PAK"))
        );
    };
}

pub use nene_derive::data;
pub use nene_derive::uniform;
pub use nene_derive::vertex;
pub use serde;
pub use serde::{Deserialize, Serialize};

#[doc(hidden)]
pub use encase as __encase;
