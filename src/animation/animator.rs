use crate::math::Mat4;
use crate::mesh::{Clip as MeshClip, Skeleton as MeshSkeleton};

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
    pub fn update(&mut self, dt: f32, clip: &MeshClip) {
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
    ///
    /// Pass the result directly to [`Material::update_joints`](crate::renderer::Material::update_joints).
    pub fn joint_matrices(&self, clip: &MeshClip, skeleton: &MeshSkeleton) -> Vec<Mat4> {
        let poses = clip.sample(self.time, skeleton.joints.len());
        skeleton.compute_joint_matrices(&poses)
    }
}

impl Default for Animator {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a WGSL `JointMatrices` storage buffer declaration for `n` joints.
///
/// Bind a `&[Mat4]` storage buffer at the desired group/binding, then use
/// `joint_mats[idx]` in your vertex shader to build the skin matrix:
///
/// ```wgsl
/// // @group(1) @binding(0) var<storage, read> joint_mats: array<mat4x4<f32>>;
///
/// let skin =
///     in.weights.x * joint_mats[in.joints.x]
///   + in.weights.y * joint_mats[in.joints.y]
///   + in.weights.z * joint_mats[in.joints.z]
///   + in.weights.w * joint_mats[in.joints.w];
/// let world_pos = model * skin * vec4<f32>(in.position, 1.0);
/// let world_nor = normalize((model * skin * vec4<f32>(in.normal, 0.0)).xyz);
/// ```
pub fn skinning_wgsl() -> &'static str {
    "@group(1) @binding(0) var<storage, read> joint_mats: array<mat4x4<f32>>;\n"
}
