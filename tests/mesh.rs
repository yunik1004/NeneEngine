use nene::math::Mat4;
use nene::mesh::{MeshVertex, Model};

fn write_temp_obj(name: &str, content: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// Build a minimal glTF JSON with an embedded base64 buffer and write to disk.
fn write_triangle_gltf(name: &str) -> std::path::PathBuf {
    // positions: 3 vertices * 3 floats * 4 bytes = 36 bytes
    let positions: &[[f32; 3]] = &[[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]];
    // normals: same layout
    let normals: &[[f32; 3]] = &[[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
    // indices: 3 * u16 = 6 bytes
    let indices: &[u16] = &[0, 1, 2];

    let mut buf: Vec<u8> = Vec::new();
    for v in positions {
        for &f in v {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let normals_offset = buf.len();
    for v in normals {
        for &f in v {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let indices_offset = buf.len();
    for &i in indices {
        buf.extend_from_slice(&i.to_le_bytes());
    }

    let b64 = base64_encode(&buf);
    let total = buf.len();
    let pos_len = normals_offset;
    let nor_len = indices_offset - normals_offset;
    let idx_len = total - indices_offset;

    let json = format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "scene": 0,
  "scenes": [{{"nodes": [0]}}],
  "nodes": [{{"mesh": 0}}],
  "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0, "NORMAL": 1}}, "indices": 2}}]}}],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
      "min": [-0.5,-0.5,0.0], "max": [0.5,0.5,0.0]}},
    {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"}},
    {{"bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR"}}
  ],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0,              "byteLength": {pos_len}}},
    {{"buffer": 0, "byteOffset": {normals_offset}, "byteLength": {nor_len}}},
    {{"buffer": 0, "byteOffset": {indices_offset}, "byteLength": {idx_len}}}
  ],
  "buffers": [{{"byteLength": {total}, "uri": "data:application/octet-stream;base64,{b64}"}}]
}}"#
    );

    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, json).unwrap();
    path
}

/// Build a glTF where the node has a translation of (1, 2, 3).
fn write_translated_gltf(name: &str) -> std::path::PathBuf {
    let positions: &[[f32; 3]] = &[[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]];
    let indices: &[u16] = &[0, 1, 2];

    let mut buf: Vec<u8> = Vec::new();
    for v in positions {
        for &f in v {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let indices_offset = buf.len();
    for &i in indices {
        buf.extend_from_slice(&i.to_le_bytes());
    }

    let b64 = base64_encode(&buf);
    let total = buf.len();
    let pos_len = indices_offset;
    let idx_len = total - indices_offset;

    // translation column-major mat4: identity + translate(1,2,3)
    let json = format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "scene": 0,
  "scenes": [{{"nodes": [0]}}],
  "nodes": [{{"mesh": 0, "translation": [1.0, 2.0, 3.0]}}],
  "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}, "indices": 1}}]}}],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
      "min": [-0.5,-0.5,0.0], "max": [0.5,0.5,0.0]}},
    {{"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}}
  ],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0,              "byteLength": {pos_len}}},
    {{"buffer": 0, "byteOffset": {indices_offset}, "byteLength": {idx_len}}}
  ],
  "buffers": [{{"byteLength": {total}, "uri": "data:application/octet-stream;base64,{b64}"}}]
}}"#
    );

    let path = std::env::temp_dir().join(name);
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

const TRIANGLE_OBJ: &str = "\
v 0.0  0.5 0.0
v -0.5 -0.5 0.0
v  0.5 -0.5 0.0
vn 0.0 0.0 1.0
vt 0.5 1.0
vt 0.0 0.0
vt 1.0 0.0
f 1/1/1 2/2/1 3/3/1
";

const QUAD_OBJ: &str = "\
v -0.5  0.5 0.0
v -0.5 -0.5 0.0
v  0.5 -0.5 0.0
v  0.5  0.5 0.0
vn 0.0 0.0 1.0
f 1//1 2//1 3//1
f 1//1 3//1 4//1
";

#[test]
fn mesh_vertex_layout() {
    let layout = MeshVertex::layout();
    assert_eq!(layout.stride, std::mem::size_of::<MeshVertex>() as u64);
    assert_eq!(layout.attributes.len(), 3);
    assert_eq!(layout.attributes[0].location, 0); // position
    assert_eq!(layout.attributes[1].location, 1); // normal
    assert_eq!(layout.attributes[2].location, 2); // uv
}

#[test]
fn mesh_vertex_size() {
    // position [f32;3] + normal [f32;3] + uv [f32;2] = 8 * 4 = 32 bytes
    assert_eq!(std::mem::size_of::<MeshVertex>(), 32);
}

#[test]
fn load_obj_triangle() {
    let path = write_temp_obj("nene_test_triangle.obj", TRIANGLE_OBJ);
    let model = Model::load(&path);
    assert_eq!(model.meshes.len(), 1);
    let mesh = &model.meshes[0];
    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.indices.len(), 3);
}

#[test]
fn load_obj_quad() {
    let path = write_temp_obj("nene_test_quad.obj", QUAD_OBJ);
    let model = Model::load(&path);
    assert_eq!(model.meshes.len(), 1);
    let mesh = &model.meshes[0];
    assert_eq!(mesh.indices.len(), 6); // 2 triangles
}

#[test]
fn load_obj_positions_correct() {
    let path = write_temp_obj("nene_test_tri_pos.obj", TRIANGLE_OBJ);
    let model = Model::load(&path);
    let mesh = &model.meshes[0];

    let positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.position).collect();
    assert!(positions.contains(&[0.0, 0.5, 0.0]));
    assert!(positions.contains(&[-0.5, -0.5, 0.0]));
    assert!(positions.contains(&[0.5, -0.5, 0.0]));
}

#[test]
fn load_obj_normals_fallback() {
    // OBJ without normals — should default to [0, 1, 0]
    let obj = "v 0.0 0.5 0.0\nv -0.5 -0.5 0.0\nv 0.5 -0.5 0.0\nf 1 2 3\n";
    let path = write_temp_obj("nene_test_no_normals.obj", obj);
    let model = Model::load(&path);
    let mesh = &model.meshes[0];
    for v in &mesh.vertices {
        assert_eq!(v.normal, [0.0, 1.0, 0.0]);
    }
}

#[test]
fn load_obj_uv_fallback() {
    // OBJ without UVs — should default to [0, 0]
    let obj = "v 0.0 0.5 0.0\nv -0.5 -0.5 0.0\nv 0.5 -0.5 0.0\nf 1 2 3\n";
    let path = write_temp_obj("nene_test_no_uvs.obj", obj);
    let model = Model::load(&path);
    let mesh = &model.meshes[0];
    for v in &mesh.vertices {
        assert_eq!(v.uv, [0.0, 0.0]);
    }
}

#[test]
fn model_multiple_meshes() {
    let obj = "\
o MeshA
v 0.0  0.5 0.0
v -0.5 -0.5 0.0
v  0.5 -0.5 0.0
f 1 2 3
o MeshB
v 0.0  0.5 1.0
v -0.5 -0.5 1.0
v  0.5 -0.5 1.0
f 4 5 6
";
    let path = write_temp_obj("nene_test_multi_mesh.obj", obj);
    let model = Model::load(&path);
    assert_eq!(model.meshes.len(), 2);
}

#[test]
fn load_gltf_triangle() {
    let path = write_triangle_gltf("nene_test_triangle.gltf");
    let model = Model::load(&path);
    assert_eq!(model.meshes.len(), 1);
    let mesh = &model.meshes[0];
    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.indices.len(), 3);
}

#[test]
fn load_gltf_positions_correct() {
    let path = write_triangle_gltf("nene_test_tri_pos.gltf");
    let model = Model::load(&path);
    let mesh = &model.meshes[0];
    let positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.position).collect();
    assert!(positions.contains(&[-0.5, -0.5, 0.0]));
    assert!(positions.contains(&[0.5, -0.5, 0.0]));
    assert!(positions.contains(&[0.0, 0.5, 0.0]));
}

#[test]
fn load_gltf_normals_correct() {
    let path = write_triangle_gltf("nene_test_tri_normals.gltf");
    let model = Model::load(&path);
    let mesh = &model.meshes[0];
    for v in &mesh.vertices {
        assert_eq!(v.normal, [0.0, 0.0, 1.0]);
    }
}

#[test]
fn load_gltf_uv_fallback() {
    // Our test glTF has no UVs — should default to [0, 0]
    let path = write_triangle_gltf("nene_test_tri_uv.gltf");
    let model = Model::load(&path);
    let mesh = &model.meshes[0];
    for v in &mesh.vertices {
        assert_eq!(v.uv, [0.0, 0.0]);
    }
}

#[test]
fn load_gltf_indices_correct() {
    let path = write_triangle_gltf("nene_test_tri_idx.gltf");
    let model = Model::load(&path);
    let mesh = &model.meshes[0];
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
fn load_obj_transform_is_identity() {
    let path = write_temp_obj("nene_test_obj_transform.obj", TRIANGLE_OBJ);
    let model = Model::load(&path);
    assert_eq!(model.meshes[0].transform, Mat4::IDENTITY);
}

#[test]
fn load_obj_no_base_color() {
    let path = write_temp_obj("nene_test_obj_color.obj", TRIANGLE_OBJ);
    let model = Model::load(&path);
    assert!(model.meshes[0].base_color.is_none());
}

#[test]
fn load_gltf_transform_is_identity() {
    let path = write_triangle_gltf("nene_test_gltf_identity.gltf");
    let model = Model::load(&path);
    assert_eq!(model.meshes[0].transform, Mat4::IDENTITY);
}

#[test]
fn load_gltf_transform_node_translation() {
    let path = write_translated_gltf("nene_test_gltf_translated.gltf");
    let model = Model::load(&path);
    let t = model.meshes[0].transform;
    // translation column should be (1, 2, 3, 1)
    assert_eq!(t.w_axis.x, 1.0);
    assert_eq!(t.w_axis.y, 2.0);
    assert_eq!(t.w_axis.z, 3.0);
}

#[test]
fn load_gltf_no_base_color() {
    let path = write_triangle_gltf("nene_test_gltf_no_tex.gltf");
    let model = Model::load(&path);
    assert!(model.meshes[0].base_color.is_none());
}

#[test]
fn mesh_bytemuck_pod() {
    // Verify MeshVertex is usable as a byte slice (Pod)
    let v = MeshVertex {
        position: [1.0, 2.0, 3.0],
        normal: [0.0, 1.0, 0.0],
        uv: [0.5, 0.5],
    };
    let _bytes: &[u8] = bytemuck::bytes_of(&v);
}
