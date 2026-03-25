use nene::{
    camera::{Camera, Projection},
    math::{Mat4, Vec3},
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn mat4_approx_eq(a: Mat4, b: Mat4) -> bool {
    let a = a.to_cols_array_2d();
    let b = b.to_cols_array_2d();
    a.iter()
        .zip(b.iter())
        .all(|(ca, cb)| ca.iter().zip(cb.iter()).all(|(&x, &y)| approx_eq(x, y)))
}

fn is_finite_mat4(m: Mat4) -> bool {
    m.to_cols_array().iter().all(|v| v.is_finite())
}

// ── Camera::perspective ───────────────────────────────────────────────────────

#[test]
fn perspective_default_target_is_origin() {
    let cam = Camera::perspective(Vec3::new(0.0, 0.0, 5.0), 45.0, 0.1, 100.0);
    assert_eq!(cam.target, Vec3::ZERO);
}

#[test]
fn perspective_position_stored() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let cam = Camera::perspective(pos, 60.0, 0.1, 100.0);
    assert_eq!(cam.position, pos);
}

#[test]
fn perspective_projection_variant() {
    let cam = Camera::perspective(Vec3::new(0.0, 0.0, 5.0), 45.0, 0.1, 100.0);
    assert!(matches!(cam.projection, Projection::Perspective { .. }));
}

#[test]
fn perspective_view_proj_is_finite() {
    let cam = Camera::perspective(Vec3::new(0.0, 0.0, 5.0), 45.0, 0.1, 100.0);
    assert!(is_finite_mat4(cam.view_proj(16.0 / 9.0)));
}

#[test]
fn perspective_view_proj_matches_manual() {
    let cam = Camera::perspective(Vec3::new(0.0, 0.0, 5.0), 45.0, 0.1, 100.0);
    let aspect = 16.0 / 9.0_f32;
    let expected = Mat4::perspective_rh(45f32.to_radians(), aspect, 0.1, 100.0)
        * Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
    assert!(mat4_approx_eq(cam.view_proj(aspect), expected));
}

// ── Camera::orthographic ─────────────────────────────────────────────────────

#[test]
fn orthographic_projection_variant() {
    let cam = Camera::orthographic(Vec3::ZERO, 10.0, -1.0, 1.0);
    assert!(matches!(cam.projection, Projection::Orthographic { .. }));
}

#[test]
fn orthographic_view_proj_is_finite() {
    let cam = Camera::orthographic(Vec3::ZERO, 10.0, -1.0, 1.0);
    assert!(is_finite_mat4(cam.view_proj(16.0 / 9.0)));
}

#[test]
fn orthographic_width_respected() {
    let width = 12.0_f32;
    let aspect = 16.0 / 9.0_f32;
    let cam = Camera::orthographic(Vec3::ZERO, width, -1.0, 1.0);
    let half_w = width * 0.5;
    let half_h = half_w / aspect;
    let expected = Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, -1.0, 1.0);
    assert!(mat4_approx_eq(cam.projection(aspect), expected));
}

// ── Camera::orthographic_bounds ───────────────────────────────────────────────

#[test]
fn orthographic_bounds_variant() {
    let cam = Camera::orthographic_bounds(-6.0, 6.0, -1.0, 11.0, -1.0, 1.0);
    assert!(matches!(
        cam.projection,
        Projection::OrthographicBounds { .. }
    ));
}

#[test]
fn orthographic_bounds_aspect_independent() {
    let cam = Camera::orthographic_bounds(-6.0, 6.0, -1.0, 11.0, -1.0, 1.0);
    // Projection must be identical regardless of aspect since bounds are explicit.
    assert!(mat4_approx_eq(
        cam.projection(1.0),
        cam.projection(16.0 / 9.0)
    ));
}

#[test]
fn orthographic_bounds_view_proj_is_finite() {
    let cam = Camera::orthographic_bounds(-6.0, 6.0, -1.0, 11.0, -1.0, 1.0);
    assert!(is_finite_mat4(cam.view_proj(1.0)));
}

#[test]
fn orthographic_bounds_matches_manual() {
    let cam = Camera::orthographic_bounds(-6.0, 6.0, -1.0, 11.0, -1.0, 1.0);
    let expected = Mat4::orthographic_rh(-6.0, 6.0, -1.0, 11.0, -1.0, 1.0);
    assert!(mat4_approx_eq(cam.projection(1.0), expected));
}

// ── view / projection ─────────────────────────────────────────────────────────

#[test]
fn view_proj_equals_projection_times_view() {
    let cam = Camera::perspective(Vec3::new(3.0, 4.0, 5.0), 60.0, 0.1, 100.0);
    let aspect = 4.0 / 3.0_f32;
    let manual = cam.projection(aspect) * cam.view();
    assert!(mat4_approx_eq(cam.view_proj(aspect), manual));
}

#[test]
fn camera_up_default_is_y() {
    let cam = Camera::perspective(Vec3::new(0.0, 0.0, 5.0), 45.0, 0.1, 100.0);
    assert_eq!(cam.up, Vec3::Y);
}
