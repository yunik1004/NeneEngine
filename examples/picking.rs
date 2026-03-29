//! Mouse picking demo — click a sphere to select it.
//!
//! Three spheres float in a 3D scene. Each click casts a ray from the camera
//! through the cursor and finds the nearest sphere the ray intersects.
//! The selected sphere is highlighted in yellow.
//!
//! Controls
//! --------
//! LMB  — select an object (click on empty space to deselect)

use nene::{
    app::{App, Config, WindowEvent, WindowId, run},
    camera::Camera,
    input::{Input, MouseButton},
    math::{Mat4, Vec3, Vec4},
    mesh::cube,
    renderer::{Context, GpuMesh, Material, MaterialBuilder, RenderPass},
    time::Time,
    ui::Ui,
};

// ── Scene ─────────────────────────────────────────────────────────────────────

struct Object {
    center: Vec3,
    radius: f32,
    color: Vec4,
}

const OBJECTS: &[(Vec3, f32, Vec4)] = &[
    (
        Vec3::new(-2.5, 0.0, 0.0),
        0.8,
        Vec4::new(0.9, 0.3, 0.3, 1.0),
    ),
    (Vec3::new(0.0, 0.0, 0.0), 1.0, Vec4::new(0.3, 0.8, 0.3, 1.0)),
    (
        Vec3::new(2.5, 0.0, 0.0),
        0.65,
        Vec4::new(0.3, 0.5, 1.0, 1.0),
    ),
];

const SELECTED_COLOR: Vec4 = Vec4::new(1.0, 0.9, 0.1, 1.0);
const CAM_POS: Vec3 = Vec3::new(0.0, 4.0, 10.0);
const CAM_TARGET: Vec3 = Vec3::ZERO;

// ── App ───────────────────────────────────────────────────────────────────────

struct PickingDemo {
    objects: Vec<Object>,
    selected: Option<usize>,
    // GPU resources — initialised in window_ready
    mesh: Option<GpuMesh>,
    materials: Vec<Material>,
    egui: Option<Ui>,
    // Logical pixel dimensions — updated each prepare, used in update for ray casting
    width: f32,
    height: f32,
}

impl App for PickingDemo {
    fn new() -> Self {
        let objects = OBJECTS
            .iter()
            .map(|&(center, radius, color)| Object {
                center,
                radius,
                color,
            })
            .collect();

        PickingDemo {
            objects,
            selected: None,
            mesh: None,
            materials: Vec::new(),
            egui: None,
            width: 1280.0,
            height: 720.0,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        let (verts, indices) = cube(Vec3::ONE).mesh();
        self.mesh = Some(GpuMesh::new(ctx, &verts, &indices));

        for obj in &self.objects {
            let mat = MaterialBuilder::new().color(obj.color).build(ctx);
            self.materials.push(mat);
        }

        self.egui = Some(Ui::new(ctx));
    }

    fn on_window_event(&mut self, _id: WindowId, event: &WindowEvent) {
        if let Some(e) = &mut self.egui {
            e.handle_event(event);
        }
    }

    fn update(&mut self, input: &Input, _time: &Time) {
        if input.mouse_pressed(MouseButton::Left) {
            let pos = input.mouse_pos();
            let aspect = self.width / self.height;

            let mut camera = Camera::perspective(CAM_POS, 45.0, 0.1, 100.0);
            camera.target = CAM_TARGET;

            let ray = camera.screen_to_ray(pos.x, pos.y, self.width, self.height, aspect);

            // Find the closest hit — AABB matches the rendered cube
            let mut best: Option<(usize, f32)> = None;
            for (i, obj) in self.objects.iter().enumerate() {
                let half = Vec3::splat(obj.radius);
                let aabb_min = obj.center - half;
                let aabb_max = obj.center + half;
                if let Some(t) = ray.cast_aabb(aabb_min, aabb_max) {
                    if best.map_or(true, |(_, bt)| t < bt) {
                        best = Some((i, t));
                    }
                }
            }
            self.selected = best.map(|(i, _)| i);
        }
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        (self.width, self.height) = ctx.logical_size();
        let aspect = self.width / self.height;

        let mut camera = Camera::perspective(CAM_POS, 45.0, 0.1, 100.0);
        camera.target = CAM_TARGET;
        let vp = camera.view_proj(aspect);

        for (i, (obj, mat)) in self
            .objects
            .iter()
            .zip(self.materials.iter_mut())
            .enumerate()
        {
            let color = if self.selected == Some(i) {
                SELECTED_COLOR
            } else {
                obj.color
            };
            let scale = obj.radius;
            mat.uniform.color = color;
            mat.uniform.view_proj = vp;
            mat.uniform.model =
                Mat4::from_translation(obj.center) * Mat4::from_scale(Vec3::splat(scale * 2.0));
            mat.flush(ctx);
        }

        let Some(egui) = &mut self.egui else { return };
        let ui_ctx = egui.begin_frame();

        let sel_text = match self.selected {
            Some(i) => format!("Selected: object {}", i + 1),
            None => "Nothing selected".to_string(),
        };

        egui::Window::new("Picking")
            .default_pos(egui::pos2(16.0, 16.0))
            .default_width(200.0)
            .resizable(false)
            .show(&ui_ctx, |ui| {
                ui.label(&sel_text);
                ui.separator();
                ui.label(egui::RichText::new("LMB  click an object").weak());
                ui.label(egui::RichText::new("Click empty space to deselect").weak());
            });

        egui.end_frame(ctx);
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(mesh) = &self.mesh {
            for mat in &self.materials {
                mat.render(pass, mesh);
            }
        }
        if let Some(e) = &self.egui {
            e.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Mouse Picking  (LMB = select)",
            width: 1280,
            height: 720,
            ..Config::default()
        }]
    }
}

fn main() {
    run::<PickingDemo>();
}
