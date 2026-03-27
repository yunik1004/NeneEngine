use nene::culling::Frustum;
use nene::math::{Mat4, Vec3};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Orthographic frustum: x∈[-hw,hw], y∈[-hh,hh].
/// Uses near=-1, far=1 so z∈[-1,1] maps cleanly to NDC [0,1]
/// without a separate view transform (eye == world).
fn ortho_frustum(hw: f32, hh: f32) -> Frustum {
    let vp = Mat4::orthographic_rh(-hw, hw, -hh, hh, -1.0, 1.0);
    Frustum::from_view_proj(vp)
}

// ── test_point ────────────────────────────────────────────────────────────────

#[test]
fn point_origin_inside() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(f.test_point(Vec3::ZERO));
}

#[test]
fn point_inside_corner() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(f.test_point(Vec3::new(9.0, 9.0, 0.5)));
}

#[test]
fn point_outside_right() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(!f.test_point(Vec3::new(11.0, 0.0, 0.5)));
}

#[test]
fn point_outside_left() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(!f.test_point(Vec3::new(-11.0, 0.0, 0.5)));
}

#[test]
fn point_outside_top() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(!f.test_point(Vec3::new(0.0, 11.0, 0.5)));
}

#[test]
fn point_outside_bottom() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(!f.test_point(Vec3::new(0.0, -11.0, 0.5)));
}

// ── test_sphere ───────────────────────────────────────────────────────────────

#[test]
fn sphere_fully_inside() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(f.test_sphere(Vec3::ZERO, 1.0));
}

#[test]
fn sphere_intersects_right_plane() {
    let f = ortho_frustum(10.0, 10.0);
    // Center just outside, but radius reaches inside.
    assert!(f.test_sphere(Vec3::new(10.5, 0.0, 0.5), 1.0));
}

#[test]
fn sphere_fully_outside() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(!f.test_sphere(Vec3::new(20.0, 0.0, 0.5), 1.0));
}

// ── test_aabb ─────────────────────────────────────────────────────────────────

#[test]
fn aabb_inside() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(f.test_aabb(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 1.0)));
}

#[test]
fn aabb_partially_overlaps_right() {
    let f = ortho_frustum(10.0, 10.0);
    // Straddles x=10 boundary.
    assert!(f.test_aabb(Vec3::new(9.0, -1.0, 0.0), Vec3::new(11.0, 1.0, 1.0)));
}

#[test]
fn aabb_fully_outside_right() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(!f.test_aabb(Vec3::new(11.0, -1.0, 0.0), Vec3::new(13.0, 1.0, 1.0)));
}

#[test]
fn aabb_fully_outside_above() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(!f.test_aabb(Vec3::new(-1.0, 11.0, 0.0), Vec3::new(1.0, 13.0, 1.0)));
}

#[test]
fn aabb_large_straddles_frustum() {
    let f = ortho_frustum(10.0, 10.0);
    // A huge AABB that contains the entire frustum.
    assert!(f.test_aabb(
        Vec3::new(-100.0, -100.0, -10.0),
        Vec3::new(100.0, 100.0, 10.0)
    ));
}

// ── test_rect_2d ──────────────────────────────────────────────────────────────

#[test]
fn rect_2d_inside() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(f.test_rect_2d(-5.0, 5.0, -5.0, 5.0));
}

#[test]
fn rect_2d_outside() {
    let f = ortho_frustum(10.0, 10.0);
    assert!(!f.test_rect_2d(11.0, 15.0, 0.0, 2.0));
}

// ── planes accessor ───────────────────────────────────────────────────────────

#[test]
fn planes_returns_six() {
    let f = ortho_frustum(5.0, 5.0);
    assert_eq!(f.planes().len(), 6);
}

#[test]
fn identity_vp_does_not_panic() {
    let f = Frustum::from_view_proj(Mat4::IDENTITY);
    // Just check it doesn't panic and origin is inside.
    assert!(f.test_point(Vec3::ZERO));
}
