//! Text rendering demo.
//!
//! Shows two ways to use [`TextRenderer`]:
//!
//! 1. **Screen overlay** — `queue` + `render`: draw text directly onto the
//!    screen each frame, zero allocation at display time.
//! 2. **Texture bake** — `render_to_texture`: rasterise text into a [`Texture`]
//!    and sample it on a spinning 3D quad.
use nene::{
    app::{App, WindowId, run},
    camera::Camera,
    input::Input,
    math::Vec3,
    mesh::{TexturedMesh, quad},
    renderer::{Context, RenderPass},
    text::TextRenderer,
    time::Time,
    window::Config,
};

struct TextDemo {
    mesh: Option<TexturedMesh>,
    text: Option<TextRenderer>,
    angle: f32,
    frame: u32,
}

impl App for TextDemo {
    fn new() -> Self {
        TextDemo {
            mesh: None,
            text: None,
            angle: 0.0,
            frame: 0,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        let (verts, indices) = quad(4.0, 1.0).mesh(); // 4:1 aspect ratio matches texture bake
        let mut mesh = TexturedMesh::new(ctx);
        mesh.set_geometry(ctx, &verts);
        mesh.set_indices(ctx, &indices);
        self.mesh = Some(mesh);
        self.text = Some(TextRenderer::new(ctx));
    }

    fn update(&mut self, _input: &Input, time: &Time) {
        self.angle += std::f32::consts::TAU * 0.3 * time.delta;
        self.frame += 1;
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let cfg = ctx.surface_config();
        let aspect = cfg.width as f32 / cfg.height as f32;
        let camera = Camera::perspective(Vec3::new(0.0, 0.0, 4.0), 45.0, 0.1, 100.0);
        let mvp = camera.view_proj(aspect) * glam::Mat4::from_rotation_y(self.angle);

        if let Some(text) = &mut self.text {
            text.queue(
                &format!("Frame: {}", self.frame),
                10.0,
                10.0,
                48.0,
                [1.0, 1.0, 1.0, 1.0],
            );
            text.queue("nene engine", 10.0, 70.0, 32.0, [0.6, 0.9, 1.0, 1.0]);
            let texture = text.render_to_texture(ctx, 512, 128);

            if let Some(mesh) = &mut self.mesh {
                mesh.set_texture(texture);
                mesh.set_transform(ctx, mvp);
            }

            // 2-D screen overlay
            text.queue("Hello, Nene!", 20.0, 20.0, 36.0, [1.0, 1.0, 1.0, 1.0]);
            text.queue(
                "↑ text baked into texture  ↑",
                20.0,
                64.0,
                20.0,
                [0.6, 0.6, 0.6, 1.0],
            );
            text.prepare(ctx);
        }
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(mesh) = &self.mesh {
            mesh.render(pass);
        }
        if let Some(text) = &self.text {
            text.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Text demo — overlay + texture bake",
            ..Config::default()
        }]
    }
}

fn main() {
    run::<TextDemo>();
}
