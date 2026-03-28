use bytemuck::{Pod, Zeroable};

use crate::{
    camera::{Camera, Projection},
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer,
        VertexAttribute, VertexBuffer, VertexFormat, VertexLayout,
    },
};

use super::map::{TileMap, TileSet};

/// Maximum number of tiles rendered per layer per frame.
/// Pre-allocates this many quads worth of vertex/index data.
pub const MAX_VISIBLE_TILES: usize = 4096;

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

        let indices = crate::renderer::quad_indices(MAX_VISIBLE_TILES as u32);
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
