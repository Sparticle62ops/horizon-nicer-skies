//! 1:1 port of Separator (Renderable).

use crate::seams;

pub struct Separator {
    x: i32,
    y: i32,
    height: i32,
}

impl Separator {
    pub fn new(x: i32, y: i32, height: i32) -> Self {
        Separator { x, y, height }
    }

    /// java: extractRenderState(GuiGraphicsExtractor, mouseX, mouseY, a).
    pub fn extract_render_state(&self, graphics: &mut seams::GuiGraphicsExtractor, _mouse_x: i32, _mouse_y: i32, _a: f32) {
        graphics.fill(self.x, self.y, self.x + 1, self.y + self.height, 0xFFFFFFFFu32 as i32);
    }
}
