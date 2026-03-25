use nene::{renderer::HeadlessContext, vertex};

fn make_ctx() -> Option<HeadlessContext> {
    HeadlessContext::new()
}

fn solid_rgba(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    pixel.repeat((width * height) as usize)
}

#[vertex]
struct TestVertex {
    position: [f32; 2],
    color: [f32; 3],
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
fn texture_create_1x1() {
    let Some(ctx) = make_ctx() else { return };
    let _ = ctx.create_texture(1, 1, &[255, 0, 0, 255]);
}

#[test]
fn texture_create_rgba() {
    let Some(ctx) = make_ctx() else { return };
    let data = solid_rgba(64, 64, [128, 64, 32, 255]);
    let _ = ctx.create_texture(64, 64, &data);
}

#[test]
fn texture_create_transparent() {
    let Some(ctx) = make_ctx() else { return };
    let data = solid_rgba(32, 32, [0, 0, 0, 0]);
    let _ = ctx.create_texture(32, 32, &data);
}

#[test]
fn texture_create_multiple() {
    let Some(ctx) = make_ctx() else { return };
    let _ = ctx.create_texture(16, 16, &solid_rgba(16, 16, [255, 0, 0, 255]));
    let _ = ctx.create_texture(32, 32, &solid_rgba(32, 32, [0, 255, 0, 255]));
    let _ = ctx.create_texture(64, 64, &solid_rgba(64, 64, [0, 0, 255, 255]));
}

#[test]
fn texture_create_non_square() {
    let Some(ctx) = make_ctx() else { return };
    let data = solid_rgba(128, 64, [255, 255, 255, 255]);
    let _ = ctx.create_texture(128, 64, &data);
}

#[test]
fn texture_load_from_memory() {
    let Some(ctx) = make_ctx() else { return };
    // Encode a small PNG in memory using image (library dep), then decode via nene
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255u8, 128, 0, 255]));
    let mut bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .unwrap();
    let _ = ctx.load_texture_from_memory(&bytes);
}
