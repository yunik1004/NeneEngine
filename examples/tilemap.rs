//! Tile map demo.
//!
//! Generates a tiny procedural atlas (4 colours), builds a two-layer map,
//! marks border tiles solid, and lets you pan with WASD / arrow keys.
//! Zoom with + and -.
//!
//! Layer 0 — floor tiles
//! Layer 1 — walls and crates on top

use nene::{
    camera::Camera,
    input::{Input, Key},
    math::Vec3,
    renderer::{Context, FilterMode, RenderPass},
    tilemap::{TileMap, TileMapRenderer, TileSet},
    window::{Config, Window},
};

// ── Atlas ─────────────────────────────────────────────────────────────────────
// 4×1 atlas, 16×16 px each → 64×16 total.
// Tile 1 = stone floor  (grey)
// Tile 2 = grass floor  (green)
// Tile 3 = wall         (dark brown)
// Tile 4 = crate        (amber)

const ATLAS_W: u32 = 64;
const ATLAS_H: u32 = 16;
const TILE_PX: u32 = 16;

fn make_atlas(ctx: &mut Context) -> nene::renderer::Texture {
    let mut pixels = vec![0u8; (ATLAS_W * ATLAS_H * 4) as usize];

    let palette: [[u8; 4]; 4] = [
        [140, 140, 145, 255], // stone
        [80, 155, 60, 255],   // grass
        [70, 45, 30, 255],    // wall
        [200, 155, 50, 255],  // crate
    ];

    for tile in 0..4u32 {
        let base_x = tile * TILE_PX;
        for py in 0..TILE_PX {
            for px in 0..TILE_PX {
                let edge = px == 0 || px == TILE_PX - 1 || py == 0 || py == TILE_PX - 1;
                let factor = if edge { 0.65f32 } else { 1.0 };
                let col = palette[tile as usize];
                let idx = ((py * ATLAS_W + base_x + px) * 4) as usize;
                pixels[idx] = (col[0] as f32 * factor) as u8;
                pixels[idx + 1] = (col[1] as f32 * factor) as u8;
                pixels[idx + 2] = (col[2] as f32 * factor) as u8;
                pixels[idx + 3] = col[3];
            }
        }
    }

    ctx.create_texture_with(ATLAS_W, ATLAS_H, &pixels, FilterMode::Nearest)
}

// ── Map ───────────────────────────────────────────────────────────────────────

const COLS: u32 = 24;
const ROWS: u32 = 18;
const TILE_WORLD: f32 = 1.0;

fn build_map() -> TileMap {
    let mut map = TileMap::new(COLS, ROWS);
    let wall_layer = map.add_layer();

    for row in 0..ROWS {
        for col in 0..COLS {
            let border = col == 0 || col == COLS - 1 || row == 0 || row == ROWS - 1;
            if border {
                map.set(col, row, 0, 1);
                map.set(col, row, wall_layer, 3);
                map.set_solid(col, row, true);
            } else if (col * 3 + row * 7) % 13 == 0 {
                // Scattered crates
                map.set(col, row, 0, 2);
                map.set(col, row, wall_layer, 4);
                map.set_solid(col, row, true);
            } else if (col + row) % 3 == 0 {
                map.set(col, row, 0, 2); // grass
            } else {
                map.set(col, row, 0, 1); // stone
            }
        }
    }
    map
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    tileset: TileSet,
    renderer: TileMapRenderer,
    camera: Camera,
    ortho_width: f32,
}

const W: u32 = 960;
const H: u32 = 600;

fn init(ctx: &mut Context) -> State {
    let texture = make_atlas(ctx);
    let tileset = TileSet::new(texture, ATLAS_W, ATLAS_H, TILE_PX, TILE_PX);
    let map = build_map();
    let renderer = TileMapRenderer::new(ctx, &tileset, &map, TILE_WORLD);

    let ortho_width = 20.0_f32;
    let cx = COLS as f32 * TILE_WORLD * 0.5;
    // Tile geometry uses -Y (row 0 = top), so centre is at -ROWS/2.
    let cy = -(ROWS as f32 * TILE_WORLD * 0.5);
    let camera = Camera::orthographic(Vec3::new(cx, cy, 1.0), ortho_width, 0.1, 100.0);

    State {
        tileset,
        renderer,
        camera,
        ortho_width,
    }
}

fn main() {
    Window::new(Config {
        title: "Tile map demo  (WASD/arrows = pan, +/- = zoom)".to_string(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            pan_zoom(&mut state.camera, &mut state.ortho_width, input, time.delta);
            let aspect = W as f32 / H as f32;
            state.renderer.prepare(ctx, &state.camera, aspect);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            state.renderer.render(pass, &state.tileset);
        },
    );
}

fn pan_zoom(camera: &mut Camera, width: &mut f32, input: &Input, dt: f32) {
    let speed = *width * dt * 1.2;

    let mut dx = 0.0f32;
    let mut dy = 0.0f32;
    if input.key_down(Key::KeyW) || input.key_down(Key::ArrowUp) {
        dy += speed;
    }
    if input.key_down(Key::KeyS) || input.key_down(Key::ArrowDown) {
        dy -= speed;
    }
    if input.key_down(Key::KeyA) || input.key_down(Key::ArrowLeft) {
        dx -= speed;
    }
    if input.key_down(Key::KeyD) || input.key_down(Key::ArrowRight) {
        dx += speed;
    }

    camera.position += Vec3::new(dx, dy, 0.0);
    camera.target += Vec3::new(dx, dy, 0.0);

    if input.key_down(Key::Equal) {
        *width *= 1.0 - dt * 2.0;
        *width = width.max(2.0);
    }
    if input.key_down(Key::Minus) {
        *width *= 1.0 + dt * 2.0;
        *width = width.min(50.0);
    }

    camera.projection = nene::camera::Projection::Orthographic {
        width: *width,
        near: 0.1,
        far: 100.0,
    };
}
