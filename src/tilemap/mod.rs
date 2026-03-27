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

use bytemuck::{Pod, Zeroable};

use crate::{
    camera::{Camera, Projection},
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, Texture, UniformBuffer,
        VertexAttribute, VertexBuffer, VertexFormat, VertexLayout,
    },
};

// ── WGSL ──────────────────────────────────────────────────────────────────────

const TILE_SHADER: &str = r#"
struct ViewProj { vp: mat4x4<f32> }
@group(0) @binding(0) var<uniform> u: ViewProj;
@group(1) @binding(0) var tex:  texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VertIn {
    @location(0) pos : vec2<f32>,
    @location(1) uv  : vec2<f32>,
    @location(2) tint: vec4<f32>,
}
struct VertOut {
    @builtin(position) clip : vec4<f32>,
    @location(0)       uv   : vec2<f32>,
    @location(1)       tint : vec4<f32>,
}

@vertex fn vs_main(in: VertIn) -> VertOut {
    var out: VertOut;
    out.clip = u.vp * vec4<f32>(in.pos, 0.0, 1.0);
    out.uv   = in.uv;
    out.tint = in.tint;
    return out;
}

@fragment fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv) * in.tint;
}
"#;

// ── Vertex ────────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct TileVert {
    pos: [f32; 2],
    uv: [f32; 2],
    tint: [f32; 4],
}

fn vert_layout() -> VertexLayout {
    use std::mem::offset_of;
    VertexLayout {
        stride: std::mem::size_of::<TileVert>() as u64,
        attributes: vec![
            VertexAttribute {
                location: 0,
                offset: offset_of!(TileVert, pos) as u64,
                format: VertexFormat::Float32x2,
            },
            VertexAttribute {
                location: 1,
                offset: offset_of!(TileVert, uv) as u64,
                format: VertexFormat::Float32x2,
            },
            VertexAttribute {
                location: 2,
                offset: offset_of!(TileVert, tint) as u64,
                format: VertexFormat::Float32x4,
            },
        ],
    }
}

// ── TileSet ───────────────────────────────────────────────────────────────────

/// A texture atlas sliced into equal-sized tiles.
///
/// Tile IDs are assigned left-to-right, top-to-bottom starting at **1**.
/// ID **0** is always the empty/transparent tile and is never drawn.
pub struct TileSet {
    /// The underlying atlas texture (share this with [`TileMapRenderer`]).
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
        let u0 = col as f32 * self.tile_w as f32 / aw;
        let v0 = row as f32 * self.tile_h as f32 / ah;
        let uw = self.tile_w as f32 / aw;
        let vw = self.tile_h as f32 / ah;
        Some([u0, v0, uw, vw])
    }

    /// Total number of tiles in the atlas.
    pub fn tile_count(&self) -> u32 {
        self.cols * self.rows
    }
}

// ── TileLayer ─────────────────────────────────────────────────────────────────

/// A single grid layer of tile IDs.
///
/// Tile ID **0** means "empty" — the tile is skipped during rendering.
#[derive(Clone)]
pub struct TileLayer {
    /// Columns (width) of the grid.
    pub cols: u32,
    /// Rows (height) of the grid.
    pub rows: u32,
    tiles: Vec<u16>,
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

// ── TileMap ───────────────────────────────────────────────────────────────────

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

// ── TileMapRenderer ───────────────────────────────────────────────────────────

/// Maximum number of tiles rendered per layer per frame.
/// Pre-allocates this many quads worth of vertex/index data.
const MAX_VISIBLE_TILES: usize = 4096;

/// GPU renderer for a [`TileMap`] with per-frame view culling.
///
/// Only tiles inside the camera's view are uploaded each frame —
/// one streaming vertex buffer per layer, one shared static index buffer.
pub struct TileMapRenderer {
    pipeline: Pipeline,
    uniform: UniformBuffer,
    /// Shared static index buffer (pre-generated for MAX_VISIBLE_TILES quads).
    ibuf: IndexBuffer,
    /// Per-layer streaming vertex buffers.
    layers: Vec<StreamLayer>,
    /// World-space side length of one tile (square).
    pub tile_size: f32,
}

struct StreamLayer {
    vbuf: VertexBuffer,
    /// Number of indices to draw this frame (updated in `prepare`).
    draw_count: u32,
}

impl TileMapRenderer {
    /// Create the renderer.
    ///
    /// `tile_size` is the world-space side length of one rendered tile.
    /// Call [`prepare`](Self::prepare) each frame to upload visible tiles.
    pub fn new(ctx: &mut Context, tile_size: f32) -> Self {
        let pipeline = ctx.create_pipeline(
            PipelineDescriptor::new(TILE_SHADER, vert_layout())
                .with_uniform()
                .with_texture()
                .with_alpha_blend(),
        );

        let uniform = ctx.create_uniform_buffer(&crate::math::Mat4::IDENTITY);

        // Static index buffer: quad i → [4i, 4i+1, 4i+2, 4i, 4i+2, 4i+3]
        let indices: Vec<u32> = (0..MAX_VISIBLE_TILES as u32)
            .flat_map(|i| {
                let b = i * 4;
                [b, b + 1, b + 2, b, b + 2, b + 3]
            })
            .collect();
        let ibuf = ctx.create_index_buffer(&indices);

        Self {
            pipeline,
            uniform,
            ibuf,
            layers: Vec::new(),
            tile_size,
        }
    }

    /// Upload view-projection and visible tile geometry for this frame.
    ///
    /// Only tiles whose world-space rect overlaps the camera view are included.
    /// Allocates per-layer streaming buffers on first call or when layer count grows.
    pub fn prepare(
        &mut self,
        ctx: &mut Context,
        map: &TileMap,
        tileset: &TileSet,
        camera: &Camera,
        aspect: f32,
    ) {
        ctx.update_uniform_buffer(&self.uniform, &camera.view_proj(aspect));

        // Ensure we have one streaming buffer per map layer.
        while self.layers.len() < map.layers.len() {
            let dummy = vec![
                TileVert {
                    pos: [0.0; 2],
                    uv: [0.0; 2],
                    tint: [0.0; 4],
                };
                MAX_VISIBLE_TILES * 4
            ];
            self.layers.push(StreamLayer {
                vbuf: ctx.create_vertex_buffer(&dummy),
                draw_count: 0,
            });
        }

        let (col_min, col_max, row_min, row_max) =
            visible_tile_range(camera, aspect, self.tile_size, map.cols, map.rows);

        for (i, map_layer) in map.layers.iter().enumerate() {
            let stream = &mut self.layers[i];

            if !map_layer.visible {
                stream.draw_count = 0;
                continue;
            }

            let mut verts: Vec<TileVert> = Vec::new();
            let ts = self.tile_size;

            'outer: for row in row_min..row_max {
                for col in col_min..col_max {
                    let id = map_layer.get(col, row);
                    let Some(uv) = tileset.uv(id) else { continue };

                    if verts.len() / 4 >= MAX_VISIBLE_TILES {
                        break 'outer;
                    }

                    let x0 = col as f32 * ts;
                    let y0 = -(row as f32 * ts);
                    let x1 = x0 + ts;
                    let y1 = y0 - ts;

                    let [u0, v0, uw, vw] = uv;
                    let t = map_layer.tint;
                    verts.extend_from_slice(&[
                        TileVert {
                            pos: [x0, y0],
                            uv: [u0, v0],
                            tint: t,
                        },
                        TileVert {
                            pos: [x1, y0],
                            uv: [u0 + uw, v0],
                            tint: t,
                        },
                        TileVert {
                            pos: [x1, y1],
                            uv: [u0 + uw, v0 + vw],
                            tint: t,
                        },
                        TileVert {
                            pos: [x0, y1],
                            uv: [u0, v0 + vw],
                            tint: t,
                        },
                    ]);
                }
            }

            stream.draw_count = (verts.len() / 4 * 6) as u32;
            if !verts.is_empty() {
                ctx.update_vertex_buffer(&stream.vbuf, &verts);
            }
        }
    }

    /// Draw all visible layers into the render pass.
    pub fn render(&self, pass: &mut RenderPass<'_>, tileset: &TileSet) {
        for stream in &self.layers {
            if stream.draw_count == 0 {
                continue;
            }
            pass.set_pipeline(&self.pipeline);
            pass.set_uniform(0, &self.uniform);
            pass.set_texture(1, &tileset.texture);
            pass.set_vertex_buffer(0, &stream.vbuf);
            pass.draw_indexed_count(&self.ibuf, stream.draw_count);
        }
    }
}

// ── View culling helpers ───────────────────────────────────────────────────────

/// Compute the inclusive tile range `[col_min, col_max) × [row_min, row_max)`
/// that is visible given the camera and aspect ratio.
///
/// Tiles use `y = -(row * tile_size)` so the tile y range is inverted.
/// One tile of padding is added on each side to avoid pop-in.
fn visible_tile_range(
    camera: &Camera,
    aspect: f32,
    tile_size: f32,
    map_cols: u32,
    map_rows: u32,
) -> (u32, u32, u32, u32) {
    let (x_min, x_max, y_min, y_max) = camera_world_bounds(camera, aspect);
    let ts = tile_size;

    // Add 1-tile margin to avoid edge pop-in.
    let col_min = ((x_min / ts).floor() as i32 - 1).max(0) as u32;
    let col_max = ((x_max / ts).ceil() as i32 + 1).min(map_cols as i32) as u32;

    // Tiles use negated Y: tile row r occupies y ∈ [-(r+1)*ts, -r*ts].
    // Screen y_max (top) → lowest row index.
    let row_min = ((-y_max / ts).floor() as i32 - 1).max(0) as u32;
    let row_max = ((-y_min / ts).ceil() as i32 + 1).min(map_rows as i32) as u32;

    (col_min, col_max, row_min, row_max)
}

/// Extract world-space axis-aligned bounds `(x_min, x_max, y_min, y_max)` from a camera.
fn camera_world_bounds(camera: &Camera, aspect: f32) -> (f32, f32, f32, f32) {
    match camera.projection {
        Projection::Orthographic { width, .. } => {
            let hw = width * 0.5;
            let hh = hw / aspect;
            let cx = camera.position.x;
            let cy = camera.position.y;
            (cx - hw, cx + hw, cy - hh, cy + hh)
        }
        Projection::OrthographicBounds {
            left,
            right,
            bottom,
            top,
            ..
        } => (left, right, bottom, top),
        Projection::Perspective { fov, .. } => {
            // Approximate at camera XY position with a wide safety margin.
            let half_h = (fov * 0.5).tan() * 50.0;
            let half_w = half_h * aspect;
            let cx = camera.position.x;
            let cy = camera.position.y;
            (cx - half_w, cx + half_w, cy - half_h, cy + half_h)
        }
    }
}
