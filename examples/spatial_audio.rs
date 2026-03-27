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
//!
//! Usage: cargo run --example spatial_audio

use nene::{
    audio::{AudioDevice, Sound, SpatialAudio},
    camera::Camera,
    debug::DebugDraw,
    input::Key,
    math::{Vec2, Vec3},
    renderer::{Context, RenderPass},
    ui::Ui,
    window::{Config, Window},
};

use std::sync::Arc;

const W: u32 = 720;
const H: u32 = 540;
const MAX_DIST: f32 = 12.0;
const ORBIT_RADIUS: f32 = 6.0;
const ORBIT_SPEED: f32 = 0.8; // radians / second
const TONE_HZ: f32 = 440.0;

struct State {
    #[allow(dead_code)] // must stay alive to keep the audio stream running
    audio: AudioDevice,
    spatial: SpatialAudio,
    source: nene::audio::SpatialSource,
    debug: DebugDraw,
    ui: Ui,
    camera: Camera,
    angle: f32,
    paused: bool,
}

fn init(ctx: &mut Context) -> State {
    let audio = AudioDevice::new();
    let sound = Arc::new(make_tone(44100, TONE_HZ, 0.5));

    let spatial = SpatialAudio::new(MAX_DIST);
    let source = spatial.play_source(&audio, &sound, Vec2::new(ORBIT_RADIUS, 0.0), true);

    State {
        audio,
        spatial,
        source,
        debug: DebugDraw::new(ctx),
        ui: Ui::new(ctx),
        camera: Camera::orthographic(Vec3::new(0.0, 0.0, 20.0), 28.0, 0.1, 100.0),
        angle: 0.0,
        paused: false,
    }
}

fn main() {
    Window::new(Config {
        title: "Spatial Audio".into(),
        width: W,
        height: H,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            let dt = time.delta;

            // ── controls ──────────────────────────────────────────────────────
            if input.key_pressed(Key::Space) {
                state.paused = !state.paused;
            }

            // ── orbit emitter around the fixed listener (origin) ──────────────
            if !state.paused {
                state.angle += ORBIT_SPEED * dt;
            }
            let emitter = Vec2::new(
                state.angle.cos() * ORBIT_RADIUS,
                state.angle.sin() * ORBIT_RADIUS,
            );
            state.source.set_position(&state.spatial, emitter);

            let opts = state.spatial.options_for(emitter);
            let lx = 0.0f32;
            let ly = 0.0f32;

            // ── draw ──────────────────────────────────────────────────────────
            let aspect = W as f32 / H as f32;
            let vp = state.camera.view_proj(aspect);

            // Max-distance ring (dim grey)
            state.debug.circle(
                Vec3::new(lx, ly, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                MAX_DIST,
                Vec3::new(0.2, 0.2, 0.2),
            );

            // Orbit-radius ring (blue)
            state.debug.circle(
                Vec3::new(lx, ly, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                ORBIT_RADIUS,
                Vec3::new(0.2, 0.4, 0.8),
            );

            // Line: listener → emitter, colour fades green→red with distance
            let t = 1.0 - opts.volume; // 0 = close (green), 1 = far (red)
            let line_color = Vec3::new(t, 1.0 - t, 0.0);
            state.debug.line(
                Vec3::new(lx, ly, 0.0),
                Vec3::new(emitter.x, emitter.y, 0.0),
                line_color,
            );

            // Listener (white square)
            let hs = 0.3;
            state.debug.aabb(
                Vec3::new(lx - hs, ly - hs, -0.1),
                Vec3::new(lx + hs, ly + hs, 0.1),
                Vec3::ONE,
            );

            // Emitter (yellow square, size proportional to volume)
            let es = 0.2 + opts.volume * 0.4;
            state.debug.aabb(
                Vec3::new(emitter.x - es, emitter.y - es, -0.1),
                Vec3::new(emitter.x + es, emitter.y + es, 0.1),
                Vec3::new(1.0, 0.9, 0.1),
            );

            state.debug.flush(ctx, vp);

            // ── UI ────────────────────────────────────────────────────────────
            state.ui.begin_frame(input, W as f32, H as f32);
            state.ui.begin_panel("Spatial Audio", 10.0, 10.0, 220.0);
            state.ui.label(&format!("volume  {:.2}", opts.volume));
            state.ui.label(&format!("pan     {:+.2}", opts.pan));
            state.ui.label(&format!("dist    {:.1} / {MAX_DIST:.0}", {
                let dx = emitter.x - lx;
                let dy = emitter.y - ly;
                (dx * dx + dy * dy).sqrt()
            }));
            state.ui.label_dim(if state.paused {
                "PAUSED (Space)"
            } else {
                "Space to pause"
            });
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

// ── tone generator ────────────────────────────────────────────────────────────

fn make_tone(sample_rate: u32, freq: f32, duration: f32) -> Sound {
    let n_frames = (sample_rate as f32 * duration) as usize;
    let samples: Vec<i16> = (0..n_frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let v = (t * freq * std::f32::consts::TAU).sin();
            (v * i16::MAX as f32) as i16
        })
        .collect();
    let data = make_wav(sample_rate, 1, &samples);
    let path = std::env::temp_dir().join("nene_spatial_tone.wav");
    std::fs::write(&path, &data).unwrap();
    Sound::load(&path)
}

fn make_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data_size = (samples.len() * 2) as u32;
    let mut buf = Vec::with_capacity(44 + samples.len() * 2);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
    buf.extend_from_slice(&(channels * 2).to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}
