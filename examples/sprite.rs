/// Demonstrates SpriteBatch: multiple sprites from one texture atlas,
/// movement with WASD, and rotation with Q/E.
use nene::{
    camera::Camera,
    input::{Key, MouseButton},
    math::Vec2,
    renderer::{Context, RenderPass},
    sprite::{Sprite, SpriteBatch, UvRect},
    texture::{FilterMode, Texture},
    window::{Config, Window},
};

struct State {
    batch: SpriteBatch,
    texture: Texture,
    camera: Camera,
    player_pos: Vec2,
    angle: f32,
}

/// Build a simple 2×2 tile checkerboard texture (64×64, 32px tiles).
fn make_texture(ctx: &mut Context) -> Texture {
    let size = 64u32;
    let tile = 32u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let checker = ((x / tile) + (y / tile)) % 2 == 0;
            let (r, g, b) = if checker {
                (220u8, 180, 255)
            } else {
                (80u8, 40, 160)
            };
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    ctx.create_texture_with(size, size, &data, FilterMode::Nearest)
}

fn init(ctx: &mut Context) -> State {
    let batch = SpriteBatch::new(ctx, 512);
    let texture = make_texture(ctx);
    let camera = Camera::orthographic_bounds(-8.0, 8.0, -4.5, 4.5, -1.0, 1.0);

    State {
        batch,
        texture,
        camera,
        player_pos: Vec2::ZERO,
        angle: 0.0,
    }
}

fn main() {
    Window::new(Config {
        title: "Sprite Demo".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            let speed = 4.0 * time.delta;

            // Move player with WASD
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
                state.player_pos += dir.normalize() * speed;
            }

            // Rotate with Q/E
            if input.key_down(Key::KeyQ) {
                state.angle += 2.0 * time.delta;
            }
            if input.key_down(Key::KeyE) {
                state.angle -= 2.0 * time.delta;
            }

            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;

            state.batch.clear();

            // Background grid of static sprites (left half of texture atlas)
            for row in -3i32..=3 {
                for col in -5i32..=5 {
                    state.batch.draw(&Sprite {
                        position: Vec2::new(col as f32 * 1.5, row as f32 * 1.5),
                        size: Vec2::splat(1.2),
                        uv: UvRect {
                            x: 0.0,
                            y: 0.0,
                            w: 0.5,
                            h: 0.5,
                        },
                        color: [0.6, 0.6, 0.6, 1.0],
                        ..Sprite::default()
                    });
                }
            }

            // Player sprite (right half of atlas), tinted red when LMB held
            let tint = if input.mouse_down(MouseButton::Left) {
                [1.0, 0.3, 0.3, 1.0]
            } else {
                [1.0, 1.0, 1.0, 1.0]
            };
            state.batch.draw(&Sprite {
                position: state.player_pos,
                size: Vec2::splat(1.0),
                rotation: state.angle,
                uv: UvRect {
                    x: 0.5,
                    y: 0.0,
                    w: 0.5,
                    h: 0.5,
                },
                color: tint,
            });

            state.batch.prepare(ctx, &state.camera, aspect);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            state.batch.render(pass, &state.texture);
        },
    );
}
