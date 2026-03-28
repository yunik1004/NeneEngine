use nene::{
    renderer::HeadlessContext,
    renderer::{FilterMode, Texture},
};

fn make_ctx() -> Option<HeadlessContext> {
    HeadlessContext::new()
}

fn solid_rgba(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    pixel.repeat((width * height) as usize)
}

#[test]
fn filter_mode_default_is_linear() {
    assert!(matches!(FilterMode::default(), FilterMode::Linear));
}

#[test]
fn filter_mode_into_wgpu_linear() {
    let f: wgpu::FilterMode = FilterMode::Linear.into();
    assert_eq!(f, wgpu::FilterMode::Linear);
}

#[test]
fn filter_mode_into_wgpu_nearest() {
    let f: wgpu::FilterMode = FilterMode::Nearest.into();
    assert_eq!(f, wgpu::FilterMode::Nearest);
}

#[test]
fn texture_create_1x1() {
    let Some(ctx) = make_ctx() else { return };
    let _: Texture = ctx.create_texture(1, 1, &[255, 0, 0, 255]);
}

#[test]
fn texture_create_with_nearest() {
    let Some(ctx) = make_ctx() else { return };
    let data = solid_rgba(8, 8, [0, 255, 0, 255]);
    let _: Texture = ctx.create_texture_with(8, 8, &data, FilterMode::Nearest);
}

#[test]
fn texture_create_with_linear() {
    let Some(ctx) = make_ctx() else { return };
    let data = solid_rgba(8, 8, [0, 0, 255, 255]);
    let _: Texture = ctx.create_texture_with(8, 8, &data, FilterMode::Linear);
}

#[test]
fn texture_create_non_square() {
    let Some(ctx) = make_ctx() else { return };
    let data = solid_rgba(128, 32, [255, 128, 0, 255]);
    let _: Texture = ctx.create_texture(128, 32, &data);
}

#[test]
fn texture_create_multiple() {
    let Some(ctx) = make_ctx() else { return };
    let _: Texture = ctx.create_texture(16, 16, &solid_rgba(16, 16, [255, 0, 0, 255]));
    let _: Texture = ctx.create_texture(32, 32, &solid_rgba(32, 32, [0, 255, 0, 255]));
    let _: Texture = ctx.create_texture(64, 64, &solid_rgba(64, 64, [0, 0, 255, 255]));
}

#[test]
fn texture_load_from_memory() {
    let Some(ctx) = make_ctx() else { return };
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255u8, 128, 0, 255]));
    let mut bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .unwrap();
    let _ = ctx
        .load_texture_from_memory(&bytes)
        .expect("failed to load texture from memory");
}

#[test]
fn create_render_target() {
    let Some(ctx) = make_ctx() else { return };
    let _target = ctx.create_render_target(256, 128);
}
