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
    app::{App, WindowId, run},
    camera::Camera,
    debug::DebugDraw,
    input::{Input, Key},
    math::Vec3,
    net::{Client, ClientEvent},
    renderer::{Context, RenderPass},
    time::Time,
    ui::Ui,
    window::Config,
};
use std::collections::HashMap;

const W: u32 = 640;
const H: u32 = 480;
const SPEED: f32 = 5.0;
const PLAYER_SIZE: f32 = 0.5;

const COLORS: &[Vec3] = &[
    Vec3::new(1.0, 0.3, 0.3),
    Vec3::new(0.3, 1.0, 0.3),
    Vec3::new(0.3, 0.3, 1.0),
    Vec3::new(1.0, 1.0, 0.3),
    Vec3::new(1.0, 0.3, 1.0),
    Vec3::new(0.3, 1.0, 1.0),
];

#[nene::data]
#[derive(Debug)]
struct PlayerState {
    id: u32,
    x: f32,
    y: f32,
}

struct RemotePlayer {
    tx: f32,
    ty: f32,
    rx: f32,
    ry: f32,
}

struct MultiplayerClientDemo {
    client: Client,
    camera: Camera,
    x: f32,
    y: f32,
    others: HashMap<u32, RemotePlayer>,
    send_timer: f32,
    status: String,
    // GPU
    debug: Option<DebugDraw>,
    ui: Option<Ui>,
}

impl App for MultiplayerClientDemo {
    fn new() -> Self {
        let client = Client::connect("127.0.0.1:7777").expect("connect failed");
        MultiplayerClientDemo {
            client,
            camera: Camera::orthographic(Vec3::new(0.0, 0.0, 10.0), 20.0, 0.1, 100.0),
            x: 0.0,
            y: 0.0,
            others: HashMap::new(),
            send_timer: 0.0,
            status: "Connecting…".into(),
            debug: None,
            ui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.debug = Some(DebugDraw::new(ctx));
        self.ui = Some(Ui::new(ctx));
    }

    fn update(&mut self, input: &Input, time: &Time) {
        let dt = time.delta;

        // ── network events ────────────────────────────────────────────────
        for ev in self.client.poll() {
            match &ev {
                ClientEvent::Connected => {
                    self.status = "Connected! Use WASD to move.".into();
                }
                ClientEvent::Disconnected => {
                    self.status = "Disconnected.".into();
                }
                ClientEvent::Message(_) => {
                    if let Some(p) = ev.into_json::<PlayerState>() {
                        if p.x.is_nan() || p.y.is_nan() {
                            self.others.remove(&p.id);
                        } else {
                            self.others
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
        if self.client.is_connected() {
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
            self.x += dx / len * SPEED * dt;
            self.y += dy / len * SPEED * dt;
        }

        // ── send position ~20 times/sec ───────────────────────────────────
        self.send_timer += dt;
        if self.send_timer >= 0.05 && self.client.is_connected() {
            self.send_timer = 0.0;
            let msg = PlayerState {
                id: 0,
                x: self.x,
                y: self.y,
            };
            let _ = self.client.send_json(&msg);
        }

        self.client.update(dt);

        // ── interpolate remote players ────────────────────────────────────
        let alpha = (15.0 * dt).min(1.0);
        for r in self.others.values_mut() {
            r.rx += (r.tx - r.rx) * alpha;
            r.ry += (r.ty - r.ry) * alpha;
        }

        // ── queue debug draws ─────────────────────────────────────────────
        let Some(debug) = &mut self.debug else { return };
        let hs = PLAYER_SIZE * 0.5;
        debug.aabb(
            Vec3::new(self.x - hs, self.y - hs, -0.1),
            Vec3::new(self.x + hs, self.y + hs, 0.1),
            Vec3::ONE,
        );
        for (&id, r) in self.others.iter() {
            let color = COLORS[id as usize % COLORS.len()];
            debug.aabb(
                Vec3::new(r.rx - hs, r.ry - hs, -0.1),
                Vec3::new(r.rx + hs, r.ry + hs, 0.1),
                color,
            );
        }
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, input: &Input) {
        let aspect = W as f32 / H as f32;
        let vp = self.camera.view_proj(aspect);
        if let Some(debug) = &mut self.debug {
            debug.flush(ctx, vp);
        }
        let Some(ui) = &mut self.ui else { return };
        ui.begin_frame(input, W as f32, H as f32);
        ui.begin_panel("Net", 10.0, 10.0, 240.0);
        ui.label_dim(&self.status);
        ui.label_dim(&format!("pos  ({:.1}, {:.1})", self.x, self.y));
        ui.label_dim(&format!("others: {}", self.others.len()));
        ui.end_panel();
        ui.end_frame(ctx);
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(debug) = &self.debug {
            debug.draw(pass);
        }
        if let Some(ui) = &self.ui {
            ui.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Multiplayer demo",
            width: W,
            height: H,
            ..Config::default()
        }]
    }
}

fn main() {
    run::<MultiplayerClientDemo>();
}
