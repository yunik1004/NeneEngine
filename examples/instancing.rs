//! GPU instancing demo — 2 500 cubes, one draw call.
//!
//! Per-vertex data  (slot 0): MeshVertex (position + normal + uv)
//! Per-instance data (slot 1): InstanceData (model matrix + color)
//!
//! Controls: nothing — camera orbits automatically.

use nene::{
    app::{App, WindowId, run},
    input::Input,
    math::{Mat4, Quat, Vec3, Vec4},
    mesh::unit_cube,
    renderer::{
        AmbientLight, Context, DirectionalLight, IndexBuffer, InstanceBuffer, InstanceData,
        Material, MaterialBuilder, RenderPass, VertexBuffer,
    },
    time::Time,
    window::Config,
};

// ── Grid ──────────────────────────────────────────────────────────────────────

const GRID: i32 = 50;
const SPACING: f32 = 2.2;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let i = (h * 6.0) as u32;
    let f = h * 6.0 - i as f32;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match i % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct InstancingDemo {
    elapsed: f64,
    mat: Option<Material>,
    vbuf: Option<VertexBuffer>,
    ibuf: Option<IndexBuffer>,
    inst_buf: Option<InstanceBuffer>,
    instances: Vec<InstanceData>,
}

impl App for InstancingDemo {
    fn new() -> Self {
        InstancingDemo {
            elapsed: 0.0,
            mat: None,
            vbuf: None,
            ibuf: None,
            inst_buf: None,
            instances: Vec::new(),
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        let (mesh_verts, indices) = unit_cube().mesh();
        self.vbuf = Some(ctx.create_vertex_buffer(&mesh_verts));
        self.ibuf = Some(ctx.create_index_buffer(&indices));

        // Placeholder instance buffer — filled each frame in prepare()
        self.instances = vec![InstanceData::new(Mat4::IDENTITY, Vec4::ONE); (GRID * GRID) as usize];
        self.inst_buf = Some(ctx.create_instance_buffer(&self.instances));

        let mut mat = MaterialBuilder::new()
            .ambient()
            .directional()
            .instanced()
            .build(ctx);
        mat.uniform.ambient = AmbientLight::new(Vec3::ONE, 0.25);
        mat.uniform.directional =
            DirectionalLight::new(Vec3::new(1.0, 2.0, 1.0).normalize(), Vec3::ONE, 0.75);
        self.mat = Some(mat);
    }

    fn update(&mut self, _input: &Input, time: &Time) {
        self.elapsed = time.elapsed;
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let t = self.elapsed as f32;

        // Camera
        let (width, height) = {
            let cfg = ctx.surface_config();
            (cfg.width, cfg.height)
        };
        let aspect = width as f32 / height as f32;
        let r = GRID as f32 * SPACING * 0.65;
        let cam_pos = Vec3::new(r * (t * 0.12).cos(), r * 0.45, r * (t * 0.12).sin());
        let view_proj = Mat4::perspective_rh(45_f32.to_radians(), aspect, 0.5, r * 3.0)
            * Mat4::look_at_rh(cam_pos, Vec3::ZERO, Vec3::Y);

        // Update per-instance transforms on CPU
        for iz in 0..GRID {
            for ix in 0..GRID {
                let x = (ix - GRID / 2) as f32 * SPACING;
                let z = (iz - GRID / 2) as f32 * SPACING;
                let speed = 0.4 + 0.6 * ((x * 0.17 + z * 0.13).abs().fract());
                let angle = t * speed;
                let model = Mat4::from_translation(Vec3::new(x, 0.0, z))
                    * Mat4::from_quat(Quat::from_rotation_y(angle));
                let hue = (ix as f32 / GRID as f32 + iz as f32 / GRID as f32) % 1.0;
                let [r, g, b] = hsv_to_rgb(hue, 0.75, 0.9);
                let idx = (iz * GRID + ix) as usize;
                self.instances[idx] = InstanceData::new(model, Vec4::new(r, g, b, 1.0));
            }
        }

        if let (Some(mat), Some(inst_buf)) = (&mut self.mat, &self.inst_buf) {
            ctx.update_instance_buffer(inst_buf, &self.instances);
            mat.uniform.view_proj = view_proj;
            mat.flush(ctx);
        }
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        let (Some(mat), Some(vbuf), Some(ibuf), Some(inst_buf)) =
            (&self.mat, &self.vbuf, &self.ibuf, &self.inst_buf)
        else {
            return;
        };
        mat.render_instanced(pass, vbuf, ibuf, inst_buf, self.instances.len() as u32);
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Instancing — 1 draw call",
            ..Config::default()
        }]
    }
}

fn main() {
    run::<InstancingDemo>();
}
