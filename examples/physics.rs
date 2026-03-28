//! Physics demo — 2D and 3D.
//!
//! Both modes simulate an object falling and bouncing off a floor.
//!
//! Controls
//! --------
//! Tab — switch between 2D (ball, ortho) and 3D (cube, perspective)

use std::f32::consts::TAU;

use nene::{
    app::{App, Config, WindowId, run},
    camera::Camera,
    input::{Input, Key},
    math::{Mat4, Vec2, Vec3, Vec4},
    mesh::unit_cube,
    renderer::{Context, FlatObject, Material, MaterialBuilder, Mesh, RenderPass},
    time::Time,
    ui::Ui,
};

// ── 2D ───────────────────────────────────────────────────────────────────────

struct State2D {
    ball: FlatObject,
    floor: FlatObject,
    world: nene::physics::d2::World,
    ball_handle: nene::physics::d2::RigidBodyHandle,
    camera: Camera,
}

fn circle_verts(radius: f32, segs: u32) -> (Vec<Vec2>, Vec<u32>) {
    let mut verts = vec![Vec2::new(0.0, 0.0)];
    for i in 0..=segs {
        let a = i as f32 / segs as f32 * TAU;
        verts.push(Vec2::new(radius * a.cos(), radius * a.sin()));
    }
    let mut idx = Vec::new();
    for i in 0..segs {
        idx.extend_from_slice(&[0, i + 1, i + 2]);
    }
    (verts, idx)
}

fn rect_verts(w: f32, h: f32) -> (Vec<Vec2>, Vec<u32>) {
    let (hw, hh) = (w * 0.5, h * 0.5);
    let v = vec![
        Vec2::new(-hw, -hh),
        Vec2::new(hw, -hh),
        Vec2::new(hw, hh),
        Vec2::new(-hw, hh),
    ];
    (v, vec![0, 1, 2, 0, 2, 3])
}

fn init_2d(ctx: &mut Context) -> State2D {
    use nene::physics::d2::{ColliderBuilder, RigidBodyBuilder, World};

    let mut world = World::new();
    let floor_h = world.add_body(RigidBodyBuilder::fixed());
    world.add_collider(ColliderBuilder::cuboid(5.0, 0.1), floor_h);
    let ball_h = world.add_body(RigidBodyBuilder::dynamic().translation(0.0, 8.0));
    world.add_collider(ColliderBuilder::ball(0.5).restitution(0.7), ball_h);

    let camera = Camera::orthographic_bounds(-6.0, 6.0, -1.0, 11.0, -1.0, 1.0);

    let (bv, bi) = circle_verts(0.5, 32);
    let (fv, fi) = rect_verts(10.0, 0.2);

    State2D {
        ball: FlatObject::new_indexed(ctx, &bv, &bi, Vec4::new(0.4, 0.7, 1.0, 1.0)),
        floor: FlatObject::new_indexed(ctx, &fv, &fi, Vec4::new(0.55, 0.55, 0.55, 1.0)),
        world,
        ball_handle: ball_h,
        camera,
    }
}

// ── 3D ───────────────────────────────────────────────────────────────────────

struct State3D {
    cube_mat: Material,
    cube_mesh: Mesh,
    floor_mat: Material,
    floor_mesh: Mesh,
    world: nene::physics::d3::World,
    cube_handle: nene::physics::d3::RigidBodyHandle,
}

fn st(sx: f32, sy: f32, sz: f32, tx: f32, ty: f32, tz: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(tx, ty, tz)) * Mat4::from_scale(Vec3::new(sx, sy, sz))
}

fn view_proj_3d(aspect: f32) -> Mat4 {
    Camera::perspective(Vec3::new(4.0, 8.0, 12.0), 45.0, 0.1, 100.0).view_proj(aspect)
}

fn init_3d(ctx: &mut Context) -> State3D {
    use nene::physics::d3::{ColliderBuilder, RigidBodyBuilder, World};

    let mut world = World::new();
    let floor_h = world.add_body(RigidBodyBuilder::fixed().translation(0.0, -0.1, 0.0));
    world.add_collider(ColliderBuilder::cuboid(5.0, 0.1, 5.0), floor_h);
    let cube_h = world.add_body(RigidBodyBuilder::dynamic().translation(0.0, 8.0, 0.0));
    world.add_collider(
        ColliderBuilder::cuboid(0.5, 0.5, 0.5).restitution(0.6),
        cube_h,
    );

    let (cube_verts, cube_indices) = unit_cube().mesh();
    let cube_mesh = Mesh::new(ctx, &cube_verts, &cube_indices);
    let floor_mesh = Mesh::new(ctx, &cube_verts, &cube_indices);

    let mut cube_mat = MaterialBuilder::new()
        .color(Vec4::new(0.6, 0.75, 1.0, 1.0))
        .build(ctx);
    cube_mat.uniform.model = st(1.0, 1.0, 1.0, 0.0, 8.0, 0.0);

    let floor_mat = MaterialBuilder::new()
        .color(Vec4::new(0.4, 0.4, 0.4, 1.0))
        .build(ctx);

    State3D {
        cube_mat,
        cube_mesh,
        floor_mat,
        floor_mesh,
        world,
        cube_handle: cube_h,
    }
}

// ── App state ─────────────────────────────────────────────────────────────────

struct PhysicsDemo {
    mode_3d: bool,
    s2d: Option<State2D>,
    s3d: Option<State3D>,
    ui: Option<Ui>,
}

impl App for PhysicsDemo {
    fn new() -> Self {
        PhysicsDemo {
            mode_3d: false,
            s2d: None,
            s3d: None,
            ui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.s2d = Some(init_2d(ctx));
        self.s3d = Some(init_3d(ctx));
        self.ui = Some(Ui::new(ctx));
    }

    fn update(&mut self, input: &Input, time: &Time) {
        if input.key_pressed(Key::Tab) {
            self.mode_3d = !self.mode_3d;
        }

        if !self.mode_3d {
            if let Some(s) = &mut self.s2d {
                s.world.step_dt(time.delta);
            }
        } else {
            if let Some(s) = &mut self.s3d {
                s.world.step_dt(time.delta);
            }
        }
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, input: &Input) {
        let (width, height) = {
            let cfg = ctx.surface_config();
            (cfg.width, cfg.height)
        };
        let aspect = width as f32 / height as f32;

        if !self.mode_3d {
            if let Some(s) = &mut self.s2d {
                let pos = s.world.position(s.ball_handle).unwrap();
                let vp = s.camera.view_proj(1.0);
                s.ball.set_transform(
                    ctx,
                    vp * Mat4::from_translation(Vec3::new(pos.x, pos.y, 0.0)),
                );
                s.floor.set_transform(ctx, vp);
            }
        } else {
            if let Some(s) = &mut self.s3d {
                let t = s.world.position(s.cube_handle).unwrap();
                let vp = view_proj_3d(aspect);
                s.cube_mat.uniform.view_proj = vp;
                s.cube_mat.uniform.model = st(1.0, 1.0, 1.0, t.x, t.y, t.z);
                s.cube_mat.flush(ctx);
                s.floor_mat.uniform.view_proj = vp;
                s.floor_mat.uniform.model = st(10.0, 0.2, 10.0, 0.0, -0.1, 0.0);
                s.floor_mat.flush(ctx);
            }
        }

        let label = if self.mode_3d { "Mode: 3D" } else { "Mode: 2D" };
        if let Some(ui) = &mut self.ui {
            ui.begin_frame(input, width as f32, height as f32);
            ui.begin_panel("Physics", 16.0, 16.0, 160.0);
            ui.label(label);
            ui.label_dim("Tab to switch");
            ui.end_panel();
            ui.end_frame(ctx);
        }
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if !self.mode_3d {
            if let Some(s) = &self.s2d {
                s.floor.render(pass);
                s.ball.render(pass);
            }
        } else {
            if let Some(s) = &self.s3d {
                s.floor_mat.render(pass, &s.floor_mesh);
                s.cube_mat.render(pass, &s.cube_mesh);
            }
        }
        if let Some(ui) = &self.ui {
            ui.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "Physics demo  (Tab = switch 2D / 3D)",
            ..Config::default()
        }]
    }
}

fn main() {
    run::<PhysicsDemo>();
}
