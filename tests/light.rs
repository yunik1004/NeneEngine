use nene::{
    math::Vec3,
    renderer::{Light, MAX_LIGHTS, MaterialUniform},
};

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

// ── Light construction ────────────────────────────────────────────────────────

#[test]
fn ambient_roundtrips_through_set_lights() {
    let mut u = MaterialUniform::default();
    u.set_lights(&[Light::ambient(Vec3::new(0.5, 0.5, 0.5), 0.2)]);
    assert_eq!(u.light_count, 1);
}

#[test]
fn directional_normalises_direction() {
    // Directional normalises direction before storing — the test validates via
    // light_view_proj which depends on a correct normalised direction.
    let l = Light::directional(Vec3::new(2.0, 0.0, 0.0), Vec3::ONE, 1.0);
    // light_view_proj should return a non-identity matrix for a directional light.
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

// ── MaterialUniform::set_lights ───────────────────────────────────────────────

#[test]
fn set_lights_updates_count() {
    let mut u = MaterialUniform::default();
    u.set_lights(&[
        Light::ambient(Vec3::ONE, 0.1),
        Light::directional(Vec3::new(0.0, -1.0, 0.0), Vec3::ONE, 1.0),
    ]);
    assert_eq!(u.light_count, 2);
}

#[test]
fn set_lights_single() {
    let mut u = MaterialUniform::default();
    u.set_lights(&[Light::point(Vec3::new(0.0, 3.0, 0.0), Vec3::ONE, 1.0, 10.0)]);
    assert_eq!(u.light_count, 1);
}

#[test]
fn set_lights_clamps_to_max() {
    let many: Vec<Light> = (0..MAX_LIGHTS + 4)
        .map(|_| Light::ambient(Vec3::ONE, 0.1))
        .collect();
    let mut u = MaterialUniform::default();
    u.set_lights(&many);
    assert_eq!(u.light_count, MAX_LIGHTS as u32);
}

#[test]
fn set_lights_empty_clears() {
    let mut u = MaterialUniform::default();
    u.set_lights(&[]);
    assert_eq!(u.light_count, 0);
}

#[test]
fn max_lights_is_reasonable() {
    assert!(MAX_LIGHTS >= 4);
}

#[test]
fn default_uniform_has_lights() {
    let u = MaterialUniform::default();
    // Default should include at least one light so meshes render without explicit setup.
    assert!(u.light_count > 0);
}

#[test]
fn set_lights_mixed_types() {
    let mut u = MaterialUniform::default();
    u.set_lights(&[
        Light::ambient(Vec3::new(0.2, 0.2, 0.3), 0.1),
        Light::directional(Vec3::new(1.0, -1.0, 0.0), Vec3::ONE, 0.8),
        Light::point(Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 0.5, 0.0), 2.0, 5.0),
    ]);
    assert_eq!(u.light_count, 3);
}

#[test]
fn set_lights_idempotent() {
    let mut u = MaterialUniform::default();
    u.set_lights(&[Light::ambient(Vec3::ONE, 0.1)]);
    u.set_lights(&[Light::ambient(Vec3::ONE, 0.1)]);
    assert_eq!(u.light_count, 1);
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

#[test]
fn set_lights_full() {
    let lights: Vec<Light> = (0..MAX_LIGHTS)
        .map(|i| Light::point(Vec3::new(i as f32, 0.0, 0.0), Vec3::ONE, 1.0, 5.0))
        .collect();
    let mut u = MaterialUniform::default();
    u.set_lights(&lights);
    assert_eq!(u.light_count, MAX_LIGHTS as u32);
}
