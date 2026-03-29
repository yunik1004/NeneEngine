//! Mouse picking — screen-to-world ray casting and intersection tests.
//!
//! # Workflow
//! 1. Get the mouse position from [`Input::mouse_pos`](crate::input::Input::mouse_pos).
//! 2. Cast a [`Ray`] from the camera: [`crate::camera::Camera::screen_to_ray`].
//! 3. Test the ray against each object's bounding volume.
//!
//! ```rust,ignore
//! fn update(&mut self, input: &Input, _time: &Time) {
//!     if input.mouse_button_pressed(MouseButton::Left) {
//!         let (x, y) = input.cursor_position();
//!         let ray = camera.screen_to_ray(x, y, width as f32, height as f32, aspect);
//!         for (i, obj) in self.objects.iter().enumerate() {
//!             if ray.cast_sphere(obj.center, obj.radius).is_some() {
//!                 self.selected = Some(i);
//!             }
//!         }
//!     }
//! }
//! ```

use crate::math::Vec3;

/// An infinite ray with an origin and a normalised direction.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    /// Creates a new ray. `direction` is normalised automatically.
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Position along the ray at parameter `t`: `origin + direction * t`.
    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Ray–AABB intersection (slab method).
    ///
    /// Returns the nearest positive `t` where the ray enters the box, or
    /// `None` if the ray misses or the box is behind the origin.
    pub fn cast_aabb(&self, min: Vec3, max: Vec3) -> Option<f32> {
        let inv = Vec3::new(
            1.0 / self.direction.x,
            1.0 / self.direction.y,
            1.0 / self.direction.z,
        );
        let t1 = (min - self.origin) * inv;
        let t2 = (max - self.origin) * inv;

        let t_min = t1.min(t2);
        let t_max = t1.max(t2);

        let t_enter = t_min.x.max(t_min.y).max(t_min.z);
        let t_exit = t_max.x.min(t_max.y).min(t_max.z);

        if t_exit >= t_enter.max(0.0) {
            Some(t_enter.max(0.0))
        } else {
            None
        }
    }

    /// Ray–sphere intersection.
    ///
    /// Returns the nearest positive `t`, or `None` if the ray misses.
    pub fn cast_sphere(&self, center: Vec3, radius: f32) -> Option<f32> {
        let oc = self.origin - center;
        let b = oc.dot(self.direction);
        let c = oc.dot(oc) - radius * radius;
        let disc = b * b - c;
        if disc < 0.0 {
            return None;
        }
        let sqrt_disc = disc.sqrt();
        let t0 = -b - sqrt_disc;
        let t1 = -b + sqrt_disc;
        if t0 >= 0.0 {
            Some(t0)
        } else if t1 >= 0.0 {
            Some(t1)
        } else {
            None
        }
    }

    /// Ray–plane intersection.
    ///
    /// `normal` must be normalised. Returns `t` if the ray hits the plane
    /// from the front side (`t ≥ 0`), otherwise `None`.
    pub fn cast_plane(&self, point: Vec3, normal: Vec3) -> Option<f32> {
        let denom = normal.dot(self.direction);
        if denom.abs() < 1e-6 {
            return None; // parallel
        }
        let t = (point - self.origin).dot(normal) / denom;
        if t >= 0.0 { Some(t) } else { None }
    }
}
