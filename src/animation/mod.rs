pub use crate::mesh::{
    AnimChannel, Channel, Clip, Joint, JointPose, Skeleton, SkinnedMesh, SkinnedVertex,
};

mod animator;
mod skinned_material;
mod state_machine;

pub use animator::{Animator, JointMatrices, skinning_wgsl};
pub use skinned_material::{SkinnedMaterial, SkinnedMaterialBuilder, SkinnedMaterialUniform};
pub use state_machine::{AnimState, StateMachine};
