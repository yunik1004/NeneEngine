use std::f32::consts::{FRAC_PI_2, PI};

use crate::math::{Quat, Vec2, Vec3, Vec4};

/// Types that can be linearly interpolated between two values.
pub trait Lerp: Copy {
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for f64 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t as f64
    }
}

impl Lerp for Vec2 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Lerp for Vec3 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Lerp for Vec4 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

/// Uses spherical interpolation (slerp) for smooth rotation.
impl Lerp for Quat {
    fn lerp(self, other: Self, t: f32) -> Self {
        self.slerp(other, t)
    }
}

/// Easing function applied to the normalised time `t ∈ [0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ease {
    Linear,
    SineIn,
    SineOut,
    SineInOut,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    QuartIn,
    QuartOut,
    QuartInOut,
    ElasticIn,
    ElasticOut,
    ElasticInOut,
    BounceIn,
    BounceOut,
    BounceInOut,
    BackIn,
    BackOut,
    BackInOut,
}

impl Ease {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Ease::Linear => t,
            Ease::SineIn => 1.0 - (t * FRAC_PI_2).cos(),
            Ease::SineOut => (t * FRAC_PI_2).sin(),
            Ease::SineInOut => -((PI * t).cos() - 1.0) / 2.0,
            Ease::QuadIn => t * t,
            Ease::QuadOut => 1.0 - (1.0 - t).powi(2),
            Ease::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Ease::CubicIn => t * t * t,
            Ease::CubicOut => 1.0 - (1.0 - t).powi(3),
            Ease::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Ease::QuartIn => t * t * t * t,
            Ease::QuartOut => 1.0 - (1.0 - t).powi(4),
            Ease::QuartInOut => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            Ease::ElasticIn => {
                if t == 0.0 {
                    return 0.0;
                }
                if t == 1.0 {
                    return 1.0;
                }
                let c = (2.0 * PI) / 3.0;
                -(2.0_f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c).sin()
            }
            Ease::ElasticOut => {
                if t == 0.0 {
                    return 0.0;
                }
                if t == 1.0 {
                    return 1.0;
                }
                let c = (2.0 * PI) / 3.0;
                2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c).sin() + 1.0
            }
            Ease::ElasticInOut => {
                if t == 0.0 {
                    return 0.0;
                }
                if t == 1.0 {
                    return 1.0;
                }
                let c = (2.0 * PI) / 4.5;
                if t < 0.5 {
                    -(2.0_f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c).sin()) / 2.0
                } else {
                    2.0_f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c).sin() / 2.0 + 1.0
                }
            }
            Ease::BounceOut => bounce_out(t),
            Ease::BounceIn => 1.0 - bounce_out(1.0 - t),
            Ease::BounceInOut => {
                if t < 0.5 {
                    (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }
            Ease::BackIn => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                C3 * t * t * t - C1 * t * t
            }
            Ease::BackOut => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
            }
            Ease::BackInOut => {
                const C2: f32 = 1.70158 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) / 2.0
                }
            }
        }
    }
}

fn bounce_out(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

/// What happens when a tween reaches its end.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum TweenLoop {
    #[default]
    Once,
    Loop,
    PingPong,
}

/// Interpolates a value of type `T` from `start` to `end` over `duration`
/// seconds using the specified [`Ease`] curve.
///
/// # Example
/// ```
/// use nene::time::{Tween, Ease};
/// use nene::math::Vec3;
///
/// let mut t = Tween::new(Vec3::ZERO, Vec3::X * 5.0, 1.0)
///     .with_ease(Ease::CubicOut);
///
/// let v = t.update(0.5);
/// assert!(v.x > 0.0 && v.x < 5.0);
/// assert!(!t.is_done());
///
/// t.update(0.5);
/// assert!(t.is_done());
/// ```
pub struct Tween<T: Lerp> {
    pub start: T,
    pub end: T,
    pub duration: f32,
    pub ease: Ease,
    pub looping: TweenLoop,
    elapsed: f32,
}

impl<T: Lerp> Tween<T> {
    pub fn new(start: T, end: T, duration: f32) -> Self {
        Self {
            start,
            end,
            duration: duration.max(0.0),
            ease: Ease::Linear,
            looping: TweenLoop::Once,
            elapsed: 0.0,
        }
    }

    pub fn with_ease(mut self, ease: Ease) -> Self {
        self.ease = ease;
        self
    }
    pub fn with_loop(mut self, looping: TweenLoop) -> Self {
        self.looping = looping;
        self
    }

    pub fn update(&mut self, dt: f32) -> T {
        if self.duration <= 0.0 {
            return self.end;
        }
        self.elapsed += dt;
        match self.looping {
            TweenLoop::Once => {
                self.elapsed = self.elapsed.min(self.duration);
            }
            TweenLoop::Loop => {
                self.elapsed = self.elapsed.rem_euclid(self.duration);
            }
            TweenLoop::PingPong => {}
        }
        self.value()
    }

    pub fn value(&self) -> T {
        self.start.lerp(self.end, self.ease.apply(self.raw_t()))
    }

    pub fn progress(&self) -> f32 {
        self.raw_t()
    }

    pub fn is_done(&self) -> bool {
        match self.looping {
            TweenLoop::Once => self.elapsed >= self.duration,
            TweenLoop::Loop | TweenLoop::PingPong => false,
        }
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
    }

    pub fn seek(&mut self, t: f32) {
        self.elapsed = t.clamp(0.0, 1.0) * self.duration;
    }

    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    fn raw_t(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        match self.looping {
            TweenLoop::Once | TweenLoop::Loop => (self.elapsed / self.duration).clamp(0.0, 1.0),
            TweenLoop::PingPong => {
                let period = self.duration * 2.0;
                let cycle = self.elapsed.rem_euclid(period);
                if cycle < self.duration {
                    cycle / self.duration
                } else {
                    (period - cycle) / self.duration
                }
            }
        }
    }
}

impl<T: Lerp + Default> Default for Tween<T> {
    fn default() -> Self {
        Self::new(T::default(), T::default(), 1.0)
    }
}
