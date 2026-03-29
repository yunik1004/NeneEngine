use std::collections::HashMap;
use std::path::Path;

use crate::math::{Mat4, Quat, Vec3};

use super::skeleton::{AnimChannel, Channel, Clip, Joint, Skeleton};
use super::vertex::{Image, Mesh, Vertex};

pub type ModelError = Box<dyn std::error::Error + Send + Sync>;

// ── Model ─────────────────────────────────────────────────────────────────────

/// A loaded model which may contain static or skinned meshes, a skeleton,
/// and animation clips.
///
/// Static and skinned meshes are stored together in [`meshes`](Model::meshes).
/// Skinned meshes have [`Mesh::skinned`] set to `true`.
pub struct Model {
    pub meshes:   Vec<Mesh>,
    /// Joint hierarchy (empty if not skinned).
    pub skeleton: Skeleton,
    /// Animation clips (empty if not animated).
    pub clips:    Vec<Clip>,
}

impl Model {
    /// Load a model from a file. Supports `.obj` and `.gltf`/`.glb`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ModelError> {
        let path = path.as_ref();
        match path.extension().and_then(|e| e.to_str()) {
            Some("obj")             => load_obj(path),
            Some("gltf") | Some("glb") => load_gltf(path),
            ext => Err(format!("Unsupported model format: {:?}", ext).into()),
        }
    }

    /// Returns `true` if any mesh carries skeletal animation data.
    pub fn is_skinned(&self) -> bool {
        self.meshes.iter().any(|m| m.skinned)
    }

    /// Returns `true` if the model has animation clips.
    pub fn is_animated(&self) -> bool {
        !self.clips.is_empty()
    }
}

fn load_obj(path: &Path) -> Result<Model, ModelError> {
    let (models, _) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )?;

    let meshes = models
        .into_iter()
        .map(|m| {
            let mesh = &m.mesh;
            let n = mesh.positions.len() / 3;
            let vertices = (0..n)
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
                    Vertex { position, normal, uv, ..Vertex::default() }
                })
                .collect();

            Mesh {
                vertices,
                indices:    mesh.indices.clone(),
                transform:  Mat4::IDENTITY,
                base_color: None,
                skinned:    false,
            }
        })
        .collect();

    Ok(Model {
        meshes,
        skeleton: Skeleton { joints: vec![] },
        clips:    vec![],
    })
}

fn load_gltf(path: &Path) -> Result<Model, ModelError> {
    let (document, buffers, images) = gltf::import(path)?;

    if let Some(skin) = document.skins().next() {
        let joint_nodes: Vec<gltf::Node> = skin.joints().collect();
        let node_to_joint: HashMap<usize, usize> = joint_nodes
            .iter()
            .enumerate()
            .map(|(ji, node)| (node.index(), ji))
            .collect();

        let ibms: Vec<Mat4> = {
            let reader = skin.reader(|b| Some(&*buffers[b.index()]));
            match reader.read_inverse_bind_matrices() {
                Some(iter) => iter.map(|m| Mat4::from_cols_array_2d(&m)).collect(),
                None => vec![Mat4::IDENTITY; joint_nodes.len()],
            }
        };

        let mut parents = vec![None; joint_nodes.len()];
        for node in document.nodes() {
            if let Some(&pi) = node_to_joint.get(&node.index()) {
                for child in node.children() {
                    if let Some(&ci) = node_to_joint.get(&child.index()) {
                        parents[ci] = Some(pi);
                    }
                }
            }
        }

        let joints: Vec<Joint> = joint_nodes
            .iter()
            .zip(ibms.iter())
            .zip(parents.iter())
            .map(|((node, &ibm), &parent)| Joint {
                name: node.name().unwrap_or("").to_string(),
                parent,
                inverse_bind: ibm,
            })
            .collect();

        let skeleton = Skeleton { joints };
        let clips = load_gltf_clips(&document, &buffers, &node_to_joint);

        let mut meshes = Vec::new();
        for scene in document.scenes() {
            for node in scene.nodes() {
                collect_skinned_node(&node, Mat4::IDENTITY, &buffers, &images, &mut meshes);
            }
        }

        Ok(Model { meshes, skeleton, clips })
    } else {
        let mut meshes = Vec::new();
        for scene in document.scenes() {
            for node in scene.nodes() {
                collect_node(&node, Mat4::IDENTITY, &buffers, &images, &mut meshes);
            }
        }

        Ok(Model {
            meshes,
            skeleton: Skeleton { joints: vec![] },
            clips:    vec![],
        })
    }
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

            let Some(pos_iter) = reader.read_positions() else { continue };
            let positions: Vec<[f32; 3]> = pos_iter.collect();
            let n = positions.len();

            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|it| it.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; n]);

            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|it| it.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; n]);

            let vertices = positions
                .into_iter()
                .zip(normals)
                .zip(uvs)
                .map(|((position, normal), uv)| Vertex { position, normal, uv, ..Vertex::default() })
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

            out.push(Mesh { vertices, indices, transform: world, base_color, skinned: false });
        }
    }

    for child in node.children() {
        collect_node(&child, world, buffers, images, out);
    }
}

fn collect_skinned_node(
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

            let Some(pos_iter) = reader.read_positions() else { continue };
            let positions: Vec<[f32; 3]> = pos_iter.collect();
            let n = positions.len();

            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|it| it.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; n]);

            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|it| it.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; n]);

            let joints: Vec<[u8; 4]> = match reader.read_joints(0) {
                Some(gltf::mesh::util::ReadJoints::U8(it)) => it.collect(),
                Some(gltf::mesh::util::ReadJoints::U16(it)) => it
                    .map(|j| [j[0] as u8, j[1] as u8, j[2] as u8, j[3] as u8])
                    .collect(),
                None => vec![[0, 0, 0, 0]; n],
            };

            let weights: Vec<[f32; 4]> = reader
                .read_weights(0)
                .map(|it| it.into_f32().collect())
                .unwrap_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; n]);

            let vertices = positions
                .into_iter()
                .zip(normals)
                .zip(uvs)
                .zip(joints)
                .zip(weights)
                .map(|((((position, normal), uv), joints), weights)| Vertex {
                    position,
                    normal,
                    uv,
                    joints,
                    weights,
                    ..Vertex::default()
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
                .and_then(|info| images.get(info.texture().source().index()).map(to_rgba8));

            out.push(Mesh { vertices, indices, transform: world, base_color, skinned: true });
        }
    }

    for child in node.children() {
        collect_skinned_node(&child, world, buffers, images, out);
    }
}

fn load_gltf_clips(
    doc: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    node_to_joint: &HashMap<usize, usize>,
) -> Vec<Clip> {
    use gltf::animation::util::ReadOutputs;

    doc.animations()
        .map(|anim| {
            let mut duration = 0.0f32;
            let channels: Vec<AnimChannel> = anim
                .channels()
                .filter_map(|ch| {
                    let &joint = node_to_joint.get(&ch.target().node().index())?;
                    let reader = ch.reader(|b| Some(&*buffers[b.index()]));
                    let times: Vec<f32> = reader.read_inputs()?.collect();
                    if let Some(&t) = times.last() {
                        duration = duration.max(t);
                    }
                    match reader.read_outputs()? {
                        ReadOutputs::Translations(it) => {
                            let values = it.map(Vec3::from).collect();
                            Some(AnimChannel::Translation(Channel { joint, times, values }))
                        }
                        ReadOutputs::Rotations(rots) => {
                            let values = rots.into_f32().map(Quat::from_array).collect();
                            Some(AnimChannel::Rotation(Channel { joint, times, values }))
                        }
                        ReadOutputs::Scales(it) => {
                            let values = it.map(Vec3::from).collect();
                            Some(AnimChannel::Scale(Channel { joint, times, values }))
                        }
                        ReadOutputs::MorphTargetWeights(_) => None,
                    }
                })
                .collect();
            Clip {
                name: anim.name().unwrap_or("").to_string(),
                duration,
                channels,
            }
        })
        .collect()
}

pub(crate) fn to_rgba8(img: &gltf::image::Data) -> Image {
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
                let r = (f32::from_le_bytes([p[0], p[1], p[2], p[3]]).clamp(0.0, 1.0) * 255.0) as u8;
                let g = (f32::from_le_bytes([p[4], p[5], p[6], p[7]]).clamp(0.0, 1.0) * 255.0) as u8;
                let b = (f32::from_le_bytes([p[8], p[9], p[10], p[11]]).clamp(0.0, 1.0) * 255.0) as u8;
                [r, g, b, 255]
            })
            .collect(),
        Format::R32G32B32A32FLOAT => img
            .pixels
            .chunks_exact(16)
            .flat_map(|p| {
                let r = (f32::from_le_bytes([p[0], p[1], p[2], p[3]]).clamp(0.0, 1.0) * 255.0) as u8;
                let g = (f32::from_le_bytes([p[4], p[5], p[6], p[7]]).clamp(0.0, 1.0) * 255.0) as u8;
                let b = (f32::from_le_bytes([p[8], p[9], p[10], p[11]]).clamp(0.0, 1.0) * 255.0) as u8;
                let a = (f32::from_le_bytes([p[12], p[13], p[14], p[15]]).clamp(0.0, 1.0) * 255.0) as u8;
                [r, g, b, a]
            })
            .collect(),
    };

    Image { width: img.width, height: img.height, data: rgba }
}
