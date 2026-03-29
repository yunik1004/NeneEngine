//! Built-in mesh primitives.

use std::f32::consts::{PI, TAU};

use super::Vertex;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn colored(x: f32, y: f32, z: f32, color: [f32; 4]) -> Vertex {
    Vertex {
        position: [x, y, z],
        color,
        ..Vertex::default()
    }
}

fn textured(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Vertex {
    Vertex {
        position,
        normal,
        uv,
        ..Vertex::default()
    }
}

// ── Flat primitives ───────────────────────────────────────────────────────────

/// Filled axis-aligned rectangle.
pub fn rect(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> Vec<Vertex> {
    let (x1, y1, x2, y2) = (x, y, x + w, y + h);
    vec![
        colored(x1, y1, 0.0, color),
        colored(x2, y1, 0.0, color),
        colored(x2, y2, 0.0, color),
        colored(x1, y1, 0.0, color),
        colored(x2, y2, 0.0, color),
        colored(x1, y2, 0.0, color),
    ]
}

/// Filled circle with 32 segments.
pub fn circle(cx: f32, cy: f32, radius: f32, color: [f32; 4]) -> Vec<Vertex> {
    circle_segments(cx, cy, radius, color, 32)
}

/// Filled circle with explicit segment count.
pub fn circle_segments(
    cx: f32,
    cy: f32,
    radius: f32,
    color: [f32; 4],
    segments: u32,
) -> Vec<Vertex> {
    let n = segments.max(3) as usize;
    let mut out = Vec::with_capacity(n * 3);
    for i in 0..n {
        let a0 = TAU * i as f32 / n as f32;
        let a1 = TAU * (i + 1) as f32 / n as f32;
        out.push(colored(cx, cy, 0.0, color));
        out.push(colored(cx + a0.cos() * radius, cy + a0.sin() * radius, 0.0, color));
        out.push(colored(cx + a1.cos() * radius, cy + a1.sin() * radius, 0.0, color));
    }
    out
}

/// Filled triangle.
pub fn triangle(a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 4]) -> Vec<Vertex> {
    vec![
        colored(a[0], a[1], 0.0, color),
        colored(b[0], b[1], 0.0, color),
        colored(c[0], c[1], 0.0, color),
    ]
}

/// Thick line.
pub fn line(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    thickness: f32,
    color: [f32; 4],
) -> Vec<Vertex> {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len = (dx * dx + dy * dy).sqrt().max(f32::EPSILON);
    let (nx, ny) = (-dy / len * thickness * 0.5, dx / len * thickness * 0.5);
    vec![
        colored(x1 + nx, y1 + ny, 0.0, color),
        colored(x2 + nx, y2 + ny, 0.0, color),
        colored(x2 - nx, y2 - ny, 0.0, color),
        colored(x1 + nx, y1 + ny, 0.0, color),
        colored(x2 - nx, y2 - ny, 0.0, color),
        colored(x1 - nx, y1 - ny, 0.0, color),
    ]
}

/// Rectangle outline (four lines).
pub fn rect_outline(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    thickness: f32,
    color: [f32; 4],
) -> Vec<Vertex> {
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
    pub fn color(self, color: [f32; 4]) -> Vec<Vertex> {
        let (hw, hh) = (self.w * 0.5, self.h * 0.5);
        rect(-hw, -hh, self.w, self.h, color)
    }

    pub fn mesh(self) -> (Vec<Vertex>, Vec<u32>) {
        let (hw, hh) = (self.w * 0.5, self.h * 0.5);
        let verts = vec![
            textured([-hw, -hh, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0]),
            textured([ hw, -hh, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0]),
            textured([ hw,  hh, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            textured([-hw,  hh, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
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
    pub fn color(self, color: [f32; 4]) -> Vec<Vertex> {
        let (hx, hy, hz) = (self.sx * 0.5, self.sy * 0.5, self.sz * 0.5);
        let tris: [[[f32; 3]; 3]; 12] = [
            [[-hx, -hy, hz], [hx, -hy, hz], [hx, hy, hz]],
            [[-hx, -hy, hz], [hx, hy, hz], [-hx, hy, hz]],
            [[hx, -hy, -hz], [-hx, -hy, -hz], [-hx, hy, -hz]],
            [[hx, -hy, -hz], [-hx, hy, -hz], [hx, hy, -hz]],
            [[hx, -hy, hz], [hx, -hy, -hz], [hx, hy, -hz]],
            [[hx, -hy, hz], [hx, hy, -hz], [hx, hy, hz]],
            [[-hx, -hy, -hz], [-hx, -hy, hz], [-hx, hy, hz]],
            [[-hx, -hy, -hz], [-hx, hy, hz], [-hx, hy, -hz]],
            [[-hx, hy, hz], [hx, hy, hz], [hx, hy, -hz]],
            [[-hx, hy, hz], [hx, hy, -hz], [-hx, hy, -hz]],
            [[-hx, -hy, -hz], [hx, -hy, -hz], [hx, -hy, hz]],
            [[-hx, -hy, -hz], [hx, -hy, hz], [-hx, -hy, hz]],
        ];
        tris.iter()
            .flat_map(|tri| tri.iter().map(|&[x, y, z]| colored(x, y, z, color)))
            .collect()
    }

    pub fn mesh(self) -> (Vec<Vertex>, Vec<u32>) {
        let (hx, hy, hz) = (self.sx * 0.5, self.sy * 0.5, self.sz * 0.5);
        struct Face {
            normal: [f32; 3],
            positions: [[f32; 3]; 4],
        }
        let faces = [
            Face {
                normal: [0., 0., 1.],
                positions: [[-hx, -hy, hz], [hx, -hy, hz], [hx, hy, hz], [-hx, hy, hz]],
            },
            Face {
                normal: [0., 0., -1.],
                positions: [
                    [hx, -hy, -hz],
                    [-hx, -hy, -hz],
                    [-hx, hy, -hz],
                    [hx, hy, -hz],
                ],
            },
            Face {
                normal: [1., 0., 0.],
                positions: [[hx, -hy, hz], [hx, -hy, -hz], [hx, hy, -hz], [hx, hy, hz]],
            },
            Face {
                normal: [-1., 0., 0.],
                positions: [
                    [-hx, -hy, -hz],
                    [-hx, -hy, hz],
                    [-hx, hy, hz],
                    [-hx, hy, -hz],
                ],
            },
            Face {
                normal: [0., 1., 0.],
                positions: [[-hx, hy, hz], [hx, hy, hz], [hx, hy, -hz], [-hx, hy, -hz]],
            },
            Face {
                normal: [0., -1., 0.],
                positions: [
                    [-hx, -hy, -hz],
                    [hx, -hy, -hz],
                    [hx, -hy, hz],
                    [-hx, -hy, hz],
                ],
            },
        ];
        let uvs = [[0.0f32, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
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
    pub fn color(self, color: [f32; 4]) -> Vec<Vertex> {
        let stacks = self.stacks.max(2) as usize;
        let slices = self.slices.max(3) as usize;
        let row = slices + 1;
        let mut pts: Vec<[f32; 3]> = Vec::with_capacity(row * (stacks + 1));
        for i in 0..=stacks {
            let phi = PI * i as f32 / stacks as f32;
            for j in 0..=slices {
                let theta = TAU * j as f32 / slices as f32;
                let nx = phi.sin() * theta.cos();
                let ny = phi.cos();
                let nz = phi.sin() * theta.sin();
                pts.push([nx * self.radius, ny * self.radius, nz * self.radius]);
            }
        }
        let mut out = Vec::new();
        for i in 0..stacks {
            for j in 0..slices {
                let [ax, ay, az] = pts[i * row + j];
                let [bx, by, bz] = pts[i * row + j + 1];
                let [cx, cy, cz] = pts[(i + 1) * row + j];
                let [dx, dy, dz] = pts[(i + 1) * row + j + 1];
                out.push(colored(ax, ay, az, color));
                out.push(colored(cx, cy, cz, color));
                out.push(colored(bx, by, bz, color));
                out.push(colored(bx, by, bz, color));
                out.push(colored(cx, cy, cz, color));
                out.push(colored(dx, dy, dz, color));
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
                let nx = phi.sin() * theta.cos();
                let ny = phi.cos();
                let nz = phi.sin() * theta.sin();
                verts.push(textured(
                    [nx * self.radius, ny * self.radius, nz * self.radius],
                    [nx, ny, nz],
                    [j as f32 / slices as f32, i as f32 / stacks as f32],
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
    pub fn color(self, color: [f32; 4]) -> Vec<Vertex> {
        let slices = self.slices.max(3) as usize;
        let hh = self.height * 0.5;
        let mut out = Vec::new();
        for i in 0..slices {
            let a0 = TAU * i as f32 / slices as f32;
            let a1 = TAU * (i + 1) as f32 / slices as f32;
            let (x0, z0) = (a0.cos() * self.radius, a0.sin() * self.radius);
            let (x1, z1) = (a1.cos() * self.radius, a1.sin() * self.radius);
            out.push(colored(x0, -hh, z0, color));
            out.push(colored(x1, -hh, z1, color));
            out.push(colored(x1,  hh, z1, color));
            out.push(colored(x0, -hh, z0, color));
            out.push(colored(x1,  hh, z1, color));
            out.push(colored(x0,  hh, z0, color));
        }
        for (y, flip) in [(-hh, true), (hh, false)] {
            for i in 0..slices {
                let a0 = TAU * i as f32 / slices as f32;
                let a1 = TAU * (i + 1) as f32 / slices as f32;
                let (x0, z0) = (a0.cos() * self.radius, a0.sin() * self.radius);
                let (x1, z1) = (a1.cos() * self.radius, a1.sin() * self.radius);
                if flip {
                    out.push(colored(0.0, y, 0.0, color));
                    out.push(colored(x1, y, z1, color));
                    out.push(colored(x0, y, z0, color));
                } else {
                    out.push(colored(0.0, y, 0.0, color));
                    out.push(colored(x0, y, z0, color));
                    out.push(colored(x1, y, z1, color));
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
            let nx = theta.cos();
            let nz = theta.sin();
            let u = i as f32 / slices as f32;
            verts.push(textured(
                [nx * self.radius, -hh, nz * self.radius],
                [nx, 0.0, nz],
                [u, 1.0],
            ));
            verts.push(textured(
                [nx * self.radius, hh, nz * self.radius],
                [nx, 0.0, nz],
                [u, 0.0],
            ));
        }
        for i in 0..slices {
            let b = (i * 2) as u32;
            indices.extend_from_slice(&[b, b + 2, b + 1, b + 1, b + 2, b + 3]);
        }
        for (y, ny, flip) in [(-hh, -1.0f32, true), (hh, 1.0f32, false)] {
            let centre = verts.len() as u32;
            verts.push(textured([0.0, y, 0.0], [0.0, ny, 0.0], [0.5, 0.5]));
            let ring_start = verts.len() as u32;
            for i in 0..slices {
                let theta = TAU * i as f32 / slices as f32;
                let (cx, cz) = (theta.cos(), theta.sin());
                verts.push(textured(
                    [cx * self.radius, y, cz * self.radius],
                    [0.0, ny, 0.0],
                    [cx * 0.5 + 0.5, cz * 0.5 + 0.5],
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

pub fn quad(w: f32, h: f32) -> QuadBuilder {
    QuadBuilder { w, h }
}
pub fn unit_quad() -> QuadBuilder {
    quad(1.0, 1.0)
}
pub fn cube(sx: f32, sy: f32, sz: f32) -> CubeBuilder {
    CubeBuilder { sx, sy, sz }
}
pub fn unit_cube() -> CubeBuilder {
    cube(1.0, 1.0, 1.0)
}
pub fn sphere(radius: f32, stacks: u32, slices: u32) -> SphereBuilder {
    SphereBuilder { radius, stacks, slices }
}
pub fn unit_sphere() -> SphereBuilder {
    sphere(1.0, 32, 32)
}
pub fn cylinder(radius: f32, height: f32, slices: u32) -> CylinderBuilder {
    CylinderBuilder { radius, height, slices }
}
