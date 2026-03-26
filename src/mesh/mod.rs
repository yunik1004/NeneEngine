use std::collections::HashMap;
use std::path::Path;

use crate::math::{Mat4, Quat, Vec3};
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

// ── SkinnedVertex ──────────────────────────────────────────────────────────────

/// Vertex with skinning data for skeletal animation.
///
/// `joints` contains up to 4 joint indices; `weights` are the corresponding
/// blend weights (should sum to 1.0). Both are sourced from `JOINTS_0` and
/// `WEIGHTS_0` glTF attributes.
///
/// In WGSL, declare the vertex inputs as:
/// ```wgsl
/// @location(0) position: vec3<f32>,
/// @location(1) normal:   vec3<f32>,
/// @location(2) uv:       vec2<f32>,
/// @location(3) joints:   vec4<u32>,  // Uint8x4 — values 0–255
/// @location(4) weights:  vec4<f32>,
/// ```
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// Joint indices (up to 4), each in range 0–255.
    pub joints: [u8; 4],
    /// Blend weights for each joint (should sum to 1.0).
    pub weights: [f32; 4],
}

impl SkinnedVertex {
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
                VertexAttribute {
                    location: 3,
                    offset: offset_of!(Self, joints) as u64,
                    format: VertexFormat::Uint8x4,
                },
                VertexAttribute {
                    location: 4,
                    offset: offset_of!(Self, weights) as u64,
                    format: VertexFormat::Float32x4,
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

/// A mesh primitive with per-vertex skinning data.
pub struct SkinnedMesh {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    /// Node transform from the glTF hierarchy (does not include skeletal deformation).
    pub transform: Mat4,
    /// Base color texture, if present in the material.
    pub base_color: Option<Image>,
}

// ── Joint / Skeleton ───────────────────────────────────────────────────────────

/// A single joint in a skeleton hierarchy.
pub struct Joint {
    pub name: String,
    /// Index of the parent joint, or `None` for root joints.
    pub parent: Option<usize>,
    /// Transforms vertices from bind-pose model space into joint local space.
    pub inverse_bind: Mat4,
}

/// A joint hierarchy used for skeletal animation.
pub struct Skeleton {
    pub joints: Vec<Joint>,
}

impl Skeleton {
    /// Compute the final per-joint skinning matrices from a set of local poses.
    ///
    /// Joints must be stored in topological order (parent before child), which
    /// glTF guarantees. The result can be uploaded directly via [`JointMatrices`].
    pub fn compute_joint_matrices(&self, poses: &[JointPose]) -> Vec<Mat4> {
        let n = self.joints.len();
        let mut global = vec![Mat4::IDENTITY; n];
        let mut result = vec![Mat4::IDENTITY; n];
        for i in 0..n {
            let local = poses[i].to_mat4();
            global[i] = match self.joints[i].parent {
                None => local,
                Some(p) => global[p] * local,
            };
            result[i] = global[i] * self.joints[i].inverse_bind;
        }
        result
    }
}

// ── JointPose ─────────────────────────────────────────────────────────────────

/// Local-space transform for one joint at a point in time.
#[derive(Clone, Copy, Debug)]
pub struct JointPose {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl JointPose {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Convert to a 4×4 TRS matrix.
    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl Default for JointPose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

// ── Keyframe channels ─────────────────────────────────────────────────────────

/// A single-target keyframe channel (translation, rotation, or scale).
pub struct Channel<T> {
    /// Index of the target joint in the skeleton.
    pub joint: usize,
    /// Monotonically increasing sample times in seconds.
    pub times: Vec<f32>,
    /// One value per time sample.
    pub values: Vec<T>,
}

impl Channel<Vec3> {
    pub fn sample(&self, t: f32) -> Vec3 {
        let (i, alpha) = find_interval(&self.times, t);
        if alpha == 0.0 || i + 1 >= self.values.len() {
            self.values[i]
        } else {
            self.values[i].lerp(self.values[i + 1], alpha)
        }
    }
}

impl Channel<Quat> {
    pub fn sample(&self, t: f32) -> Quat {
        let (i, alpha) = find_interval(&self.times, t);
        if alpha == 0.0 || i + 1 >= self.values.len() {
            self.values[i]
        } else {
            self.values[i].slerp(self.values[i + 1], alpha)
        }
    }
}

/// Returns the lower keyframe index and the blend factor [0, 1].
fn find_interval(times: &[f32], t: f32) -> (usize, f32) {
    if times.len() <= 1 || t <= times[0] {
        return (0, 0.0);
    }
    let last = times.len() - 1;
    if t >= times[last] {
        return (last.saturating_sub(1), 1.0);
    }
    let i = times.partition_point(|&x| x <= t).saturating_sub(1);
    let alpha = (t - times[i]) / (times[i + 1] - times[i]);
    (i, alpha)
}

/// One animation channel targeting a specific joint property.
pub enum AnimChannel {
    Translation(Channel<Vec3>),
    Rotation(Channel<Quat>),
    Scale(Channel<Vec3>),
}

// ── Clip ──────────────────────────────────────────────────────────────────────

/// A named animation clip containing all keyframe channels.
pub struct Clip {
    pub name: String,
    /// Total duration in seconds.
    pub duration: f32,
    pub channels: Vec<AnimChannel>,
}

impl Clip {
    /// Sample all joint poses at time `t` (in seconds).
    ///
    /// Joints not mentioned in any channel default to [`JointPose::IDENTITY`].
    pub fn sample(&self, t: f32, num_joints: usize) -> Vec<JointPose> {
        let mut poses = vec![JointPose::IDENTITY; num_joints];
        for channel in &self.channels {
            match channel {
                AnimChannel::Translation(ch) => poses[ch.joint].translation = ch.sample(t),
                AnimChannel::Rotation(ch) => poses[ch.joint].rotation = ch.sample(t),
                AnimChannel::Scale(ch) => poses[ch.joint].scale = ch.sample(t),
            }
        }
        poses
    }
}

// ── Model ─────────────────────────────────────────────────────────────────────

/// A loaded model, which may contain static meshes, skinned meshes, a skeleton,
/// and animation clips.
pub struct Model {
    /// Static (non-skinned) mesh primitives.
    pub meshes: Vec<Mesh>,
    /// Skinned mesh primitives (empty if not skinned).
    pub skinned_meshes: Vec<SkinnedMesh>,
    /// Joint hierarchy (empty joints vec if not skinned).
    pub skeleton: Skeleton,
    /// Animation clips (empty if not animated).
    pub clips: Vec<Clip>,
}

impl Model {
    /// Load a model from a file. Supports `.obj` and `.gltf`/`.glb`.
    /// Automatically detects skinned/animated glTF.
    pub fn load(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        match path.extension().and_then(|e| e.to_str()) {
            Some("obj") => load_obj(path),
            Some("gltf") | Some("glb") => load_gltf(path),
            ext => panic!("Unsupported model format: {:?}", ext),
        }
    }

    /// Returns `true` if the model has skinned meshes.
    pub fn is_skinned(&self) -> bool {
        !self.skinned_meshes.is_empty()
    }

    /// Returns `true` if the model has animation clips.
    pub fn is_animated(&self) -> bool {
        !self.clips.is_empty()
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

    Model {
        meshes,
        skinned_meshes: vec![],
        skeleton: Skeleton { joints: vec![] },
        clips: vec![],
    }
}

fn load_gltf(path: &Path) -> Model {
    let (document, buffers, images) = gltf::import(path).expect("Failed to load glTF file");

    // Detect if there is a skin — if so, load as skinned/animated model.
    if let Some(skin) = document.skins().next() {
        // Map glTF node index → local joint index
        let joint_nodes: Vec<gltf::Node> = skin.joints().collect();
        let node_to_joint: HashMap<usize, usize> = joint_nodes
            .iter()
            .enumerate()
            .map(|(ji, node)| (node.index(), ji))
            .collect();

        // Inverse bind matrices
        let ibms: Vec<Mat4> = {
            let reader = skin.reader(|b| Some(&*buffers[b.index()]));
            match reader.read_inverse_bind_matrices() {
                Some(iter) => iter.map(|m| Mat4::from_cols_array_2d(&m)).collect(),
                None => vec![Mat4::IDENTITY; joint_nodes.len()],
            }
        };

        // Parent relationships
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

        let mut skinned_meshes = Vec::new();
        for scene in document.scenes() {
            for node in scene.nodes() {
                collect_skinned_node(&node, Mat4::IDENTITY, &buffers, &images, &mut skinned_meshes);
            }
        }

        Model {
            meshes: vec![],
            skinned_meshes,
            skeleton,
            clips,
        }
    } else {
        // Non-skinned glTF
        let mut meshes = Vec::new();
        for scene in document.scenes() {
            for node in scene.nodes() {
                collect_node(&node, Mat4::IDENTITY, &buffers, &images, &mut meshes);
            }
        }

        Model {
            meshes,
            skinned_meshes: vec![],
            skeleton: Skeleton { joints: vec![] },
            clips: vec![],
        }
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

fn collect_skinned_node(
    node: &gltf::Node,
    parent_transform: Mat4,
    buffers: &[gltf::buffer::Data],
    images: &[gltf::image::Data],
    out: &mut Vec<SkinnedMesh>,
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
                .map(
                    |((((position, normal), uv), joints), weights)| SkinnedVertex {
                        position,
                        normal,
                        uv,
                        joints,
                        weights,
                    },
                )
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
                    images
                        .get(info.texture().source().index())
                        .map(to_rgba8)
                });

            out.push(SkinnedMesh {
                vertices,
                indices,
                transform: world,
                base_color,
            });
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
                            Some(AnimChannel::Translation(Channel {
                                joint,
                                times,
                                values,
                            }))
                        }
                        ReadOutputs::Rotations(rots) => {
                            let values = rots.into_f32().map(Quat::from_array).collect();
                            Some(AnimChannel::Rotation(Channel {
                                joint,
                                times,
                                values,
                            }))
                        }
                        ReadOutputs::Scales(it) => {
                            let values = it.map(Vec3::from).collect();
                            Some(AnimChannel::Scale(Channel {
                                joint,
                                times,
                                values,
                            }))
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
