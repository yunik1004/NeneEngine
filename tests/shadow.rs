use nene::shadow::{SHADOW_WGSL, ShadowMap};

#[test]
fn shadow_wgsl_contains_shadow_factor() {
    assert!(SHADOW_WGSL.contains("fn shadow_factor"));
}

#[test]
fn shadow_wgsl_contains_pcf() {
    // PCF uses a loop over a 3x3 kernel
    assert!(SHADOW_WGSL.contains("textureSampleCompare"));
}

#[test]
fn shadow_wgsl_nonempty() {
    assert!(!SHADOW_WGSL.is_empty());
}

#[test]
fn shadow_map_size() {
    use nene::renderer::HeadlessContext;
    let Some(ctx) = HeadlessContext::new() else {
        return;
    };
    let map: ShadowMap = ctx.create_shadow_map(512);
    assert_eq!(map.size, 512);
}

#[test]
fn shadow_map_size_1024() {
    use nene::renderer::HeadlessContext;
    let Some(ctx) = HeadlessContext::new() else {
        return;
    };
    let map: ShadowMap = ctx.create_shadow_map(1024);
    assert_eq!(map.size, 1024);
}
