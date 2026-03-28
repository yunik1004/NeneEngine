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
    ai::pathfinding::{TileMapGraph, find_path},
    app::{App, Config, WindowId, run},
    input::{Input, Key, MouseButton},
    renderer::{Context, FilterMode, RenderPass, Texture},
    tilemap::{TileMap, TileMapRenderer, TileSet},
    time::Time,
    ui::Ui,
};

const COLS: u32 = 20;
const ROWS: u32 = 15;
const TILE: f32 = 1.0;
const W: u32 = 800;
const H: u32 = 600;

fn make_camera() -> nene::camera::Camera {
    nene::camera::Camera::orthographic(
        nene::math::Vec3::new(COLS as f32 * TILE * 0.5, -(ROWS as f32 * TILE * 0.5), 1.0),
        COLS as f32 * TILE,
        -10.0,
        10.0,
    )
}

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

    write_tile(&mut pixels, 0, 60, 60, 60, 255);
    write_tile(&mut pixels, 1, 120, 80, 40, 255);
    write_tile(&mut pixels, 2, 255, 255, 255, 200);

    ctx.create_texture_with(S as u32 * 3, S as u32, &pixels, FilterMode::Nearest)
}

fn random_walls(map: &mut TileMap) {
    let mut seed = 0x1234u32;
    let mut next = move || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };

    for r in 0..map.rows {
        for c in 0..map.cols {
            map.set_solid(c, r, false);
            map.set(c, r, 0, 1);
        }
    }
    for r in 1..map.rows - 1 {
        for c in 1..map.cols - 1 {
            if next() % 4 == 0 {
                map.set_solid(c, r, true);
                map.set(c, r, 0, 2);
            }
        }
    }
}

fn rebuild_overlay(
    overlay: &mut TileMap,
    path: &Option<Vec<(u32, u32)>>,
    start: (u32, u32),
    goal: (u32, u32),
) {
    for r in 0..ROWS {
        for c in 0..COLS {
            overlay.set(c, r, 0, 0);
        }
    }
    if let Some(p) = path {
        for &(c, r) in p {
            overlay.set(c, r, 0, 3);
            overlay.layers[0].tint = [0.1, 0.9, 0.1, 0.7];
        }
    }
    overlay.set(start.0, start.1, 1, 3);
    overlay.layers[1].tint = [1.0, 0.9, 0.1, 0.85];
    overlay.set(goal.0, goal.1, 2, 3);
    overlay.layers[2].tint = [1.0, 0.2, 0.2, 0.85];
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

struct PathfindingDemo {
    map: TileMap,
    overlay: TileMap,
    camera: nene::camera::Camera,
    start: (u32, u32),
    goal: (u32, u32),
    diagonal: bool,
    path: Option<Vec<(u32, u32)>>,
    // GPU
    tileset: Option<TileSet>,
    renderer: Option<TileMapRenderer>,
    overlay_renderer: Option<TileMapRenderer>,
    ui: Option<Ui>,
}

impl App for PathfindingDemo {
    fn new() -> Self {
        let mut map = TileMap::new(COLS, ROWS);
        random_walls(&mut map);

        let mut overlay = TileMap::new(COLS, ROWS);
        overlay.add_layer();
        overlay.add_layer();

        let start = (1, 1);
        let goal = (COLS - 2, ROWS - 2);
        let path = find_path(&TileMapGraph::new(&map, false), start, goal);
        rebuild_overlay(&mut overlay, &path, start, goal);

        PathfindingDemo {
            map,
            overlay,
            camera: make_camera(),
            start,
            goal,
            diagonal: false,
            path,
            tileset: None,
            renderer: None,
            overlay_renderer: None,
            ui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        let texture = build_atlas(ctx);
        self.tileset = Some(TileSet::new(texture, 48, 16, 16, 16));
        self.renderer = Some(TileMapRenderer::new(ctx, TILE));
        self.overlay_renderer = Some(TileMapRenderer::new(ctx, TILE));
        self.ui = Some(Ui::new(ctx));
    }

    fn update(&mut self, input: &Input, _time: &Time) {
        let mut changed = false;
        let mp = input.mouse_pos();
        let (mx, my) = (mp.x, mp.y);

        if input.mouse_pressed(MouseButton::Left) {
            if let Some(tile) = screen_to_tile(mx, my) {
                if !self.map.is_solid(tile.0, tile.1) {
                    self.start = tile;
                    changed = true;
                }
            }
        }
        if input.mouse_pressed(MouseButton::Right) {
            if let Some(tile) = screen_to_tile(mx, my) {
                if !self.map.is_solid(tile.0, tile.1) {
                    self.goal = tile;
                    changed = true;
                }
            }
        }
        if input.key_pressed(Key::KeyD) {
            self.diagonal = !self.diagonal;
            changed = true;
        }
        if input.key_pressed(Key::KeyR) {
            random_walls(&mut self.map);
            self.map.set_solid(self.start.0, self.start.1, false);
            self.map.set(self.start.0, self.start.1, 0, 1);
            self.map.set_solid(self.goal.0, self.goal.1, false);
            self.map.set(self.goal.0, self.goal.1, 0, 1);
            changed = true;
        }

        if changed {
            self.path = find_path(
                &TileMapGraph::new(&self.map, self.diagonal),
                self.start,
                self.goal,
            );
            rebuild_overlay(&mut self.overlay, &self.path, self.start, self.goal);
        }
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, input: &Input) {
        let aspect = W as f32 / H as f32;
        let (Some(renderer), Some(overlay_renderer), Some(tileset)) = (
            &mut self.renderer,
            &mut self.overlay_renderer,
            &self.tileset,
        ) else {
            return;
        };

        renderer.prepare(ctx, &self.map, tileset, &self.camera, aspect);
        overlay_renderer.prepare(ctx, &self.overlay, tileset, &self.camera, aspect);

        let Some(ui) = &mut self.ui else { return };
        ui.begin_frame(input, W as f32, H as f32);
        ui.begin_panel("Info", 10.0, 10.0, 200.0);
        ui.label("Pathfinding");
        ui.separator();
        ui.label_dim(&format!("start  ({}, {})", self.start.0, self.start.1));
        ui.label_dim(&format!("goal   ({}, {})", self.goal.0, self.goal.1));
        ui.separator();
        let mode = if self.diagonal { "8-dir" } else { "4-dir" };
        ui.label_dim(&format!("mode   {mode}  [D]"));
        let steps = self.path.as_ref().map_or(0, |p| p.len());
        if self.path.is_some() {
            ui.label_dim(&format!("path   {steps} tiles"));
        } else {
            ui.label_dim("path   none");
        }
        ui.separator();
        ui.label_dim("LMB  set start");
        ui.label_dim("RMB  set goal");
        ui.label_dim("R    new walls");
        ui.end_panel();
        ui.end_frame(ctx);
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        let (Some(renderer), Some(overlay_renderer), Some(tileset)) =
            (&self.renderer, &self.overlay_renderer, &self.tileset)
        else {
            return;
        };
        renderer.render(pass, tileset);
        overlay_renderer.render(pass, tileset);
        if let Some(ui) = &self.ui {
            ui.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Pathfinding demo",
            width: W,
            height: H,
            ..Config::default()
        }]
    }
}

fn main() {
    run::<PathfindingDemo>();
}
