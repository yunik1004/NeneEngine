//! Tile map system.
//!
//! # Quick start
//! ```no_run
//! use nene::tilemap::{TileSet, TileMap, TileMapRenderer};
//!
//! // In init:
//! // let tileset = TileSet::new(texture, 256, 256, 16, 16); // 256×256 atlas, 16×16 tiles
//! // let mut map = TileMap::new(20, 15);           // 20 cols × 15 rows
//! // map.set(col, row, layer, tile_id);
//! // map.set_solid(col, row, true);
//! // let renderer = TileMapRenderer::new(ctx, tile_world_size);
//!
//! // In update (view-culled — only visible tiles uploaded):
//! // renderer.prepare(ctx, &map, &tileset, &camera, aspect);
//!
//! // In render:
//! // renderer.render(pass, &tileset);
//! ```

mod map;
mod renderer;

pub use map::{TileLayer, TileMap, TileSet};
pub use renderer::{MAX_VISIBLE_TILES, TileMapRenderer};
