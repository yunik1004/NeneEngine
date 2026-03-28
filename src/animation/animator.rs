use encase::ShaderType;

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
    pub fn joint_matrices(&self, clip: &MeshClip, skeleton: &MeshSkeleton) -> Vec<Mat4> {
        let poses = clip.sample(self.time, skeleton.joints.len());
        skeleton.compute_joint_matrices(&poses)
    }

    /// Pack joint matrices into a fixed-size [`JointMatrices<N>`] ready for GPU upload.
    ///
    /// # Panics
    /// If the skeleton has more joints than `N`.
    pub fn joint_buffer<const N: usize>(
        &self,
        clip: &MeshClip,
        skeleton: &MeshSkeleton,
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
