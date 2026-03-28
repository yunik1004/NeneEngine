//! Sprite and frustum culling demo.
//!
//! 2 000 coloured sprites are scattered across a 200×200 world.
//! Each frame only those inside the camera frustum are submitted to the GPU —
//! visible in the "Culling" stats panel.
//!
//! Controls
//! --------
//! WASD / arrows — move player (white square); camera follows
//! Q / E         — rotate player
//! + / -         — zoom in / out
//! LMB (held)    — tint player red

use nene::{
    camera::Camera,
    camera::Frustum,
    input::{Input, Key, MouseButton},
    math::{Vec2, Vec3},
    renderer::{Context, FilterMode, RenderPass, Texture},
    sprite::{Sprite, SpriteBatch, UvRect},
    ui::Ui,
    window::{Config, Window},
};

const W: u32 = 960;
const H: u32 = 600;
const OBJECT_COUNT: usize = 2000;
const WORLD_SIZE: f32 = 200.0;
const SPRITE_SIZE: f32 = 1.5;

// ── 5-tile atlas: 4 world colours + 1 white player tile ──────────────────────

const ATLAS_W: u32 = 80;
const ATLAS_H: u32 = 16;
const TILE_PX: u32 = 16;

fn make_texture(ctx: &mut Context) -> Texture {
    let colors: [[u8; 3]; 5] = [
        [220, 80, 80],   // red
        [80, 200, 80],   // green
        [80, 120, 220],  // blue
        [220, 180, 60],  // yellow
        [240, 240, 240], // white (player)
    ];
    let mut px = vec![0u8; (ATLAS_W * ATLAS_H * 4) as usize];
    for tile in 0..5u32 {
        let bx = tile * TILE_PX;
        let [r, g, b] = colors[tile as usize];
        for py in 0..ATLAS_H {
            for tx in 0..TILE_PX {
                let edge = tx == 0 || tx == TILE_PX - 1 || py == 0 || py == ATLAS_H - 1;
                let f = if edge { 0.6f32 } else { 1.0 };
                let i = ((py * ATLAS_W + bx + tx) * 4) as usize;
                px[i] = (r as f32 * f) as u8;
                px[i + 1] = (g as f32 * f) as u8;
                px[i + 2] = (b as f32 * f) as u8;
                px[i + 3] = 255;
            }
        }
    }
    ctx.create_texture_with(ATLAS_W, ATLAS_H, &px, FilterMode::Nearest)
}

fn tile_uv(i: usize) -> UvRect {
    let uw = TILE_PX as f32 / ATLAS_W as f32;
    UvRect {
        x: uw * i as f32,
        y: 0.0,
        w: uw,
        h: 1.0,
    }
}

// ── World objects ─────────────────────────────────────────────────────────────

struct Object {
    pos: Vec2,
    uv: UvRect,
}

fn spawn_objects() -> Vec<Object> {
    let mut rng: u64 = 0xDEAD_BEEF_1234_5678;
    let mut rand = || -> f32 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        (rng as f32) / (u64::MAX as f32)
    };
    (0..OBJECT_COUNT)
        .map(|i| Object {
            pos: Vec2::new((rand() - 0.5) * WORLD_SIZE, (rand() - 0.5) * WORLD_SIZE),
            uv: tile_uv(i % 4),
        })
        .collect()
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    batch: SpriteBatch,
    texture: Texture,
    objects: Vec<Object>,
    player_pos: Vec2,
    player_angle: f32,
    camera: Camera,
    ortho_width: f32,
    ui: Ui,
    visible_count: usize,
}

fn init(ctx: &mut Context) -> State {
    State {
        batch: SpriteBatch::new(ctx, OBJECT_COUNT + 1),
        texture: make_texture(ctx),
        objects: spawn_objects(),
        player_pos: Vec2::ZERO,
        player_angle: 0.0,
        camera: Camera::orthographic(Vec3::new(0.0, 0.0, 1.0), 40.0, 0.1, 100.0),
        ortho_width: 40.0,
        ui: Ui::new(ctx),
        visible_count: 0,
    }
}

fn main() {
    Window::new(Config {
        title: "Sprite + Culling  (WASD=move  Q/E=rotate  +/-=zoom  LMB=tint)",
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            let dt = time.delta;

            // ── player movement ────────────────────────────────────────────────
            let mut dir = Vec2::ZERO;
            if input.key_down(Key::KeyW) || input.key_down(Key::ArrowUp) {
                dir.y += 1.0;
            }
            if input.key_down(Key::KeyS) || input.key_down(Key::ArrowDown) {
                dir.y -= 1.0;
            }
            if input.key_down(Key::KeyA) || input.key_down(Key::ArrowLeft) {
                dir.x -= 1.0;
            }
            if input.key_down(Key::KeyD) || input.key_down(Key::ArrowRight) {
                dir.x += 1.0;
            }
            if dir != Vec2::ZERO {
                state.player_pos += dir.normalize() * 8.0 * dt;
            }
            if input.key_down(Key::KeyQ) {
                state.player_angle += 2.0 * dt;
            }
            if input.key_down(Key::KeyE) {
                state.player_angle -= 2.0 * dt;
            }

            // ── zoom + camera follows player ───────────────────────────────────
            apply_zoom(&mut state.ortho_width, input, dt);
            let px = state.player_pos.x;
            let py = state.player_pos.y;
            state.camera.position = Vec3::new(px, py, 1.0);
            state.camera.target = Vec3::new(px, py, 0.0);
            state.camera.projection = nene::camera::Projection::Orthographic {
                width: state.ortho_width,
                near: 0.1,
                far: 100.0,
            };

            // ── frustum culling ────────────────────────────────────────────────
            let aspect = W as f32 / H as f32;
            let vp = state.camera.view_proj(aspect);
            let frustum = Frustum::from_view_proj(vp);

            state.batch.clear();
            let hs = SPRITE_SIZE * 0.5;
            let mut visible = 0usize;
            for obj in &state.objects {
                let min = Vec3::new(obj.pos.x - hs, obj.pos.y - hs, -0.1);
                let max = Vec3::new(obj.pos.x + hs, obj.pos.y + hs, 0.1);
                if frustum.test_aabb(min, max) {
                    state.batch.queue(&Sprite {
                        position: obj.pos,
                        size: Vec2::splat(SPRITE_SIZE),
                        uv: obj.uv,
                        ..Sprite::default()
                    });
                    visible += 1;
                }
            }
            state.visible_count = visible;

            // Player sprite (white tile, tinted red on LMB)
            let tint = if input.mouse_down(MouseButton::Left) {
                [1.0, 0.3, 0.3, 1.0]
            } else {
                [1.0, 1.0, 1.0, 1.0]
            };
            state.batch.queue(&Sprite {
                position: state.player_pos,
                size: Vec2::splat(1.0),
                rotation: state.player_angle,
                uv: tile_uv(4),
                color: tint,
            });

            state.batch.prepare(ctx, &state.camera, aspect);

            // ── UI ─────────────────────────────────────────────────────────────
            state.ui.begin_frame(input, W as f32, H as f32);
            state.ui.begin_panel("Culling", 16.0, 16.0, 180.0);
            state
                .ui
                .label_dim(&format!("visible  {}", state.visible_count));
            state
                .ui
                .label_dim(&format!("culled   {}", OBJECT_COUNT - state.visible_count));
            state.ui.label_dim(&format!("total    {OBJECT_COUNT}"));
            let pct = state.visible_count as f32 / OBJECT_COUNT as f32 * 100.0;
            state.ui.label_dim(&format!("draw%    {:.1}", pct));
            state.ui.end_panel();
            state.ui.end_frame(ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            state.batch.render(pass, &state.texture);
            state.ui.render(pass);
        },
    );
}

fn apply_zoom(width: &mut f32, input: &Input, dt: f32) {
    if input.key_down(Key::Equal) {
        *width = (*width * (1.0 - dt * 2.0)).max(5.0);
    }
    if input.key_down(Key::Minus) {
        *width = (*width * (1.0 + dt * 2.0)).min(200.0);
    }
}
