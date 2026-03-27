use nene::debug::{DebugBuffer, color};
use nene::math::Vec3;

// ── color constants ───────────────────────────────────────────────────────────

#[test]
fn color_red() {
    assert_eq!(color::RED, Vec3::new(1.0, 0.0, 0.0));
}

#[test]
fn color_green() {
    assert_eq!(color::GREEN, Vec3::new(0.0, 1.0, 0.0));
}

#[test]
fn color_blue() {
    assert_eq!(color::BLUE, Vec3::new(0.0, 0.5, 1.0));
}

#[test]
fn color_yellow() {
    assert_eq!(color::YELLOW, Vec3::new(1.0, 1.0, 0.0));
}

#[test]
fn color_white() {
    assert_eq!(color::WHITE, Vec3::new(1.0, 1.0, 1.0));
}

#[test]
fn color_gray() {
    assert_eq!(color::GRAY, Vec3::new(0.5, 0.5, 0.5));
}

#[test]
fn color_cyan() {
    assert_eq!(color::CYAN, Vec3::new(0.0, 1.0, 1.0));
}

#[test]
fn color_magenta() {
    assert_eq!(color::MAGENTA, Vec3::new(1.0, 0.0, 1.0));
}

#[test]
fn color_orange() {
    assert_eq!(color::ORANGE, Vec3::new(1.0, 0.5, 0.0));
}

// ── DebugBuffer vertex accumulation ──────────────────────────────────────────

#[test]
fn line_adds_two_vertices() {
    let mut b = DebugBuffer::new();
    b.line(Vec3::ZERO, Vec3::X, color::RED);
    assert_eq!(b.vertex_count(), 2);
}

#[test]
fn ray_adds_two_vertices() {
    let mut b = DebugBuffer::new();
    b.ray(Vec3::ZERO, Vec3::Y, 5.0, color::CYAN);
    assert_eq!(b.vertex_count(), 2);
}

#[test]
fn axes_adds_six_vertices() {
    let mut b = DebugBuffer::new();
    b.axes(Vec3::ZERO, 1.0);
    assert_eq!(b.vertex_count(), 6);
}

#[test]
fn aabb_adds_24_vertices() {
    // 12 edges × 2 endpoints = 24
    let mut b = DebugBuffer::new();
    b.aabb(Vec3::splat(-1.0), Vec3::splat(1.0), color::WHITE);
    assert_eq!(b.vertex_count(), 24);
}

#[test]
fn circle_adds_48_vertices() {
    // 24 segments × 2 endpoints = 48
    let mut b = DebugBuffer::new();
    b.circle(Vec3::ZERO, Vec3::Y, 1.0, color::RED);
    assert_eq!(b.vertex_count(), 48);
}

#[test]
fn sphere_adds_144_vertices() {
    // 3 circles × 48 = 144
    let mut b = DebugBuffer::new();
    b.sphere(Vec3::ZERO, 1.0, color::GREEN);
    assert_eq!(b.vertex_count(), 144);
}

#[test]
fn multiple_primitives_accumulate() {
    let mut b = DebugBuffer::new();
    b.line(Vec3::ZERO, Vec3::X, color::RED); // 2
    b.line(Vec3::ZERO, Vec3::Y, color::GREEN); // 2
    b.axes(Vec3::ZERO, 1.0); // 6
    assert_eq!(b.vertex_count(), 10);
}

#[test]
fn buffer_starts_empty() {
    let b = DebugBuffer::new();
    assert_eq!(b.vertex_count(), 0);
}

#[test]
fn ray_endpoint_is_correct() {
    let mut b = DebugBuffer::new();
    b.ray(Vec3::ZERO, Vec3::X, 3.0, color::RED);
    // verts[0] = origin, verts[1] = origin + dir * length = (3, 0, 0)
    let end = Vec3::from(b.verts[1].pos);
    assert!((end - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5);
}

#[test]
fn line_color_stored_correctly() {
    let mut b = DebugBuffer::new();
    b.line(Vec3::ZERO, Vec3::X, color::YELLOW);
    assert_eq!(b.verts[0].col, color::YELLOW.to_array());
    assert_eq!(b.verts[1].col, color::YELLOW.to_array());
}
