use nene::animation::{
    AnimChannel, AnimatedModel, Animator, Channel, Clip, Joint, JointPose, Skeleton,
    SkinnedVertex, skinning_wgsl,
};
use nene::math::{Mat4, Vec3};

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn vec3_approx(a: Vec3, b: Vec3) -> bool {
    approx_eq(a.x, b.x) && approx_eq(a.y, b.y) && approx_eq(a.z, b.z)
}

// ── JointPose ──────────────────────────────────────────────────────────────────

#[test]
fn joint_pose_identity_is_mat4_identity() {
    let m = JointPose::IDENTITY.to_mat4();
    assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-5));
}

#[test]
fn joint_pose_translation_only() {
    let pose = JointPose { translation: Vec3::new(1.0, 2.0, 3.0), ..JointPose::IDENTITY };
    let m = pose.to_mat4();
    let p = m.transform_point3(Vec3::ZERO);
    assert!(vec3_approx(p, Vec3::new(1.0, 2.0, 3.0)));
}

// ── Channel sampling ──────────────────────────────────────────────────────────

#[test]
fn channel_single_keyframe_returns_value() {
    let ch: Channel<Vec3> = Channel {
        joint: 0,
        times: vec![0.0],
        values: vec![Vec3::new(1.0, 2.0, 3.0)],
    };
    assert!(vec3_approx(ch.sample(0.0), Vec3::new(1.0, 2.0, 3.0)));
    assert!(vec3_approx(ch.sample(99.0), Vec3::new(1.0, 2.0, 3.0)));
}

#[test]
fn channel_lerp_translation() {
    let ch: Channel<Vec3> = Channel {
        joint: 0,
        times: vec![0.0, 1.0],
        values: vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)],
    };
    assert!(vec3_approx(ch.sample(0.5), Vec3::new(1.0, 0.0, 0.0)));
}

#[test]
fn channel_clamps_before_first() {
    let ch: Channel<Vec3> = Channel {
        joint: 0,
        times: vec![1.0, 2.0],
        values: vec![Vec3::new(5.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 0.0)],
    };
    assert!(vec3_approx(ch.sample(0.0), Vec3::new(5.0, 0.0, 0.0)));
}

#[test]
fn channel_clamps_after_last() {
    let ch: Channel<Vec3> = Channel {
        joint: 0,
        times: vec![0.0, 1.0],
        values: vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)],
    };
    assert!(vec3_approx(ch.sample(5.0), Vec3::new(1.0, 0.0, 0.0)));
}

// ── Clip::sample ──────────────────────────────────────────────────────────────

#[test]
fn clip_sample_fills_missing_joints_with_identity() {
    let clip = Clip { name: "".into(), duration: 1.0, channels: vec![] };
    let poses = clip.sample(0.5, 3);
    assert_eq!(poses.len(), 3);
    assert!(poses[0].to_mat4().abs_diff_eq(Mat4::IDENTITY, 1e-5));
}

#[test]
fn clip_sample_sets_translation() {
    let clip = Clip {
        name: "".into(),
        duration: 1.0,
        channels: vec![AnimChannel::Translation(Channel {
            joint: 1,
            times: vec![0.0, 1.0],
            values: vec![Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0)],
        })],
    };
    let poses = clip.sample(0.5, 2);
    assert!(vec3_approx(poses[1].translation, Vec3::new(2.0, 0.0, 0.0)));
}

// ── Skeleton::compute_joint_matrices ─────────────────────────────────────────

#[test]
fn single_root_joint_identity_pose() {
    let skeleton = Skeleton {
        joints: vec![Joint {
            name: "root".into(),
            parent: None,
            inverse_bind: Mat4::IDENTITY,
        }],
    };
    let poses = vec![JointPose::IDENTITY];
    let mats = skeleton.compute_joint_matrices(&poses);
    assert!(mats[0].abs_diff_eq(Mat4::IDENTITY, 1e-5));
}

#[test]
fn parent_child_transform_propagates() {
    // Root translated by (1, 0, 0); child has identity pose.
    // Expected: child joint matrix = translation(1,0,0) * identity * identity_ibm
    let skeleton = Skeleton {
        joints: vec![
            Joint { name: "root".into(), parent: None, inverse_bind: Mat4::IDENTITY },
            Joint { name: "child".into(), parent: Some(0), inverse_bind: Mat4::IDENTITY },
        ],
    };
    let poses = vec![
        JointPose { translation: Vec3::new(1.0, 0.0, 0.0), ..JointPose::IDENTITY },
        JointPose::IDENTITY,
    ];
    let mats = skeleton.compute_joint_matrices(&poses);
    let child_origin = mats[1].transform_point3(Vec3::ZERO);
    assert!(vec3_approx(child_origin, Vec3::new(1.0, 0.0, 0.0)));
}

#[test]
fn inverse_bind_cancels_bind_pose() {
    // Joint at bind-pose position (2, 0, 0) with matching inverse_bind.
    let bind = Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0));
    let inv_bind = bind.inverse();
    let skeleton = Skeleton {
        joints: vec![Joint { name: "j".into(), parent: None, inverse_bind: inv_bind }],
    };
    let poses = vec![JointPose { translation: Vec3::new(2.0, 0.0, 0.0), ..JointPose::IDENTITY }];
    let mats = skeleton.compute_joint_matrices(&poses);
    // In bind pose the joint matrix should be identity (no net deformation)
    assert!(mats[0].abs_diff_eq(Mat4::IDENTITY, 1e-5));
}

// ── Animator ─────────────────────────────────────────────────────────────────

#[test]
fn animator_advances_time() {
    let clip = Clip { name: "".into(), duration: 2.0, channels: vec![] };
    let mut anim = Animator::new();
    anim.update(0.5, &clip);
    assert!(approx_eq(anim.time, 0.5));
}

#[test]
fn animator_loops() {
    let clip = Clip { name: "".into(), duration: 1.0, channels: vec![] };
    let mut anim = Animator::new();
    anim.looping = true;
    anim.update(1.7, &clip);
    assert!(approx_eq(anim.time, 0.7));
}

#[test]
fn animator_no_loop_clamps() {
    let clip = Clip { name: "".into(), duration: 1.0, channels: vec![] };
    let mut anim = Animator::new();
    anim.looping = false;
    anim.update(5.0, &clip);
    assert!(approx_eq(anim.time, 1.0));
}

#[test]
fn animator_speed_multiplier() {
    let clip = Clip { name: "".into(), duration: 10.0, channels: vec![] };
    let mut anim = Animator::new();
    anim.speed = 2.0;
    anim.update(1.0, &clip);
    assert!(approx_eq(anim.time, 2.0));
}

// ── SkinnedVertex ─────────────────────────────────────────────────────────────

#[test]
fn skinned_vertex_size() {
    // position(12) + normal(12) + uv(8) + joints(4) + weights(16) = 52 bytes
    assert_eq!(std::mem::size_of::<SkinnedVertex>(), 52);
}

#[test]
fn skinned_vertex_layout_stride() {
    let layout = SkinnedVertex::layout();
    assert_eq!(layout.stride, 52);
    assert_eq!(layout.attributes.len(), 5);
}

// ── skinning_wgsl ─────────────────────────────────────────────────────────────

#[test]
fn skinning_wgsl_contains_joint_matrices() {
    let wgsl = skinning_wgsl(64);
    assert!(wgsl.contains("JointMatrices"));
    assert!(wgsl.contains("array<mat4x4<f32>, 64>"));
}

// ── AnimatedModel::load ───────────────────────────────────────────────────────

#[test]
fn load_non_skinned_gltf_returns_none() {
    // A glTF without a skin should return None from AnimatedModel::load.
    // Reuse the sample cube glTF from the gltf example (positions only, no skin).
    let json = r#"{
        "asset": {"version":"2.0"},
        "scene": 0,
        "scenes": [{"nodes":[0]}],
        "nodes": [{"mesh":0}],
        "meshes": [{"primitives":[{"attributes":{"POSITION":0},"indices":1}]}],
        "accessors": [
            {"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"},
            {"bufferView":1,"componentType":5125,"count":3,"type":"SCALAR"}
        ],
        "bufferViews": [
            {"buffer":0,"byteOffset":0,"byteLength":36},
            {"buffer":0,"byteOffset":36,"byteLength":12}
        ],
        "buffers": [{"byteLength":48,"uri":"data:application/octet-stream;base64,AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAIAAAA="}]
    }"#;
    let path = std::env::temp_dir().join("nene_test_no_skin.gltf");
    std::fs::write(&path, json).unwrap();
    assert!(AnimatedModel::load(&path).is_none());
}
