/// Load and render a glTF model with diffuse lighting and shadow mapping.
///
/// Usage: cargo run --example gltf -- path/to/model.gltf|glb
/// (argument optional — defaults to a built-in cube)
use nene::{
    app::{App, Config, WindowId, run},
    camera::Camera,
    math::{Mat4, Vec3, Vec4},
    mesh::Model,
    renderer::{
        AmbientLight, Context, DirectionalLight, FilterMode, GpuMesh, HasShadow, HasTexture,
        Material, MaterialBuilder, RenderPass, ShadowMap, Texture,
    },
    time::Time,
};

fn camera_view_proj(aspect: f32) -> Mat4 {
    Camera::perspective(Vec3::new(0.0, 2.0, 6.0), 45.0, 0.1, 100.0).view_proj(aspect)
}

struct GltfDemo {
    model: Model,
    ambient: AmbientLight,
    directional: DirectionalLight,
    angle: f32,
    mat: Option<Material<HasTexture, HasShadow>>,
    meshes: Vec<GpuMesh>,
    textures: Vec<Texture>,
    shadow_map: Option<ShadowMap>,
    transforms: Vec<Mat4>,
}

impl App for GltfDemo {
    fn new() -> Self {
        let path = match std::env::args().nth(1) {
            Some(p) => p,
            None => {
                let tmp = write_sample_gltf();
                tmp.to_str().unwrap().to_string()
            }
        };
        GltfDemo {
            model: Model::load(&path).expect("failed to load model"),
            ambient: AmbientLight::new(Vec3::ONE, 0.15),
            directional: DirectionalLight::new(
                Vec3::new(1.0, -2.0, -1.0),
                Vec3::new(1.0, 0.95, 0.9),
                1.0,
            ),
            angle: 0.0,
            mat: None,
            meshes: Vec::new(),
            textures: Vec::new(),
            shadow_map: None,
            transforms: Vec::new(),
        }
    }

    fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
        self.mat = Some(
            MaterialBuilder::new()
                .ambient()
                .directional()
                .texture()
                .shadow()
                .casts_shadow()
                .build(ctx),
        );
        self.shadow_map = Some(ctx.create_shadow_map(1024));

        for mesh in self.model.meshes.iter().filter(|m| !m.skinned) {
            self.meshes.push(GpuMesh::from_mesh(ctx, mesh));
            self.textures.push(match &mesh.base_color {
                Some(img) => {
                    ctx.create_texture_with(img.width, img.height, &img.data, FilterMode::Linear)
                }
                None => ctx.create_texture_with(1, 1, &[255, 255, 255, 255], FilterMode::Nearest),
            });
            self.transforms.push(mesh.transform);
        }
    }

    fn update(&mut self, _input: &nene::input::Input, time: &Time) {
        self.angle += std::f32::consts::TAU * 0.1 * time.delta;
    }

    fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &nene::input::Input) {
        let (Some(mat), Some(shadow_map)) = (&mut self.mat, &self.shadow_map) else {
            return;
        };
        let cfg = ctx.surface_config();
        let aspect = cfg.width as f32 / cfg.height as f32;
        let vp = camera_view_proj(aspect);
        let light_vp = self.directional.light_view_proj(Vec3::ZERO, 5.0);
        let rot = Mat4::from_rotation_y(self.angle);

        for i in 0..self.meshes.len() {
            mat.uniform.view_proj = vp;
            mat.uniform.model = rot * self.transforms[i];
            mat.uniform.light_vp = light_vp;
            mat.uniform.color = Vec4::ONE;
            mat.uniform.ambient = self.ambient;
            mat.uniform.directional = self.directional;
            mat.flush(ctx);
            ctx.shadow_pass(shadow_map, |pass| mat.shadow_draw(pass, &self.meshes[i]));
        }
    }

    fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
        let (Some(mat), Some(shadow_map)) = (&self.mat, &self.shadow_map) else {
            return;
        };
        for i in 0..self.meshes.len() {
            mat.render(pass, &self.meshes[i], &self.textures[i], shadow_map);
        }
    }

    fn windows() -> Vec<Config> {
        vec![Config {
            title: "glTF",
            ..Config::default()
        }]
    }
}

fn main() {
    run::<GltfDemo>();
}

/// Write a minimal cube glTF (with positions, normals, uvs) to a temp file and return its path.
fn write_sample_gltf() -> std::path::PathBuf {
    let verts: &[[[f32; 3]; 2]; 24] = &[
        // +Z
        [[-0.5, -0.5, 0.5], [0., 0., 1.]],
        [[0.5, -0.5, 0.5], [0., 0., 1.]],
        [[0.5, 0.5, 0.5], [0., 0., 1.]],
        [[-0.5, 0.5, 0.5], [0., 0., 1.]],
        // -Z
        [[0.5, -0.5, -0.5], [0., 0., -1.]],
        [[-0.5, -0.5, -0.5], [0., 0., -1.]],
        [[-0.5, 0.5, -0.5], [0., 0., -1.]],
        [[0.5, 0.5, -0.5], [0., 0., -1.]],
        // +Y
        [[-0.5, 0.5, 0.5], [0., 1., 0.]],
        [[0.5, 0.5, 0.5], [0., 1., 0.]],
        [[0.5, 0.5, -0.5], [0., 1., 0.]],
        [[-0.5, 0.5, -0.5], [0., 1., 0.]],
        // -Y
        [[-0.5, -0.5, -0.5], [0., -1., 0.]],
        [[0.5, -0.5, -0.5], [0., -1., 0.]],
        [[0.5, -0.5, 0.5], [0., -1., 0.]],
        [[-0.5, -0.5, 0.5], [0., -1., 0.]],
        // +X
        [[0.5, -0.5, 0.5], [1., 0., 0.]],
        [[0.5, -0.5, -0.5], [1., 0., 0.]],
        [[0.5, 0.5, -0.5], [1., 0., 0.]],
        [[0.5, 0.5, 0.5], [1., 0., 0.]],
        // -X
        [[-0.5, -0.5, -0.5], [-1., 0., 0.]],
        [[-0.5, -0.5, 0.5], [-1., 0., 0.]],
        [[-0.5, 0.5, 0.5], [-1., 0., 0.]],
        [[-0.5, 0.5, -0.5], [-1., 0., 0.]],
    ];
    let uvs: &[[f32; 2]; 24] = &[
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
        [0., 1.],
        [1., 1.],
        [1., 0.],
        [0., 0.],
    ];
    let indices: Vec<u32> = (0..6u32)
        .flat_map(|f| {
            let b = f * 4;
            [b, b + 1, b + 2, b, b + 2, b + 3]
        })
        .collect();

    let mut buf: Vec<u8> = Vec::new();
    let pos_off = buf.len();
    for v in verts {
        for &f in &v[0] {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let pos_len = buf.len() - pos_off;
    let nor_off = buf.len();
    for v in verts {
        for &f in &v[1] {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let nor_len = buf.len() - nor_off;
    let uv_off = buf.len();
    for uv in uvs {
        for &f in uv {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let uv_len = buf.len() - uv_off;
    let idx_off = buf.len();
    for &i in &indices {
        buf.extend_from_slice(&i.to_le_bytes());
    }
    let idx_len = buf.len() - idx_off;

    let b64 = base64_encode(&buf);
    let total = buf.len();
    let json = format!(
        r#"{{
  "asset": {{"version":"2.0"}},
  "scene": 0,
  "scenes": [{{"nodes":[0]}}],
  "nodes": [{{"mesh":0}}],
  "meshes": [{{"primitives":[{{"attributes":{{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2}},"indices":3}}]}}],
  "accessors": [
    {{"bufferView":0,"componentType":5126,"count":24,"type":"VEC3","min":[-0.5,-0.5,-0.5],"max":[0.5,0.5,0.5]}},
    {{"bufferView":1,"componentType":5126,"count":24,"type":"VEC3"}},
    {{"bufferView":2,"componentType":5126,"count":24,"type":"VEC2"}},
    {{"bufferView":3,"componentType":5125,"count":36,"type":"SCALAR"}}
  ],
  "bufferViews": [
    {{"buffer":0,"byteOffset":{pos_off},"byteLength":{pos_len}}},
    {{"buffer":0,"byteOffset":{nor_off},"byteLength":{nor_len}}},
    {{"buffer":0,"byteOffset":{uv_off}, "byteLength":{uv_len}}},
    {{"buffer":0,"byteOffset":{idx_off},"byteLength":{idx_len}}}
  ],
  "buffers": [{{"byteLength":{total},"uri":"data:application/octet-stream;base64,{b64}"}}]
}}"#
    );
    let path = std::env::temp_dir().join("nene_sample_cube.gltf");
    std::fs::write(&path, json).unwrap();
    path
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 0x3f) as usize] as char);
        out.push(T[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}
