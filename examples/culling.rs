//! Frustum culling demo.
//!
//! Spawns 2 000 sprites scattered over a large world.
//! Each frame, only sprites whose AABB intersects the camera frustum are
//! submitted to the sprite batch — the rest are skipped entirely.
//!
//! Controls
//! --------
//! WASD / arrows  — pan camera
//! + / -          — zoom in / out
//!
//! The right panel shows how many sprites were visible vs. total.

use nene::{
    camera::Camera,
    culling::Frustum,
    input::{Input, Key},
    math::{Vec2, Vec3},
    renderer::{Context, FilterMode, RenderPass, Texture},
    sprite::{Sprite, SpriteBatch, UvRect},
    ui::Ui,
    window::{Config, Window},
};

// ── Atlas (four solid-colour tiles 16×16 in a 64×16 strip) ───────────────────

const ATLAS_W: u32 = 64;
const ATLAS_H: u32 = 16;
const TILE: u32 = 16;

fn make_texture(ctx: &mut Context) -> Texture {
    let colors: [[u8; 3]; 4] = [
        [220, 80, 80],  // red
        [80, 200, 80],  // green
        [80, 120, 220], // blue
        [220, 180, 60], // yellow
    ];
    let mut px = vec![0u8; (ATLAS_W * ATLAS_H * 4) as usize];
    for tile in 0..4u32 {
        let bx = tile * TILE;
        let [r, g, b] = colors[tile as usize];
        for py in 0..ATLAS_H {
            for tx in 0..TILE {
                let edge = tx == 0 || tx == TILE - 1 || py == 0 || py == ATLAS_H - 1;
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

// UV rects for the four tiles
fn tile_uv(i: usize) -> UvRect {
    let uw = TILE as f32 / ATLAS_W as f32;
    UvRect {
        x: uw * i as f32,
        y: 0.0,
        w: uw,
        h: 1.0,
    }
}

// ── World objects ─────────────────────────────────────────────────────────────

const OBJECT_COUNT: usize = 2000;
const WORLD_SIZE: f32 = 200.0;
const SPRITE_SIZE: f32 = 1.5;

struct Object {
    pos: Vec2,
    uv: UvRect,
}

fn spawn_objects() -> Vec<Object> {
    // Deterministic LCG so we get the same layout every run.
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

const W: u32 = 960;
const H: u32 = 600;

struct State {
    batch: SpriteBatch,
    texture: Texture,
    objects: Vec<Object>,
    camera: Camera,
    ortho_width: f32,
    ui: Ui,
    visible_count: usize,
}

fn init(ctx: &mut Context) -> State {
    State {
        batch: SpriteBatch::new(ctx, OBJECT_COUNT),
        texture: make_texture(ctx),
        objects: spawn_objects(),
        camera: Camera::orthographic(Vec3::new(0.0, 0.0, 1.0), 40.0, 0.1, 100.0),
        ortho_width: 40.0,
        ui: Ui::new(ctx),
        visible_count: 0,
    }
}

fn main() {
    Window::new(Config {
        title: "Frustum culling demo  (WASD = pan, +/- = zoom)".into(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        // ── update ──────────────────────────────────────────────────────────
        |state, ctx, input, time| {
            pan_zoom(&mut state.camera, &mut state.ortho_width, input, time.delta);

            let aspect = W as f32 / H as f32;
            let vp = state.camera.view_proj(aspect);
            let frustum = Frustum::from_view_proj(vp);

            // Submit only objects whose AABB overlaps the frustum.
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
            state.batch.prepare(ctx, &state.camera, aspect);

            // UI
            state.ui.begin_frame(input, W as f32, H as f32);
            state.ui.begin_panel("Culling stats", 16.0, 16.0, 200.0);
            state.ui.label("Frustum culling");
            state.ui.separator();
            state
                .ui
                .label_dim(&format!("visible  {}", state.visible_count));
            state
                .ui
                .label_dim(&format!("culled   {}", OBJECT_COUNT - state.visible_count));
            state.ui.label_dim(&format!("total    {}", OBJECT_COUNT));
            state.ui.separator();
            let pct = state.visible_count as f32 / OBJECT_COUNT as f32 * 100.0;
            state.ui.label_dim(&format!("draw%    {:.1}", pct));
            state.ui.end_panel();
            state.ui.end_frame(ctx);
        },
        |_, _| {},
        // ── render ──────────────────────────────────────────────────────────
        |state, pass: &mut RenderPass| {
            state.batch.render(pass, &state.texture);
            state.ui.render(pass);
        },
    );
}

fn pan_zoom(camera: &mut Camera, width: &mut f32, input: &Input, dt: f32) {
    let speed = *width * dt * 1.2;
    let mut d = Vec3::ZERO;
    if input.key_down(Key::KeyW) || input.key_down(Key::ArrowUp) {
        d.y += speed;
    }
    if input.key_down(Key::KeyS) || input.key_down(Key::ArrowDown) {
        d.y -= speed;
    }
    if input.key_down(Key::KeyA) || input.key_down(Key::ArrowLeft) {
        d.x -= speed;
    }
    if input.key_down(Key::KeyD) || input.key_down(Key::ArrowRight) {
        d.x += speed;
    }
    camera.position += d;
    camera.target = camera.position + Vec3::NEG_Z;

    if input.key_down(Key::Equal) {
        *width *= 1.0 - dt * 2.0;
        *width = width.max(5.0);
    }
    if input.key_down(Key::Minus) {
        *width *= 1.0 + dt * 2.0;
        *width = width.min(200.0);
    }
    camera.projection = nene::camera::Projection::Orthographic {
        width: *width,
        near: 0.1,
        far: 100.0,
    };
}
