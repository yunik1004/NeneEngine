/// 2D physics: a ball falls and bounces off a floor (orthographic view).
use std::f32::consts::TAU;

use nene::{
    math::{Mat4, Vec3},
    physics2d::{ColliderBuilder, RigidBodyBuilder, RigidBodyHandle, World},
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    uniform, vertex,
    window::{Config, Window},
};

const SHADER: &str = r#"
struct Transform {
    mvp:   mat4x4<f32>,
    color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Transform;

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
    return u.mvp * vec4<f32>(position, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return u.color;
}
"#;

#[vertex]
struct Vert {
    position: [f32; 2],
}

#[uniform]
struct Transform {
    mvp: [[f32; 4]; 4],
    color: [f32; 4],
}

struct State {
    pipeline: Pipeline,
    ball_vb: VertexBuffer,
    ball_ib: IndexBuffer,
    ball_uniform: UniformBuffer,
    floor_vb: VertexBuffer,
    floor_ib: IndexBuffer,
    floor_uniform: UniformBuffer,
    world: World,
    ball_handle: RigidBodyHandle,
    ortho: Mat4,
}

fn circle_mesh(radius: f32, segments: u32) -> (Vec<Vert>, Vec<u32>) {
    let mut verts = vec![Vert {
        position: [0.0, 0.0],
    }];
    for i in 0..=segments {
        let a = i as f32 / segments as f32 * TAU;
        verts.push(Vert {
            position: [radius * a.cos(), radius * a.sin()],
        });
    }
    let mut idx = Vec::new();
    for i in 0..segments {
        idx.extend_from_slice(&[0, i + 1, i + 2]);
    }
    (verts, idx)
}

fn rect_mesh(w: f32, h: f32) -> (Vec<Vert>, Vec<u32>) {
    let (hw, hh) = (w / 2.0, h / 2.0);
    let verts = vec![
        Vert {
            position: [-hw, -hh],
        },
        Vert {
            position: [hw, -hh],
        },
        Vert { position: [hw, hh] },
        Vert {
            position: [-hw, hh],
        },
    ];
    (verts, vec![0, 1, 2, 0, 2, 3])
}

fn build_ortho() -> Mat4 {
    Mat4::orthographic_rh(-6.0, 6.0, -1.0, 11.0, -1.0, 1.0)
}

fn mvp(ortho: Mat4, x: f32, y: f32) -> [[f32; 4]; 4] {
    (ortho * Mat4::from_translation(Vec3::new(x, y, 0.0))).to_cols_array_2d()
}

fn init(ctx: &mut Context) -> State {
    let mut world = World::new(); // gravity (0, -9.81)

    // Fixed floor at y = 0
    let floor_body = RigidBodyBuilder::fixed().build();
    let floor_handle = world.add_body(floor_body);
    world.add_collider(ColliderBuilder::cuboid(5.0, 0.1).build(), floor_handle);

    // Dynamic ball starting at y = 8
    let ball_body = RigidBodyBuilder::dynamic().translation(0.0, 8.0).build();
    let ball_handle = world.add_body(ball_body);
    world.add_collider(
        ColliderBuilder::ball(0.5).restitution(0.7).build(),
        ball_handle,
    );

    let ortho = build_ortho();

    let (ball_v, ball_i) = circle_mesh(0.5, 32);
    let (floor_v, floor_i) = rect_mesh(10.0, 0.2);

    let ball_vb = ctx.create_vertex_buffer(&ball_v);
    let ball_ib = ctx.create_index_buffer(&ball_i);
    let floor_vb = ctx.create_vertex_buffer(&floor_v);
    let floor_ib = ctx.create_index_buffer(&floor_i);

    let ball_uniform = ctx.create_uniform_buffer(&Transform {
        mvp: mvp(ortho, 0.0, 8.0),
        color: [0.4, 0.7, 1.0, 1.0],
    });
    let floor_uniform = ctx.create_uniform_buffer(&Transform {
        mvp: mvp(ortho, 0.0, 0.0),
        color: [0.55, 0.55, 0.55, 1.0],
    });

    let pipeline =
        ctx.create_pipeline(PipelineDescriptor::new(SHADER, Vert::layout()).with_uniform());

    State {
        pipeline,
        ball_vb,
        ball_ib,
        ball_uniform,
        floor_vb,
        floor_ib,
        floor_uniform,
        world,
        ball_handle,
        ortho,
    }
}

fn main() {
    Window::new(Config {
        title: "Physics 2D".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        |state, ctx| {
            state.world.step();
            let pos = state.world.body(state.ball_handle).unwrap().translation();
            ctx.update_uniform_buffer(
                &state.ball_uniform,
                &Transform {
                    mvp: mvp(state.ortho, pos.x, pos.y),
                    color: [0.4, 0.7, 1.0, 1.0],
                },
            );
        },
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);

            pass.set_uniform(0, &state.floor_uniform);
            pass.set_vertex_buffer(0, &state.floor_vb);
            pass.draw_indexed(&state.floor_ib);

            pass.set_uniform(0, &state.ball_uniform);
            pass.set_vertex_buffer(0, &state.ball_vb);
            pass.draw_indexed(&state.ball_ib);
        },
    );
}
