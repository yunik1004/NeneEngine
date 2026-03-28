use nene::renderer::HeadlessContext;

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
fn submit_empty_multiple_times() {
    let Some(ctx) = make_ctx() else { return };
    ctx.submit_empty();
    ctx.submit_empty();
    ctx.submit_empty();
}
