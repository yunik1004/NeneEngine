use bytemuck::{Pod, Zeroable};

use crate::math::Vec3;

// ── WGSL snippets ─────────────────────────────────────────────────────────────

/// WGSL struct + helper function for a directional light.
///
/// Bind with `var<uniform> dir_light: DirectionalLight;` and call
/// `directional_light(dir_light, normal)` to get a linear-space RGB contribution.
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
///
/// Bind with `var<uniform> pt_light: PointLight;` and call
/// `point_light(pt_light, world_pos, normal)` to get a linear-space RGB contribution.
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

// ── GPU-ready uniform structs ──────────────────────────────────────────────────

/// GPU layout for [`DirectionalLight`] (32 bytes, matches WGSL struct).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct DirectionalLightUniform {
    pub direction: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
    pub _pad: f32,
}

/// GPU layout for [`PointLight`] (32 bytes, matches WGSL struct).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PointLightUniform {
    pub position: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
    pub radius: f32,
}

// ── Public types ──────────────────────────────────────────────────────────────

/// A light that shines uniformly from one direction (e.g. the sun).
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    /// Direction the light travels (points *toward* the source to get normals right,
    /// but the WGSL helper negates it so you can pass the natural "from origin" direction).
    pub direction: Vec3,
    /// Linear RGB color. `[1.0, 1.0, 1.0]` is white.
    pub color: [f32; 3],
    /// Multiplier for the light contribution. `1.0` is full brightness.
    pub intensity: f32,
}

impl DirectionalLight {
    pub fn new(direction: Vec3, color: [f32; 3], intensity: f32) -> Self {
        Self {
            direction,
            color,
            intensity,
        }
    }

    pub fn to_uniform(&self) -> DirectionalLightUniform {
        DirectionalLightUniform {
            direction: self.direction.normalize().to_array(),
            intensity: self.intensity,
            color: self.color,
            _pad: 0.0,
        }
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(1.0, -2.0, -1.0),
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
        }
    }
}

/// A light that radiates in all directions from a point in world space.
#[derive(Debug, Clone)]
pub struct PointLight {
    pub position: Vec3,
    /// Linear RGB color.
    pub color: [f32; 3],
    pub intensity: f32,
    /// Distance at which the light reaches zero.
    pub radius: f32,
}

impl PointLight {
    pub fn new(position: Vec3, color: [f32; 3], intensity: f32, radius: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            radius,
        }
    }

    pub fn to_uniform(&self) -> PointLightUniform {
        PointLightUniform {
            position: self.position.to_array(),
            intensity: self.intensity,
            color: self.color,
            radius: self.radius,
        }
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 3.0, 3.0),
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            radius: 10.0,
        }
    }
}
