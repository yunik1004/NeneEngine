use super::{Pipeline, PipelineDescriptor, RenderContext, RenderPass, RenderTarget, UniformBuffer};

// ── Settings ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum ToneMap {
    /// No tone mapping — colors may clip above 1.0.
    None,
    /// Simple Reinhard operator: `c / (c + 1)`.
    Reinhard,
    /// ACES filmic curve — good contrast and saturation.
    #[default]
    Aces,
}

/// Per-frame post-processing settings.
///
/// After changing any field, call [`PostProcessStack::apply_settings`] to upload
/// the new values to the GPU.
#[derive(Clone, Copy, Debug)]
pub struct PostProcessSettings {
    pub tone_map: ToneMap,
    /// Linear exposure multiplier applied before tone mapping. Default: `1.0`.
    pub exposure: f32,
    /// Vignette strength. `0.0` = none, `1.0` = heavy dark edges. Default: `0.0`.
    pub vignette: f32,
    /// Gamma exponent for final gamma-correction (`1.0` skips it). Default: `2.2`.
    pub gamma: f32,
    /// `1.0` = unchanged, `0.0` = fully desaturated, `>1.0` = boosted. Default: `1.0`.
    pub saturation: f32,
    /// `1.0` = unchanged, `<1.0` = flat, `>1.0` = punchy. Default: `1.0`.
    pub contrast: f32,
}

impl Default for PostProcessSettings {
    fn default() -> Self {
        Self {
            tone_map: ToneMap::Aces,
            exposure: 1.0,
            vignette: 0.0,
            gamma: 2.2,
            saturation: 1.0,
            contrast: 1.0,
        }
    }
}

// ── Internal GPU uniform ───────────────────────────────────────────────────────

#[derive(encase::ShaderType)]
struct PostUniform {
    exposure: f32,
    vignette: f32,
    tone_map: u32, // 0=none, 1=reinhard, 2=aces
    gamma: f32,
    saturation: f32,
    contrast: f32,
}

impl From<&PostProcessSettings> for PostUniform {
    fn from(s: &PostProcessSettings) -> Self {
        Self {
            exposure: s.exposure,
            vignette: s.vignette,
            tone_map: match s.tone_map {
                ToneMap::None => 0,
                ToneMap::Reinhard => 1,
                ToneMap::Aces => 2,
            },
            gamma: s.gamma,
            saturation: s.saturation,
            contrast: s.contrast,
        }
    }
}

// ── Shader ────────────────────────────────────────────────────────────────────

const SHADER: &str = r#"
struct PostSettings {
    exposure:   f32,
    vignette:   f32,
    tone_map:   u32,
    gamma:      f32,
    saturation: f32,
    contrast:   f32,
};
@group(0) @binding(0) var<uniform> u: PostSettings;
@group(1) @binding(0) var t_scene: texture_2d<f32>;
@group(1) @binding(1) var s_scene: sampler;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Fullscreen triangle covering NDC [-1,1]x[-1,1] (no vertex buffer needed).
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
    );
    return VSOut(vec4<f32>(pos[vi], 0.0, 1.0), uv[vi]);
}

fn aces(x: vec3<f32>) -> vec3<f32> {
    return clamp(x * (2.51 * x + 0.03) / (x * (2.43 * x + 0.59) + 0.14),
                 vec3<f32>(0.0), vec3<f32>(1.0));
}

fn reinhard(x: vec3<f32>) -> vec3<f32> {
    return x / (x + vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    var c = textureSample(t_scene, s_scene, in.uv).rgb;

    // Exposure
    c *= u.exposure;

    // Tone mapping
    switch u.tone_map {
        case 1u: { c = reinhard(c); }
        case 2u: { c = aces(c); }
        default: {}
    }

    // Gamma correction
    c = pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / u.gamma));

    // Saturation
    let lum = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    c = mix(vec3<f32>(lum), c, u.saturation);

    // Contrast
    c = clamp((c - 0.5) * u.contrast + 0.5, vec3<f32>(0.0), vec3<f32>(1.0));

    // Vignette
    let uv = in.uv - vec2<f32>(0.5);
    let vig = 1.0 - dot(uv, uv) * u.vignette * 4.0;
    c *= max(vig, 0.0);

    return vec4<f32>(c, 1.0);
}
"#;

// ── PostProcessStack ──────────────────────────────────────────────────────────

/// Off-screen post-processing stack.
///
/// # Usage
/// ```ignore
/// // init
/// let pp = PostProcessStack::new(&mut ctx, width, height);
///
/// // pre_render: render scene into the internal target
/// pp.scene_pass(&mut ctx, |pass| {
///     pass.set_pipeline(&my_pipeline);
///     // ... draw scene ...
/// });
///
/// // render: apply effects to the swapchain pass
/// pp.apply(pass);
/// ```
pub struct PostProcessStack {
    pub settings: PostProcessSettings,
    scene: RenderTarget,
    pipeline: Pipeline,
    ubuf: UniformBuffer,
}

impl PostProcessStack {
    /// Create a new stack with default settings.
    pub fn new(ctx: &mut impl RenderContext, width: u32, height: u32) -> Self {
        Self::with_settings(ctx, width, height, PostProcessSettings::default())
    }

    /// Create a new stack with custom settings.
    pub fn with_settings(
        ctx: &mut impl RenderContext,
        width: u32,
        height: u32,
        settings: PostProcessSettings,
    ) -> Self {
        let scene = ctx.create_scene_target(width, height);
        let pipeline = ctx.create_pipeline(
            PipelineDescriptor::fullscreen_pass(SHADER)
                .with_uniform()
                .with_texture(),
        );
        let ubuf = ctx.create_uniform_buffer(&PostUniform::from(&settings));
        Self {
            settings,
            scene,
            pipeline,
            ubuf,
        }
    }

    /// Render the scene into the internal off-screen target.
    ///
    /// Call this from `pre_render` or `update` before the main `render` callback.
    pub fn scene_pass<F: FnOnce(&mut RenderPass<'_>)>(
        &self,
        ctx: &mut impl RenderContext,
        draw: F,
    ) {
        ctx.render_to_target(&self.scene, draw);
    }

    /// Upload [`settings`](Self::settings) to the GPU.
    ///
    /// Must be called after mutating `self.settings` for the change to take effect.
    pub fn apply_settings(&self, ctx: &mut impl RenderContext) {
        ctx.update_uniform_buffer(&self.ubuf, &PostUniform::from(&self.settings));
    }

    /// Apply all post-process effects to the current (swapchain) render pass.
    ///
    /// Call this as the first or only draw in the `render` callback.
    pub fn apply(&self, pass: &mut RenderPass) {
        pass.set_pipeline(&self.pipeline);
        pass.set_uniform(0, &self.ubuf);
        pass.set_texture(1, self.scene.texture());
        pass.draw(0..3);
    }

    /// Recreate render targets after a window resize.
    pub fn resize(&mut self, ctx: &mut impl RenderContext, width: u32, height: u32) {
        self.scene = ctx.create_scene_target(width, height);
    }
}
