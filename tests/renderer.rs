use nene::{renderer::HeadlessContext, vertex};

fn make_ctx() -> Option<HeadlessContext> {
    HeadlessContext::new()
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
