pub const ATLAS_SIZE: u32 = 1024;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct TextVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub(super) struct CachedGlyph {
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub width: u32,
    pub height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
}

pub(super) struct RowPacker {
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

impl RowPacker {
    pub fn new() -> Self {
        Self {
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
        }
    }

    pub fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.cursor_x + w > ATLAS_SIZE {
            self.cursor_y += self.row_height + 1;
            self.cursor_x = 0;
            self.row_height = 0;
        }
        if self.cursor_y + h > ATLAS_SIZE {
            return None;
        }
        let pos = (self.cursor_x, self.cursor_y);
        self.cursor_x += w + 1;
        if h > self.row_height {
            self.row_height = h;
        }
        Some(pos)
    }
}

pub(super) struct TextEntry {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: [f32; 4],
}
