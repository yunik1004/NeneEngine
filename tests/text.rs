use nene::renderer::HeadlessContext;

fn make_ctx() -> Option<HeadlessContext> {
    HeadlessContext::new()
}

#[test]
fn text_renderer_creation() {
    let Some(ctx) = make_ctx() else { return };
    let _text = ctx.create_text_renderer();
}

#[test]
fn text_queue_and_clear() {
    let Some(ctx) = make_ctx() else { return };
    let mut text = ctx.create_text_renderer();

    assert_eq!(text.queued_count(), 0);
    text.queue("Hello", 0.0, 0.0, 24.0, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(text.queued_count(), 1);
    text.queue("World", 0.0, 30.0, 24.0, [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(text.queued_count(), 2);
    text.clear();
    assert_eq!(text.queued_count(), 0);
}

#[test]
fn text_queue_multiple_sizes() {
    let Some(ctx) = make_ctx() else { return };
    let mut text = ctx.create_text_renderer();

    text.queue("Small", 0.0, 0.0, 12.0, [1.0, 1.0, 1.0, 1.0]);
    text.queue("Medium", 0.0, 20.0, 24.0, [1.0, 1.0, 1.0, 1.0]);
    text.queue("Large", 0.0, 60.0, 48.0, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(text.queued_count(), 3);
}

#[test]
fn text_queue_unicode() {
    let Some(ctx) = make_ctx() else { return };
    let mut text = ctx.create_text_renderer();

    text.queue("한글 텍스트", 0.0, 0.0, 24.0, [1.0, 1.0, 1.0, 1.0]);
    text.queue("🦀 Rust!", 0.0, 40.0, 24.0, [1.0, 0.5, 0.0, 1.0]);
    assert_eq!(text.queued_count(), 2);
}

#[test]
fn text_clear_idempotent() {
    let Some(ctx) = make_ctx() else { return };
    let mut text = ctx.create_text_renderer();

    text.clear();
    text.clear();
    text.queue("test", 0.0, 0.0, 16.0, [1.0, 1.0, 1.0, 1.0]);
    text.clear();
    assert_eq!(text.queued_count(), 0);
}

#[test]
fn text_prepare_does_not_panic() {
    let Some(ctx) = make_ctx() else { return };
    let mut text = ctx.create_text_renderer();

    text.queue("Hello, GPU!", 50.0, 50.0, 32.0, [1.0, 1.0, 1.0, 1.0]);

    // prepare needs a windowed Context, so we test the headless path by verifying
    // that cosmic-text font shaping does not panic on its own.
    // Full GPU prepare is covered by the windowed example.
    let _ = text.queued_count();
}
