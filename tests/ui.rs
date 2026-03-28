use nene::renderer::HeadlessContext;
use nene::ui::{Theme, Ui};

fn make_ctx() -> Option<HeadlessContext> {
    HeadlessContext::new()
}

// ── GPU: creation ─────────────────────────────────────────────────────────────

#[test]
fn ui_new_headless() {
    let Some(ctx) = make_ctx() else { return };
    let _ui = Ui::new_headless(&ctx);
}

#[test]
fn ui_begin_end_panel_no_crash() {
    let Some(ctx) = make_ctx() else { return };
    let mut ui = Ui::new_headless(&ctx);
    // Simulate a frame with panels and widgets (no GPU upload — just CPU state)
    ui.begin_panel("Test", 10.0, 10.0, 200.0);
    ui.label("hello");
    ui.end_panel();
}

#[test]
fn ui_multiple_panels() {
    let Some(ctx) = make_ctx() else { return };
    let mut ui = Ui::new_headless(&ctx);
    ui.begin_panel("A", 0.0, 0.0, 100.0);
    ui.label("a");
    ui.end_panel();
    ui.begin_panel("B", 200.0, 0.0, 100.0);
    ui.label("b");
    ui.end_panel();
}

#[test]
fn ui_all_widgets_no_crash() {
    let Some(ctx) = make_ctx() else { return };
    let mut ui = Ui::new_headless(&ctx);
    let mut checked = false;
    let mut val = 0.5_f32;
    ui.begin_panel("All", 0.0, 0.0, 250.0);
    ui.label("label");
    ui.label_dim("dim");
    let _ = ui.button("btn");
    ui.checkbox("check", &mut checked);
    ui.slider("val", &mut val, 0.0, 1.0);
    ui.separator();
    ui.end_panel();
}

#[test]
fn ui_button_returns_false_without_click() {
    let Some(ctx) = make_ctx() else { return };
    let mut ui = Ui::new_headless(&ctx);
    ui.begin_panel("P", 0.0, 0.0, 200.0);
    // No input injected → button must not fire
    assert!(!ui.button("No click"));
    ui.end_panel();
}

#[test]
fn ui_checkbox_default_unchanged() {
    let Some(ctx) = make_ctx() else { return };
    let mut ui = Ui::new_headless(&ctx);
    let mut v = false;
    ui.begin_panel("P", 0.0, 0.0, 200.0);
    let changed = ui.checkbox("box", &mut v);
    ui.end_panel();
    assert!(!changed);
    assert!(!v);
}

#[test]
fn ui_slider_value_unchanged_without_drag() {
    let Some(ctx) = make_ctx() else { return };
    let mut ui = Ui::new_headless(&ctx);
    let mut v = 0.5_f32;
    ui.begin_panel("P", 0.0, 0.0, 200.0);
    let changed = ui.slider("s", &mut v, 0.0, 1.0);
    ui.end_panel();
    assert!(!changed);
    assert!((v - 0.5).abs() < 1e-5);
}

#[test]
fn ui_theme_can_be_mutated() {
    let Some(ctx) = make_ctx() else { return };
    let mut ui = Ui::new_headless(&ctx);
    ui.theme.font_size = 18.0;
    assert_eq!(ui.theme.font_size, 18.0);
}

// ── CPU-only tests ─────────────────────────────────────────────────────────────

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
    // right edge is exclusive
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

// ── Theme defaults ────────────────────────────────────────────────────────────

#[test]
fn theme_default_font_size_positive() {
    assert!(Theme::default().font_size > 0.0);
}

#[test]
fn theme_default_item_height_positive() {
    assert!(Theme::default().item_height > 0.0);
}

#[test]
fn theme_clone() {
    let t = Theme::default();
    let _ = t.clone();
}
