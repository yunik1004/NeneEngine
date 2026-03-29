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
    app::{App, Config, WindowEvent, WindowId, run},
    camera::Camera,
    debug::DebugDraw,
    input::{ActionMap, Input, Key},
    math::Vec3,
    net::{Client, ClientEvent},
    renderer::{Context, RenderPass},
    time::Time,
    ui::Ui,
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

#[derive(Hash, PartialEq, Eq)]
enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
}

struct MultiplayerClientDemo {
    client: Client,
    camera: Camera,
    x: f32,
    y: f32,
    others: HashMap<u32, RemotePlayer>,
    send_timer: f32,
    status: String,
    bindings: ActionMap<Action>,
    debug: Option<DebugDraw>,
    egui: Option<Ui>,
}

impl App for MultiplayerClientDemo {
    fn new() -> Self {
        let client = Client::connect("127.0.0.1:7777").expect("connect failed");
        let mut bindings = ActionMap::new();
        bindings
            .bind(Action::MoveUp, Key::KeyW)
            .bind(Action::MoveUp, Key::ArrowUp)
            .bind(Action::MoveDown, Key::KeyS)
            .bind(Action::MoveDown, Key::ArrowDown)
            .bind(Action::MoveLeft, Key::KeyA)
            .bind(Action::MoveLeft, Key::ArrowLeft)
            .bind(Action::MoveRight, Key::KeyD)
            .bind(Action::MoveRight, Key::ArrowRight);
        MultiplayerClientDemo {
            client,
            camera: Camera::orthographic(Vec3::new(0.0, 0.0, 10.0), 20.0, 0.1, 100.0),
            x: 0.0,
            y: 0.0,
            others: HashMap::new(),
            send_timer: 0.0,
            status: "Connecting…".into(),
            bindings,
            debug: None,
            egui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.debug = Some(DebugDraw::new(ctx));
        self.egui = Some(Ui::new(ctx));
    }

    fn on_window_event(&mut self, _id: WindowId, event: &WindowEvent) {
        if let Some(e) = &mut self.egui {
            e.handle_event(event);
        }
    }

    fn update(&mut self, input: &Input, time: &Time) {
        let dt = time.delta;

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

        if self.client.is_connected() {
            let mut dx = 0.0f32;
            let mut dy = 0.0f32;
            if self.bindings.down(input, &Action::MoveUp) {
                dy += 1.0;
            }
            if self.bindings.down(input, &Action::MoveDown) {
                dy -= 1.0;
            }
            if self.bindings.down(input, &Action::MoveLeft) {
                dx -= 1.0;
            }
            if self.bindings.down(input, &Action::MoveRight) {
                dx += 1.0;
            }
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            self.x += dx / len * SPEED * dt;
            self.y += dy / len * SPEED * dt;
        }

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

        let alpha = (15.0 * dt).min(1.0);
        for r in self.others.values_mut() {
            r.rx += (r.tx - r.rx) * alpha;
            r.ry += (r.ty - r.ry) * alpha;
        }

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

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let aspect = W as f32 / H as f32;
        let vp = self.camera.view_proj(aspect);
        if let Some(debug) = &mut self.debug {
            debug.flush(ctx, vp);
        }

        let Some(egui) = &mut self.egui else { return };
        let ui_ctx = egui.begin_frame();

        egui::Window::new("Net")
            .default_pos(egui::pos2(10.0, 10.0))
            .default_width(240.0)
            .resizable(false)
            .show(&ui_ctx, |ui| {
                ui.label(egui::RichText::new(&self.status).weak());
                ui.label(
                    egui::RichText::new(format!("pos  ({:.1}, {:.1})", self.x, self.y)).weak(),
                );
                ui.label(egui::RichText::new(format!("others: {}", self.others.len())).weak());
            });

        egui.end_frame(ctx);
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(debug) = &self.debug {
            debug.draw(pass);
        }
        if let Some(e) = &self.egui {
            e.render(pass);
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
