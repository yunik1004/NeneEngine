//! Built-in mesh primitives.

use std::f32::consts::{PI, TAU};

use crate::math::{Vec2, Vec3, Vec4};

use super::Vertex;

// ── Flat primitives ───────────────────────────────────────────────────────────

/// Filled axis-aligned rectangle.
pub fn rect(pos: Vec2, size: Vec2, color: Vec4) -> Vec<Vertex> {
    let (x1, y1) = (pos.x, pos.y);
    let (x2, y2) = (pos.x + size.x, pos.y + size.y);
    vec![
        Vertex {
            position: Vec3::new(x1, y1, 0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: Vec3::new(x2, y1, 0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: Vec3::new(x2, y2, 0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: Vec3::new(x1, y1, 0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: Vec3::new(x2, y2, 0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: Vec3::new(x1, y2, 0.0),
            color,
            ..Vertex::default()
        },
    ]
}

/// Filled circle with 32 segments.
pub fn circle(center: Vec2, radius: f32, color: Vec4) -> Vec<Vertex> {
    circle_segments(center, radius, color, 32)
}

/// Filled circle with explicit segment count.
pub fn circle_segments(center: Vec2, radius: f32, color: Vec4, segments: u32) -> Vec<Vertex> {
    let n = segments.max(3) as usize;
    let mut out = Vec::with_capacity(n * 3);
    let c = center.extend(0.0);
    for i in 0..n {
        let a0 = TAU * i as f32 / n as f32;
        let a1 = TAU * (i + 1) as f32 / n as f32;
        out.push(Vertex {
            position: c,
            color,
            ..Vertex::default()
        });
        out.push(Vertex {
            position: Vec3::new(
                center.x + a0.cos() * radius,
                center.y + a0.sin() * radius,
                0.0,
            ),
            color,
            ..Vertex::default()
        });
        out.push(Vertex {
            position: Vec3::new(
                center.x + a1.cos() * radius,
                center.y + a1.sin() * radius,
                0.0,
            ),
            color,
            ..Vertex::default()
        });
    }
    out
}

/// Filled triangle.
pub fn triangle(a: Vec2, b: Vec2, c: Vec2, color: Vec4) -> Vec<Vertex> {
    vec![
        Vertex {
            position: a.extend(0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: b.extend(0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: c.extend(0.0),
            color,
            ..Vertex::default()
        },
    ]
}

/// Thick line.
pub fn line(a: Vec2, b: Vec2, thickness: f32, color: Vec4) -> Vec<Vertex> {
    let d = b - a;
    let len = d.length().max(f32::EPSILON);
    let n = Vec2::new(-d.y, d.x) / len * thickness * 0.5;
    vec![
        Vertex {
            position: (a + n).extend(0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: (b + n).extend(0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: (b - n).extend(0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: (a + n).extend(0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: (b - n).extend(0.0),
            color,
            ..Vertex::default()
        },
        Vertex {
            position: (a - n).extend(0.0),
            color,
            ..Vertex::default()
        },
    ]
}

/// Rectangle outline (four lines).
pub fn rect_outline(pos: Vec2, size: Vec2, thickness: f32, color: Vec4) -> Vec<Vertex> {
    let (tl, tr, br, bl) = (
        pos,
        pos + Vec2::new(size.x, 0.0),
        pos + size,
        pos + Vec2::new(0.0, size.y),
    );
    let mut out = Vec::new();
    out.extend(line(tl, tr, thickness, color));
    out.extend(line(tr, br, thickness, color));
    out.extend(line(br, bl, thickness, color));
    out.extend(line(bl, tl, thickness, color));
    out
}

// ── Shape builders ────────────────────────────────────────────────────────────

/// Axis-aligned quad in the XY plane, centred at the origin.
pub struct QuadBuilder {
    size: Vec2,
}

impl QuadBuilder {
    pub fn color(self, color: Vec4) -> Vec<Vertex> {
        let h = self.size * 0.5;
        rect(-h, self.size, color)
    }

    pub fn mesh(self) -> (Vec<Vertex>, Vec<u32>) {
        let h = self.size * 0.5;
        let verts = vec![
            Vertex {
                position: Vec3::new(-h.x, -h.y, 0.0),
                normal: Vec3::Z,
                uv: Vec2::new(0.0, 1.0),
                ..Vertex::default()
            },
            Vertex {
                position: Vec3::new(h.x, -h.y, 0.0),
                normal: Vec3::Z,
                uv: Vec2::new(1.0, 1.0),
                ..Vertex::default()
            },
            Vertex {
                position: Vec3::new(h.x, h.y, 0.0),
                normal: Vec3::Z,
                uv: Vec2::new(1.0, 0.0),
                ..Vertex::default()
            },
            Vertex {
                position: Vec3::new(-h.x, h.y, 0.0),
                normal: Vec3::Z,
                uv: Vec2::new(0.0, 0.0),
                ..Vertex::default()
            },
        ];
        (verts, vec![0, 1, 2, 0, 2, 3])
    }
}

/// Axis-aligned box centred at the origin.
pub struct CubeBuilder {
    size: Vec3,
}

impl CubeBuilder {
    pub fn color(self, color: Vec4) -> Vec<Vertex> {
        let h = self.size * 0.5;
        let tris: [[Vec3; 3]; 12] = [
            [
                Vec3::new(-h.x, -h.y, h.z),
                Vec3::new(h.x, -h.y, h.z),
                Vec3::new(h.x, h.y, h.z),
            ],
            [
                Vec3::new(-h.x, -h.y, h.z),
                Vec3::new(h.x, h.y, h.z),
                Vec3::new(-h.x, h.y, h.z),
            ],
            [
                Vec3::new(h.x, -h.y, -h.z),
                Vec3::new(-h.x, -h.y, -h.z),
                Vec3::new(-h.x, h.y, -h.z),
            ],
            [
                Vec3::new(h.x, -h.y, -h.z),
                Vec3::new(-h.x, h.y, -h.z),
                Vec3::new(h.x, h.y, -h.z),
            ],
            [
                Vec3::new(h.x, -h.y, h.z),
                Vec3::new(h.x, -h.y, -h.z),
                Vec3::new(h.x, h.y, -h.z),
            ],
            [
                Vec3::new(h.x, -h.y, h.z),
                Vec3::new(h.x, h.y, -h.z),
                Vec3::new(h.x, h.y, h.z),
            ],
            [
                Vec3::new(-h.x, -h.y, -h.z),
                Vec3::new(-h.x, -h.y, h.z),
                Vec3::new(-h.x, h.y, h.z),
            ],
            [
                Vec3::new(-h.x, -h.y, -h.z),
                Vec3::new(-h.x, h.y, h.z),
                Vec3::new(-h.x, h.y, -h.z),
            ],
            [
                Vec3::new(-h.x, h.y, h.z),
                Vec3::new(h.x, h.y, h.z),
                Vec3::new(h.x, h.y, -h.z),
            ],
            [
                Vec3::new(-h.x, h.y, h.z),
                Vec3::new(h.x, h.y, -h.z),
                Vec3::new(-h.x, h.y, -h.z),
            ],
            [
                Vec3::new(-h.x, -h.y, -h.z),
                Vec3::new(h.x, -h.y, -h.z),
                Vec3::new(h.x, -h.y, h.z),
            ],
            [
                Vec3::new(-h.x, -h.y, -h.z),
                Vec3::new(h.x, -h.y, h.z),
                Vec3::new(-h.x, -h.y, h.z),
            ],
        ];
        tris.iter()
            .flat_map(|tri| {
                tri.iter().map(|&position| Vertex {
                    position,
                    color,
                    ..Vertex::default()
                })
            })
            .collect()
    }

    pub fn mesh(self) -> (Vec<Vertex>, Vec<u32>) {
        let h = self.size * 0.5;
        struct Face {
            normal: Vec3,
            positions: [Vec3; 4],
        }
        let faces = [
            Face {
                normal: Vec3::Z,
                positions: [
                    Vec3::new(-h.x, -h.y, h.z),
                    Vec3::new(h.x, -h.y, h.z),
                    Vec3::new(h.x, h.y, h.z),
                    Vec3::new(-h.x, h.y, h.z),
                ],
            },
            Face {
                normal: Vec3::NEG_Z,
                positions: [
                    Vec3::new(h.x, -h.y, -h.z),
                    Vec3::new(-h.x, -h.y, -h.z),
                    Vec3::new(-h.x, h.y, -h.z),
                    Vec3::new(h.x, h.y, -h.z),
                ],
            },
            Face {
                normal: Vec3::X,
                positions: [
                    Vec3::new(h.x, -h.y, h.z),
                    Vec3::new(h.x, -h.y, -h.z),
                    Vec3::new(h.x, h.y, -h.z),
                    Vec3::new(h.x, h.y, h.z),
                ],
            },
            Face {
                normal: Vec3::NEG_X,
                positions: [
                    Vec3::new(-h.x, -h.y, -h.z),
                    Vec3::new(-h.x, -h.y, h.z),
                    Vec3::new(-h.x, h.y, h.z),
                    Vec3::new(-h.x, h.y, -h.z),
                ],
            },
            Face {
                normal: Vec3::Y,
                positions: [
                    Vec3::new(-h.x, h.y, h.z),
                    Vec3::new(h.x, h.y, h.z),
                    Vec3::new(h.x, h.y, -h.z),
                    Vec3::new(-h.x, h.y, -h.z),
                ],
            },
            Face {
                normal: Vec3::NEG_Y,
                positions: [
                    Vec3::new(-h.x, -h.y, -h.z),
                    Vec3::new(h.x, -h.y, -h.z),
                    Vec3::new(h.x, -h.y, h.z),
                    Vec3::new(-h.x, -h.y, h.z),
                ],
            },
        ];
        let uvs = [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
        ];
        let mut verts = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for face in &faces {
            let base = verts.len() as u32;
            for (i, &position) in face.positions.iter().enumerate() {
                verts.push(Vertex {
                    position,
                    normal: face.normal,
                    uv: uvs[i],
                    ..Vertex::default()
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        (verts, indices)
    }
}

/// UV sphere centred at the origin.
pub struct SphereBuilder {
    radius: f32,
    stacks: u32,
    slices: u32,
}

impl SphereBuilder {
    pub fn color(self, color: Vec4) -> Vec<Vertex> {
        let stacks = self.stacks.max(2) as usize;
        let slices = self.slices.max(3) as usize;
        let row = slices + 1;
        let mut pts: Vec<Vec3> = Vec::with_capacity(row * (stacks + 1));
        for i in 0..=stacks {
            let phi = PI * i as f32 / stacks as f32;
            for j in 0..=slices {
                let theta = TAU * j as f32 / slices as f32;
                pts.push(Vec3::new(
                    phi.sin() * theta.cos() * self.radius,
                    phi.cos() * self.radius,
                    phi.sin() * theta.sin() * self.radius,
                ));
            }
        }
        let mut out = Vec::new();
        for i in 0..stacks {
            for j in 0..slices {
                let [a, b, c, d] = [
                    pts[i * row + j],
                    pts[i * row + j + 1],
                    pts[(i + 1) * row + j],
                    pts[(i + 1) * row + j + 1],
                ];
                out.extend([a, c, b, b, c, d].map(|position| Vertex {
                    position,
                    color,
                    ..Vertex::default()
                }));
            }
        }
        out
    }

    pub fn mesh(self) -> (Vec<Vertex>, Vec<u32>) {
        let stacks = self.stacks.max(2) as usize;
        let slices = self.slices.max(3) as usize;
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        for i in 0..=stacks {
            let phi = PI * i as f32 / stacks as f32;
            for j in 0..=slices {
                let theta = TAU * j as f32 / slices as f32;
                let normal = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
                verts.push(Vertex {
                    position: normal * self.radius,
                    normal,
                    uv: Vec2::new(j as f32 / slices as f32, i as f32 / stacks as f32),
                    ..Vertex::default()
                });
            }
        }
        let row = slices + 1;
        for i in 0..stacks {
            for j in 0..slices {
                let a = (i * row + j) as u32;
                let b = (i * row + j + 1) as u32;
                let c = ((i + 1) * row + j) as u32;
                let d = ((i + 1) * row + j + 1) as u32;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
        (verts, indices)
    }
}

/// Cylinder centred at the origin along the Y axis.
pub struct CylinderBuilder {
    radius: f32,
    height: f32,
    slices: u32,
}

impl CylinderBuilder {
    pub fn color(self, color: Vec4) -> Vec<Vertex> {
        let slices = self.slices.max(3) as usize;
        let hh = self.height * 0.5;
        let mut out = Vec::new();
        for i in 0..slices {
            let a0 = TAU * i as f32 / slices as f32;
            let a1 = TAU * (i + 1) as f32 / slices as f32;
            let p0 = Vec3::new(a0.cos() * self.radius, 0.0, a0.sin() * self.radius);
            let p1 = Vec3::new(a1.cos() * self.radius, 0.0, a1.sin() * self.radius);
            out.extend(
                [
                    p0 - Vec3::Y * hh,
                    p1 - Vec3::Y * hh,
                    p1 + Vec3::Y * hh,
                    p0 - Vec3::Y * hh,
                    p1 + Vec3::Y * hh,
                    p0 + Vec3::Y * hh,
                ]
                .map(|position| Vertex {
                    position,
                    color,
                    ..Vertex::default()
                }),
            );
        }
        for (y, flip) in [(-hh, true), (hh, false)] {
            let cap = Vec3::new(0.0, y, 0.0);
            for i in 0..slices {
                let a0 = TAU * i as f32 / slices as f32;
                let a1 = TAU * (i + 1) as f32 / slices as f32;
                let p0 = Vec3::new(a0.cos() * self.radius, y, a0.sin() * self.radius);
                let p1 = Vec3::new(a1.cos() * self.radius, y, a1.sin() * self.radius);
                let tri = if flip { [cap, p1, p0] } else { [cap, p0, p1] };
                out.extend(tri.map(|position| Vertex {
                    position,
                    color,
                    ..Vertex::default()
                }));
            }
        }
        out
    }

    pub fn mesh(self) -> (Vec<Vertex>, Vec<u32>) {
        let slices = self.slices.max(3) as usize;
        let hh = self.height * 0.5;
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        for i in 0..=slices {
            let theta = TAU * i as f32 / slices as f32;
            let normal = Vec3::new(theta.cos(), 0.0, theta.sin());
            let u = i as f32 / slices as f32;
            verts.push(Vertex {
                position: normal * self.radius - Vec3::Y * hh,
                normal,
                uv: Vec2::new(u, 1.0),
                ..Vertex::default()
            });
            verts.push(Vertex {
                position: normal * self.radius + Vec3::Y * hh,
                normal,
                uv: Vec2::new(u, 0.0),
                ..Vertex::default()
            });
        }
        for i in 0..slices {
            let b = (i * 2) as u32;
            indices.extend_from_slice(&[b, b + 2, b + 1, b + 1, b + 2, b + 3]);
        }
        for (y, normal, flip) in [(-hh, Vec3::NEG_Y, true), (hh, Vec3::Y, false)] {
            let centre = verts.len() as u32;
            verts.push(Vertex {
                position: Vec3::new(0.0, y, 0.0),
                normal,
                uv: Vec2::new(0.5, 0.5),
                ..Vertex::default()
            });
            let ring_start = verts.len() as u32;
            for i in 0..slices {
                let theta = TAU * i as f32 / slices as f32;
                let (cx, cz) = (theta.cos(), theta.sin());
                verts.push(Vertex {
                    position: Vec3::new(cx * self.radius, y, cz * self.radius),
                    normal,
                    uv: Vec2::new(cx * 0.5 + 0.5, cz * 0.5 + 0.5),
                    ..Vertex::default()
                });
            }
            for i in 0..slices as u32 {
                let a = ring_start + i;
                let b = ring_start + (i + 1) % slices as u32;
                if flip {
                    indices.extend_from_slice(&[centre, b, a]);
                } else {
                    indices.extend_from_slice(&[centre, a, b]);
                }
            }
        }
        (verts, indices)
    }
}

// ── Constructor functions ─────────────────────────────────────────────────────

pub fn quad(size: Vec2) -> QuadBuilder {
    QuadBuilder { size }
}
pub fn cube(size: Vec3) -> CubeBuilder {
    CubeBuilder { size }
}
pub fn sphere(radius: f32, stacks: u32, slices: u32) -> SphereBuilder {
    SphereBuilder {
        radius,
        stacks,
        slices,
    }
}

pub fn cylinder(radius: f32, height: f32, slices: u32) -> CylinderBuilder {
    CylinderBuilder {
        radius,
        height,
        slices,
    }
}
