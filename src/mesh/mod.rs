use std::path::Path;

use crate::renderer::{VertexAttribute, VertexFormat, VertexLayout};

/// A single vertex in a mesh.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl MeshVertex {
    pub fn layout() -> VertexLayout {
        use std::mem::offset_of;
        VertexLayout {
            stride: std::mem::size_of::<Self>() as u64,
            attributes: vec![
                VertexAttribute {
                    location: 0,
                    offset: offset_of!(Self, position) as u64,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    location: 1,
                    offset: offset_of!(Self, normal) as u64,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    location: 2,
                    offset: offset_of!(Self, uv) as u64,
                    format: VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// A single mesh primitive (triangles).
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

/// A loaded model, which may contain multiple meshes.
pub struct Model {
    pub meshes: Vec<Mesh>,
}

impl Model {
    /// Load a model from a file. Supports `.obj` and `.gltf`/`.glb`.
    pub fn load(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        match path.extension().and_then(|e| e.to_str()) {
            Some("obj") => load_obj(path),
            Some("gltf") | Some("glb") => load_gltf(path),
            ext => panic!("Unsupported model format: {:?}", ext),
        }
    }
}

fn load_obj(path: &Path) -> Model {
    let (models, _) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )
    .expect("Failed to load OBJ file");

    let meshes = models
        .into_iter()
        .map(|m| {
            let mesh = &m.mesh;
            let vertex_count = mesh.positions.len() / 3;

            let vertices = (0..vertex_count)
                .map(|i| {
                    let position = [
                        mesh.positions[i * 3],
                        mesh.positions[i * 3 + 1],
                        mesh.positions[i * 3 + 2],
                    ];
                    let normal = if mesh.normals.len() >= (i + 1) * 3 {
                        [
                            mesh.normals[i * 3],
                            mesh.normals[i * 3 + 1],
                            mesh.normals[i * 3 + 2],
                        ]
                    } else {
                        [0.0, 1.0, 0.0]
                    };
                    let uv = if mesh.texcoords.len() >= (i + 1) * 2 {
                        [mesh.texcoords[i * 2], 1.0 - mesh.texcoords[i * 2 + 1]]
                    } else {
                        [0.0, 0.0]
                    };
                    MeshVertex {
                        position,
                        normal,
                        uv,
                    }
                })
                .collect();

            Mesh {
                vertices,
                indices: mesh.indices.clone(),
            }
        })
        .collect();

    Model { meshes }
}

fn load_gltf(path: &Path) -> Model {
    let (document, buffers, _) = gltf::import(path).expect("Failed to load glTF file");

    let mut meshes = Vec::new();

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

            let Some(pos_iter) = reader.read_positions() else {
                continue;
            };
            let positions: Vec<[f32; 3]> = pos_iter.collect();

            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|it| it.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|it| it.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            let vertices = positions
                .into_iter()
                .zip(normals)
                .zip(uvs)
                .map(|((position, normal), uv)| MeshVertex {
                    position,
                    normal,
                    uv,
                })
                .collect();

            let indices = reader
                .read_indices()
                .map(|it| it.into_u32().collect())
                .unwrap_or_default();

            meshes.push(Mesh { vertices, indices });
        }
    }

    Model { meshes }
}
