use crate::math::{Mat4, Quat, Vec3};

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
        let n = self.joints.len().min(poses.len());
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

    /// Linearly interpolate translation and scale; spherically interpolate rotation.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(other.translation, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
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
pub(super) fn find_interval(times: &[f32], t: f32) -> (usize, f32) {
    if times.len() <= 1 || t <= times[0] {
        return (0, 0.0);
    }
    let last = times.len() - 1;
    if t >= times[last] {
        return (last.saturating_sub(1), 1.0);
    }
    let i = times.partition_point(|&x| x <= t).saturating_sub(1);
    let dt = times[i + 1] - times[i];
    let alpha = if dt > 0.0 { (t - times[i]) / dt } else { 0.0 };
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
                AnimChannel::Translation(ch) => {
                    if let Some(pose) = poses.get_mut(ch.joint) {
                        pose.translation = ch.sample(t);
                    }
                }
                AnimChannel::Rotation(ch) => {
                    if let Some(pose) = poses.get_mut(ch.joint) {
                        pose.rotation = ch.sample(t);
                    }
                }
                AnimChannel::Scale(ch) => {
                    if let Some(pose) = poses.get_mut(ch.joint) {
                        pose.scale = ch.sample(t);
                    }
                }
            }
        }
        poses
    }
}
