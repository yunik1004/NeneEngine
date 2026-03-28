use encase::ShaderType;

use crate::math::{Mat4, Vec3};

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
/// Matches the WGSL `AmbientLight` struct from [`AMBIENT_LIGHT_WGSL`].
#[derive(Debug, Clone, Copy, ShaderType)]
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
/// Matches the WGSL `DirectionalLight` struct from [`DIRECTIONAL_LIGHT_WGSL`].
#[derive(Debug, Clone, Copy, ShaderType)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub intensity: f32,
    pub color: Vec3,
}

impl DirectionalLight {
    pub fn new(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            direction: direction.normalize(),
            intensity,
            color,
        }
    }

    /// Compute an orthographic view-projection matrix for shadow mapping.
    ///
    /// `scene_center` is the center of the scene to cover, `scene_radius` is the
    /// half-size of the orthographic frustum in world units.
    pub fn light_view_proj(&self, scene_center: Vec3, scene_radius: f32) -> Mat4 {
        let up = if self.direction.abs().dot(Vec3::Y) > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let pos = scene_center - self.direction * scene_radius;
        let view = Mat4::look_at_rh(pos, scene_center, up);
        let proj = Mat4::orthographic_rh(
            -scene_radius,
            scene_radius,
            -scene_radius,
            scene_radius,
            0.0,
            scene_radius * 2.0,
        );
        proj * view
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self::new(Vec3::new(1.0, -2.0, -1.0), Vec3::ONE, 1.0)
    }
}

/// A light that radiates in all directions from a point in world space.
///
/// Matches the WGSL `PointLight` struct from [`POINT_LIGHT_WGSL`].
#[derive(Debug, Clone, Copy, ShaderType)]
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
/// Matches the WGSL struct from [`point_light_array_wgsl`].
///
/// # Example
/// ```rust,no_run
/// # use nene::renderer::{PointLight, PointLightArray};
/// # use nene::math::Vec3;
/// let a = PointLight::new(Vec3::new(1.0, 2.0, 0.0), Vec3::ONE, 1.0, 10.0);
/// let b = PointLight::new(Vec3::new(-1.0, 2.0, 0.0), Vec3::ONE, 1.0, 10.0);
/// let arr = PointLightArray::<8>::new(&[a, b]);
/// ```
#[derive(Clone, Copy, ShaderType)]
pub struct PointLightArray<const N: usize> {
    pub count: u32,
    pub lights: [PointLight; N],
}

impl<const N: usize> PointLightArray<N> {
    pub fn new(lights: &[PointLight]) -> Self {
        assert!(lights.len() <= N, "too many point lights (max {N})");
        let mut arr = Self {
            count: lights.len() as u32,
            lights: [PointLight::default(); N],
        };
        arr.lights[..lights.len()].copy_from_slice(lights);
        arr
    }
}
