use crate::renderer::Texture;

/// A texture atlas sliced into equal-sized tiles.
///
/// Tile IDs are assigned left-to-right, top-to-bottom starting at **1**.
/// ID **0** is always the empty/transparent tile and is never drawn.
pub struct TileSet {
    /// The underlying atlas texture (share this with [`TileMapRenderer`](super::renderer::TileMapRenderer)).
    pub texture: Texture,
    /// Number of tile columns in the atlas.
    pub cols: u32,
    /// Number of tile rows in the atlas.
    pub rows: u32,
    /// Tile width in pixels.
    pub tile_w: u32,
    /// Tile height in pixels.
    pub tile_h: u32,
    /// Atlas total width in pixels.
    atlas_w: u32,
    /// Atlas total height in pixels.
    atlas_h: u32,
}

impl TileSet {
    /// Create a tileset from an atlas texture.
    ///
    /// - `atlas_w` / `atlas_h` — pixel dimensions of the full atlas image.
    /// - `tile_w` / `tile_h`   — pixel dimensions of a single tile.
    pub fn new(texture: Texture, atlas_w: u32, atlas_h: u32, tile_w: u32, tile_h: u32) -> Self {
        let cols = (atlas_w / tile_w).max(1);
        let rows = (atlas_h / tile_h).max(1);
        Self {
            texture,
            cols,
            rows,
            tile_w,
            tile_h,
            atlas_w,
            atlas_h,
        }
    }

    /// Normalized UV rect for a tile ID (1-based). Returns `None` for ID 0 or out-of-range.
    pub fn uv(&self, id: u16) -> Option<[f32; 4]> {
        if id == 0 {
            return None;
        }
        let idx = (id - 1) as u32;
        let col = idx % self.cols;
        let row = idx / self.cols;
        if row >= self.rows {
            return None;
        }
        let aw = self.atlas_w as f32;
        let ah = self.atlas_h as f32;
        let u0 = (col as f32 * self.tile_w as f32 / aw).min(1.0);
        let v0 = (row as f32 * self.tile_h as f32 / ah).min(1.0);
        let uw = (self.tile_w as f32 / aw).min(1.0 - u0);
        let vw = (self.tile_h as f32 / ah).min(1.0 - v0);
        Some([u0, v0, uw, vw])
    }

    /// Total number of tiles in the atlas.
    pub fn tile_count(&self) -> u32 {
        self.cols * self.rows
    }
}

/// A single grid layer of tile IDs.
///
/// Tile ID **0** means "empty" — the tile is skipped during rendering.
#[derive(Clone)]
pub struct TileLayer {
    /// Columns (width) of the grid.
    pub cols: u32,
    /// Rows (height) of the grid.
    pub rows: u32,
    pub(super) tiles: Vec<u16>,
    /// Optional tint applied to every tile in this layer.
    pub tint: [f32; 4],
    /// Whether this layer is rendered.
    pub visible: bool,
}

impl TileLayer {
    /// Create an empty layer (all tiles = 0).
    pub fn new(cols: u32, rows: u32) -> Self {
        Self {
            cols,
            rows,
            tiles: vec![0; (cols * rows) as usize],
            tint: [1.0; 4],
            visible: true,
        }
    }

    /// Get the tile ID at `(col, row)`.
    pub fn get(&self, col: u32, row: u32) -> u16 {
        if col < self.cols && row < self.rows {
            self.tiles[(row * self.cols + col) as usize]
        } else {
            0
        }
    }

    /// Set the tile ID at `(col, row)`. ID 0 = empty.
    pub fn set(&mut self, col: u32, row: u32, id: u16) {
        if col < self.cols && row < self.rows {
            self.tiles[(row * self.cols + col) as usize] = id;
        }
    }

    /// Fill a rectangular region with `id`.
    pub fn fill_rect(&mut self, col: u32, row: u32, w: u32, h: u32, id: u16) {
        for r in row..row.saturating_add(h).min(self.rows) {
            for c in col..col.saturating_add(w).min(self.cols) {
                self.set(c, r, id);
            }
        }
    }
}

/// A multi-layer tile map with an optional solid-collision bitmask.
pub struct TileMap {
    /// Map width in tiles.
    pub cols: u32,
    /// Map height in tiles.
    pub rows: u32,
    /// Ordered render layers (index 0 = bottom).
    pub layers: Vec<TileLayer>,
    /// Per-cell solid flag for collision queries.
    solid: Vec<bool>,
}

impl TileMap {
    /// Create an empty map with one default layer.
    pub fn new(cols: u32, rows: u32) -> Self {
        Self {
            cols,
            rows,
            layers: vec![TileLayer::new(cols, rows)],
            solid: vec![false; (cols * rows) as usize],
        }
    }

    /// Add a new render layer and return its index.
    pub fn add_layer(&mut self) -> usize {
        self.layers.push(TileLayer::new(self.cols, self.rows));
        self.layers.len() - 1
    }

    /// Set a tile ID in `layer` at `(col, row)`.
    pub fn set(&mut self, col: u32, row: u32, layer: usize, id: u16) {
        if let Some(l) = self.layers.get_mut(layer) {
            l.set(col, row, id);
        }
    }

    /// Get the tile ID in `layer` at `(col, row)`.
    pub fn get(&self, col: u32, row: u32, layer: usize) -> u16 {
        self.layers.get(layer).map_or(0, |l| l.get(col, row))
    }

    /// Mark a cell as solid (for collision detection).
    pub fn set_solid(&mut self, col: u32, row: u32, solid: bool) {
        if col < self.cols && row < self.rows {
            self.solid[(row * self.cols + col) as usize] = solid;
        }
    }

    /// Returns `true` if the cell at `(col, row)` is solid.
    pub fn is_solid(&self, col: u32, row: u32) -> bool {
        if col < self.cols && row < self.rows {
            self.solid[(row * self.cols + col) as usize]
        } else {
            true // out-of-bounds treated as solid walls
        }
    }

    /// Returns `true` if the world-space AABB `(wx, wy, ww, wh)` overlaps any solid tile.
    ///
    /// `tile_size` is the world-space side length of one tile.
    pub fn aabb_solid(&self, wx: f32, wy: f32, ww: f32, wh: f32, tile_size: f32) -> bool {
        let col_min = (wx / tile_size).floor() as i32;
        let col_max = ((wx + ww) / tile_size).ceil() as i32;
        let row_min = (wy / tile_size).floor() as i32;
        let row_max = ((wy + wh) / tile_size).ceil() as i32;

        for r in row_min..row_max {
            for c in col_min..col_max {
                if c < 0 || r < 0 {
                    return true;
                }
                if self.is_solid(c as u32, r as u32) {
                    return true;
                }
            }
        }
        false
    }
}
