use nene::renderer::{
    postprocess::{PostProcessSettings, PostProcessStack, ToneMap},
    HeadlessContext,
};

fn make_ctx() -> Option<HeadlessContext> {
    HeadlessContext::new()
}

#[test]
fn tone_map_default_is_aces() {
    assert!(matches!(ToneMap::default(), ToneMap::Aces));
}

#[test]
fn settings_default_values() {
    let s = PostProcessSettings::default();
    assert!(matches!(s.tone_map, ToneMap::Aces));
    assert_eq!(s.exposure, 1.0);
    assert_eq!(s.vignette, 0.0);
    assert_eq!(s.gamma, 2.2);
    assert_eq!(s.saturation, 1.0);
    assert_eq!(s.contrast, 1.0);
}

#[test]
fn stack_new() {
    let Some(ctx) = make_ctx() else { return };
    let mut ctx = ctx;
    let _stack = PostProcessStack::new(&mut ctx, 320, 240);
}

#[test]
fn stack_with_settings() {
    let Some(ctx) = make_ctx() else { return };
    let mut ctx = ctx;
    let settings = PostProcessSettings {
        tone_map: ToneMap::Reinhard,
        exposure: 1.5,
        vignette: 0.3,
        gamma: 2.2,
        saturation: 0.8,
        contrast: 1.1,
    };
    let _stack = PostProcessStack::with_settings(&mut ctx, 320, 240, settings);
}

#[test]
fn stack_apply_settings() {
    let Some(ctx) = make_ctx() else { return };
    let mut ctx = ctx;
    let mut stack = PostProcessStack::new(&mut ctx, 320, 240);
    stack.settings.exposure = 2.0;
    stack.settings.tone_map = ToneMap::None;
    stack.apply_settings(&mut ctx);
}

#[test]
fn stack_resize() {
    let Some(ctx) = make_ctx() else { return };
    let mut ctx = ctx;
    let mut stack = PostProcessStack::new(&mut ctx, 320, 240);
    stack.resize(&mut ctx, 640, 480);
}
