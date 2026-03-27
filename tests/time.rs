use nene::time::{FixedTime, Time};

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

// ── FixedTime ──────────────────────────────────────────────────────────────────

fn make_fixed(hz: f32, step: u32, tick: u64) -> FixedTime {
    FixedTime { delta: 1.0 / hz, step, tick }
}

#[test]
fn fixed_delta_equals_reciprocal_hz() {
    let ft = make_fixed(60.0, 0, 0);
    assert!((ft.delta - 1.0 / 60.0).abs() < 1e-7);
}

#[test]
fn fixed_step_zero_on_first_tick_of_frame() {
    let ft = make_fixed(60.0, 0, 0);
    assert_eq!(ft.step, 0);
}

#[test]
fn fixed_step_increments_within_frame() {
    let ft = make_fixed(60.0, 3, 10);
    assert_eq!(ft.step, 3);
}

#[test]
fn fixed_tick_counter_advances() {
    let ft = make_fixed(60.0, 0, 999);
    assert_eq!(ft.tick, 999);
}

#[test]
fn fixed_time_is_copy() {
    let ft = make_fixed(30.0, 1, 42);
    let ft2 = ft;
    assert!((ft2.delta - ft.delta).abs() < 1e-9);
}

/// Simulate the accumulator loop and verify tick counts across frames.
#[test]
fn accumulator_fires_correct_tick_count() {
    // At 20 Hz fixed, a 100 ms frame should fire exactly 2 ticks.
    let fixed_step = 1.0_f32 / 20.0; // 50 ms
    let frame_dt = 0.100_f32;        // 100 ms

    let mut acc = 0.0_f32;
    acc += frame_dt;
    let mut ticks = 0u32;
    while acc >= fixed_step {
        acc -= fixed_step;
        ticks += 1;
    }
    assert_eq!(ticks, 2);
    assert!((acc - 0.0).abs() < 1e-6);
}

/// A very slow frame should not exceed MAX_FIXED_STEPS ticks.
#[test]
fn accumulator_capped_by_max_fixed_steps() {
    use nene::window::MAX_FIXED_STEPS;
    let fixed_step = 1.0_f32 / 60.0;
    let frame_dt = 10.0_f32; // absurdly slow

    let max_acc = fixed_step * MAX_FIXED_STEPS as f32;
    let mut acc = 0.0_f32;
    acc += frame_dt;
    if acc > max_acc { acc = max_acc; }

    let mut ticks = 0u32;
    while acc >= fixed_step {
        acc -= fixed_step;
        ticks += 1;
    }
    assert!(ticks <= MAX_FIXED_STEPS, "ticks={ticks} exceeds MAX_FIXED_STEPS={MAX_FIXED_STEPS}");
}
