//! Spatial audio demo — positional pan and distance attenuation.
//!
//! A sound emitter (yellow dot) orbits the listener (white dot) in a circle.
//! The circle turns from green → red as the emitter moves away from the
//! listener.  A line from the listener to the emitter shows the current
//! direction, and a UI panel shows live volume and pan values.
//!
//! Controls
//! --------
//!   Space — pause / resume orbit

use std::sync::Arc;

use nene::{
    app::{App, Config, WindowEvent, WindowId, run},
    audio::{AudioDevice, Sound, SpatialAudio},
    camera::Camera,
    debug::DebugDraw,
    input::{ActionMap, Input, Key},
    math::{Vec2, Vec3},
    renderer::{Context, RenderPass},
    time::Time,
    ui::Ui,
};

const W: u32 = 720;
const H: u32 = 540;
const MAX_DIST: f32 = 12.0;
const ORBIT_RADIUS: f32 = 6.0;
const ORBIT_SPEED: f32 = 0.8;
const TONE_HZ: f32 = 440.0;

#[derive(Hash, PartialEq, Eq)]
enum Action {
    TogglePause,
}

struct SpatialAudioDemo {
    #[allow(dead_code)]
    audio: AudioDevice,
    spatial: SpatialAudio,
    source: nene::audio::SpatialSource,
    camera: Camera,
    angle: f32,
    paused: bool,
    emitter_pos: Vec2,
    opts_volume: f32,
    opts_pan: f32,
    bindings: ActionMap<Action>,
    debug: Option<DebugDraw>,
    egui: Option<Ui>,
}

impl App for SpatialAudioDemo {
    fn new() -> Self {
        let audio = AudioDevice::new().expect("no audio device");
        let sound = Arc::new(Sound::sine_wave(TONE_HZ, 0.5, 44100));
        let spatial = SpatialAudio::new(MAX_DIST);
        let source = spatial.play_source(&audio, &sound, Vec2::new(ORBIT_RADIUS, 0.0), true);
        let mut bindings = ActionMap::new();
        bindings.bind(Action::TogglePause, Key::Space);
        SpatialAudioDemo {
            audio,
            spatial,
            source,
            camera: Camera::orthographic(Vec3::new(0.0, 0.0, 20.0), 28.0, 0.1, 100.0),
            angle: 0.0,
            paused: false,
            emitter_pos: Vec2::new(ORBIT_RADIUS, 0.0),
            opts_volume: 1.0,
            opts_pan: 0.0,
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
        if self.bindings.pressed(input, &Action::TogglePause) {
            self.paused = !self.paused;
        }

        if !self.paused {
            self.angle += ORBIT_SPEED * time.delta;
        }
        self.emitter_pos = Vec2::new(
            self.angle.cos() * ORBIT_RADIUS,
            self.angle.sin() * ORBIT_RADIUS,
        );
        self.source.set_position(&self.spatial, self.emitter_pos);

        let opts = self.spatial.options_for(self.emitter_pos);
        self.opts_volume = opts.volume;
        self.opts_pan = opts.pan;

        let Some(debug) = &mut self.debug else { return };

        debug.circle(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            MAX_DIST,
            Vec3::new(0.2, 0.2, 0.2),
        );
        debug.circle(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            ORBIT_RADIUS,
            Vec3::new(0.2, 0.4, 0.8),
        );

        let t = 1.0 - self.opts_volume;
        debug.line(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(self.emitter_pos.x, self.emitter_pos.y, 0.0),
            Vec3::new(t, 1.0 - t, 0.0),
        );

        let hs = 0.3;
        debug.aabb(Vec3::new(-hs, -hs, -0.1), Vec3::new(hs, hs, 0.1), Vec3::ONE);

        let es = 0.2 + self.opts_volume * 0.4;
        debug.aabb(
            Vec3::new(self.emitter_pos.x - es, self.emitter_pos.y - es, -0.1),
            Vec3::new(self.emitter_pos.x + es, self.emitter_pos.y + es, 0.1),
            Vec3::new(1.0, 0.9, 0.1),
        );
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let aspect = W as f32 / H as f32;
        let vp = self.camera.view_proj(aspect);
        if let Some(debug) = &mut self.debug {
            debug.flush(ctx, vp);
        }

        let Some(egui) = &mut self.egui else { return };
        let ui_ctx = egui.begin_frame();

        let dist = self.emitter_pos.length();
        egui::Window::new("Spatial Audio")
            .default_pos(egui::pos2(10.0, 10.0))
            .default_width(220.0)
            .resizable(false)
            .show(&ui_ctx, |ui| {
                ui.label(format!("volume  {:.2}", self.opts_volume));
                ui.label(format!("pan     {:+.2}", self.opts_pan));
                ui.label(format!("dist    {:.1} / {MAX_DIST:.0}", dist));
                ui.label(
                    egui::RichText::new(if self.paused {
                        "PAUSED (Space)"
                    } else {
                        "Space to pause"
                    })
                    .weak(),
                );
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
            title: "Spatial Audio",
            width: W,
            height: H,
            ..Config::default()
        }]
    }
}

fn main() {
    run::<SpatialAudioDemo>();
}
