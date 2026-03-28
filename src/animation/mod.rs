// Re-export data types from mesh for backwards compatibility.
pub use crate::mesh::{
    AnimChannel, Channel, Clip, Joint, JointPose, Skeleton, SkinnedMesh, SkinnedVertex,
};
use encase::ShaderType;

use crate::math::Mat4;
use crate::mesh::{Clip as MeshClip, Skeleton as MeshSkeleton};

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

// ── AnimState ─────────────────────────────────────────────────────────────────

/// One state in an [`StateMachine`].
pub struct AnimState {
    /// Identifier used with [`StateMachine::trigger`].
    pub name: String,
    /// Index into the `clips` slice passed to [`StateMachine::update`].
    pub clip_index: usize,
    /// Whether the clip loops. Default: `true`.
    pub looping: bool,
    /// Playback speed multiplier. Default: `1.0`.
    pub speed: f32,
}

// ── BlendState (private) ──────────────────────────────────────────────────────

struct BlendState {
    target: usize,
    target_time: f32,
    elapsed: f32,
    duration: f32,
}

// ── StateMachine ──────────────────────────────────────────────────────────────

/// Crossfade-based animation state machine.
///
/// Each state references one [`Clip`] by index. Call [`trigger`](Self::trigger)
/// to request a crossfade to another state; call [`update`](Self::update) every
/// frame and [`joint_matrices`](Self::joint_matrices) to obtain blended skinning
/// matrices for GPU upload.
///
/// # Quick start
/// ```no_run
/// # use nene::animation::{StateMachine, AnimState};
/// let mut sm = StateMachine::new();
/// sm.add_state(AnimState { name: "idle".into(), clip_index: 0, looping: true,  speed: 1.0 });
/// sm.add_state(AnimState { name: "walk".into(), clip_index: 1, looping: true,  speed: 1.0 });
/// sm.add_state(AnimState { name: "jump".into(), clip_index: 2, looping: false, speed: 1.2 });
///
/// // In the update callback:
/// // sm.update(dt, &model.clips);
/// // let mats = sm.joint_matrices(&model.clips, &model.skeleton);
/// ```
pub struct StateMachine {
    pub states: Vec<AnimState>,
    /// Index of the currently active state.
    pub current: usize,
    /// Playback time within the current state (seconds).
    pub time: f32,
    blend: Option<BlendState>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            current: 0,
            time: 0.0,
            blend: None,
        }
    }

    /// Append a state and return its index.
    pub fn add_state(&mut self, state: AnimState) -> usize {
        let idx = self.states.len();
        self.states.push(state);
        idx
    }

    /// Request a crossfade to the state named `name` over `blend_duration` seconds.
    ///
    /// - If `name` is not found, this is a no-op.
    /// - If the target is already active with no blend in progress, this is a no-op.
    /// - If `blend_duration <= 0`, the switch is immediate.
    /// - If a blend is already in progress it is finalised and a new one begins.
    pub fn trigger(&mut self, name: &str, blend_duration: f32) {
        let Some(target) = self.states.iter().position(|s| s.name == name) else {
            return;
        };
        if target == self.current && self.blend.is_none() {
            return;
        }
        // Finalise any in-progress blend first.
        if let Some(b) = self.blend.take() {
            self.current = b.target;
            self.time = b.target_time;
        }
        if blend_duration <= 0.0 {
            self.current = target;
            self.time = 0.0;
        } else {
            self.blend = Some(BlendState {
                target,
                target_time: 0.0,
                elapsed: 0.0,
                duration: blend_duration,
            });
        }
    }

    /// Advance both the current and blend-target states by `dt` seconds.
    pub fn update(&mut self, dt: f32, clips: &[Clip]) {
        // Advance current state.
        if self.current < self.states.len() {
            advance_time(&mut self.time, &self.states[self.current], clips, dt);
        }

        // Advance blend target and check for completion.
        let blend_done = if let Some(ref mut b) = self.blend {
            if b.target < self.states.len() {
                let state = &self.states[b.target];
                advance_time(&mut b.target_time, state, clips, dt);
            }
            b.elapsed += dt;
            b.elapsed >= b.duration
        } else {
            false
        };

        if blend_done {
            let b = self.blend.take().unwrap();
            self.current = b.target;
            self.time = b.target_time;
        }
    }

    /// Compute blended per-joint skinning matrices.
    ///
    /// Pass the same `clips` slice used for [`update`](Self::update).
    pub fn joint_matrices(&self, clips: &[Clip], skeleton: &Skeleton) -> Vec<Mat4> {
        let n = skeleton.joints.len();
        let from_poses = sample_at(&self.states, self.current, self.time, clips, n);
        match &self.blend {
            None => skeleton.compute_joint_matrices(&from_poses),
            Some(b) => {
                let t = (b.elapsed / b.duration).clamp(0.0, 1.0);
                let to_poses = sample_at(&self.states, b.target, b.target_time, clips, n);
                let blended: Vec<JointPose> = from_poses
                    .iter()
                    .zip(to_poses.iter())
                    .map(|(a, b)| a.lerp(*b, t))
                    .collect();
                skeleton.compute_joint_matrices(&blended)
            }
        }
    }

    /// Pack joint matrices into a fixed-size [`JointMatrices<N>`] for GPU upload.
    ///
    /// # Panics
    /// If the skeleton has more joints than `N`.
    pub fn joint_buffer<const N: usize>(
        &self,
        clips: &[Clip],
        skeleton: &Skeleton,
    ) -> JointMatrices<N> {
        let mats = self.joint_matrices(clips, skeleton);
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

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn advance_time(time: &mut f32, state: &AnimState, clips: &[Clip], dt: f32) {
    if state.clip_index >= clips.len() {
        return;
    }
    let clip = &clips[state.clip_index];
    if clip.duration <= 0.0 {
        return;
    }
    *time += dt * state.speed;
    if state.looping {
        *time = time.rem_euclid(clip.duration);
    } else {
        *time = time.clamp(0.0, clip.duration);
    }
}

fn sample_at(
    states: &[AnimState],
    state_idx: usize,
    time: f32,
    clips: &[Clip],
    n: usize,
) -> Vec<JointPose> {
    if state_idx >= states.len() {
        return vec![JointPose::IDENTITY; n];
    }
    let state = &states[state_idx];
    if state.clip_index < clips.len() {
        clips[state.clip_index].sample(time, n)
    } else {
        vec![JointPose::IDENTITY; n]
    }
}
