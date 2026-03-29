# nene

소형 2D/3D Rust 게임 엔진. wgpu 기반 렌더러 위에 물리, 애니메이션, ECS, 오디오, 네트워킹 등 게임에 필요한 기능을 제공한다.

## 빠른 시작

```rust
use nene::app::{App, Config, WindowId, run};
use nene::input::Input;
use nene::math::{Mat4, Vec4};
use nene::renderer::{Context, GpuMesh, Material, MaterialBuilder, RenderPass};
use nene::time::Time;

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

## 앱 라이프사이클

```
App::new()            — 순수 초기화 (GPU 없음)
App::window_ready()   — GPU 리소스 생성
loop:
  App::update()       — 게임 로직, 입력 처리
  App::prepare()      — GPU 버퍼 업로드, UI
  App::render()       — 드로우 콜
```

## 모듈

| 모듈 | 설명 |
|------|------|
| `app` | 앱 트레이트, 윈도우 설정, 이벤트 루프 |
| `renderer` | wgpu 컨텍스트, 머티리얼, 메시, 텍스처, 그림자 |
| `camera` | 원근/직교 카메라, 프러스텀 컬링, 레이캐스팅 |
| `math` | glam 재익스포트 (Vec2/3/4, Mat4, Quat 등) |
| `mesh` | Vertex, OBJ/glTF 로더, 절차적 메시 |
| `input` | 키보드/마우스/게임패드, 액션 바인딩 |
| `physics` | Rapier 2D/3D 래퍼 |
| `animation` | 스켈레탈 애니메이션, 상태 머신, 트윈 |
| `picking` | 레이-AABB/구/평면 교차 테스트 |
| `ecs` | 엔티티-컴포넌트-시스템 |
| `scene` | 씬 그래프, 트랜스폼 계층 |
| `particle` | GPU 파티클 시스템 |
| `sprite` | 2D 스프라이트, 스프라이트시트 |
| `audio` | 공간 오디오 (pan, 거리 감쇠) |
| `ui` | egui 통합 |
| `text` | 텍스트 렌더러 (화면 오버레이 / 텍스처) |
| `tilemap` | 타일맵 |
| `ai` | 경로 탐색 (A*) |
| `net` | 멀티플레이어 네트워킹 (tokio) |
| `event` | 타입 안전 이벤트 버스 |
| `time` | 델타타임, 고정 타임스텝, 이징 |
| `asset` | 에셋 로딩, PAK 아카이브 |
| `persist` | 세이브 데이터 직렬화 |
| `locale` | 다국어 지원 |
| `debug` | 프레임 프로파일러 |

## 예제

```bash
cargo run --example <이름>
```

| 예제 | 설명 |
|------|------|
| `egui_demo` | egui 슬라이더, 텍스트 입력, 체크박스 |
| `ui` | UI + 퍼시스턴스 + 로컬라이제이션 |
| `input` | 직접 입력 쿼리와 이벤트 버스 |
| `physics` | 2D/3D 물리 시뮬레이션 (Tab으로 전환) |
| `picking` | 마우스 클릭으로 3D 오브젝트 선택 |
| `state_machine` | 스켈레탈 애니메이션 상태 머신 |
| `fixed_update` | 고정 타임스텝 + 프레임 프로파일러 |
| `instancing` | GPU 인스턴싱 (큐브 2,500개, 1 드로우콜) |
| `sprite` | 스프라이트 + 프러스텀 컬링 (2,000개) |
| `particle` | 파티클 시스템 (불꽃, 스파크) |
| `ecs` | ECS 엔티티/컴포넌트/시스템 |
| `scene` | 씬 그래프 계층 (태양→행성→달) |
| `pathfinding` | A* 경로 탐색 |
| `gltf` | glTF 모델 로딩 + 그림자 매핑 |
| `text_texture` | 텍스트 렌더링 |
| `spatial_audio` | 공간 오디오 |
| `multiplayer_client` | 멀티플레이어 클라이언트 |
| `multiplayer_server` | 멀티플레이어 릴레이 서버 |

## 의존성

- [wgpu](https://github.com/gfx-rs/wgpu) — 크로스 플랫폼 GPU
- [winit](https://github.com/rust-windowing/winit) — 윈도우/이벤트
- [rapier](https://rapier.rs) — 물리 (2D/3D)
- [glam](https://github.com/bitshifter/glam-rs) — 수학
- [egui](https://github.com/emilk/egui) — 즉시 모드 UI
- [tokio](https://tokio.rs) — 비동기 네트워킹
