use nene::math::{Vec2, Vec3};
use nene::time::{Ease, Lerp, Tween, TweenLoop};

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

// ── Lerp ──────────────────────────────────────────────────────────────────────

#[test]
fn lerp_f32_midpoint() {
    assert!(approx(0.0_f32.lerp(10.0, 0.5), 5.0));
}

#[test]
fn lerp_f32_endpoints() {
    assert!(approx(3.0_f32.lerp(7.0, 0.0), 3.0));
    assert!(approx(3.0_f32.lerp(7.0, 1.0), 7.0));
}

#[test]
fn lerp_vec3_midpoint() {
    let v = Vec3::ZERO.lerp(Vec3::new(2.0, 4.0, 6.0), 0.5);
    assert!(approx(v.x, 1.0) && approx(v.y, 2.0) && approx(v.z, 3.0));
}

// ── Ease ──────────────────────────────────────────────────────────────────────

#[test]
fn ease_linear_midpoint() {
    assert!(approx(Ease::Linear.apply(0.5), 0.5));
}

#[test]
fn ease_all_zero_at_t0() {
    let eases = [
        Ease::Linear,
        Ease::SineIn,
        Ease::SineOut,
        Ease::SineInOut,
        Ease::QuadIn,
        Ease::QuadOut,
        Ease::QuadInOut,
        Ease::CubicIn,
        Ease::CubicOut,
        Ease::CubicInOut,
        Ease::QuartIn,
        Ease::QuartOut,
        Ease::QuartInOut,
        Ease::ElasticIn,
        Ease::ElasticOut,
        Ease::ElasticInOut,
        Ease::BounceIn,
        Ease::BounceOut,
        Ease::BounceInOut,
        Ease::BackIn,
        Ease::BackOut,
        Ease::BackInOut,
    ];
    for e in eases {
        assert!(approx(e.apply(0.0), 0.0), "{e:?} should be 0 at t=0");
    }
}

#[test]
fn ease_all_one_at_t1() {
    let eases = [
        Ease::Linear,
        Ease::SineIn,
        Ease::SineOut,
        Ease::SineInOut,
        Ease::QuadIn,
        Ease::QuadOut,
        Ease::QuadInOut,
        Ease::CubicIn,
        Ease::CubicOut,
        Ease::CubicInOut,
        Ease::QuartIn,
        Ease::QuartOut,
        Ease::QuartInOut,
        Ease::ElasticIn,
        Ease::ElasticOut,
        Ease::ElasticInOut,
        Ease::BounceIn,
        Ease::BounceOut,
        Ease::BounceInOut,
        Ease::BackIn,
        Ease::BackOut,
        Ease::BackInOut,
    ];
    for e in eases {
        let v = e.apply(1.0);
        assert!(approx(v, 1.0), "{e:?} should be 1 at t=1, got {v}");
    }
}

#[test]
fn ease_quad_in_is_t_squared() {
    assert!(approx(Ease::QuadIn.apply(0.5), 0.25));
    assert!(approx(Ease::QuadIn.apply(0.25), 0.0625));
}

#[test]
fn ease_quad_out_is_symmetric_to_in() {
    // quad_out(t) = 1 - quad_in(1-t)
    let t = 0.3;
    let expected = 1.0 - (1.0 - t) * (1.0 - t);
    assert!(approx(Ease::QuadOut.apply(t), expected));
}

#[test]
fn ease_clamps_below_zero() {
    assert!(approx(Ease::Linear.apply(-0.5), 0.0));
}

#[test]
fn ease_clamps_above_one() {
    assert!(approx(Ease::Linear.apply(1.5), 1.0));
}

// ── Tween ─────────────────────────────────────────────────────────────────────

#[test]
fn tween_starts_at_start() {
    let t: Tween<f32> = Tween::new(3.0, 7.0, 1.0);
    assert!(approx(t.value(), 3.0));
}

#[test]
fn tween_ends_at_end() {
    let mut t: Tween<f32> = Tween::new(3.0, 7.0, 1.0);
    t.update(1.0);
    assert!(approx(t.value(), 7.0));
}

#[test]
fn tween_linear_midpoint() {
    let mut t: Tween<f32> = Tween::new(0.0, 10.0, 1.0);
    t.update(0.5);
    assert!(approx(t.value(), 5.0));
}

#[test]
fn tween_is_done_after_full_duration() {
    let mut t: Tween<f32> = Tween::new(0.0, 1.0, 0.5);
    assert!(!t.is_done());
    t.update(0.5);
    assert!(t.is_done());
}

#[test]
fn tween_not_done_mid_way() {
    let mut t: Tween<f32> = Tween::new(0.0, 1.0, 1.0);
    t.update(0.4);
    assert!(!t.is_done());
}

#[test]
fn tween_clamps_past_end() {
    let mut t: Tween<f32> = Tween::new(0.0, 5.0, 1.0);
    t.update(99.0);
    assert!(approx(t.value(), 5.0));
}

#[test]
fn tween_reset_returns_to_start() {
    let mut t: Tween<f32> = Tween::new(0.0, 1.0, 1.0);
    t.update(1.0);
    t.reset();
    assert!(approx(t.value(), 0.0));
    assert!(!t.is_done());
}

#[test]
fn tween_seek_midpoint() {
    let mut t: Tween<f32> = Tween::new(0.0, 10.0, 2.0);
    t.seek(0.5);
    assert!(approx(t.value(), 5.0));
}

#[test]
fn tween_loop_wraps() {
    let mut t: Tween<f32> = Tween::new(0.0, 1.0, 1.0).with_loop(TweenLoop::Loop);
    t.update(1.7);
    // Should be at 0.7 through the loop
    assert!(approx(t.value(), 0.7));
    assert!(!t.is_done());
}

#[test]
fn tween_pingpong_reverses() {
    let mut t: Tween<f32> = Tween::new(0.0, 10.0, 1.0).with_loop(TweenLoop::PingPong);
    t.update(0.5);
    assert!(approx(t.value(), 5.0)); // forward
    t.update(0.5);
    assert!(approx(t.value(), 10.0)); // at peak
    t.update(0.5);
    assert!(approx(t.value(), 5.0)); // returning
    t.update(0.5);
    assert!(approx(t.value(), 0.0)); // back at start
}

#[test]
fn tween_with_ease_cubic_out() {
    let mut t: Tween<f32> = Tween::new(0.0, 1.0, 1.0).with_ease(Ease::CubicOut);
    t.update(0.5);
    // cubic_out(0.5) = 1 - (0.5)^3 = 1 - 0.125 = 0.875... wait
    // CubicOut: 1 - (1-t)^3
    // at t=0.5: 1 - 0.5^3 = 1 - 0.125 = 0.875
    assert!(approx(t.value(), 0.875));
}

#[test]
fn tween_vec3_interpolates() {
    let mut t = Tween::new(Vec3::ZERO, Vec3::new(4.0, 8.0, 0.0), 2.0);
    t.update(1.0); // halfway
    let v = t.value();
    assert!(approx(v.x, 2.0) && approx(v.y, 4.0));
}

#[test]
fn tween_progress_at_half() {
    let mut t: Tween<f32> = Tween::new(0.0, 1.0, 2.0);
    t.update(1.0);
    assert!(approx(t.progress(), 0.5));
}

#[test]
fn tween_elapsed_tracked() {
    let mut t: Tween<f32> = Tween::new(0.0, 1.0, 1.0);
    t.update(0.3);
    assert!(approx(t.elapsed(), 0.3));
}

#[test]
fn tween_zero_duration_returns_end() {
    let mut t: Tween<f32> = Tween::new(0.0, 5.0, 0.0);
    assert!(approx(t.update(0.0), 5.0));
    assert!(approx(t.value(), 5.0));
}

#[test]
fn tween_vec2_works() {
    let mut t = Tween::new(Vec2::ZERO, Vec2::new(10.0, 20.0), 1.0);
    t.update(0.5);
    let v = t.value();
    assert!(approx(v.x, 5.0) && approx(v.y, 10.0));
}
