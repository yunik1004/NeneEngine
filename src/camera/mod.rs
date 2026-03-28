use crate::math::{Mat4, Vec3, Vec4};

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

// ── Frustum ───────────────────────────────────────────────────────────────────

/// Six-plane view frustum extracted from a view-projection matrix.
///
/// Uses the Gribb-Hartmann method. Compatible with wgpu's NDC convention
/// (x,y ∈ [-1,1]; z ∈ [0,1]).
///
/// Each plane is stored as `Vec4(a, b, c, d)` where the half-space
/// `ax + by + cz + d ≥ 0` is the *inside*.
#[derive(Clone, Debug)]
pub struct Frustum {
    /// Planes in order: left, right, bottom, top, near, far.
    planes: [Vec4; 6],
}

impl Frustum {
    /// Extract the frustum from a combined view-projection matrix.
    pub fn from_view_proj(vp: Mat4) -> Self {
        let m = vp.transpose();
        let row = |i: usize| m.col(i);
        let planes = [
            row(3) + row(0), // left
            row(3) - row(0), // right
            row(3) + row(1), // bottom
            row(3) - row(1), // top
            row(2),          // near
            row(3) - row(2), // far
        ];
        Self {
            planes: planes.map(normalize_plane),
        }
    }

    /// Returns `true` if `point` is inside (or on) the frustum.
    pub fn test_point(&self, point: Vec3) -> bool {
        let p = Vec4::new(point.x, point.y, point.z, 1.0);
        self.planes.iter().all(|plane| plane.dot(p) >= 0.0)
    }

    /// Returns `true` if the sphere is at least partially inside the frustum.
    pub fn test_sphere(&self, center: Vec3, radius: f32) -> bool {
        let p = Vec4::new(center.x, center.y, center.z, 1.0);
        self.planes.iter().all(|plane| plane.dot(p) >= -radius)
    }

    /// Returns `true` if the AABB is at least partially inside the frustum.
    pub fn test_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            let px = if plane.x >= 0.0 { max.x } else { min.x };
            let py = if plane.y >= 0.0 { max.y } else { min.y };
            let pz = if plane.z >= 0.0 { max.z } else { min.z };
            if plane.x * px + plane.y * py + plane.z * pz + plane.w < 0.0 {
                return false;
            }
        }
        true
    }

    /// Convenience wrapper for 2D / tilemap use (z = 0 plane).
    pub fn test_rect_2d(&self, x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> bool {
        self.test_aabb(Vec3::new(x_min, y_min, 0.0), Vec3::new(x_max, y_max, 0.0))
    }

    /// Raw plane access (left, right, bottom, top, near, far).
    pub fn planes(&self) -> &[Vec4; 6] {
        &self.planes
    }
}

fn normalize_plane(p: Vec4) -> Vec4 {
    let len = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
    if len > 1e-8 { p / len } else { p }
}
