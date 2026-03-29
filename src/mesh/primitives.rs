//! Built-in mesh primitives.

use std::f32::consts::{PI, TAU};

use crate::math::{Vec2, Vec3, Vec4};

use super::Vertex;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn colored(position: Vec3, color: Vec4) -> Vertex {
    Vertex { position, color, ..Vertex::default() }
}

fn textured(position: Vec3, normal: Vec3, uv: Vec2) -> Vertex {
    Vertex { position, normal, uv, ..Vertex::default() }
}

// ── Flat primitives ───────────────────────────────────────────────────────────

/// Filled axis-aligned rectangle.
pub fn rect(x: f32, y: f32, w: f32, h: f32, color: Vec4) -> Vec<Vertex> {
    let (x1, y1, x2, y2) = (x, y, x + w, y + h);
    vec![
        colored(Vec3::new(x1, y1, 0.0), color),
        colored(Vec3::new(x2, y1, 0.0), color),
        colored(Vec3::new(x2, y2, 0.0), color),
        colored(Vec3::new(x1, y1, 0.0), color),
        colored(Vec3::new(x2, y2, 0.0), color),
        colored(Vec3::new(x1, y2, 0.0), color),
    ]
}

/// Filled circle with 32 segments.
pub fn circle(cx: f32, cy: f32, radius: f32, color: Vec4) -> Vec<Vertex> {
    circle_segments(cx, cy, radius, color, 32)
}

/// Filled circle with explicit segment count.
pub fn circle_segments(cx: f32, cy: f32, radius: f32, color: Vec4, segments: u32) -> Vec<Vertex> {
    let n = segments.max(3) as usize;
    let mut out = Vec::with_capacity(n * 3);
    let centre = Vec3::new(cx, cy, 0.0);
    for i in 0..n {
        let a0 = TAU * i as f32 / n as f32;
        let a1 = TAU * (i + 1) as f32 / n as f32;
        out.push(colored(centre, color));
        out.push(colored(Vec3::new(cx + a0.cos() * radius, cy + a0.sin() * radius, 0.0), color));
        out.push(colored(Vec3::new(cx + a1.cos() * radius, cy + a1.sin() * radius, 0.0), color));
    }
    out
}

/// Filled triangle.
pub fn triangle(a: Vec2, b: Vec2, c: Vec2, color: Vec4) -> Vec<Vertex> {
    vec![
        colored(a.extend(0.0), color),
        colored(b.extend(0.0), color),
        colored(c.extend(0.0), color),
    ]
}

/// Thick line.
pub fn line(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Vec4) -> Vec<Vertex> {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len = (dx * dx + dy * dy).sqrt().max(f32::EPSILON);
    let (nx, ny) = (-dy / len * thickness * 0.5, dx / len * thickness * 0.5);
    vec![
        colored(Vec3::new(x1 + nx, y1 + ny, 0.0), color),
        colored(Vec3::new(x2 + nx, y2 + ny, 0.0), color),
        colored(Vec3::new(x2 - nx, y2 - ny, 0.0), color),
        colored(Vec3::new(x1 + nx, y1 + ny, 0.0), color),
        colored(Vec3::new(x2 - nx, y2 - ny, 0.0), color),
        colored(Vec3::new(x1 - nx, y1 - ny, 0.0), color),
    ]
}

/// Rectangle outline (four lines).
pub fn rect_outline(x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Vec4) -> Vec<Vertex> {
    let mut out = Vec::new();
    out.extend(line(x, y, x + w, y, thickness, color));
    out.extend(line(x + w, y, x + w, y + h, thickness, color));
    out.extend(line(x + w, y + h, x, y + h, thickness, color));
    out.extend(line(x, y + h, x, y, thickness, color));
    out
}

// ── Shape builders ────────────────────────────────────────────────────────────

/// Axis-aligned quad in the XY plane, centred at the origin.
pub struct QuadBuilder {
    w: f32,
    h: f32,
}

impl QuadBuilder {
    pub fn color(self, color: Vec4) -> Vec<Vertex> {
        let (hw, hh) = (self.w * 0.5, self.h * 0.5);
        rect(-hw, -hh, self.w, self.h, color)
    }

    pub fn mesh(self) -> (Vec<Vertex>, Vec<u32>) {
        let (hw, hh) = (self.w * 0.5, self.h * 0.5);
        let n = Vec3::Z;
        let verts = vec![
            textured(Vec3::new(-hw, -hh, 0.0), n, Vec2::new(0.0, 1.0)),
            textured(Vec3::new( hw, -hh, 0.0), n, Vec2::new(1.0, 1.0)),
            textured(Vec3::new( hw,  hh, 0.0), n, Vec2::new(1.0, 0.0)),
            textured(Vec3::new(-hw,  hh, 0.0), n, Vec2::new(0.0, 0.0)),
        ];
        (verts, vec![0, 1, 2, 0, 2, 3])
    }
}

/// Axis-aligned box centred at the origin.
pub struct CubeBuilder {
    sx: f32,
    sy: f32,
    sz: f32,
}

impl CubeBuilder {
    pub fn color(self, color: Vec4) -> Vec<Vertex> {
        let (hx, hy, hz) = (self.sx * 0.5, self.sy * 0.5, self.sz * 0.5);
        let tris: [[Vec3; 3]; 12] = [
            [Vec3::new(-hx, -hy,  hz), Vec3::new( hx, -hy,  hz), Vec3::new( hx,  hy,  hz)],
            [Vec3::new(-hx, -hy,  hz), Vec3::new( hx,  hy,  hz), Vec3::new(-hx,  hy,  hz)],
            [Vec3::new( hx, -hy, -hz), Vec3::new(-hx, -hy, -hz), Vec3::new(-hx,  hy, -hz)],
            [Vec3::new( hx, -hy, -hz), Vec3::new(-hx,  hy, -hz), Vec3::new( hx,  hy, -hz)],
            [Vec3::new( hx, -hy,  hz), Vec3::new( hx, -hy, -hz), Vec3::new( hx,  hy, -hz)],
            [Vec3::new( hx, -hy,  hz), Vec3::new( hx,  hy, -hz), Vec3::new( hx,  hy,  hz)],
            [Vec3::new(-hx, -hy, -hz), Vec3::new(-hx, -hy,  hz), Vec3::new(-hx,  hy,  hz)],
            [Vec3::new(-hx, -hy, -hz), Vec3::new(-hx,  hy,  hz), Vec3::new(-hx,  hy, -hz)],
            [Vec3::new(-hx,  hy,  hz), Vec3::new( hx,  hy,  hz), Vec3::new( hx,  hy, -hz)],
            [Vec3::new(-hx,  hy,  hz), Vec3::new( hx,  hy, -hz), Vec3::new(-hx,  hy, -hz)],
            [Vec3::new(-hx, -hy, -hz), Vec3::new( hx, -hy, -hz), Vec3::new( hx, -hy,  hz)],
            [Vec3::new(-hx, -hy, -hz), Vec3::new( hx, -hy,  hz), Vec3::new(-hx, -hy,  hz)],
        ];
        tris.iter()
            .flat_map(|tri| tri.iter().map(|&p| colored(p, color)))
            .collect()
    }

    pub fn mesh(self) -> (Vec<Vertex>, Vec<u32>) {
        let (hx, hy, hz) = (self.sx * 0.5, self.sy * 0.5, self.sz * 0.5);
        struct Face { normal: Vec3, positions: [Vec3; 4] }
        let faces = [
            Face { normal: Vec3::Z,    positions: [Vec3::new(-hx,-hy, hz), Vec3::new( hx,-hy, hz), Vec3::new( hx, hy, hz), Vec3::new(-hx, hy, hz)] },
            Face { normal: Vec3::NEG_Z, positions: [Vec3::new( hx,-hy,-hz), Vec3::new(-hx,-hy,-hz), Vec3::new(-hx, hy,-hz), Vec3::new( hx, hy,-hz)] },
            Face { normal: Vec3::X,    positions: [Vec3::new( hx,-hy, hz), Vec3::new( hx,-hy,-hz), Vec3::new( hx, hy,-hz), Vec3::new( hx, hy, hz)] },
            Face { normal: Vec3::NEG_X, positions: [Vec3::new(-hx,-hy,-hz), Vec3::new(-hx,-hy, hz), Vec3::new(-hx, hy, hz), Vec3::new(-hx, hy,-hz)] },
            Face { normal: Vec3::Y,    positions: [Vec3::new(-hx, hy, hz), Vec3::new( hx, hy, hz), Vec3::new( hx, hy,-hz), Vec3::new(-hx, hy,-hz)] },
            Face { normal: Vec3::NEG_Y, positions: [Vec3::new(-hx,-hy,-hz), Vec3::new( hx,-hy,-hz), Vec3::new( hx,-hy, hz), Vec3::new(-hx,-hy, hz)] },
        ];
        let uvs = [Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, 0.0)];
        let mut verts = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for face in &faces {
            let base = verts.len() as u32;
            for (i, &pos) in face.positions.iter().enumerate() {
                verts.push(textured(pos, face.normal, uvs[i]));
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
                let a = pts[i * row + j];
                let b = pts[i * row + j + 1];
                let c = pts[(i + 1) * row + j];
                let d = pts[(i + 1) * row + j + 1];
                out.push(colored(a, color));
                out.push(colored(c, color));
                out.push(colored(b, color));
                out.push(colored(b, color));
                out.push(colored(c, color));
                out.push(colored(d, color));
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
                let n = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
                verts.push(textured(
                    n * self.radius,
                    n,
                    Vec2::new(j as f32 / slices as f32, i as f32 / stacks as f32),
                ));
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
            out.push(colored(p0 - Vec3::Y * hh, color));
            out.push(colored(p1 - Vec3::Y * hh, color));
            out.push(colored(p1 + Vec3::Y * hh, color));
            out.push(colored(p0 - Vec3::Y * hh, color));
            out.push(colored(p1 + Vec3::Y * hh, color));
            out.push(colored(p0 + Vec3::Y * hh, color));
        }
        for (y, flip) in [(-hh, true), (hh, false)] {
            let cap_centre = Vec3::new(0.0, y, 0.0);
            for i in 0..slices {
                let a0 = TAU * i as f32 / slices as f32;
                let a1 = TAU * (i + 1) as f32 / slices as f32;
                let p0 = Vec3::new(a0.cos() * self.radius, y, a0.sin() * self.radius);
                let p1 = Vec3::new(a1.cos() * self.radius, y, a1.sin() * self.radius);
                if flip {
                    out.push(colored(cap_centre, color));
                    out.push(colored(p1, color));
                    out.push(colored(p0, color));
                } else {
                    out.push(colored(cap_centre, color));
                    out.push(colored(p0, color));
                    out.push(colored(p1, color));
                }
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
            let n = Vec3::new(theta.cos(), 0.0, theta.sin());
            let u = i as f32 / slices as f32;
            verts.push(textured(n * self.radius - Vec3::Y * hh, n, Vec2::new(u, 1.0)));
            verts.push(textured(n * self.radius + Vec3::Y * hh, n, Vec2::new(u, 0.0)));
        }
        for i in 0..slices {
            let b = (i * 2) as u32;
            indices.extend_from_slice(&[b, b + 2, b + 1, b + 1, b + 2, b + 3]);
        }
        for (y, ny, flip) in [(-hh, Vec3::NEG_Y, true), (hh, Vec3::Y, false)] {
            let centre = verts.len() as u32;
            verts.push(textured(Vec3::new(0.0, y, 0.0), ny, Vec2::new(0.5, 0.5)));
            let ring_start = verts.len() as u32;
            for i in 0..slices {
                let theta = TAU * i as f32 / slices as f32;
                let (cx, cz) = (theta.cos(), theta.sin());
                verts.push(textured(
                    Vec3::new(cx * self.radius, y, cz * self.radius),
                    ny,
                    Vec2::new(cx * 0.5 + 0.5, cz * 0.5 + 0.5),
                ));
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

pub fn quad(w: f32, h: f32) -> QuadBuilder { QuadBuilder { w, h } }
pub fn unit_quad() -> QuadBuilder { quad(1.0, 1.0) }
pub fn cube(sx: f32, sy: f32, sz: f32) -> CubeBuilder { CubeBuilder { sx, sy, sz } }
pub fn unit_cube() -> CubeBuilder { cube(1.0, 1.0, 1.0) }
pub fn sphere(radius: f32, stacks: u32, slices: u32) -> SphereBuilder {
    SphereBuilder { radius, stacks, slices }
}
pub fn unit_sphere() -> SphereBuilder { sphere(1.0, 32, 32) }
pub fn cylinder(radius: f32, height: f32, slices: u32) -> CylinderBuilder {
    CylinderBuilder { radius, height, slices }
}
