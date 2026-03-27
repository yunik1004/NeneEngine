//! Frustum culling.
//!
//! Extract a [`Frustum`] from a view-projection matrix, then test
//! points, spheres, and AABBs against it.
//!
//! ```
//! use nene::culling::Frustum;
//! use nene::math::{Mat4, Vec3};
//!
//! let vp = Mat4::IDENTITY;
//! let frustum = Frustum::from_view_proj(vp);
//! assert!(frustum.test_point(Vec3::ZERO));
//! ```

use crate::math::{Mat4, Vec3, Vec4};

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
        // Transpose so rows of vp become accessible as columns of `m`.
        let m = vp.transpose();
        let row = |i: usize| m.col(i);

        // wgpu NDC: z ∈ [0, 1]  →  near = row2, far = row3 - row2
        let planes = [
            row(3) + row(0), // left
            row(3) - row(0), // right
            row(3) + row(1), // bottom
            row(3) - row(1), // top
            row(2),          // near  (wgpu: z_ndc ≥ 0)
            row(3) - row(2), // far   (wgpu: z_ndc ≤ 1)
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

    /// Returns `true` if the axis-aligned bounding box is at least partially
    /// inside the frustum.
    ///
    /// `min` and `max` are the minimum and maximum corners in world space.
    pub fn test_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            // Positive vertex: corner most along the plane normal.
            let px = if plane.x >= 0.0 { max.x } else { min.x };
            let py = if plane.y >= 0.0 { max.y } else { min.y };
            let pz = if plane.z >= 0.0 { max.z } else { min.z };
            if plane.x * px + plane.y * py + plane.z * pz + plane.w < 0.0 {
                return false;
            }
        }
        true
    }

    /// Returns `true` if the 2D axis-aligned rectangle (z = 0 plane) is at
    /// least partially inside the frustum.
    ///
    /// Convenience wrapper for 2D / tilemap use.
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
