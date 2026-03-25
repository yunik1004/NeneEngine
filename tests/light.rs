use nene::{
    light::{DirectionalLight, DirectionalLightUniform, PointLight, PointLightUniform},
    math::Vec3,
};

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

// ── DirectionalLight ──────────────────────────────────────────────────────────

#[test]
fn directional_default_fields() {
    let light = DirectionalLight::default();
    assert_eq!(light.color, [1.0, 1.0, 1.0]);
    assert_eq!(light.intensity, 1.0);
    assert!(light.direction.length() > 0.0);
}

#[test]
fn directional_new_stores_fields() {
    let dir = Vec3::new(0.0, -1.0, 0.0);
    let light = DirectionalLight::new(dir, [0.5, 0.5, 1.0], 2.0);
    assert_eq!(light.direction, dir);
    assert_eq!(light.color, [0.5, 0.5, 1.0]);
    assert_eq!(light.intensity, 2.0);
}

#[test]
fn directional_to_uniform_direction_is_normalized() {
    let light = DirectionalLight::new(Vec3::new(2.0, 0.0, 0.0), [1.0, 1.0, 1.0], 1.0);
    let u = light.to_uniform();
    let len = (u.direction[0].powi(2) + u.direction[1].powi(2) + u.direction[2].powi(2)).sqrt();
    assert!(approx_eq(len, 1.0));
}

#[test]
fn directional_to_uniform_color_and_intensity() {
    let light = DirectionalLight::new(Vec3::X, [0.2, 0.4, 0.8], 3.0);
    let u = light.to_uniform();
    assert_eq!(u.color, [0.2, 0.4, 0.8]);
    assert_eq!(u.intensity, 3.0);
}

#[test]
fn directional_to_uniform_pad_is_zero() {
    let light = DirectionalLight::default();
    let u = light.to_uniform();
    assert_eq!(u._pad, 0.0);
}

#[test]
fn directional_uniform_size() {
    // Must be 32 bytes to match WGSL struct layout.
    assert_eq!(std::mem::size_of::<DirectionalLightUniform>(), 32);
}

#[test]
fn directional_uniform_is_pod() {
    // Verify bytemuck cast works without panic.
    let u = DirectionalLight::default().to_uniform();
    let _bytes: &[u8] = bytemuck::bytes_of(&u);
}

// ── PointLight ────────────────────────────────────────────────────────────────

#[test]
fn point_default_fields() {
    let light = PointLight::default();
    assert_eq!(light.color, [1.0, 1.0, 1.0]);
    assert_eq!(light.intensity, 1.0);
    assert!(light.radius > 0.0);
}

#[test]
fn point_new_stores_fields() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let light = PointLight::new(pos, [1.0, 0.5, 0.0], 4.0, 15.0);
    assert_eq!(light.position, pos);
    assert_eq!(light.color, [1.0, 0.5, 0.0]);
    assert_eq!(light.intensity, 4.0);
    assert_eq!(light.radius, 15.0);
}

#[test]
fn point_to_uniform_position() {
    let pos = Vec3::new(3.0, 1.0, -2.0);
    let light = PointLight::new(pos, [1.0, 1.0, 1.0], 1.0, 10.0);
    let u = light.to_uniform();
    assert_eq!(u.position, [3.0, 1.0, -2.0]);
}

#[test]
fn point_to_uniform_color_intensity_radius() {
    let light = PointLight::new(Vec3::ZERO, [0.1, 0.2, 0.3], 2.5, 8.0);
    let u = light.to_uniform();
    assert_eq!(u.color, [0.1, 0.2, 0.3]);
    assert_eq!(u.intensity, 2.5);
    assert_eq!(u.radius, 8.0);
}

#[test]
fn point_uniform_size() {
    // Must be 32 bytes to match WGSL struct layout.
    assert_eq!(std::mem::size_of::<PointLightUniform>(), 32);
}

#[test]
fn point_uniform_is_pod() {
    let u = PointLight::default().to_uniform();
    let _bytes: &[u8] = bytemuck::bytes_of(&u);
}
