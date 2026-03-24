use nene::{renderer::HeadlessContext, vertex};

fn write_temp_png(name: &str, width: u32, height: u32, pixel: [u8; 4]) -> std::path::PathBuf {
    let img = image::RgbaImage::from_pixel(width, height, image::Rgba(pixel));
    let path = std::env::temp_dir().join(name);
    img.save(&path).unwrap();
    path
}

#[vertex]
struct TestVertex {
    position: [f32; 2],
    color: [f32; 3],
}

fn make_ctx() -> Option<HeadlessContext> {
    HeadlessContext::new()
}

#[test]
fn headless_context_creation() {
    let ctx = HeadlessContext::new();
    assert!(ctx.is_some(), "Failed to create headless GPU context");
}

#[test]
fn headless_context_submit_empty() {
    let Some(ctx) = make_ctx() else { return };
    ctx.submit_empty();
}

#[test]
fn vertex_buffer_creation() {
    let Some(ctx) = make_ctx() else { return };

    let vertices = &[
        TestVertex {
            position: [0.0, 0.5],
            color: [1.0, 0.0, 0.0],
        },
        TestVertex {
            position: [-0.5, -0.5],
            color: [0.0, 1.0, 0.0],
        },
        TestVertex {
            position: [0.5, -0.5],
            color: [0.0, 0.0, 1.0],
        },
    ];

    let buffer = ctx.create_vertex_buffer(vertices);
    assert_eq!(
        buffer.size(),
        (std::mem::size_of::<TestVertex>() * 3) as u64
    );
}

#[test]
fn vertex_buffer_single() {
    let Some(ctx) = make_ctx() else { return };

    let vertices = &[TestVertex {
        position: [0.0, 0.0],
        color: [1.0, 1.0, 1.0],
    }];
    let buffer = ctx.create_vertex_buffer(vertices);
    assert_eq!(buffer.size(), std::mem::size_of::<TestVertex>() as u64);
}

#[test]
fn multiple_vertex_buffers() {
    let Some(ctx) = make_ctx() else { return };

    let v = TestVertex {
        position: [0.0, 0.0],
        color: [0.0, 0.0, 0.0],
    };
    let buf_a = ctx.create_vertex_buffer(&[v]);
    let buf_b = ctx.create_vertex_buffer(&[v, v]);

    assert_eq!(buf_a.size(), std::mem::size_of::<TestVertex>() as u64);
    assert_eq!(buf_b.size(), (std::mem::size_of::<TestVertex>() * 2) as u64);
}

#[test]
fn submit_empty_multiple_times() {
    let Some(ctx) = make_ctx() else { return };
    ctx.submit_empty();
    ctx.submit_empty();
    ctx.submit_empty();
}

#[test]
fn texture_load_1x1() {
    let Some(ctx) = make_ctx() else { return };
    let path = write_temp_png("nene_test_1x1.png", 1, 1, [255, 0, 0, 255]);
    let _ = ctx.load_texture(&path);
}

#[test]
fn texture_load_rgba() {
    let Some(ctx) = make_ctx() else { return };
    let path = write_temp_png("nene_test_rgba.png", 64, 64, [128, 64, 32, 255]);
    let _ = ctx.load_texture(&path);
}

#[test]
fn texture_load_transparent() {
    let Some(ctx) = make_ctx() else { return };
    let path = write_temp_png("nene_test_transparent.png", 32, 32, [0, 0, 0, 0]);
    let _ = ctx.load_texture(&path);
}

#[test]
fn texture_load_multiple() {
    let Some(ctx) = make_ctx() else { return };
    let path_a = write_temp_png("nene_test_tex_a.png", 16, 16, [255, 0, 0, 255]);
    let path_b = write_temp_png("nene_test_tex_b.png", 32, 32, [0, 255, 0, 255]);
    let path_c = write_temp_png("nene_test_tex_c.png", 64, 64, [0, 0, 255, 255]);
    let _ = ctx.load_texture(&path_a);
    let _ = ctx.load_texture(&path_b);
    let _ = ctx.load_texture(&path_c);
}

#[test]
fn texture_load_non_square() {
    let Some(ctx) = make_ctx() else { return };
    let path = write_temp_png("nene_test_nonsquare.png", 128, 64, [255, 255, 255, 255]);
    let _ = ctx.load_texture(&path);
}
