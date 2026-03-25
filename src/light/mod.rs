use bytemuck::{Pod, Zeroable};

use crate::math::Vec3;

// ── WGSL snippets ─────────────────────────────────────────────────────────────

/// WGSL struct + helper function for ambient light.
pub const AMBIENT_LIGHT_WGSL: &str = r#"
struct AmbientLight {
    color:     vec3<f32>,
    intensity: f32,
}

fn ambient_light(light: AmbientLight) -> vec3<f32> {
    return light.color * light.intensity;
}
"#;

/// WGSL struct + helper function for a directional light.
pub const DIRECTIONAL_LIGHT_WGSL: &str = r#"
struct DirectionalLight {
    direction: vec3<f32>,
    intensity: f32,
    color:     vec3<f32>,
    _pad:      f32,
}

fn directional_light(light: DirectionalLight, normal: vec3<f32>) -> vec3<f32> {
    let d = max(dot(normalize(normal), normalize(-light.direction)), 0.0);
    return light.color * (light.intensity * d);
}
"#;

/// WGSL struct + helper function for a point light.
pub const POINT_LIGHT_WGSL: &str = r#"
struct PointLight {
    position:  vec3<f32>,
    intensity: f32,
    color:     vec3<f32>,
    radius:    f32,
}

fn point_light(light: PointLight, world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let to_light = light.position - world_pos;
    let dist     = length(to_light);
    let atten    = clamp(1.0 - (dist / light.radius), 0.0, 1.0);
    let atten2   = atten * atten;
    let d        = max(dot(normalize(normal), normalize(to_light)), 0.0);
    return light.color * (light.intensity * d * atten2);
}
"#;

/// Generate a WGSL struct + helper for an array of `n` point lights.
///
/// Combine with [`POINT_LIGHT_WGSL`] (include it first) and bind a
/// [`PointLightArray<N>`] uniform with the same `N`.
pub fn point_light_array_wgsl(n: usize) -> String {
    format!(
        r#"
struct PointLightArray {{
    count:  u32,
    _pad:   vec3<u32>,
    lights: array<PointLight, {n}>,
}}

fn point_lights(arr: PointLightArray, world_pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {{
    var result = vec3<f32>(0.0);
    for (var i = 0u; i < arr.count; i++) {{
        result += point_light(arr.lights[i], world_pos, normal);
    }}
    return result;
}}
"#
    )
}

// ── Light types ───────────────────────────────────────────────────────────────

/// Uniform base lighting applied to all surfaces regardless of normal or position.
///
/// Implements [`Pod`] — pass directly to [`Context::create_uniform_buffer`].
/// Matches the WGSL `AmbientLight` struct.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct AmbientLight {
    pub color: Vec3,
    pub intensity: f32,
}

impl AmbientLight {
    pub fn new(color: Vec3, intensity: f32) -> Self {
        Self { color, intensity }
    }
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self::new(Vec3::ONE, 0.1)
    }
}

/// A light that shines uniformly from one direction (e.g. the sun).
///
/// Implements [`Pod`] — pass directly to [`Context::create_uniform_buffer`] or
/// [`Context::update_uniform_buffer`]. Matches the WGSL `DirectionalLight` struct.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub intensity: f32,
    pub color: Vec3,
    pub _pad: f32,
}

impl DirectionalLight {
    pub fn new(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            direction: direction.normalize(),
            intensity,
            color,
            _pad: 0.0,
        }
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self::new(Vec3::new(1.0, -2.0, -1.0), Vec3::ONE, 1.0)
    }
}

/// A light that radiates in all directions from a point in world space.
///
/// Implements [`Pod`] — pass directly to [`Context::create_uniform_buffer`] or
/// [`Context::update_uniform_buffer`]. Matches the WGSL `PointLight` struct.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct PointLight {
    pub position: Vec3,
    pub intensity: f32,
    /// Linear RGB color.
    pub color: Vec3,
    /// Distance at which the light reaches zero.
    pub radius: f32,
}

impl PointLight {
    pub fn new(position: Vec3, color: Vec3, intensity: f32, radius: f32) -> Self {
        Self {
            position,
            intensity,
            color,
            radius,
        }
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 3.0, 3.0), Vec3::ONE, 1.0, 10.0)
    }
}

// ── PointLightArray ───────────────────────────────────────────────────────────

/// An array of `N` point lights.
///
/// Implements [`Pod`] — pass directly to [`Context::create_uniform_buffer`] or
/// [`Context::update_uniform_buffer`]. Matches the WGSL struct from [`point_light_array_wgsl`].
///
/// # Example
/// ```rust
/// let arr = PointLightArray::<8>::new(&[light_a, light_b]);
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PointLightArray<const N: usize> {
    pub count: u32,
    pub _pad: [u32; 3],
    pub lights: [PointLight; N],
}

unsafe impl<const N: usize> bytemuck::Pod for PointLightArray<N> {}
unsafe impl<const N: usize> bytemuck::Zeroable for PointLightArray<N> {}

impl<const N: usize> PointLightArray<N> {
    pub fn new(lights: &[PointLight]) -> Self {
        assert!(lights.len() <= N, "too many point lights (max {N})");
        let mut arr = Self {
            count: lights.len() as u32,
            _pad: [0; 3],
            lights: [PointLight::zeroed(); N],
        };
        arr.lights[..lights.len()].copy_from_slice(lights);
        arr
    }
}
