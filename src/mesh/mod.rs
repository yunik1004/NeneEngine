use std::path::Path;

use crate::math::Mat4;
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

/// Raw RGBA8 image data returned from a glTF material.
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// RGBA8 pixels, row-major.
    pub data: Vec<u8>,
}

/// A single mesh primitive (triangles).
pub struct Mesh {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    /// World-space transform accumulated from the node hierarchy.
    pub transform: Mat4,
    /// Base color texture from the primitive's material, if present.
    pub base_color: Option<Image>,
}

/// A loaded model, which may contain multiple mesh primitives.
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
                transform: Mat4::IDENTITY,
                base_color: None,
            }
        })
        .collect();

    Model { meshes }
}

fn load_gltf(path: &Path) -> Model {
    let (document, buffers, images) = gltf::import(path).expect("Failed to load glTF file");

    let mut meshes = Vec::new();

    for scene in document.scenes() {
        for node in scene.nodes() {
            collect_node(&node, Mat4::IDENTITY, &buffers, &images, &mut meshes);
        }
    }

    Model { meshes }
}

fn collect_node(
    node: &gltf::Node,
    parent_transform: Mat4,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
    out: &mut Vec<Mesh>,
) {
    let local = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world = parent_transform * local;

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                continue;
            }
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

            let base_color = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
                .and_then(|info| {
                    let idx = info.texture().source().index();
                    images.get(idx).map(to_rgba8)
                });

            out.push(Mesh {
                vertices,
                indices,
                transform: world,
                base_color,
            });
        }
    }

    for child in node.children() {
        collect_node(&child, world, buffers, images, out);
    }
}

fn to_rgba8(img: &gltf::image::Data) -> Image {
    use gltf::image::Format;
    let rgba = match img.format {
        Format::R8G8B8 => img
            .pixels
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        Format::R8G8B8A8 => img.pixels.clone(),
        Format::R8 => img.pixels.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        Format::R8G8 => img
            .pixels
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[1], 0, 255])
            .collect(),
        Format::R16 => img
            .pixels
            .chunks_exact(2)
            .flat_map(|p| {
                let v = (u16::from_le_bytes([p[0], p[1]]) >> 8) as u8;
                [v, v, v, 255]
            })
            .collect(),
        Format::R16G16 => img
            .pixels
            .chunks_exact(4)
            .flat_map(|p| {
                let r = (u16::from_le_bytes([p[0], p[1]]) >> 8) as u8;
                let g = (u16::from_le_bytes([p[2], p[3]]) >> 8) as u8;
                [r, g, 0, 255]
            })
            .collect(),
        Format::R16G16B16 => img
            .pixels
            .chunks_exact(6)
            .flat_map(|p| {
                let r = (u16::from_le_bytes([p[0], p[1]]) >> 8) as u8;
                let g = (u16::from_le_bytes([p[2], p[3]]) >> 8) as u8;
                let b = (u16::from_le_bytes([p[4], p[5]]) >> 8) as u8;
                [r, g, b, 255]
            })
            .collect(),
        Format::R16G16B16A16 => img
            .pixels
            .chunks_exact(8)
            .flat_map(|p| {
                let r = (u16::from_le_bytes([p[0], p[1]]) >> 8) as u8;
                let g = (u16::from_le_bytes([p[2], p[3]]) >> 8) as u8;
                let b = (u16::from_le_bytes([p[4], p[5]]) >> 8) as u8;
                let a = (u16::from_le_bytes([p[6], p[7]]) >> 8) as u8;
                [r, g, b, a]
            })
            .collect(),
        Format::R32G32B32FLOAT => img
            .pixels
            .chunks_exact(12)
            .flat_map(|p| {
                let r =
                    (f32::from_le_bytes([p[0], p[1], p[2], p[3]]).clamp(0.0, 1.0) * 255.0) as u8;
                let g =
                    (f32::from_le_bytes([p[4], p[5], p[6], p[7]]).clamp(0.0, 1.0) * 255.0) as u8;
                let b =
                    (f32::from_le_bytes([p[8], p[9], p[10], p[11]]).clamp(0.0, 1.0) * 255.0) as u8;
                [r, g, b, 255]
            })
            .collect(),
        Format::R32G32B32A32FLOAT => img
            .pixels
            .chunks_exact(16)
            .flat_map(|p| {
                let r =
                    (f32::from_le_bytes([p[0], p[1], p[2], p[3]]).clamp(0.0, 1.0) * 255.0) as u8;
                let g =
                    (f32::from_le_bytes([p[4], p[5], p[6], p[7]]).clamp(0.0, 1.0) * 255.0) as u8;
                let b =
                    (f32::from_le_bytes([p[8], p[9], p[10], p[11]]).clamp(0.0, 1.0) * 255.0) as u8;
                let a = (f32::from_le_bytes([p[12], p[13], p[14], p[15]]).clamp(0.0, 1.0) * 255.0)
                    as u8;
                [r, g, b, a]
            })
            .collect(),
    };

    Image {
        width: img.width,
        height: img.height,
        data: rgba,
    }
}
