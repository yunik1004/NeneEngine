use std::collections::HashMap;
use std::path::Path;

use encase::ShaderType;

use crate::math::{Mat4, Quat, Vec3};
use crate::mesh::Image;
use crate::renderer::{VertexAttribute, VertexFormat, VertexLayout};

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

// ── Animator ──────────────────────────────────────────────────────────────────

/// Tracks playback state for one animation clip.
pub struct Animator {
    /// Current playback time in seconds.
    pub time: f32,
    /// Playback speed multiplier. Default: `1.0`.
    pub speed: f32,
    /// Whether to loop at the end. Default: `true`.
    pub looping: bool,
}

impl Animator {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            speed: 1.0,
            looping: true,
        }
    }

    /// Advance the animation by `dt` seconds.
    pub fn update(&mut self, dt: f32, clip: &Clip) {
        if clip.duration <= 0.0 {
            return;
        }
        self.time += dt * self.speed;
        if self.looping {
            self.time = self.time.rem_euclid(clip.duration);
        } else {
            self.time = self.time.clamp(0.0, clip.duration);
        }
    }

    /// Compute per-joint skinning matrices for the current time.
    pub fn joint_matrices(&self, clip: &Clip, skeleton: &Skeleton) -> Vec<Mat4> {
        let poses = clip.sample(self.time, skeleton.joints.len());
        skeleton.compute_joint_matrices(&poses)
    }

    /// Pack joint matrices into a fixed-size [`JointMatrices<N>`] ready for GPU upload.
    ///
    /// # Panics
    /// If the skeleton has more joints than `N`.
    pub fn joint_buffer<const N: usize>(
        &self,
        clip: &Clip,
        skeleton: &Skeleton,
    ) -> JointMatrices<N> {
        let mats = self.joint_matrices(clip, skeleton);
        assert!(
            mats.len() <= N,
            "skeleton has {} joints but JointMatrices buffer is only {N}",
            mats.len()
        );
        let mut buf = JointMatrices {
            mats: [Mat4::IDENTITY; N],
        };
        buf.mats[..mats.len()].copy_from_slice(&mats);
        buf
    }
}

impl Default for Animator {
    fn default() -> Self {
        Self::new()
    }
}

// ── JointMatrices ─────────────────────────────────────────────────────────────

/// Fixed-size array of joint skinning matrices for GPU upload.
///
/// Matches the WGSL struct produced by [`skinning_wgsl`]:
/// ```wgsl
/// struct JointMatrices { mats: array<mat4x4<f32>, N> }
/// ```
///
/// Bind as a uniform and fill each frame via [`Animator::joint_buffer`].
#[derive(Clone, Copy, ShaderType)]
pub struct JointMatrices<const N: usize> {
    pub mats: [Mat4; N],
}

// ── skinning_wgsl ─────────────────────────────────────────────────────────────

/// Generate a WGSL `JointMatrices` struct for `n` joints.
///
/// Bind a [`JointMatrices<N>`] uniform at the desired group/binding, then use
/// `joint_mats.mats[idx]` in your vertex shader to build the skin matrix:
///
/// ```wgsl
/// // @group(1) @binding(0) var<uniform> joint_mats: JointMatrices;
///
/// let skin =
///     in.weights.x * joint_mats.mats[in.joints.x]
///   + in.weights.y * joint_mats.mats[in.joints.y]
///   + in.weights.z * joint_mats.mats[in.joints.z]
///   + in.weights.w * joint_mats.mats[in.joints.w];
/// let world_pos = model * skin * vec4<f32>(in.position, 1.0);
/// let world_nor = normalize((model * skin * vec4<f32>(in.normal, 0.0)).xyz);
/// ```
pub fn skinning_wgsl(n: usize) -> String {
    format!(
        r#"
struct JointMatrices {{
    mats: array<mat4x4<f32>, {n}>,
}}
"#
    )
}

// ── SkinnedMesh / AnimatedModel ────────────────────────────────────────────────

/// A mesh primitive with per-vertex skinning data.
pub struct SkinnedMesh {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    /// Node transform from the glTF hierarchy (does not include skeletal deformation).
    pub transform: Mat4,
    /// Base color texture, if present in the material.
    pub base_color: Option<Image>,
}

/// A glTF model that includes a skeleton and one or more animation clips.
pub struct AnimatedModel {
    pub meshes: Vec<SkinnedMesh>,
    pub skeleton: Skeleton,
    pub clips: Vec<Clip>,
}

impl AnimatedModel {
    /// Load an animated glTF/GLB file.
    ///
    /// Returns `None` if the file has no skin (i.e. is not a skinned model).
    pub fn load(path: impl AsRef<Path>) -> Option<Self> {
        let (doc, buffers, images) = gltf::import(path.as_ref()).ok()?;

        let skin = doc.skins().next()?;

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

        // Parent relationships (traverse all nodes, find which joints are children)
        let mut parents = vec![None; joint_nodes.len()];
        for node in doc.nodes() {
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

        let clips = load_clips(&doc, &buffers, &node_to_joint);

        let mut meshes = Vec::new();
        for scene in doc.scenes() {
            for node in scene.nodes() {
                collect_skinned_node(&node, Mat4::IDENTITY, &buffers, &images, &mut meshes);
            }
        }

        Some(AnimatedModel {
            meshes,
            skeleton,
            clips,
        })
    }
}

// ── glTF helpers ──────────────────────────────────────────────────────────────

fn load_clips(
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
                        .map(crate::mesh::to_rgba8)
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
