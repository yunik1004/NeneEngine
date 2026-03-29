use nene::{
    math::Vec3,
    renderer::{Light, MAX_LIGHTS},
};

// ── Light construction ────────────────────────────────────────────────────────

#[test]
fn directional_normalises_direction() {
    let l = Light::directional(Vec3::new(2.0, 0.0, 0.0), Vec3::ONE, 1.0);
    let vp = l.light_view_proj(Vec3::ZERO, 5.0);
    assert_ne!(vp, glam::Mat4::IDENTITY);
}

#[test]
fn directional_light_view_proj_not_identity() {
    let l = Light::directional(Vec3::new(1.0, -2.0, -1.0), Vec3::ONE, 1.0);
    let vp = l.light_view_proj(Vec3::ZERO, 5.0);
    assert_ne!(vp, glam::Mat4::IDENTITY);
}

#[test]
fn ambient_light_view_proj_is_identity() {
    let l = Light::ambient(Vec3::ONE, 0.1);
    assert_eq!(l.light_view_proj(Vec3::ZERO, 5.0), glam::Mat4::IDENTITY);
}

#[test]
fn point_light_view_proj_is_identity() {
    let l = Light::point(Vec3::new(1.0, 2.0, 3.0), Vec3::ONE, 1.0, 10.0);
    assert_eq!(l.light_view_proj(Vec3::ZERO, 5.0), glam::Mat4::IDENTITY);
}

#[test]
fn max_lights_is_reasonable() {
    assert!(MAX_LIGHTS >= 4);
}

#[test]
fn light_view_proj_different_centers() {
    let l = Light::directional(Vec3::new(0.0, -1.0, 0.0), Vec3::ONE, 1.0);
    let vp1 = l.light_view_proj(Vec3::ZERO, 5.0);
    let vp2 = l.light_view_proj(Vec3::new(10.0, 0.0, 0.0), 5.0);
    assert_ne!(vp1, vp2);
}

#[test]
fn light_view_proj_different_radii() {
    let l = Light::directional(Vec3::new(1.0, -1.0, 0.0), Vec3::ONE, 1.0);
    let vp1 = l.light_view_proj(Vec3::ZERO, 5.0);
    let vp2 = l.light_view_proj(Vec3::ZERO, 10.0);
    assert_ne!(vp1, vp2);
}
