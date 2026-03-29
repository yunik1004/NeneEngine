// ── Rect geometry ─────────────────────────────────────────────────────────────

fn rect_contains(r: [f32; 4], p: [f32; 2]) -> bool {
    p[0] >= r[0] && p[0] < r[0] + r[2] && p[1] >= r[1] && p[1] < r[1] + r[3]
}

#[test]
fn contains_inside() {
    assert!(rect_contains([10.0, 20.0, 100.0, 30.0], [50.0, 30.0]));
}

#[test]
fn contains_top_left_corner() {
    assert!(rect_contains([10.0, 20.0, 100.0, 30.0], [10.0, 20.0]));
}

#[test]
fn not_contains_right_edge() {
    assert!(!rect_contains([10.0, 20.0, 100.0, 30.0], [110.0, 30.0]));
}

#[test]
fn not_contains_bottom_edge() {
    assert!(!rect_contains([10.0, 20.0, 100.0, 30.0], [50.0, 50.0]));
}

#[test]
fn not_contains_outside_left() {
    assert!(!rect_contains([10.0, 20.0, 100.0, 30.0], [9.9, 25.0]));
}

#[test]
fn not_contains_outside_above() {
    assert!(!rect_contains([10.0, 20.0, 100.0, 30.0], [50.0, 19.9]));
}

// ── Widget ID hashing ─────────────────────────────────────────────────────────

fn widget_id(label: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in label.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h | 1
}

#[test]
fn widget_id_nonzero() {
    assert_ne!(widget_id("button"), 0);
    assert_ne!(widget_id(""), 0);
}

#[test]
fn widget_id_same_label_same_id() {
    assert_eq!(widget_id("Speed"), widget_id("Speed"));
}

#[test]
fn widget_id_different_labels_different_ids() {
    assert_ne!(widget_id("Speed"), widget_id("Volume"));
    assert_ne!(widget_id("A"), widget_id("B"));
}

// ── Slider value clamping ─────────────────────────────────────────────────────

fn slider_value(mouse_x: f32, rect_x: f32, rect_w: f32, min: f32, max: f32) -> f32 {
    let t = ((mouse_x - rect_x) / rect_w).clamp(0.0, 1.0);
    min + t * (max - min)
}

#[test]
fn slider_at_left_edge_gives_min() {
    let v = slider_value(10.0, 10.0, 100.0, 0.0, 1.0);
    assert!((v - 0.0).abs() < 1e-5);
}

#[test]
fn slider_at_right_edge_gives_max() {
    let v = slider_value(110.0, 10.0, 100.0, 0.0, 1.0);
    assert!((v - 1.0).abs() < 1e-5);
}

#[test]
fn slider_at_midpoint() {
    let v = slider_value(60.0, 10.0, 100.0, 0.0, 10.0);
    assert!((v - 5.0).abs() < 1e-4);
}

#[test]
fn slider_clamps_below_min() {
    let v = slider_value(-100.0, 10.0, 100.0, 2.0, 8.0);
    assert!((v - 2.0).abs() < 1e-5);
}

#[test]
fn slider_clamps_above_max() {
    let v = slider_value(9999.0, 10.0, 100.0, 2.0, 8.0);
    assert!((v - 8.0).abs() < 1e-5);
}
