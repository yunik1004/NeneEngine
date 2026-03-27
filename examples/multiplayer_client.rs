//! Multiplayer demo — game client.
//!
//! Run `multiplayer_server` first, then open two terminals and run:
//!
//!   cargo run --example multiplayer_client
//!
//! Controls
//! --------
//! WASD / Arrow keys — move your player (white square)
//! Other players appear as colored squares.

use nene::{
    camera::Camera,
    debug::DebugDraw,
    input::Key,
    math::Vec3,
    net::{Client, ClientEvent},
    renderer::{Context, RenderPass},
    ui::Ui,
    window::{Config, Window},
};
use std::collections::HashMap;

const W: u32 = 640;
const H: u32 = 480;
const SPEED: f32 = 5.0;
const PLAYER_SIZE: f32 = 0.5;

// Colors assigned to remote players by their ID
const COLORS: &[Vec3] = &[
    Vec3::new(1.0, 0.3, 0.3), // red
    Vec3::new(0.3, 1.0, 0.3), // green
    Vec3::new(0.3, 0.3, 1.0), // blue
    Vec3::new(1.0, 1.0, 0.3), // yellow
    Vec3::new(1.0, 0.3, 1.0), // magenta
    Vec3::new(0.3, 1.0, 1.0), // cyan
];

#[nene::data]
#[derive(Debug)]
struct PlayerState {
    id: u32,
    x: f32,
    y: f32,
}

struct RemotePlayer {
    /// Latest received position (network target)
    tx: f32,
    ty: f32,
    /// Smoothly interpolated render position
    rx: f32,
    ry: f32,
}

struct State {
    client: Client,
    debug: DebugDraw,
    ui: Ui,
    camera: Camera,
    /// Our position
    x: f32,
    y: f32,
    /// Other players: id → RemotePlayer
    others: HashMap<u32, RemotePlayer>,
    send_timer: f32,
    status: String,
}

fn init(ctx: &mut Context) -> State {
    let client = Client::connect("127.0.0.1:7777").expect("connect failed");

    State {
        client,
        debug: DebugDraw::new(ctx),
        ui: Ui::new(ctx),
        camera: Camera::orthographic(Vec3::new(0.0, 0.0, 10.0), 20.0, 0.1, 100.0),
        x: 0.0,
        y: 0.0,
        others: HashMap::new(),
        send_timer: 0.0,
        status: "Connecting…".into(),
    }
}

fn main() {
    Window::new(Config {
        title: "Multiplayer demo".into(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            let dt = time.delta;

            // ── network events ────────────────────────────────────────────────
            for ev in state.client.poll() {
                match &ev {
                    ClientEvent::Connected => {
                        state.status = "Connected! Use WASD to move.".into();
                    }
                    ClientEvent::Disconnected => {
                        state.status = "Disconnected.".into();
                    }
                    ClientEvent::Message(_) => {
                        if let Some(p) = ev.into_json::<PlayerState>() {
                            if p.x.is_nan() || p.y.is_nan() {
                                state.others.remove(&p.id);
                            } else {
                                state
                                    .others
                                    .entry(p.id)
                                    .and_modify(|r| {
                                        r.tx = p.x;
                                        r.ty = p.y;
                                    })
                                    .or_insert(RemotePlayer {
                                        tx: p.x,
                                        ty: p.y,
                                        rx: p.x,
                                        ry: p.y,
                                    });
                            }
                        }
                    }
                }
            }

            // ── movement ──────────────────────────────────────────────────────
            if state.client.is_connected() {
                let mut dx = 0.0f32;
                let mut dy = 0.0f32;
                if input.key_down(Key::KeyW) || input.key_down(Key::ArrowUp) {
                    dy += 1.0;
                }
                if input.key_down(Key::KeyS) || input.key_down(Key::ArrowDown) {
                    dy -= 1.0;
                }
                if input.key_down(Key::KeyA) || input.key_down(Key::ArrowLeft) {
                    dx -= 1.0;
                }
                if input.key_down(Key::KeyD) || input.key_down(Key::ArrowRight) {
                    dx += 1.0;
                }
                let len = (dx * dx + dy * dy).sqrt().max(1.0);
                state.x += dx / len * SPEED * dt;
                state.y += dy / len * SPEED * dt;
            }

            // ── send position ~20 times/sec ───────────────────────────────────
            state.send_timer += dt;
            if state.send_timer >= 0.05 && state.client.is_connected() {
                state.send_timer = 0.0;
                let msg = PlayerState {
                    id: 0,
                    x: state.x,
                    y: state.y,
                };
                let _ = state.client.send_json(&msg);
            }

            state.client.update(dt);

            // ── interpolate remote players ─────────────────────────────────────
            // t=0 → no movement, t=1 → instant snap; ~15 gives smooth but responsive feel
            let alpha = (15.0 * dt).min(1.0);
            for r in state.others.values_mut() {
                r.rx += (r.tx - r.rx) * alpha;
                r.ry += (r.ty - r.ry) * alpha;
            }

            // ── draw ──────────────────────────────────────────────────────────
            let hs = PLAYER_SIZE * 0.5;
            let aspect = W as f32 / H as f32;
            let vp = state.camera.view_proj(aspect);

            // Own player (white)
            state.debug.aabb(
                Vec3::new(state.x - hs, state.y - hs, -0.1),
                Vec3::new(state.x + hs, state.y + hs, 0.1),
                Vec3::ONE,
            );

            // Other players (colored)
            for (&id, r) in state.others.iter() {
                let color = COLORS[id as usize % COLORS.len()];
                state.debug.aabb(
                    Vec3::new(r.rx - hs, r.ry - hs, -0.1),
                    Vec3::new(r.rx + hs, r.ry + hs, 0.1),
                    color,
                );
            }

            state.debug.flush(ctx, vp);

            // ── UI ────────────────────────────────────────────────────────────
            state.ui.begin_frame(input, W as f32, H as f32);
            state.ui.begin_panel("Net", 10.0, 10.0, 240.0);
            state.ui.label_dim(&state.status);
            state
                .ui
                .label_dim(&format!("pos  ({:.1}, {:.1})", state.x, state.y));
            state
                .ui
                .label_dim(&format!("others: {}", state.others.len()));
            state.ui.end_panel();
            state.ui.end_frame(ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            state.debug.draw(pass);
            state.ui.render(pass);
        },
    );
}
