//! Physics demo — 2D and 3D.
//!
//! Both modes simulate an object falling and bouncing off a floor.
//!
//! Controls
//! --------
//! Tab — switch between 2D (ball, ortho) and 3D (cube, perspective)

use std::f32::consts::TAU;

use nene::{
    camera::Camera,
    input::Key,
    math::{Mat4, Vec3, Vec4},
    mesh::Model,
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    ui::Ui,
    uniform, vertex,
    window::{Config, Window},
};

// ── 2D ───────────────────────────────────────────────────────────────────────

const SHADER_2D: &str = r#"
struct Transform { mvp: mat4x4<f32>, color: vec4<f32> };
@group(0) @binding(0) var<uniform> u: Transform;
@vertex  fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return u.mvp * vec4<f32>(pos, 0.0, 1.0);
}
@fragment fn fs_main() -> @location(0) vec4<f32> { return u.color; }
"#;

#[vertex]
struct Vert2D {
    position: [f32; 2],
}

#[uniform]
struct Transform2D {
    mvp: Mat4,
    color: Vec4,
}

struct State2D {
    pipeline: Pipeline,
    ball_vb: VertexBuffer,
    ball_ib: IndexBuffer,
    ball_uni: UniformBuffer,
    floor_vb: VertexBuffer,
    floor_ib: IndexBuffer,
    floor_uni: UniformBuffer,
    world: nene::physics::d2::World,
    ball_handle: nene::physics::d2::RigidBodyHandle,
    camera: Camera,
}

fn circle_verts(radius: f32, segs: u32) -> (Vec<Vert2D>, Vec<u32>) {
    let mut verts = vec![Vert2D {
        position: [0.0, 0.0],
    }];
    for i in 0..=segs {
        let a = i as f32 / segs as f32 * TAU;
        verts.push(Vert2D {
            position: [radius * a.cos(), radius * a.sin()],
        });
    }
    let mut idx = Vec::new();
    for i in 0..segs {
        idx.extend_from_slice(&[0, i + 1, i + 2]);
    }
    (verts, idx)
}

fn rect_verts(w: f32, h: f32) -> (Vec<Vert2D>, Vec<u32>) {
    let (hw, hh) = (w * 0.5, h * 0.5);
    let v = vec![
        Vert2D {
            position: [-hw, -hh],
        },
        Vert2D {
            position: [hw, -hh],
        },
        Vert2D { position: [hw, hh] },
        Vert2D {
            position: [-hw, hh],
        },
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
    let mvp = camera.view_proj(1.0);

    let ball_uni = ctx.create_uniform_buffer(&Transform2D {
        mvp: mvp * Mat4::from_translation(Vec3::new(0.0, 8.0, 0.0)),
        color: Vec4::new(0.4, 0.7, 1.0, 1.0),
    });
    let floor_uni = ctx.create_uniform_buffer(&Transform2D {
        mvp,
        color: Vec4::new(0.55, 0.55, 0.55, 1.0),
    });
    let pipeline =
        ctx.create_pipeline(PipelineDescriptor::new(SHADER_2D, Vert2D::layout()).with_uniform());

    State2D {
        pipeline,
        ball_vb: ctx.create_vertex_buffer(&bv),
        ball_ib: ctx.create_index_buffer(&bi),
        ball_uni,
        floor_vb: ctx.create_vertex_buffer(&fv),
        floor_ib: ctx.create_index_buffer(&fi),
        floor_uni,
        world,
        ball_handle: ball_h,
        camera,
    }
}

// ── 3D ───────────────────────────────────────────────────────────────────────

const SHADER_3D: &str = r#"
struct Cam { view_proj: mat4x4<f32>, model: mat4x4<f32> };
@group(0) @binding(0) var<uniform> cam: Cam;
struct VIn  { @location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32> };
struct VOut { @builtin(position) clip: vec4<f32>, @location(0) n: vec3<f32> };
@vertex fn vs_main(v: VIn) -> VOut {
    return VOut(cam.view_proj * cam.model * vec4<f32>(v.position, 1.0), v.normal);
}
@fragment fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(1.0, 2.0, 3.0));
    let d = max(dot(normalize(v.n), light), 0.0);
    return vec4<f32>(vec3<f32>(0.6, 0.75, 1.0) * (0.2 + 0.8 * d), 1.0);
}
"#;

const SHADER_3D_FLOOR: &str = r#"
struct Cam { view_proj: mat4x4<f32>, model: mat4x4<f32> };
@group(0) @binding(0) var<uniform> cam: Cam;
struct VIn { @location(0) position: vec3<f32>, @location(1) normal: vec3<f32>, @location(2) uv: vec2<f32> };
@vertex fn vs_main(v: VIn) -> @builtin(position) vec4<f32> {
    return cam.view_proj * cam.model * vec4<f32>(v.position, 1.0);
}
@fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(0.4, 0.4, 0.4, 1.0); }
"#;

const CUBE_OBJ: &str = "\
v -0.5 -0.5  0.5\nv  0.5 -0.5  0.5\nv  0.5  0.5  0.5\nv -0.5  0.5  0.5\n\
v -0.5 -0.5 -0.5\nv  0.5 -0.5 -0.5\nv  0.5  0.5 -0.5\nv -0.5  0.5 -0.5\n\
vn 0 0 1\nvn 0 0 -1\nvn 0 1 0\nvn 0 -1 0\nvn 1 0 0\nvn -1 0 0\n\
f 1//1 2//1 3//1\nf 1//1 3//1 4//1\nf 6//2 5//2 8//2\nf 6//2 8//2 7//2\n\
f 4//3 3//3 7//3\nf 4//3 7//3 8//3\nf 5//4 6//4 2//4\nf 5//4 2//4 1//4\n\
f 2//5 6//5 7//5\nf 2//5 7//5 3//5\nf 5//6 1//6 4//6\nf 5//6 4//6 8//6\n";

#[uniform]
struct CamUniform3D {
    view_proj: Mat4,
    model: Mat4,
}

struct State3D {
    cube_pipeline: Pipeline,
    floor_pipeline: Pipeline,
    cube_vb: VertexBuffer,
    cube_ib: IndexBuffer,
    floor_vb: VertexBuffer,
    floor_ib: IndexBuffer,
    cube_uni: UniformBuffer,
    floor_uni: UniformBuffer,
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

    let path = std::env::temp_dir().join("nene_physics_cube.obj");
    std::fs::write(&path, CUBE_OBJ).unwrap();
    let model = Model::load(&path).expect("failed to load model");
    let mesh = &model.meshes[0];

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let vp = view_proj_3d(aspect);

    let cube_uni = ctx.create_uniform_buffer(&CamUniform3D {
        view_proj: vp,
        model: st(1.0, 1.0, 1.0, 0.0, 8.0, 0.0),
    });
    let floor_uni = ctx.create_uniform_buffer(&CamUniform3D {
        view_proj: vp,
        model: st(10.0, 0.2, 10.0, 0.0, -0.1, 0.0),
    });

    let cube_pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER_3D, nene::mesh::MeshVertex::layout())
            .with_uniform()
            .with_depth(),
    );
    let floor_pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER_3D_FLOOR, nene::mesh::MeshVertex::layout())
            .with_uniform()
            .with_depth(),
    );

    State3D {
        cube_pipeline,
        floor_pipeline,
        cube_vb: ctx.create_vertex_buffer(&mesh.vertices),
        cube_ib: ctx.create_index_buffer(&mesh.indices),
        floor_vb: ctx.create_vertex_buffer(&mesh.vertices),
        floor_ib: ctx.create_index_buffer(&mesh.indices),
        cube_uni,
        floor_uni,
        world,
        cube_handle: cube_h,
    }
}

// ── Combined state ────────────────────────────────────────────────────────────

struct State {
    mode_3d: bool,
    s2d: State2D,
    s3d: State3D,
    ui: Ui,
}

fn main() {
    Window::new(Config {
        title: "Physics demo  (Tab = switch 2D / 3D)".into(),
        ..Config::default()
    })
    .run_with_update(
        |ctx| State {
            mode_3d: false,
            s2d: init_2d(ctx),
            s3d: init_3d(ctx),
            ui: Ui::new(ctx),
        },
        |state, ctx, input, time| {
            if input.key_pressed(Key::Tab) {
                state.mode_3d = !state.mode_3d;
            }

            let cfg = ctx.surface_config();
            let aspect = cfg.width as f32 / cfg.height as f32;

            if !state.mode_3d {
                // 2D update
                state.s2d.world.step_dt(time.delta);
                let pos = state.s2d.world.position(state.s2d.ball_handle).unwrap();
                let mvp = state.s2d.camera.view_proj(1.0)
                    * Mat4::from_translation(Vec3::new(pos.x, pos.y, 0.0));
                ctx.update_uniform_buffer(
                    &state.s2d.ball_uni,
                    &Transform2D {
                        mvp,
                        color: Vec4::new(0.4, 0.7, 1.0, 1.0),
                    },
                );
            } else {
                // 3D update
                state.s3d.world.step_dt(time.delta);
                let t = state.s3d.world.position(state.s3d.cube_handle).unwrap();
                let vp = view_proj_3d(aspect);
                ctx.update_uniform_buffer(
                    &state.s3d.cube_uni,
                    &CamUniform3D {
                        view_proj: vp,
                        model: st(1.0, 1.0, 1.0, t.x, t.y, t.z),
                    },
                );
                ctx.update_uniform_buffer(
                    &state.s3d.floor_uni,
                    &CamUniform3D {
                        view_proj: vp,
                        model: st(10.0, 0.2, 10.0, 0.0, -0.1, 0.0),
                    },
                );
            }

            // UI
            let label = if state.mode_3d {
                "Mode: 3D"
            } else {
                "Mode: 2D"
            };
            state
                .ui
                .begin_frame(input, cfg.width as f32, cfg.height as f32);
            state.ui.begin_panel("Physics", 16.0, 16.0, 160.0);
            state.ui.label(label);
            state.ui.label_dim("Tab to switch");
            state.ui.end_panel();
            state.ui.end_frame(ctx);
        },
        |_, _| {},
        |state, pass: &mut RenderPass| {
            if !state.mode_3d {
                let s = &state.s2d;
                pass.set_pipeline(&s.pipeline);
                pass.set_uniform(0, &s.floor_uni);
                pass.set_vertex_buffer(0, &s.floor_vb);
                pass.draw_indexed(&s.floor_ib);
                pass.set_uniform(0, &s.ball_uni);
                pass.set_vertex_buffer(0, &s.ball_vb);
                pass.draw_indexed(&s.ball_ib);
            } else {
                let s = &state.s3d;
                pass.set_pipeline(&s.floor_pipeline);
                pass.set_uniform(0, &s.floor_uni);
                pass.set_vertex_buffer(0, &s.floor_vb);
                pass.draw_indexed(&s.floor_ib);
                pass.set_pipeline(&s.cube_pipeline);
                pass.set_uniform(0, &s.cube_uni);
                pass.set_vertex_buffer(0, &s.cube_vb);
                pass.draw_indexed(&s.cube_ib);
            }
            state.ui.render(pass);
        },
    );
}
