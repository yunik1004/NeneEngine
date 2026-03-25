use nene::time::Time;

fn make_time(delta: f32, elapsed: f64, frame: u64) -> Time {
    Time {
        delta,
        elapsed,
        frame,
    }
}

#[test]
fn fps_normal() {
    let t = make_time(1.0 / 60.0, 0.0, 1);
    let fps = t.fps();
    assert!((fps - 60.0).abs() < 0.1, "expected ~60 fps, got {fps}");
}

#[test]
fn fps_zero_delta_returns_zero() {
    let t = make_time(0.0, 0.0, 0);
    assert_eq!(t.fps(), 0.0);
}

#[test]
fn fps_144hz() {
    let t = make_time(1.0 / 144.0, 0.0, 1);
    assert!((t.fps() - 144.0).abs() < 0.5);
}

#[test]
fn fps_30hz() {
    let t = make_time(1.0 / 30.0, 0.0, 1);
    assert!((t.fps() - 30.0).abs() < 0.1);
}

#[test]
fn delta_stored() {
    let t = make_time(0.016, 1.0, 10);
    assert!((t.delta - 0.016).abs() < 1e-6);
}

#[test]
fn elapsed_stored() {
    let t = make_time(0.016, 42.5, 100);
    assert!((t.elapsed - 42.5).abs() < 1e-9);
}

#[test]
fn frame_stored() {
    let t = make_time(0.016, 0.0, 999);
    assert_eq!(t.frame, 999);
}

#[test]
fn frame_zero_on_first() {
    let t = make_time(0.0, 0.0, 0);
    assert_eq!(t.frame, 0);
}

#[test]
fn large_delta_clamped_scenario() {
    // Simulates what the window runner does: delta is clamped to 250 ms.
    // After a 1-second freeze, delta should not exceed 0.25.
    let frozen_delta: f32 = 1.0_f32.min(0.25);
    let t = make_time(frozen_delta, 10.0, 600);
    assert!(t.delta <= 0.25);
    assert!(t.fps() >= 4.0);
}
