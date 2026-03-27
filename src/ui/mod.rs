//! Immediate-mode UI.
//!
//! # Quick start
//! ```no_run
//! use nene::ui::Ui;
//!
//! // In init:
//! // let mut ui = Ui::new(&ctx);
//!
//! // In update (each frame):
//! // ui.begin_frame(&input, width as f32, height as f32);
//! // ui.begin_panel("Settings", 10.0, 10.0, 220.0);
//! // ui.label("Hello world");
//! // if ui.button("Click me") { /* … */ }
//! // ui.slider("Speed", &mut speed, 0.0, 10.0);
//! // ui.end_panel();
//! // ui.end_frame(&ctx);
//!
//! // In render:
//! // ui.render(&mut pass);
//! ```

use crate::input::{Input, MouseButton};
use crate::renderer::{
    Pipeline, PipelineDescriptor, RenderContext, RenderPass, UniformBuffer, VertexAttribute,
    VertexBuffer, VertexFormat, VertexLayout,
};
use crate::text::TextRenderer;

// ── WGSL ──────────────────────────────────────────────────────────────────────

const QUAD_SHADER: &str = r#"
struct Screen { width: f32, height: f32 }
@group(0) @binding(0) var<uniform> screen: Screen;

struct VertOut {
    @builtin(position) clip  : vec4<f32>,
    @location(0)       color : vec4<f32>,
}

@vertex fn vs_main(
    @location(0) pos  : vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertOut {
    let ndc = vec2<f32>(
        pos.x / screen.width  * 2.0 - 1.0,
        1.0 - pos.y / screen.height * 2.0,
    );
    var out: VertOut;
    out.clip  = vec4<f32>(ndc, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

// ── Theme ─────────────────────────────────────────────────────────────────────

/// Visual style for all UI widgets.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Semi-transparent panel background.
    pub panel_bg: [f32; 4],
    pub item_bg: [f32; 4],
    pub item_hover: [f32; 4],
    pub item_active: [f32; 4],
    pub text_color: [f32; 4],
    pub text_dim: [f32; 4],
    /// Accent colour used for slider fill, checkbox tick, etc.
    pub accent: [f32; 4],
    pub slider_track: [f32; 4],
    pub font_size: f32,
    /// Height of one interactive item row (pixels).
    pub item_height: f32,
    /// Vertical gap between items (pixels).
    pub item_padding: f32,
    /// Horizontal/vertical padding inside the panel (pixels).
    pub panel_padding: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            panel_bg: [0.11, 0.11, 0.14, 0.93],
            item_bg: [0.20, 0.20, 0.25, 1.0],
            item_hover: [0.28, 0.28, 0.36, 1.0],
            item_active: [0.25, 0.50, 0.90, 1.0],
            text_color: [0.95, 0.95, 0.95, 1.0],
            text_dim: [0.58, 0.58, 0.65, 1.0],
            accent: [0.28, 0.58, 1.00, 1.0],
            slider_track: [0.15, 0.15, 0.20, 1.0],
            font_size: 18.0,
            item_height: 30.0,
            item_padding: 6.0,
            panel_padding: 10.0,
        }
    }
}

// ── Internal GPU types ────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct UiVert {
    pos: [f32; 2],
    color: [f32; 4],
}

#[derive(encase::ShaderType)]
struct ScreenUniform {
    width: f32,
    height: f32,
}

// ── Rect helper ───────────────────────────────────────────────────────────────

/// `[x, y, w, h]` in screen pixels, top-left origin.
type Rect = [f32; 4];

fn rect_contains(r: Rect, p: [f32; 2]) -> bool {
    p[0] >= r[0] && p[0] < r[0] + r[2] && p[1] >= r[1] && p[1] < r[1] + r[3]
}

// ── Widget ID ─────────────────────────────────────────────────────────────────

/// Stable widget ID derived from a label string (FNV-1a hash).
fn widget_id(label: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in label.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h | 1 // ensure non-zero (0 = "no widget")
}

// ── Panel layout ──────────────────────────────────────────────────────────────

struct Panel {
    x: f32,
    y: f32,
    w: f32,
    /// Current vertical write cursor (absolute screen Y).
    cursor_y: f32,
    /// Tallest Y reached so far — used to draw the panel background.
    bottom: f32,
}

impl Panel {
    /// Pixel-space rect for the next item.
    fn next_rect(&self, item_height: f32, padding: f32) -> Rect {
        [
            self.x + padding,
            self.cursor_y,
            self.w - padding * 2.0,
            item_height,
        ]
    }
}

// ── Queued text ───────────────────────────────────────────────────────────────

struct TextCmd {
    text: String,
    x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
}

// ── Max quads ─────────────────────────────────────────────────────────────────

const MAX_QUADS: usize = 4096;
const MAX_VERTS: usize = MAX_QUADS * 6;

// ── Ui ────────────────────────────────────────────────────────────────────────

/// Immediate-mode UI context + GPU renderer.
///
/// Call [`begin_frame`](Self::begin_frame) before any widget calls,
/// [`end_frame`](Self::end_frame) after the last widget, and
/// [`render`](Self::render) inside the render pass.
pub struct Ui {
    pub theme: Theme,

    // Layout stack
    panels: Vec<Panel>,

    // Input snapshot for this frame
    mouse_pos: [f32; 2],
    mouse_pressed: bool,
    mouse_down: bool,
    mouse_released: bool,

    // Retained hot/active state (stable across frames via label hash)
    hot: u32,
    active: u32,

    // CPU draw lists
    verts: Vec<UiVert>,
    texts: Vec<TextCmd>,

    // GPU
    pipeline: Pipeline,
    vbuf: VertexBuffer,
    ubuf: UniformBuffer,

    // Text renderer
    text: TextRenderer,

    screen_w: f32,
    screen_h: f32,

    // DPI scale: physical / logical (updated each end_frame)
    scale_x: f32,
    scale_y: f32,
}

impl Ui {
    /// Create the UI context. Accepts both [`Context`](crate::renderer::Context) and
    /// [`HeadlessContext`](crate::renderer::HeadlessContext).
    pub fn new(ctx: &impl RenderContext) -> Self {
        let vert_layout = VertexLayout {
            stride: std::mem::size_of::<UiVert>() as u64,
            attributes: vec![
                VertexAttribute {
                    offset: 0,
                    location: 0,
                    format: VertexFormat::Float32x2,
                },
                VertexAttribute {
                    offset: 8,
                    location: 1,
                    format: VertexFormat::Float32x4,
                },
            ],
        };

        let pipeline = ctx.create_pipeline(
            PipelineDescriptor::new(QUAD_SHADER, vert_layout)
                .with_uniform()
                .with_alpha_blend(),
        );

        let dummy = vec![
            UiVert {
                pos: [0.0; 2],
                color: [0.0; 4]
            };
            MAX_VERTS
        ];
        let vbuf = ctx.create_vertex_buffer(&dummy);

        let ubuf = ctx.create_uniform_buffer(&ScreenUniform {
            width: 1.0,
            height: 1.0,
        });

        Self {
            theme: Theme::default(),
            panels: Vec::new(),
            mouse_pos: [0.0; 2],
            mouse_pressed: false,
            mouse_down: false,
            mouse_released: false,
            hot: 0,
            active: 0,
            verts: Vec::with_capacity(MAX_VERTS),
            texts: Vec::new(),
            pipeline,
            vbuf,
            ubuf,
            text: ctx.create_text_renderer(),
            screen_w: 1.0,
            screen_h: 1.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }

    // ── Frame lifecycle ───────────────────────────────────────────────────────

    /// Call once per frame, before any widget calls.
    pub fn begin_frame(&mut self, input: &Input, screen_w: f32, screen_h: f32) {
        // input.mouse_pos() is already in logical coords (window divided by DPI scale)
        let mp = input.mouse_pos();
        self.mouse_pos = [mp.x, mp.y];
        self.mouse_pressed = input.mouse_pressed(MouseButton::Left);
        self.mouse_down = input.mouse_down(MouseButton::Left);
        self.mouse_released = input.mouse_released(MouseButton::Left);
        self.screen_w = screen_w;
        self.screen_h = screen_h;
        self.hot = 0;
        self.verts.clear();
        self.texts.clear();
        self.panels.clear();
    }

    /// Call after the last widget and before the render pass.
    /// Uploads geometry and text to the GPU.
    pub fn end_frame(&mut self, ctx: &crate::renderer::Context) {
        // Compute DPI scale: physical surface size / logical widget size
        let sc = ctx.surface_config();
        let phys_w = sc.width as f32;
        let phys_h = sc.height as f32;
        self.scale_x = if self.screen_w > 0.0 {
            phys_w / self.screen_w
        } else {
            1.0
        };
        self.scale_y = if self.screen_h > 0.0 {
            phys_h / self.screen_h
        } else {
            1.0
        };

        // Shader NDC conversion uses physical pixel coords
        ctx.update_uniform_buffer(
            &self.ubuf,
            &ScreenUniform {
                width: phys_w,
                height: phys_h,
            },
        );

        // Scale vertex positions from logical → physical before uploading
        let count = self.verts.len().min(MAX_VERTS);
        if count > 0 {
            let scaled: Vec<UiVert> = self.verts[..count]
                .iter()
                .map(|v| UiVert {
                    pos: [v.pos[0] * self.scale_x, v.pos[1] * self.scale_y],
                    color: v.color,
                })
                .collect();
            ctx.update_vertex_buffer(&self.vbuf, &scaled);
        }

        // Scale text positions and sizes from logical → physical before queuing
        let sx = self.scale_x;
        let sy = self.scale_y;
        for cmd in &self.texts {
            self.text.queue(
                &cmd.text,
                cmd.x * sx,
                cmd.y * sy,
                cmd.size * sx.min(sy),
                cmd.color,
            );
        }
        self.text.prepare(ctx);
    }

    /// Draw all UI into the current render pass.
    pub fn render(&self, pass: &mut RenderPass) {
        if !self.verts.is_empty() {
            pass.set_pipeline(&self.pipeline);
            pass.set_uniform(0, &self.ubuf);
            pass.set_vertex_buffer(0, &self.vbuf);
            pass.draw(0..self.verts.len() as u32);
        }
        self.text.render(pass);
    }

    // ── Panel ─────────────────────────────────────────────────────────────────

    /// Begin a vertically-stacking panel at `(x, y)` with fixed width `w`.
    ///
    /// Always call [`end_panel`](Self::end_panel) after the last widget.
    pub fn begin_panel(&mut self, title: &str, x: f32, y: f32, w: f32) {
        let pad = self.theme.panel_padding;
        let title_h = self.theme.item_height;

        // We don't know the height yet; draw background in end_panel.
        let cursor_y = y + title_h + pad;
        self.panels.push(Panel {
            x,
            y,
            w,
            cursor_y,
            bottom: cursor_y,
        });

        // Title bar
        let title_rect: Rect = [x, y, w, title_h];
        let title_color = darken(self.theme.panel_bg, 0.6);
        self.push_quad(title_rect, title_color);
        self.push_text_left(title, title_rect, pad, self.theme.text_color);
    }

    /// Close the current panel. Draws the panel background covering all items.
    pub fn end_panel(&mut self) {
        let Some(panel) = self.panels.pop() else {
            return;
        };
        let h = panel.bottom - panel.y;
        let bg_rect: Rect = [panel.x, panel.y, panel.w, h];
        // Insert the background quads at the front (drawn first = behind widgets).
        // Push background to verts — we use a simple approach: re-insert at start
        // by collecting the existing verts, prepending the bg, and reassembling.
        let bg_verts = quad_verts(bg_rect, self.theme.panel_bg);
        // Drain current verts, prepend bg
        let existing: Vec<UiVert> = self.verts.drain(..).collect();
        self.verts.extend_from_slice(&bg_verts);
        self.verts.extend_from_slice(&existing);
    }

    // ── Widgets ───────────────────────────────────────────────────────────────

    /// Non-interactive text label.
    pub fn label(&mut self, text: &str) {
        let pad = self.theme.panel_padding;
        let rect = self.layout_rect();
        self.advance();
        self.push_text_left(text, rect, pad, self.theme.text_color);
    }

    /// Dimmed secondary label.
    pub fn label_dim(&mut self, text: &str) {
        let pad = self.theme.panel_padding;
        let rect = self.layout_rect();
        self.advance();
        self.push_text_left(text, rect, pad, self.theme.text_dim);
    }

    /// Clickable button. Returns `true` on the frame it is released.
    pub fn button(&mut self, label: &str) -> bool {
        let id = widget_id(label);
        let rect = self.layout_rect();
        self.advance();

        let hovered = rect_contains(rect, self.mouse_pos);
        if hovered {
            self.hot = id;
        }
        if self.active == id && self.mouse_released {
            self.active = 0;
            if hovered {
                self.push_quad(rect, self.theme.item_active);
                self.push_text_center(label, rect, self.theme.text_color);
                return true;
            }
        }
        if hovered && self.mouse_pressed {
            self.active = id;
        }

        let color = if self.active == id && hovered {
            self.theme.item_active
        } else if self.hot == id {
            self.theme.item_hover
        } else {
            self.theme.item_bg
        };
        self.push_quad(rect, color);
        self.push_text_center(label, rect, self.theme.text_color);
        false
    }

    /// Toggle checkbox. Returns `true` when the value changes.
    pub fn checkbox(&mut self, label: &str, value: &mut bool) -> bool {
        let id = widget_id(label);
        let rect = self.layout_rect();
        self.advance();

        let hovered = rect_contains(rect, self.mouse_pos);
        if hovered {
            self.hot = id;
        }

        let mut changed = false;
        if hovered && self.mouse_pressed {
            *value = !*value;
            changed = true;
        }

        let bg = if hovered {
            self.theme.item_hover
        } else {
            self.theme.item_bg
        };
        self.push_quad(rect, bg);

        let box_size = rect[3] - 6.0;
        let bx = rect[0] + 3.0;
        let by = rect[1] + 3.0;
        let box_rect: Rect = [bx, by, box_size, box_size];
        self.push_quad(box_rect, self.theme.slider_track);
        if *value {
            let inset = 3.0;
            let tick_rect: Rect = [
                bx + inset,
                by + inset,
                box_size - inset * 2.0,
                box_size - inset * 2.0,
            ];
            self.push_quad(tick_rect, self.theme.accent);
        }

        let pad = self.theme.panel_padding;
        let text_rect: Rect = [rect[0] + box_size + 10.0, rect[1], rect[2], rect[3]];
        self.push_text_left(label, text_rect, pad, self.theme.text_color);
        changed
    }

    /// Horizontal drag slider. Returns `true` while the value is being changed.
    pub fn slider(&mut self, label: &str, value: &mut f32, min: f32, max: f32) -> bool {
        let id = widget_id(label);
        let rect = self.layout_rect();
        self.advance();

        let hovered = rect_contains(rect, self.mouse_pos);
        if hovered {
            self.hot = id;
        }

        let mut changed = false;
        if self.active == id {
            if self.mouse_down {
                let t = ((self.mouse_pos[0] - rect[0]) / rect[2]).clamp(0.0, 1.0);
                *value = min + t * (max - min);
                changed = true;
            } else {
                self.active = 0;
            }
        } else if hovered && self.mouse_pressed {
            self.active = id;
        }

        // Track
        self.push_quad(rect, self.theme.slider_track);

        // Fill
        let t = if max > min {
            (*value - min) / (max - min)
        } else {
            0.0
        };
        let fill_w = rect[2] * t.clamp(0.0, 1.0);
        if fill_w > 0.0 {
            let fill_color = if self.active == id {
                self.theme.item_active
            } else {
                self.theme.accent
            };
            self.push_quad([rect[0], rect[1], fill_w, rect[3]], fill_color);
        }

        let pad = self.theme.panel_padding;
        let text = format!("{}  {:.2}", label, value);
        self.push_text_left(&text, rect, pad, self.theme.text_color);
        changed
    }

    /// Thin horizontal separator line.
    pub fn separator(&mut self) {
        let pad = self.theme.panel_padding;
        let sep_h = 1.0;
        let panel = self.panels.last().expect("call begin_panel first");
        let rect: Rect = [
            panel.x + pad,
            panel.cursor_y + 4.0,
            panel.w - pad * 2.0,
            sep_h,
        ];
        let advance = sep_h + 8.0;
        if let Some(p) = self.panels.last_mut() {
            p.cursor_y += advance;
            p.bottom = p.cursor_y;
        }
        self.push_quad(rect, self.theme.text_dim);
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn layout_rect(&self) -> Rect {
        let panel = self.panels.last().expect("call begin_panel first");
        panel.next_rect(self.theme.item_height, self.theme.panel_padding)
    }

    fn advance(&mut self) {
        let step = self.theme.item_height + self.theme.item_padding;
        if let Some(p) = self.panels.last_mut() {
            p.cursor_y += step;
            p.bottom = p.cursor_y;
        }
    }

    fn push_quad(&mut self, rect: Rect, color: [f32; 4]) {
        if self.verts.len() + 6 <= MAX_VERTS {
            self.verts.extend_from_slice(&quad_verts(rect, color));
        }
    }

    fn push_text_left(&mut self, text: &str, rect: Rect, pad: f32, color: [f32; 4]) {
        let y = rect[1] + (rect[3] - self.theme.font_size) * 0.5;
        self.texts.push(TextCmd {
            text: text.to_owned(),
            x: rect[0] + pad,
            y,
            size: self.theme.font_size,
            color,
        });
    }

    fn push_text_center(&mut self, text: &str, rect: Rect, color: [f32; 4]) {
        // Approximate center: cosmic-text measures per-glyph, so we use a rough estimate.
        // A more accurate version would measure the text width first.
        let approx_w = text.len() as f32 * self.theme.font_size * 0.55;
        let x = rect[0] + (rect[2] - approx_w) * 0.5;
        let y = rect[1] + (rect[3] - self.theme.font_size) * 0.5;
        self.texts.push(TextCmd {
            text: text.to_owned(),
            x,
            y,
            size: self.theme.font_size,
            color,
        });
    }
}

// ── Quad geometry ─────────────────────────────────────────────────────────────

fn quad_verts(r: Rect, c: [f32; 4]) -> [UiVert; 6] {
    let (x0, y0, x1, y1) = (r[0], r[1], r[0] + r[2], r[1] + r[3]);
    let v = |px, py| UiVert {
        pos: [px, py],
        color: c,
    };
    [
        v(x0, y0),
        v(x1, y0),
        v(x1, y1),
        v(x0, y0),
        v(x1, y1),
        v(x0, y1),
    ]
}

fn darken(c: [f32; 4], factor: f32) -> [f32; 4] {
    [c[0] * factor, c[1] * factor, c[2] * factor, c[3]]
}
