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
    app::{App, Config, WindowId, run},
    audio::{AudioDevice, Sound, SpatialAudio},
    camera::Camera,
    debug::DebugDraw,
    input::{Input, Key},
    math::{Vec2, Vec3},
    renderer::{Context, RenderPass},
    time::Time,
    ui::Ui,
};

const W: u32 = 720;
const H: u32 = 540;
const MAX_DIST: f32 = 12.0;
const ORBIT_RADIUS: f32 = 6.0;
const ORBIT_SPEED: f32 = 0.8; // radians / second
const TONE_HZ: f32 = 440.0;

struct SpatialAudioDemo {
    #[allow(dead_code)] // must stay alive to keep the audio stream running
    audio: AudioDevice,
    spatial: SpatialAudio,
    source: nene::audio::SpatialSource,
    camera: Camera,
    angle: f32,
    paused: bool,
    debug: Option<DebugDraw>,
    ui: Option<Ui>,
}

impl App for SpatialAudioDemo {
    fn new() -> Self {
        let audio = AudioDevice::new().expect("no audio device");
        let sound = Arc::new(Sound::sine_wave(TONE_HZ, 0.5, 44100));
        let spatial = SpatialAudio::new(MAX_DIST);
        let source = spatial.play_source(&audio, &sound, Vec2::new(ORBIT_RADIUS, 0.0), true);
        SpatialAudioDemo {
            audio,
            spatial,
            source,
            camera: Camera::orthographic(Vec3::new(0.0, 0.0, 20.0), 28.0, 0.1, 100.0),
            angle: 0.0,
            paused: false,
            debug: None,
            ui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.debug = Some(DebugDraw::new(ctx));
        self.ui = Some(Ui::new(ctx));
    }

    fn update(&mut self, input: &Input, time: &Time) {
        if input.key_pressed(Key::Space) {
            self.paused = !self.paused;
        }

        if !self.paused {
            self.angle += ORBIT_SPEED * time.delta;
        }
        let emitter = Vec2::new(
            self.angle.cos() * ORBIT_RADIUS,
            self.angle.sin() * ORBIT_RADIUS,
        );
        self.source.set_position(&self.spatial, emitter);

        let opts = self.spatial.options_for(emitter);
        let (lx, ly) = (0.0f32, 0.0f32);

        let Some(debug) = &mut self.debug else { return };

        debug.circle(
            Vec3::new(lx, ly, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            MAX_DIST,
            Vec3::new(0.2, 0.2, 0.2),
        );
        debug.circle(
            Vec3::new(lx, ly, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            ORBIT_RADIUS,
            Vec3::new(0.2, 0.4, 0.8),
        );

        let t = 1.0 - opts.volume;
        debug.line(
            Vec3::new(lx, ly, 0.0),
            Vec3::new(emitter.x, emitter.y, 0.0),
            Vec3::new(t, 1.0 - t, 0.0),
        );

        let hs = 0.3;
        debug.aabb(
            Vec3::new(lx - hs, ly - hs, -0.1),
            Vec3::new(lx + hs, ly + hs, 0.1),
            Vec3::ONE,
        );

        let es = 0.2 + opts.volume * 0.4;
        debug.aabb(
            Vec3::new(emitter.x - es, emitter.y - es, -0.1),
            Vec3::new(emitter.x + es, emitter.y + es, 0.1),
            Vec3::new(1.0, 0.9, 0.1),
        );

        let Some(ui) = &mut self.ui else { return };
        ui.begin_frame(input, W as f32, H as f32);
        ui.begin_panel("Spatial Audio", 10.0, 10.0, 220.0);
        ui.label(&format!("volume  {:.2}", opts.volume));
        ui.label(&format!("pan     {:+.2}", opts.pan));
        ui.label(&format!("dist    {:.1} / {MAX_DIST:.0}", {
            let dx = emitter.x - lx;
            let dy = emitter.y - ly;
            (dx * dx + dy * dy).sqrt()
        }));
        ui.label_dim(if self.paused {
            "PAUSED (Space)"
        } else {
            "Space to pause"
        });
        ui.end_panel();
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let aspect = W as f32 / H as f32;
        let vp = self.camera.view_proj(aspect);
        if let Some(debug) = &mut self.debug {
            debug.flush(ctx, vp);
        }
        if let Some(ui) = &mut self.ui {
            ui.end_frame(ctx);
        }
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
