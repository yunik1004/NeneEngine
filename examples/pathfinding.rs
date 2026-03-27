//! Pathfinding demo.
//!
//! Displays a tile map with walls. Left-click sets the start tile,
//! right-click sets the goal tile. The A* path is highlighted in green.
//!
//! Controls
//! --------
//! Left-click   — set start (yellow)
//! Right-click  — set goal  (red)
//! D            — toggle diagonal movement
//! R            — randomise walls

use nene::{
    input::{Input, Key, MouseButton},
    pathfinding::find_path,
    renderer::{Context, FilterMode, RenderPass, Texture},
    tilemap::{TileMap, TileMapRenderer, TileSet},
    ui::Ui,
    window::{Config, Window},
};

const COLS: u32 = 20;
const ROWS: u32 = 15;
const TILE: f32 = 1.0; // world units per tile
const W: u32 = 800;
const H: u32 = 600;

// Camera fits exactly COLS tiles wide.
fn make_camera() -> nene::camera::Camera {
    nene::camera::Camera::orthographic(
        nene::math::Vec3::new(COLS as f32 * TILE * 0.5, -(ROWS as f32 * TILE * 0.5), 1.0),
        COLS as f32 * TILE,
        -10.0,
        10.0,
    )
}

// ── Tileset ───────────────────────────────────────────────────────────────────

/// Build a tiny 3×1 atlas (floor / wall / overlay) at runtime.
/// Each tile is 16×16 px.
///  tile 1 = floor   (dark grey)
///  tile 2 = wall    (brown)
///  tile 3 = overlay (semi-transparent; tinted per use)
fn build_atlas(ctx: &mut Context) -> Texture {
    const S: usize = 16;
    let mut pixels = vec![0u8; S * 3 * S * 4];

    let write_tile = |pixels: &mut Vec<u8>, tx: usize, r: u8, g: u8, b: u8, a: u8| {
        for py in 0..S {
            for px in 0..S {
                let i = ((py * S * 3) + (tx * S + px)) * 4;
                pixels[i] = r;
                pixels[i + 1] = g;
                pixels[i + 2] = b;
                pixels[i + 3] = a;
            }
        }
    };

    write_tile(&mut pixels, 0, 60, 60, 60, 255); // floor
    write_tile(&mut pixels, 1, 120, 80, 40, 255); // wall
    write_tile(&mut pixels, 2, 255, 255, 255, 200); // overlay (tinted)

    ctx.create_texture_with(S as u32 * 3, S as u32, &pixels, FilterMode::Nearest)
}

// ── State ─────────────────────────────────────────────────────────────────────

struct State {
    map: TileMap,
    tileset: TileSet,
    renderer: TileMapRenderer,
    overlay_renderer: TileMapRenderer,
    overlay: TileMap,
    camera: nene::camera::Camera,
    ui: Ui,
    start: (u32, u32),
    goal: (u32, u32),
    diagonal: bool,
    path: Option<Vec<(u32, u32)>>,
}

fn random_walls(map: &mut TileMap) {
    // Simple deterministic "random" using a cheap LCG.
    let mut seed = 0x1234u32;
    let mut next = move || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };

    for r in 0..map.rows {
        for c in 0..map.cols {
            map.set_solid(c, r, false);
            map.set(c, r, 0, 1); // floor
        }
    }
    // ~25% wall density, keep border clear
    for r in 1..map.rows - 1 {
        for c in 1..map.cols - 1 {
            if next() % 4 == 0 {
                map.set_solid(c, r, true);
                map.set(c, r, 0, 2); // wall tile
            }
        }
    }
}

fn rebuild_overlay(state: &mut State) {
    // Clear overlay
    for r in 0..ROWS {
        for c in 0..COLS {
            state.overlay.set(c, r, 0, 0);
        }
    }

    // Path (green)
    if let Some(ref path) = state.path {
        for &(c, r) in path {
            state.overlay.set(c, r, 0, 3);
            state.overlay.layers[0].tint = [0.1, 0.9, 0.1, 0.7];
        }
    }

    // Start (yellow overlay)
    state.overlay.set(state.start.0, state.start.1, 1, 3);
    state.overlay.layers[1].tint = [1.0, 0.9, 0.1, 0.85];

    // Goal (red overlay)
    state.overlay.set(state.goal.0, state.goal.1, 2, 3);
    state.overlay.layers[2].tint = [1.0, 0.2, 0.2, 0.85];
}

fn screen_to_tile(sx: f32, sy: f32) -> Option<(u32, u32)> {
    let wx = sx / W as f32 * COLS as f32 * TILE;
    let wy = -(sy / H as f32 * ROWS as f32 * TILE);
    let col = (wx / TILE).floor() as i32;
    let row = ((-wy) / TILE).floor() as i32;
    if col < 0 || row < 0 || col >= COLS as i32 || row >= ROWS as i32 {
        return None;
    }
    Some((col as u32, row as u32))
}

fn init(ctx: &mut Context) -> State {
    let texture = build_atlas(ctx);
    let tileset = TileSet::new(texture, 48, 16, 16, 16);

    let mut map = TileMap::new(COLS, ROWS);
    random_walls(&mut map);

    let mut overlay = TileMap::new(COLS, ROWS);
    // 3 overlay layers: path / start / goal
    overlay.add_layer();
    overlay.add_layer();

    let renderer = TileMapRenderer::new(ctx, TILE);
    let overlay_renderer = TileMapRenderer::new(ctx, TILE);

    let start = (1, 1);
    let goal = (COLS - 2, ROWS - 2);

    let path = find_path(&map, start, goal, false);

    let mut state = State {
        map,
        tileset,
        renderer,
        overlay_renderer,
        overlay,
        camera: make_camera(),
        ui: Ui::new(ctx),
        start,
        goal,
        diagonal: false,
        path,
    };
    rebuild_overlay(&mut state);
    state
}

fn main() {
    Window::new(Config {
        title: "Pathfinding demo".into(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, _time| {
            handle_input(state, input);

            let aspect = W as f32 / H as f32;
            state
                .renderer
                .prepare(ctx, &state.map, &state.tileset, &state.camera, aspect);
            state.overlay_renderer.prepare(
                ctx,
                &state.overlay,
                &state.tileset,
                &state.camera,
                aspect,
            );

            // UI panel
            state.ui.begin_frame(input, W as f32, H as f32);
            state.ui.begin_panel("Info", 10.0, 10.0, 200.0);
            state.ui.label("Pathfinding");
            state.ui.separator();
            state
                .ui
                .label_dim(&format!("start  ({}, {})", state.start.0, state.start.1));
            state
                .ui
                .label_dim(&format!("goal   ({}, {})", state.goal.0, state.goal.1));
            state.ui.separator();
            let mode = if state.diagonal { "8-dir" } else { "4-dir" };
            state.ui.label_dim(&format!("mode   {mode}  [D]"));
            let steps = state.path.as_ref().map_or(0, |p| p.len());
            if state.path.is_some() {
                state.ui.label_dim(&format!("path   {steps} tiles"));
            } else {
                state.ui.label_dim("path   none");
            }
            state.ui.separator();
            state.ui.label_dim("LMB  set start");
            state.ui.label_dim("RMB  set goal");
            state.ui.label_dim("R    new walls");
            state.ui.end_panel();
            state.ui.end_frame(ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            state.renderer.render(pass, &state.tileset);
            state.overlay_renderer.render(pass, &state.tileset);
            state.ui.render(pass);
        },
    );
}

fn handle_input(state: &mut State, input: &Input) {
    let mut changed = false;

    let mp = input.mouse_pos();
    let (mx, my) = (mp.x, mp.y);

    if input.mouse_pressed(MouseButton::Left) {
        if let Some(tile) = screen_to_tile(mx, my) {
            if !state.map.is_solid(tile.0, tile.1) {
                state.start = tile;
                changed = true;
            }
        }
    }
    if input.mouse_pressed(MouseButton::Right) {
        if let Some(tile) = screen_to_tile(mx, my) {
            if !state.map.is_solid(tile.0, tile.1) {
                state.goal = tile;
                changed = true;
            }
        }
    }
    if input.key_pressed(Key::KeyD) {
        state.diagonal = !state.diagonal;
        changed = true;
    }
    if input.key_pressed(Key::KeyR) {
        random_walls(&mut state.map);
        // Make sure start/goal are not on walls
        state.map.set_solid(state.start.0, state.start.1, false);
        state.map.set(state.start.0, state.start.1, 0, 1);
        state.map.set_solid(state.goal.0, state.goal.1, false);
        state.map.set(state.goal.0, state.goal.1, 0, 1);
        changed = true;
    }

    if changed {
        state.path = find_path(&state.map, state.start, state.goal, state.diagonal);
        rebuild_overlay(state);
    }
}
