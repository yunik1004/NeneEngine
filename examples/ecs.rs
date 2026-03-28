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
//! Controls
//! --------
//! WASD / Arrow keys — move player
//! Space             — spawn 10 more enemies

use nene::{
    app::{App, WindowId, run},
    ecs::{Entity, World},
    input::{Input, Key},
    mesh::{ColorMesh, circle},
    renderer::{Context, RenderPass},
    time::Time,
    ui::Ui,
    window::Config,
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
struct Color {
    r: f32,
    g: f32,
    b: f32,
}

// Marker components
struct Player;
struct Enemy;
struct InRange;

// ── App ───────────────────────────────────────────────────────────────────────

struct EcsDemo {
    world: World,
    player: Entity,
    draw: Option<ColorMesh>,
    ui: Option<Ui>,
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
        Color {
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

// ── App impl ──────────────────────────────────────────────────────────────────

impl App for EcsDemo {
    fn new() -> Self {
        let mut world = World::new();
        let player = world.spawn((
            Position {
                x: W / 2.0,
                y: H / 2.0,
            },
            Velocity { x: 0.0, y: 0.0 },
            Radius(14.0),
            Color {
                r: 0.3,
                g: 0.8,
                b: 1.0,
            },
            Player,
        ));
        for i in 0..30 {
            spawn_enemy(&mut world, i * 137 + 42);
        }
        EcsDemo {
            world,
            player,
            draw: None,
            ui: None,
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.draw = Some(ColorMesh::new(ctx));
        self.ui = Some(Ui::new(ctx));
    }

    fn update(&mut self, input: &Input, time: &Time) {
        let dt = time.delta;
        let w = &mut self.world;
        let seed = (time.elapsed * 1_000.0) as u32;

        // Player input
        {
            let vel = w.get_mut::<Velocity>(self.player).unwrap();
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

        // Movement system
        w.view_mut(|_, pos: &mut Position, vel: &Velocity| {
            pos.x = (pos.x + vel.x * dt).clamp(0.0, W);
            pos.y = (pos.y + vel.y * dt).clamp(0.0, H);
        });

        // Seek system
        let (px, py) = {
            let p = w.get::<Position>(self.player).unwrap();
            (p.x, p.y)
        };
        w.view_mut(|_, vel: &mut Velocity, pos: &Position| {
            let dx = px - pos.x;
            let dy = py - pos.y;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            vel.x = dx / len * ENEMY_SPEED;
            vel.y = dy / len * ENEMY_SPEED;
        });

        // Range system
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

        // Attack system
        for hp in w.query_mut::<Health>().with::<InRange>().values_mut() {
            hp.0 -= ATTACK_DPS * dt;
        }

        // Color
        for c in w
            .query_mut::<Color>()
            .with::<Enemy>()
            .with::<InRange>()
            .values_mut()
        {
            *c = Color {
                r: 1.0,
                g: 0.55,
                b: 0.1,
            };
        }
        for c in w
            .query_mut::<Color>()
            .with::<Enemy>()
            .without::<InRange>()
            .values_mut()
        {
            *c = Color {
                r: 0.9,
                g: 0.2,
                b: 0.2,
            };
        }

        // Reap system
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

        // UI
        let Some(ui) = &mut self.ui else { return };
        ui.begin_frame(input, W, H);
        ui.begin_panel("ECS", 10.0, 10.0, 210.0);
        ui.label("Queries");
        ui.separator();
        let total = w.query::<Position>().iter().count();
        let enemies = w.query::<Health>().with::<Enemy>().iter().count();
        let in_range = w.query::<Health>().with::<InRange>().iter().count();
        ui.label_dim(&format!("total entities  {total}"));
        ui.label_dim(&format!("enemies         {enemies}"));
        ui.label_dim(&format!("in range        {in_range}"));
        ui.separator();
        ui.label_dim("WASD   move player");
        ui.label_dim("Space  +10 enemies");
        ui.end_panel();
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        if let Some(draw) = &mut self.draw {
            let ortho = glam::Mat4::orthographic_rh(0.0, W, H, 0.0, -1.0, 1.0);
            draw.set_transform(ctx, ortho);
            let verts: Vec<_> = self
                .world
                .query::<Position>()
                .iter()
                .filter_map(|(e, pos)| {
                    let r = self.world.get::<Radius>(e)?;
                    let c = self.world.get::<Color>(e)?;
                    Some(circle(pos.x, pos.y, r.0, [c.r, c.g, c.b, 1.0]))
                })
                .flatten()
                .collect();
            draw.set_geometry(ctx, &verts);
        }
        if let Some(ui) = &mut self.ui {
            ui.end_frame(ctx);
        }
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        if let Some(draw) = &self.draw {
            draw.render(pass);
        }
        if let Some(ui) = &self.ui {
            ui.render(pass);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "ECS demo  (WASD = move, Space = spawn enemies)",
            width: W as u32,
            height: H as u32,
            ..Config::default()
        }]
    }
}

fn main() {
    run::<EcsDemo>();
}
