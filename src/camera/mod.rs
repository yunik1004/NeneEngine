use crate::math::{Mat4, Vec3};

/// Projection mode for a [`Camera`].
#[derive(Debug, Clone, Copy)]
pub enum Projection {
    Perspective {
        /// Vertical field of view in radians.
        fov: f32,
        near: f32,
        far: f32,
    },
    /// Centered orthographic; height derived from `width / aspect`.
    Orthographic { width: f32, near: f32, far: f32 },
    /// Orthographic with explicit world-space bounds (aspect-independent).
    OrthographicBounds {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    },
}

/// A camera that produces a view-projection matrix.
#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    /// Up vector. Defaults to `Vec3::Y`.
    pub up: Vec3,
    pub projection: Projection,
}

impl Camera {
    /// Perspective camera looking at the origin.
    pub fn perspective(position: Vec3, fov_degrees: f32, near: f32, far: f32) -> Self {
        Self {
            position,
            target: Vec3::ZERO,
            up: Vec3::Y,
            projection: Projection::Perspective {
                fov: fov_degrees.to_radians(),
                near,
                far,
            },
        }
    }

    /// Orthographic camera. `width` is the total horizontal extent of the view.
    /// Height is computed from `width / aspect`.
    pub fn orthographic(position: Vec3, width: f32, near: f32, far: f32) -> Self {
        Self {
            position,
            target: position + Vec3::NEG_Z,
            up: Vec3::Y,
            projection: Projection::Orthographic { width, near, far },
        }
    }

    /// Orthographic camera with explicit world-space bounds.
    /// Ignores aspect ratio — use when you need a fixed viewport (e.g. 2D games).
    pub fn orthographic_bounds(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position: Vec3::ZERO,
            target: Vec3::new(0.0, 0.0, -1.0),
            up: Vec3::Y,
            projection: Projection::OrthographicBounds {
                left,
                right,
                bottom,
                top,
                near,
                far,
            },
        }
    }

    /// View matrix (world → camera space).
    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Projection matrix for the given aspect ratio (width / height).
    pub fn projection(&self, aspect: f32) -> Mat4 {
        match self.projection {
            Projection::Perspective { fov, near, far } => {
                Mat4::perspective_rh(fov, aspect, near, far)
            }
            Projection::Orthographic { width, near, far } => {
                let half_w = width * 0.5;
                let half_h = half_w / aspect;
                Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, near, far)
            }
            Projection::OrthographicBounds {
                left,
                right,
                bottom,
                top,
                near,
                far,
            } => Mat4::orthographic_rh(left, right, bottom, top, near, far),
        }
    }

    /// Combined view-projection matrix.
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }
}
