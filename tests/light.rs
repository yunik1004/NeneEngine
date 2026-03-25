use nene::{
    light::{DirectionalLight, PointLight, PointLightArray},
    math::Vec3,
};

fn approx_eq(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

// ── DirectionalLight ──────────────────────────────────────────────────────────

#[test]
fn directional_default_fields() {
    let light = DirectionalLight::default();
    assert_eq!(light.color, Vec3::ONE);
    assert_eq!(light.intensity, 1.0);
    assert!(light.direction.length() > 0.0);
}

#[test]
fn directional_new_stores_fields() {
    let dir = Vec3::new(0.0, -1.0, 0.0);
    let light = DirectionalLight::new(dir, Vec3::new(0.5, 0.5, 1.0), 2.0);
    assert!(approx_eq(light.direction.length(), 1.0));
    assert_eq!(light.color, Vec3::new(0.5, 0.5, 1.0));
    assert_eq!(light.intensity, 2.0);
}

#[test]
fn directional_direction_is_normalized() {
    let light = DirectionalLight::new(Vec3::new(2.0, 0.0, 0.0), Vec3::ONE, 1.0);
    assert!(approx_eq(light.direction.length(), 1.0));
}

#[test]
fn directional_size() {
    // Must be 32 bytes to match WGSL struct layout.
    assert_eq!(std::mem::size_of::<DirectionalLight>(), 32);
}

#[test]
fn directional_is_pod() {
    let light = DirectionalLight::default();
    let _bytes: &[u8] = bytemuck::bytes_of(&light);
}

// ── PointLight ────────────────────────────────────────────────────────────────

#[test]
fn point_default_fields() {
    let light = PointLight::default();
    assert_eq!(light.color, Vec3::ONE);
    assert_eq!(light.intensity, 1.0);
    assert!(light.radius > 0.0);
}

#[test]
fn point_new_stores_fields() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let light = PointLight::new(pos, Vec3::new(1.0, 0.5, 0.0), 4.0, 15.0);
    assert_eq!(light.position, pos);
    assert_eq!(light.color, Vec3::new(1.0, 0.5, 0.0));
    assert_eq!(light.intensity, 4.0);
    assert_eq!(light.radius, 15.0);
}

#[test]
fn point_size() {
    // Must be 32 bytes to match WGSL struct layout.
    assert_eq!(std::mem::size_of::<PointLight>(), 32);
}

#[test]
fn point_is_pod() {
    let light = PointLight::default();
    let _bytes: &[u8] = bytemuck::bytes_of(&light);
}

// ── PointLightArray ───────────────────────────────────────────────────────────

#[test]
fn array_empty() {
    let arr = PointLightArray::<4>::new(&[]);
    assert_eq!(arr.count, 0);
}

#[test]
fn array_stores_lights() {
    let a = PointLight::new(Vec3::new(1.0, 0.0, 0.0), Vec3::ONE, 1.0, 5.0);
    let b = PointLight::new(Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 2.0, 8.0);
    let arr = PointLightArray::<4>::new(&[a, b]);
    assert_eq!(arr.count, 2);
    assert_eq!(arr.lights[0].position, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(arr.lights[1].position, Vec3::new(0.0, 2.0, 0.0));
}

#[test]
fn array_full() {
    let lights: Vec<PointLight> = (0..8)
        .map(|i| PointLight::new(Vec3::new(i as f32, 0.0, 0.0), Vec3::ONE, 1.0, 5.0))
        .collect();
    let arr = PointLightArray::<8>::new(&lights);
    assert_eq!(arr.count, 8);
}

#[test]
#[should_panic]
fn array_too_many_lights_panics() {
    let lights: Vec<PointLight> = (0..5)
        .map(|i| PointLight::new(Vec3::new(i as f32, 0.0, 0.0), Vec3::ONE, 1.0, 5.0))
        .collect();
    PointLightArray::<4>::new(&lights);
}

#[test]
fn array_is_pod() {
    let arr = PointLightArray::<4>::new(&[PointLight::default()]);
    let _bytes: &[u8] = bytemuck::bytes_of(&arr);
}

#[test]
fn array_size() {
    // 16 bytes header + N × 32 bytes
    assert_eq!(std::mem::size_of::<PointLightArray<8>>(), 16 + 8 * 32);
}
