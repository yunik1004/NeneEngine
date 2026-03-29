/// Scene-graph demo: sun → planet → moon hierarchy with debug overlays.
use nene::{
    app::{App, Config, WindowId, run},
    camera::Camera,
    debug::{DebugDraw, color},
    input::Input,
    math::{Mat4, Quat, Vec3, Vec4},
    mesh::unit_cube,
    renderer::{Context, GpuMesh, Material, MaterialBuilder, RenderPass},
    scene::{Node, NodeId, Scene, Transform},
    time::Time,
};

struct BodyConfig {
    id: NodeId,
    color: Vec4,
    scale: f32,
    speed: f32,
    orbit_radius: f32,
}

struct Body {
    id: NodeId,
    mat: Material,
    scale: f32,
    angle: f32,
    speed: f32,
    orbit_radius: f32,
}

struct SceneDemo {
    scene: Scene,
    body_configs: Vec<BodyConfig>,
    camera: Camera,
    mesh: Option<GpuMesh>,
    debug: Option<DebugDraw>,
    bodies: Vec<Body>,
}

impl App for SceneDemo {
    fn new() -> Self {
        let mut scene = Scene::new();
        let sun = scene.add_node(Node::named("sun"));
        let planet = scene
            .add_child(
                sun,
                Node::named("planet")
                    .with_transform(Transform::from_position(Vec3::new(3.5, 0., 0.))),
            )
            .expect("sun is a valid node");
        let moon = scene
            .add_child(
                planet,
                Node::named("moon")
                    .with_transform(Transform::from_position(Vec3::new(1.4, 0., 0.))),
            )
            .expect("planet is a valid node");
        scene.update();

        SceneDemo {
            scene,
            body_configs: vec![
                BodyConfig {
                    id: sun,
                    color: Vec4::new(1.0, 0.85, 0.1, 1.),
                    scale: 0.9,
                    speed: 0.4,
                    orbit_radius: 0.,
                },
                BodyConfig {
                    id: planet,
                    color: Vec4::new(0.2, 0.5, 1.0, 1.),
                    scale: 0.5,
                    speed: 1.1,
                    orbit_radius: 3.5,
                },
                BodyConfig {
                    id: moon,
                    color: Vec4::new(0.8, 0.8, 0.8, 1.),
                    scale: 0.22,
                    speed: 2.8,
                    orbit_radius: 1.4,
                },
            ],
            camera: Camera::perspective(Vec3::new(0., 5., 12.), 45., 0.1, 100.),
            mesh: None,
            debug: None,
            bodies: Vec::new(),
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        let (verts, indices) = unit_cube().mesh();
        self.mesh = Some(GpuMesh::new(ctx, &verts, &indices));
        self.debug = Some(DebugDraw::new(ctx));

        self.bodies = self
            .body_configs
            .iter()
            .map(|cfg| Body {
                id: cfg.id,
                mat: MaterialBuilder::new()
                    .color(cfg.color)
                    .directional()
                    .build(ctx),
                scale: cfg.scale,
                angle: 0.0,
                speed: cfg.speed,
                orbit_radius: cfg.orbit_radius,
            })
            .collect();
    }

    fn update(&mut self, _input: &Input, time: &Time) {
        let dt = time.delta;
        for body in &mut self.bodies {
            body.angle += dt * body.speed;
        }
        if let Some(n) = self.scene.get_mut(self.bodies[0].id) {
            n.transform.rotation = Quat::from_rotation_y(self.bodies[0].angle);
        }
        for i in 1..3 {
            let r = self.bodies[i].orbit_radius;
            if let Some(n) = self.scene.get_mut(self.bodies[i].id) {
                n.transform.position =
                    Quat::from_rotation_y(self.bodies[i].angle) * Vec3::new(r, 0., 0.);
            }
        }
        self.scene.update();

        let Some(debug) = &mut self.debug else { return };
        debug.axes(Vec3::ZERO, 2.0);
        debug.circle(
            Vec3::ZERO,
            Vec3::Y,
            self.bodies[1].orbit_radius,
            color::GRAY,
        );
        let planet_pos = self
            .scene
            .get(self.bodies[1].id)
            .map(|n| n.world_transform().w_axis.truncate())
            .unwrap_or(Vec3::ZERO);
        debug.circle(
            planet_pos,
            Vec3::Y,
            self.bodies[2].orbit_radius,
            color::GRAY,
        );
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let cfg = ctx.surface_config();
        let vp = self.camera.view_proj(cfg.width as f32 / cfg.height as f32);

        for body in &mut self.bodies {
            let model = self
                .scene
                .get(body.id)
                .map(|n| n.world_transform())
                .unwrap_or(Mat4::IDENTITY)
                * Mat4::from_scale(Vec3::splat(body.scale));
            body.mat.uniform.view_proj = vp;
            body.mat.uniform.model = model;
            body.mat.flush(ctx);
        }

        if let Some(debug) = &mut self.debug {
            debug.flush(ctx, vp);
        }
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        let Some(mesh) = &self.mesh else { return };
        for body in &self.bodies {
            body.mat.render(pass, mesh);
        }
        if let Some(debug) = &self.debug {
            debug.draw(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Scene Graph — sun / planet / moon",
            ..Config::default()
        }]
    }
}

fn main() {
    run::<SceneDemo>();
}
