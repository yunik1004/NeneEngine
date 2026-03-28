pub use crate::mesh::{
    AnimChannel, Channel, Clip, Joint, JointPose, Skeleton, SkinnedMesh, SkinnedVertex,
};

mod animator;
mod state_machine;

pub use animator::{Animator, JointMatrices, skinning_wgsl};
pub use state_machine::{AnimState, StateMachine};
