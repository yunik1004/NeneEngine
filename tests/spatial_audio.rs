use nene::audio::SpatialAudio;
use nene::math::Vec2;

// ── options_for ───────────────────────────────────────────────────────────────

#[test]
fn full_volume_at_origin() {
    let spatial = SpatialAudio::new(10.0);
    let opts = spatial.options_for(Vec2::ZERO);
    assert!((opts.volume - 1.0).abs() < 1e-5);
    assert!(opts.pan.abs() < 1e-5);
}

#[test]
fn silent_at_max_distance() {
    let spatial = SpatialAudio::new(10.0);
    let opts = spatial.options_for(Vec2::new(10.0, 0.0));
    assert!(opts.volume < 1e-5);
}

#[test]
fn silent_beyond_max_distance() {
    let spatial = SpatialAudio::new(10.0);
    let opts = spatial.options_for(Vec2::new(20.0, 0.0));
    assert_eq!(opts.volume, 0.0);
}

#[test]
fn half_volume_at_half_distance() {
    let spatial = SpatialAudio::new(10.0);
    let opts = spatial.options_for(Vec2::new(5.0, 0.0));
    assert!((opts.volume - 0.5).abs() < 1e-5);
}

#[test]
fn pan_right_for_positive_x() {
    let spatial = SpatialAudio::new(10.0);
    let opts = spatial.options_for(Vec2::new(10.0, 0.0));
    assert!((opts.pan - 1.0).abs() < 1e-5);
}

#[test]
fn pan_left_for_negative_x() {
    let spatial = SpatialAudio::new(10.0);
    let opts = spatial.options_for(Vec2::new(-10.0, 0.0));
    assert!((opts.pan - (-1.0)).abs() < 1e-5);
}

#[test]
fn pan_center_directly_above() {
    // Emitter directly above listener → no horizontal offset → pan = 0
    let spatial = SpatialAudio::new(10.0);
    let opts = spatial.options_for(Vec2::new(0.0, 5.0));
    assert!(opts.pan.abs() < 1e-5);
}

#[test]
fn pan_clamped_beyond_max_distance() {
    let spatial = SpatialAudio::new(10.0);
    let opts = spatial.options_for(Vec2::new(100.0, 0.0));
    assert_eq!(opts.pan, 1.0);
}

// ── listener offset ───────────────────────────────────────────────────────────

#[test]
fn listener_offset_affects_volume() {
    let mut spatial = SpatialAudio::new(10.0);
    spatial.listener = Vec2::new(8.0, 0.0);
    // Emitter at origin: distance = 8 → volume = 1 - 8/10 = 0.2
    let opts = spatial.options_for(Vec2::ZERO);
    assert!((opts.volume - 0.2).abs() < 1e-5);
}

#[test]
fn listener_offset_affects_pan() {
    let mut spatial = SpatialAudio::new(10.0);
    spatial.listener = Vec2::new(5.0, 0.0);
    // Emitter at origin: dx = -5 → pan = -5/10 = -0.5
    let opts = spatial.options_for(Vec2::ZERO);
    assert!((opts.pan - (-0.5)).abs() < 1e-5);
}
