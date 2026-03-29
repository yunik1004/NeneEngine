use encase::ShaderType;

use crate::math::{Mat4, Vec3};

// ── Kind constants ─────────────────────────────────────────────────────────────

pub(crate) const KIND_AMBIENT: u32 = 0;
pub(crate) const KIND_DIRECTIONAL: u32 = 1;
pub(crate) const KIND_POINT: u32 = 2;

// ── GpuLight (GPU-side flat struct) ───────────────────────────────────────────

/// Flat GPU representation of any [`Light`] variant.
///
/// Stored in `MaterialUniform::lights`; the WGSL shader dispatches on `kind`.
///
/// Field layout (WGSL-compatible via encase):
/// ```text
/// position:  vec3<f32>  offset  0  (reused as `direction` for Directional)
/// kind:      u32        offset 12
/// color:     vec3<f32>  offset 16
/// intensity: f32        offset 28
/// radius:    f32        offset 32
///                              +12 padding → 48 bytes per light
/// ```
#[derive(Clone, Copy, ShaderType)]
pub(crate) struct GpuLight {
    /// World-space position (Point) or incoming direction (Directional).
    /// Unused for Ambient.
    pub position: Vec3,
    /// `0` = Ambient, `1` = Directional, `2` = Point.
    pub kind: u32,
    pub color: Vec3,
    pub intensity: f32,
    /// Falloff radius (Point only).
    pub radius: f32,
}

impl Default for GpuLight {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            kind: KIND_AMBIENT,
            color: Vec3::ZERO,
            intensity: 0.0,
            radius: 0.0,
        }
    }
}

// ── Light enum (Rust API) ─────────────────────────────────────────────────────

/// Maximum number of lights that can be active simultaneously.
pub const MAX_LIGHTS: usize = 8;

/// A scene light.
///
/// Pass a slice to [`Context::set_lights`](crate::renderer::Context::set_lights) each frame to update the
/// scene-wide light list shared by all [`Material`](crate::renderer::Material)s built with `.lights()`.
///
/// # Example
/// ```no_run
/// # use nene::renderer::{Context, Light};
/// # use nene::math::Vec3;
/// # fn example(ctx: &mut Context) {
/// ctx.set_lights(&[
///     Light::ambient(Vec3::ONE, 0.15),
///     Light::directional(Vec3::new(1.0, -2.0, -1.0), Vec3::ONE, 1.0),
/// ]);
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub enum Light {
    Ambient {
        color: Vec3,
        intensity: f32,
    },
    Directional {
        /// Incoming light direction (need not be normalised; normalised on upload).
        direction: Vec3,
        color: Vec3,
        intensity: f32,
    },
    Point {
        position: Vec3,
        color: Vec3,
        intensity: f32,
        /// Distance at which the light reaches zero.
        radius: f32,
    },
}

impl Light {
    pub fn ambient(color: Vec3, intensity: f32) -> Self {
        Self::Ambient { color, intensity }
    }

    pub fn directional(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self::Directional {
            direction: direction.normalize(),
            color,
            intensity,
        }
    }

    pub fn point(position: Vec3, color: Vec3, intensity: f32, radius: f32) -> Self {
        Self::Point {
            position,
            color,
            intensity,
            radius,
        }
    }

    /// Compute an orthographic view-projection matrix for shadow mapping.
    ///
    /// Only meaningful for `Directional` lights; returns `Mat4::IDENTITY` for
    /// other variants.
    ///
    /// `scene_center` is the world-space point to look at; `scene_radius` is
    /// the half-size of the orthographic frustum in world units.
    pub fn light_view_proj(&self, scene_center: Vec3, scene_radius: f32) -> Mat4 {
        let Self::Directional { direction, .. } = self else {
            return Mat4::IDENTITY;
        };
        let up = if direction.abs().dot(Vec3::Y) > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let pos = scene_center - *direction * scene_radius;
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

    pub(crate) fn to_gpu(self) -> GpuLight {
        match self {
            Self::Ambient { color, intensity } => GpuLight {
                position: Vec3::ZERO,
                kind: KIND_AMBIENT,
                color,
                intensity,
                radius: 0.0,
            },
            Self::Directional {
                direction,
                color,
                intensity,
            } => GpuLight {
                position: direction,
                kind: KIND_DIRECTIONAL,
                color,
                intensity,
                radius: 0.0,
            },
            Self::Point {
                position,
                color,
                intensity,
                radius,
            } => GpuLight {
                position,
                kind: KIND_POINT,
                color,
                intensity,
                radius,
            },
        }
    }
}

// ── SceneLightsUniform ────────────────────────────────────────────────────────

/// Scene-level light uniform — owned by [`Context`], uploaded once per frame.
///
/// Use [`Context::set_lights`] to update lights. The default contains a soft
/// directional + dim ambient so meshes render without explicit setup.
#[derive(Clone, Copy, encase::ShaderType)]
pub(crate) struct SceneLightsUniform {
    pub light_count: u32,
    pub lights: [GpuLight; MAX_LIGHTS],
}

impl SceneLightsUniform {
    pub(crate) fn set(&mut self, lights: &[Light]) {
        let n = lights.len().min(MAX_LIGHTS);
        self.light_count = n as u32;
        for (i, l) in lights.iter().take(n).enumerate() {
            self.lights[i] = l.to_gpu();
        }
    }
}

impl Default for SceneLightsUniform {
    fn default() -> Self {
        let mut lights = [GpuLight::default(); MAX_LIGHTS];
        lights[0] = Light::directional(Vec3::new(1.0, -2.0, -1.0), Vec3::ONE, 1.0).to_gpu();
        lights[1] = Light::ambient(Vec3::ONE, 0.1).to_gpu();
        Self {
            light_count: 2,
            lights,
        }
    }
}

// ── WGSL snippets ─────────────────────────────────────────────────────────────

/// WGSL `GpuLight` struct declaration.
///
/// Included automatically by the material shader when `.lights()` is used.
/// The struct layout matches [`GpuLight`] as encoded by encase.
pub(crate) const GPU_LIGHT_WGSL: &str = r#"
struct GpuLight {
    position:  vec3<f32>,
    kind:      u32,
    color:     vec3<f32>,
    intensity: f32,
    radius:    f32,
}
"#;

/// Generate the WGSL `SceneLights` struct + uniform binding at `group`.
pub(crate) fn scene_lights_wgsl(group: u32) -> String {
    format!(
        "
struct SceneLights {{
    light_count: u32,
    lights:      array<GpuLight, {MAX_LIGHTS}>,
}}
@group({group}) @binding(0) var<uniform> scene: SceneLights;
"
    )
}
