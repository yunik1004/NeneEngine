/// Scene-graph demo: sun → planet → moon hierarchy.
///
/// The sun rotates in place; the planet orbits the sun; the moon orbits the planet.
/// Each body's world transform is read from the scene graph and uploaded to its
/// own uniform buffer in the update phase.
use nene::{
    camera::Camera,
    math::{Mat4, Quat, Vec3, Vec4},
    mesh::MeshVertex,
    renderer::{
        Context, IndexBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer, VertexBuffer,
    },
    scene::{Node, NodeId, Scene, Transform},
    uniform,
    window::{Config, Window},
};

const SHADER: &str = r#"
struct U {
    view_proj: mat4x4<f32>,
    model:     mat4x4<f32>,
    color:     vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;

struct VO {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) pos: vec3<f32>,
    @location(1) nor: vec3<f32>,
    @location(2) _uv: vec2<f32>,
) -> VO {
    var o: VO;
    o.clip   = u.view_proj * u.model * vec4<f32>(pos, 1.0);
    o.normal = normalize((u.model * vec4<f32>(nor, 0.0)).xyz);
    return o;
}

@fragment
fn fs_main(in: VO) -> @location(0) vec4<f32> {
    let sun_dir = normalize(vec3<f32>(1.0, 1.5, 1.0));
    let diff    = max(dot(in.normal, sun_dir), 0.15);
    return vec4<f32>(u.color.rgb * diff, 1.0);
}
"#;

#[uniform]
struct U {
    view_proj: Mat4,
    model: Mat4,
    color: Vec4,
}

fn cube() -> (Vec<MeshVertex>, Vec<u32>) {
    let p: [[f32; 3]; 8] = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let n: [[f32; 3]; 6] = [
        [0., 0., -1.],
        [0., 0., 1.],
        [-1., 0., 0.],
        [1., 0., 0.],
        [0., -1., 0.],
        [0., 1., 0.],
    ];
    let faces: [([usize; 4], usize); 6] = [
        ([0, 1, 2, 3], 0),
        ([5, 4, 7, 6], 1),
        ([4, 0, 3, 7], 2),
        ([1, 5, 6, 2], 3),
        ([4, 5, 1, 0], 4),
        ([3, 2, 6, 7], 5),
    ];
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for (quad, ni) in faces {
        let base = verts.len() as u32;
        for &pi in &quad {
            verts.push(MeshVertex {
                position: p[pi],
                normal: n[ni],
                uv: [0., 0.],
            });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

struct Body {
    id: NodeId,
    ubuf: UniformBuffer,
    color: Vec4,
    scale: f32,
    angle: f32,
    speed: f32,
    orbit_radius: f32,
}

struct State {
    pipeline: Pipeline,
    vbuf: VertexBuffer,
    ibuf: IndexBuffer,
    scene: Scene,
    bodies: [Body; 3],
    camera: Camera,
}

fn init(ctx: &mut Context) -> State {
    let (verts, indices) = cube();
    let vbuf = ctx.create_vertex_buffer(&verts);
    let ibuf = ctx.create_index_buffer(&indices);

    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER, MeshVertex::layout())
            .with_uniform()
            .with_depth(),
    );

    let blank = U {
        view_proj: Mat4::IDENTITY,
        model: Mat4::IDENTITY,
        color: Vec4::ONE,
    };

    let mut scene = Scene::new();
    let sun = scene.add_node(Node::named("sun"));
    let planet = scene.add_child(
        sun,
        Node::named("planet").with_transform(Transform::from_position(Vec3::new(3.5, 0., 0.))),
    );
    let moon = scene.add_child(
        planet,
        Node::named("moon").with_transform(Transform::from_position(Vec3::new(1.4, 0., 0.))),
    );
    scene.update();

    let bodies = [
        Body {
            id: sun,
            ubuf: ctx.create_uniform_buffer(&blank),
            color: Vec4::new(1.0, 0.85, 0.1, 1.),
            scale: 0.9,
            angle: 0.,
            speed: 0.4,
            orbit_radius: 0.,
        },
        Body {
            id: planet,
            ubuf: ctx.create_uniform_buffer(&blank),
            color: Vec4::new(0.2, 0.5, 1.0, 1.),
            scale: 0.5,
            angle: 0.,
            speed: 1.1,
            orbit_radius: 3.5,
        },
        Body {
            id: moon,
            ubuf: ctx.create_uniform_buffer(&blank),
            color: Vec4::new(0.8, 0.8, 0.8, 1.),
            scale: 0.22,
            angle: 0.,
            speed: 2.8,
            orbit_radius: 1.4,
        },
    ];

    State {
        pipeline,
        vbuf,
        ibuf,
        scene,
        bodies,
        camera: Camera::perspective(Vec3::new(0., 5., 12.), 45., 0.1, 100.),
    }
}

fn update(
    state: &mut State,
    ctx: &mut Context,
    _input: &nene::input::Input,
    time: &nene::time::Time,
) {
    let dt = time.delta;

    for body in &mut state.bodies {
        body.angle += dt * body.speed;
    }

    // Sun rotates in place; planet and moon orbit their parent via local position.
    state.scene.get_mut(state.bodies[0].id).transform.rotation =
        Quat::from_rotation_y(state.bodies[0].angle);
    for i in 1..3 {
        let r = state.bodies[i].orbit_radius;
        state.scene.get_mut(state.bodies[i].id).transform.position =
            Quat::from_rotation_y(state.bodies[i].angle) * Vec3::new(r, 0., 0.);
    }

    state.scene.update();

    let cfg = ctx.surface_config();
    let aspect = cfg.width as f32 / cfg.height as f32;
    let vp = state.camera.view_proj(aspect);

    for body in &state.bodies {
        let model =
            state.scene.get(body.id).world_transform() * Mat4::from_scale(Vec3::splat(body.scale));
        ctx.update_uniform_buffer(
            &body.ubuf,
            &U {
                view_proj: vp,
                model,
                color: body.color,
            },
        );
    }
}

fn main() {
    Window::new(Config {
        title: "Scene Graph — sun / planet / moon".to_string(),
        ..Config::default()
    })
    .run_with_update(
        init,
        update,
        |_, _| {},
        |state, pass: &mut RenderPass| {
            pass.set_pipeline(&state.pipeline);
            pass.set_vertex_buffer(0, &state.vbuf);
            for body in &state.bodies {
                pass.set_uniform(0, &body.ubuf);
                pass.draw_indexed(&state.ibuf);
            }
        },
    );
}
