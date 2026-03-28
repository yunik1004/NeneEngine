//! ECS demo — entities, components, and filtered queries.
//!
//! Spawns a player and a crowd of enemies. Each frame:
//!
//!  1. **Movement system** — entities with `Position` + `Velocity` move.
//!  2. **Seek system**     — enemies steer toward the player.
//!  3. **Range system**    — nearby enemies gain an `InRange` marker.
//!  4. **Attack system**   — only `InRange` enemies lose health (filtered query).
//!  5. **Reap system**     — dead enemies are despawned.
//!
//! The UI panel shows live counts, demonstrating filtered vs unfiltered queries.
//!
//! Controls
//! --------
//! WASD / Arrow keys — move player
//! Space             — spawn 10 more enemies

use nene::vertex;
use nene::{
    ecs::{Entity, World},
    input::{Input, Key},
    renderer::{
        Context, InstanceBuffer, Pipeline, PipelineDescriptor, RenderPass, UniformBuffer,
        VertexBuffer,
    },
    time::Time,
    ui::Ui,
    uniform,
    window::{Config, Window},
};

// ── Constants ─────────────────────────────────────────────────────────────────

const W: f32 = 900.0;
const H: f32 = 600.0;
const PLAYER_SPEED: f32 = 180.0;
const ENEMY_SPEED: f32 = 60.0;
const ATTACK_RANGE: f32 = 80.0;
const ATTACK_DPS: f32 = 30.0;
const ENEMY_HP: f32 = 100.0;

// ── Components ────────────────────────────────────────────────────────────────

struct Position {
    x: f32,
    y: f32,
}
struct Velocity {
    x: f32,
    y: f32,
}
struct Health(f32);
struct Radius(f32);
struct DrawColor {
    r: f32,
    g: f32,
    b: f32,
}

// Marker components — zero-size, used as query filters
struct Player;
struct Enemy;
struct InRange;

// ── Shader ────────────────────────────────────────────────────────────────────

const SHADER: &str = r#"
struct Scene { half_w: f32, half_h: f32 }
@group(0) @binding(0) var<uniform> scene: Scene;

// Per-vertex (unit quad, slot 0)
struct VIn {
    @location(0) local: vec2<f32>,
    // Per-instance (slot 1): pos_r = (x, y, radius, _), slot 2: color
    @location(1) pos_r: vec4<f32>,
    @location(2) color: vec3<f32>,
}
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv:    vec2<f32>,
    @location(1) color: vec3<f32>,
}

@vertex fn vs_main(v: VIn) -> VOut {
    let world = v.pos_r.xy + v.local * v.pos_r.z;
    let ndc   = vec2(world.x / scene.half_w, -world.y / scene.half_h);
    return VOut(vec4(ndc, 0.0, 1.0), v.local, v.color);
}

@fragment fn fs_main(v: VOut) -> @location(0) vec4<f32> {
    if dot(v.uv, v.uv) > 1.0 { discard; }
    return vec4(v.color, 1.0);
}
"#;

// ── GPU vertex / uniform types ────────────────────────────────────────────────

#[vertex]
struct QuadVtx {
    local: [f32; 2], // location 0
}

/// Per-instance: pos (xy), radius packed into z, color rgb.
/// `radius` is in `pos_r[2]` so we can use a supported array type.
#[vertex]
struct InstVtx {
    pos_r: [f32; 4], // location 1 — .xy = position, .z = radius, .w unused
    color: [f32; 3], // location 2
}

#[uniform]
struct SceneUniform {
    half_w: f32,
    half_h: f32,
}

// ── Quad geometry (unit circle bounding quad) ─────────────────────────────────

fn quad_verts() -> Vec<QuadVtx> {
    vec![
        QuadVtx { local: [-1.0, 1.0] },
        QuadVtx {
            local: [-1.0, -1.0],
        },
        QuadVtx { local: [1.0, -1.0] },
        QuadVtx { local: [-1.0, 1.0] },
        QuadVtx { local: [1.0, -1.0] },
        QuadVtx { local: [1.0, 1.0] },
    ]
}

// ── App state ─────────────────────────────────────────────────────────────────

struct State {
    world: World,
    player: Entity,
    ui: Ui,
    pipeline: Pipeline,
    scene_buf: UniformBuffer,
    quad_buf: VertexBuffer,
    inst_buf: InstanceBuffer,
    inst_cap: usize,
    inst_data: Vec<InstVtx>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn spawn_enemy(world: &mut World, seed: u32) {
    let frac = (seed % 1000) as f32 / 1000.0;
    let (x, y) = match (seed / 1000) % 4 {
        0 => (frac * W, 0.0),
        1 => (W, frac * H),
        2 => (frac * W, H),
        _ => (0.0, frac * H),
    };
    world.spawn((
        Position { x, y },
        Velocity { x: 0.0, y: 0.0 },
        Health(ENEMY_HP),
        Radius(10.0),
        DrawColor {
            r: 0.9,
            g: 0.2,
            b: 0.2,
        },
        Enemy,
    ));
}

fn dist2(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    (ax - bx).powi(2) + (ay - by).powi(2)
}

// ── Init ──────────────────────────────────────────────────────────────────────

fn init(ctx: &mut Context) -> State {
    let scene_buf = ctx.create_uniform_buffer(&SceneUniform {
        half_w: W / 2.0,
        half_h: H / 2.0,
    });
    let quad_buf = ctx.create_vertex_buffer(&quad_verts());
    const INIT_CAP: usize = 256;
    let inst_buf = ctx.create_instance_buffer::<InstVtx>(&vec![
        InstVtx {
            pos_r: [0.0; 4],
            color: [0.0; 3]
        };
        INIT_CAP
    ]);

    let pipeline = ctx.create_pipeline(
        PipelineDescriptor::new(SHADER, QuadVtx::layout())
            .with_instance_layout(InstVtx::layout().at_locations(1))
            .with_uniform(),
    );

    let mut world = World::new();
    let player = world.spawn((
        Position {
            x: W / 2.0,
            y: H / 2.0,
        },
        Velocity { x: 0.0, y: 0.0 },
        Radius(14.0),
        DrawColor {
            r: 0.3,
            g: 0.8,
            b: 1.0,
        },
        Player,
    ));
    for i in 0..30 {
        spawn_enemy(&mut world, i * 137 + 42);
    }

    State {
        world,
        player,
        ui: Ui::new(ctx),
        pipeline,
        scene_buf,
        quad_buf,
        inst_buf,
        inst_cap: INIT_CAP,
        inst_data: Vec::new(),
    }
}

// ── Update ────────────────────────────────────────────────────────────────────

fn update(state: &mut State, ctx: &mut Context, input: &Input, time: &Time) {
    let dt = time.delta;
    let w = &mut state.world;
    let seed = (time.elapsed * 1_000.0) as u32;

    // ── Player input ──────────────────────────────────────────────────────────
    {
        let vel = w.get_mut::<Velocity>(state.player).unwrap();
        vel.x = 0.0;
        vel.y = 0.0;
        if input.key_down(Key::KeyA) || input.key_down(Key::ArrowLeft) {
            vel.x -= PLAYER_SPEED;
        }
        if input.key_down(Key::KeyD) || input.key_down(Key::ArrowRight) {
            vel.x += PLAYER_SPEED;
        }
        if input.key_down(Key::KeyW) || input.key_down(Key::ArrowUp) {
            vel.y -= PLAYER_SPEED;
        }
        if input.key_down(Key::KeyS) || input.key_down(Key::ArrowDown) {
            vel.y += PLAYER_SPEED;
        }
    }

    if input.key_pressed(Key::Space) {
        for i in 0..10 {
            spawn_enemy(w, seed.wrapping_add(i * 73));
        }
    }

    // ── Movement system ───────────────────────────────────────────────────────
    w.view_mut(|_, pos: &mut Position, vel: &Velocity| {
        pos.x = (pos.x + vel.x * dt).clamp(0.0, W);
        pos.y = (pos.y + vel.y * dt).clamp(0.0, H);
    });

    // ── Seek system — enemies steer toward player ─────────────────────────────
    let (px, py) = {
        let p = w.get::<Position>(state.player).unwrap();
        (p.x, p.y)
    };
    w.view_mut(|_, vel: &mut Velocity, pos: &Position| {
        let dx = px - pos.x;
        let dy = py - pos.y;
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        vel.x = dx / len * ENEMY_SPEED;
        vel.y = dy / len * ENEMY_SPEED;
    });

    // ── Range system — tag/untag InRange marker ───────────────────────────────
    let mut in_range: Vec<Entity> = Vec::new();
    let mut out_range: Vec<Entity> = Vec::new();
    w.view(|e, pos: &Position, _: &Enemy| {
        if dist2(pos.x, pos.y, px, py) < ATTACK_RANGE * ATTACK_RANGE {
            in_range.push(e);
        } else {
            out_range.push(e);
        }
    });
    for e in in_range {
        w.insert(e, InRange);
    }
    for e in out_range {
        w.remove::<InRange>(e);
    }

    // ── Attack system — filtered query: only InRange enemies take damage ──────
    for (_, hp) in w.query_mut::<Health>().with::<InRange>() {
        hp.0 -= ATTACK_DPS * dt;
    }

    // ── Color — orange when InRange, red otherwise ────────────────────────────
    for (_, c) in w.query_mut::<DrawColor>().with::<Enemy>().with::<InRange>() {
        *c = DrawColor {
            r: 1.0,
            g: 0.55,
            b: 0.1,
        };
    }
    for (_, c) in w
        .query_mut::<DrawColor>()
        .with::<Enemy>()
        .without::<InRange>()
    {
        *c = DrawColor {
            r: 0.9,
            g: 0.2,
            b: 0.2,
        };
    }

    // ── Reap system — despawn dead enemies ────────────────────────────────────
    let dead: Vec<Entity> = w
        .query::<Health>()
        .with::<Enemy>()
        .iter()
        .filter(|(_, hp)| hp.0 <= 0.0)
        .map(|(e, _)| e)
        .collect();
    for e in dead {
        w.despawn(e);
    }

    // ── UI ────────────────────────────────────────────────────────────────────
    state.ui.begin_frame(input, W, H);
    state.ui.begin_panel("ECS", 10.0, 10.0, 210.0);
    state.ui.label("Queries");
    state.ui.separator();
    let total = w.query::<Position>().iter().count();
    let enemies = w.query::<Health>().with::<Enemy>().iter().count();
    let in_range = w.query::<Health>().with::<InRange>().iter().count();
    state.ui.label_dim(&format!("total entities  {total}"));
    state.ui.label_dim(&format!("enemies         {enemies}"));
    state.ui.label_dim(&format!("in range        {in_range}"));
    state.ui.separator();
    state.ui.label_dim("WASD   move player");
    state.ui.label_dim("Space  +10 enemies");
    state.ui.end_panel();
    state.ui.end_frame(ctx);

    // ── Build instance data ───────────────────────────────────────────────────
    state.inst_data.clear();
    for (e, pos) in w.query::<Position>().iter() {
        let Some(r) = w.get::<Radius>(e) else {
            continue;
        };
        let Some(c) = w.get::<DrawColor>(e) else {
            continue;
        };
        state.inst_data.push(InstVtx {
            pos_r: [pos.x - W / 2.0, pos.y - H / 2.0, r.0, 0.0],
            color: [c.r, c.g, c.b],
        });
    }
    // Recreate the buffer if capacity is exceeded.
    if state.inst_data.len() > state.inst_cap {
        state.inst_cap = state.inst_data.len() * 2;
        state.inst_buf = ctx.create_instance_buffer::<InstVtx>(&vec![
            InstVtx {
                pos_r: [0.0; 4],
                color: [0.0; 3]
            };
            state.inst_cap
        ]);
    }
    ctx.update_instance_buffer(&state.inst_buf, &state.inst_data);
}

// ── Render ────────────────────────────────────────────────────────────────────

fn render(state: &mut State, pass: &mut RenderPass) {
    pass.set_pipeline(&state.pipeline);
    pass.set_uniform(0, &state.scene_buf);
    pass.set_vertex_buffer(0, &state.quad_buf);
    pass.set_instance_buffer(1, &state.inst_buf);
    pass.draw_instanced(0..6, state.inst_data.len() as u32);
    state.ui.render(pass);
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    Window::new(Config {
        title: "ECS demo  (WASD = move, Space = spawn enemies)".into(),
        width: W as u32,
        height: H as u32,
        ..Config::default()
    })
    .run_with_update(init, update, |_, _| {}, render);
}
