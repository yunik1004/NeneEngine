# nene

A small 2D/3D game engine in Rust. Built on a wgpu renderer with physics, animation, ECS, audio, networking, and more.

## Quick start

```rust
use nene::app::{App, Config, WindowId, run};
use nene::input::Input;
use nene::math::Mat4;
use nene::renderer::{Context, GpuMesh, Material, MaterialBuilder, RenderPass};

struct MyGame {
    mat: Option<Material>,
    mesh: Option<GpuMesh>,
}

impl App for MyGame {
    fn new() -> Self {
        MyGame { mat: None, mesh: None }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.mat = Some(MaterialBuilder::new().lights().build(ctx));
        self.mesh = Some(GpuMesh::new(ctx, &[], &[]));
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &Input) {
        let Some(mat) = &mut self.mat else { return };
        mat.uniform.view_proj = Mat4::IDENTITY;
        mat.flush(ctx);
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        let (Some(mat), Some(mesh)) = (&self.mat, &self.mesh) else { return };
        mat.render(pass, mesh);
    }

    fn windows() -> Vec<Config> {
        vec![Config { title: "My Game", ..Config::default() }]
    }
}

fn main() { run::<MyGame>(); }
```

## App lifecycle

```
App::new()            — pure init, no GPU
App::window_ready()   — create GPU resources
loop:
  App::update()       — game logic, input
  App::prepare()      — upload GPU buffers, build UI
  App::render()       — draw calls
```

## Modules

| Module | Description |
|--------|-------------|
| `app` | App trait, window config, event loop |
| `renderer` | wgpu context, materials, meshes, textures, shadows |
| `camera` | Perspective/orthographic camera, frustum culling, ray casting |
| `math` | glam re-exports (Vec2/3/4, Mat4, Quat, …) |
| `mesh` | Vertex type, OBJ/glTF loader, procedural meshes |
| `input` | Keyboard, mouse, gamepad, action bindings |
| `physics` | Rapier 2D/3D wrapper |
| `animation` | Skeletal animation, state machine, tweening |
| `picking` | Ray–AABB / ray–sphere / ray–plane intersection |
| `ecs` | Entity-component-system |
| `scene` | Scene graph, transform hierarchy |
| `particle` | GPU particle system |
| `sprite` | 2D sprites and spritesheets |
| `audio` | Spatial audio (pan, distance attenuation) |
| `ui` | egui integration |
| `text` | Text renderer (screen overlay and texture) |
| `tilemap` | Tile map |
| `ai` | Pathfinding (A*) |
| `net` | Multiplayer networking (tokio) |
| `event` | Type-safe event bus |
| `time` | Delta time, fixed timestep, easing |
| `asset` | Asset loading, PAK archive |
| `persist` | Save data serialization |
| `locale` | Localization |
| `debug` | Frame profiler |

## Examples

```bash
cargo run --example <name>
```

| Example | Description |
|---------|-------------|
| `egui_demo` | egui slider, text input, checkbox |
| `ui` | UI + persistence + localization |
| `input` | Direct input queries and event bus |
| `physics` | 2D/3D physics simulation (Tab to switch) |
| `picking` | Click to select 3D objects with mouse ray casting |
| `state_machine` | Skeletal animation state machine with crossfade |
| `fixed_update` | Fixed timestep + frame profiler |
| `instancing` | GPU instancing — 2,500 cubes in one draw call |
| `sprite` | Sprites + frustum culling (2,000 sprites) |
| `particle` | Particle system (fire column, spark burst) |
| `ecs` | Entities, components, and systems |
| `scene` | Scene graph hierarchy (sun → planet → moon) |
| `pathfinding` | A* pathfinding on a tile map |
| `gltf` | glTF model loading + shadow mapping |
| `text_texture` | Text rendering (overlay and texture) |
| `spatial_audio` | Positional audio with pan and attenuation |
| `multiplayer_client` | Multiplayer game client |
| `multiplayer_server` | Multiplayer relay server |

## Dependencies

- [wgpu](https://github.com/gfx-rs/wgpu) — cross-platform GPU
- [winit](https://github.com/rust-windowing/winit) — windowing and events
- [rapier](https://rapier.rs) — physics (2D/3D)
- [glam](https://github.com/bitshifter/glam-rs) — math
- [egui](https://github.com/emilk/egui) — immediate mode UI
- [tokio](https://tokio.rs) — async networking
