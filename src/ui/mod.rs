//! egui UI integration ([`EguiUi`]).
//!
//! # Quick start
//! ```no_run
//! use nene::{
//!     app::{App, Config, WindowId, WindowEvent, run},
//!     renderer::{Context, RenderPass},
//!     ui::EguiUi,
//! };
//!
//! struct MyApp { egui: Option<EguiUi> }
//!
//! impl App for MyApp {
//!     fn new() -> Self { MyApp { egui: None } }
//!
//!     fn window_ready(&mut self, _id: WindowId, ctx: &mut Context) {
//!         self.egui = Some(EguiUi::new(ctx));
//!     }
//!
//!     fn on_window_event(&mut self, _id: WindowId, event: &WindowEvent) {
//!         if let Some(e) = &mut self.egui { e.handle_event(event); }
//!     }
//!
//!     fn prepare(&mut self, _id: WindowId, ctx: &mut Context, _input: &nene::input::Input) {
//!         let Some(egui) = &mut self.egui else { return };
//!         let ui = egui.begin_frame();
//!         egui::Window::new("Hello").show(&ui, |ui| { ui.label("World"); });
//!         egui.end_frame(ctx);
//!     }
//!
//!     fn render(&mut self, _id: WindowId, pass: &mut RenderPass) {
//!         if let Some(e) = &self.egui { e.render(pass); }
//!     }
//! }
//! ```

pub mod egui_ui;
pub use egui_ui::EguiUi;
