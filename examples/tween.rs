//! Tween easing visualiser.
//!
//! One horizontal track per easing function.
//! Each ball travels left → right driven by its own [`Tween<f32>`].
//!
//! Controls
//! ─────────
//!   Space — toggle PingPong / Once (restarts on completion) mode

use nene::{
    input::Key,
    math::{Mat4, Vec3, Vec4},
    renderer::{Context, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer},
    tween::{Ease, Tween, TweenLoop},
    uniform, vertex,
    window::{Config, Window},
};

// ── Shader ────────────────────────────────────────────────────────────────────

const SHADER: &str = r#"
struct U { mvp: mat4x4<f32>, color: vec4<f32> }
@group(0) @binding(0) var<uniform> u: U;

@vertex fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return u.mvp * vec4<f32>(pos, 0.0, 1.0);
}
@fragment fn fs_main() -> @location(0) vec4<f32> { return u.color; }
"#;

#[vertex]
struct Vert {
    pos: [f32; 2],
}

#[uniform]
struct Uniforms {
    mvp: Mat4,
    color: Vec4,
}

// ── Easing catalogue ──────────────────────────────────────────────────────────

const EASES: &[(Ease, &str)] = &[
    (Ease::Linear, "Linear"),
    (Ease::SineIn, "SineIn"),
    (Ease::SineOut, "SineOut"),
    (Ease::SineInOut, "SineInOut"),
    (Ease::QuadIn, "QuadIn"),
    (Ease::QuadOut, "QuadOut"),
    (Ease::CubicOut, "CubicOut"),
    (Ease::CubicInOut, "CubicInOut"),
    (Ease::QuartOut, "QuartOut"),
    (Ease::ElasticOut, "ElasticOut"),
    (Ease::BounceOut, "BounceOut"),
    (Ease::BackIn, "BackIn"),
    (Ease::BackOut, "BackOut"),
    (Ease::BackInOut, "BackInOut"),
];

const DURATION: f32 = 1.8;
const W: f32 = 960.0;
const H: f32 = 560.0;
const MARGIN: f32 = 100.0;

// ── Per-track GPU state ───────────────────────────────────────────────────────

struct Track {
    tween: Tween<f32>,
    /// Uniform for the ball (updated each frame in `update`).
    ball_ubuf: UniformBuffer,
    /// Uniform for the track rail.
    rail_ubuf: UniformBuffer,
    color: Vec4,
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    pipeline: Pipeline,
    /// Unit quad: [(-0.5,-0.5) .. (0.5,0.5)]
    unit_quad: VertexBuffer,
    tracks: Vec<Track>,
    ping_pong: bool,
}

fn hsv(h: f32, s: f32, v: f32) -> Vec4 {
    let i = (h * 6.0) as u32;
    let f = h * 6.0 - i as f32;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Vec4::new(r, g, b, 1.0)
}

fn ortho2d() -> Mat4 {
    Mat4::orthographic_rh(0.0, W, 0.0, H, -1.0, 1.0)
}

/// Scale + translate a unit quad to (cx, cy) with half-extents (hw, hh).
fn quad_mvp(cx: f32, cy: f32, hw: f32, hh: f32) -> Mat4 {
    ortho2d()
        * Mat4::from_translation(Vec3::new(cx, cy, 0.0))
        * Mat4::from_scale(Vec3::new(hw * 2.0, hh * 2.0, 1.0))
}

fn make_tracks(ctx: &Context, ping_pong: bool) -> Vec<Track> {
    let n = EASES.len();
    let loop_mode = if ping_pong {
        TweenLoop::PingPong
    } else {
        TweenLoop::Once
    };
    let row_h = H / n as f32;
    let track_w = W - MARGIN * 2.0;

    EASES
        .iter()
        .enumerate()
        .map(|(i, &(ease, _))| {
            let color = hsv(i as f32 / n as f32, 0.75, 0.9);
            let cy = (i as f32 + 0.5) * row_h;

            // Rail — thin horizontal line at rest position
            let rail_mvp = quad_mvp(MARGIN + track_w / 2.0, cy, track_w / 2.0, 1.0);
            let rail_color = Vec4::new(color.x * 0.25, color.y * 0.25, color.z * 0.25, 1.0);

            let tween = Tween::new(0.0_f32, 1.0, DURATION)
                .with_ease(ease)
                .with_loop(loop_mode);

            Track {
                tween,
                ball_ubuf: ctx.create_uniform_buffer(&Uniforms {
                    mvp: quad_mvp(MARGIN, cy, 10.0, 10.0),
                    color,
                }),
                rail_ubuf: ctx.create_uniform_buffer(&Uniforms {
                    mvp: rail_mvp,
                    color: rail_color,
                }),
                color,
            }
        })
        .collect()
}

fn init(ctx: &mut Context) -> State {
    let pipeline =
        ctx.create_pipeline(PipelineDescriptor::new(SHADER, Vert::layout()).with_uniform());

    let unit_quad = ctx.create_vertex_buffer(&[
        Vert { pos: [-0.5, -0.5] },
        Vert { pos: [0.5, -0.5] },
        Vert { pos: [0.5, 0.5] },
        Vert { pos: [-0.5, -0.5] },
        Vert { pos: [0.5, 0.5] },
        Vert { pos: [-0.5, 0.5] },
    ]);

    let tracks = make_tracks(ctx, false);

    State {
        pipeline,
        unit_quad,
        tracks,
        ping_pong: false,
    }
}

fn main() {
    Window::new(Config {
        title: "Tween — easing visualiser  [Space: toggle PingPong]".to_string(),
        width: W as u32,
        height: H as u32,
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx, input, time| {
            if input.key_pressed(Key::Space) {
                state.ping_pong = !state.ping_pong;
                state.tracks = make_tracks(ctx, state.ping_pong);
            }

            let n = state.tracks.len();
            let row_h = H / n as f32;
            let track_w = W - MARGIN * 2.0;
            let ball_r = (row_h * 0.30).max(6.0);

            for (i, track) in state.tracks.iter_mut().enumerate() {
                track.tween.update(time.delta);
                if track.tween.is_done() {
                    track.tween.reset();
                }

                let t = track.tween.value();
                let cy = (i as f32 + 0.5) * row_h;
                let bx = MARGIN + t * track_w;

                ctx.update_uniform_buffer(
                    &track.ball_ubuf,
                    &Uniforms {
                        mvp: quad_mvp(bx, cy, ball_r, ball_r),
                        color: track.color,
                    },
                );
            }
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_vertex_buffer(0, &state.unit_quad);

            for track in &state.tracks {
                // Rail
                pass.set_uniform(0, &track.rail_ubuf);
                pass.draw(0..6);
                // Ball
                pass.set_uniform(0, &track.ball_ubuf);
                pass.draw(0..6);
            }
        },
    );
}
