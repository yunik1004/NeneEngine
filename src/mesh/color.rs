use bytemuck::{Pod, Zeroable};

/// Colored vertex. 2-D shapes set `pos.z = 0.0`; 3-D shapes use any z.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ColorVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}
