//! 1:1 port of TooltippedCheckbox.

use crate::seams;

pub struct TooltippedCheckbox {
    pub checkbox: seams::Checkbox,
}

impl TooltippedCheckbox {
    pub fn new(i: i32, j: i32, component: seams::Component, font: &seams::Font, bl: bool, on_value_change: seams::CheckboxOnValueChange, tooltip: Option<seams::Tooltip>) -> Self {
        // java: super(i, j, 128, component, font, bl, onValueChange);
        let checkbox = seams::Checkbox::new(i, j, 128, component, font, bl, on_value_change);
        checkbox.set_tooltip(tooltip);
        TooltippedCheckbox { checkbox }
    }

    /// java: extractContents override: alpha 1.0 active, 0.5 inactive.
    pub fn extract_contents(&mut self, gui_graphics: &mut seams::GuiGraphicsExtractor, i: i32, j: i32, f: f32) {
        self.checkbox.set_alpha(if self.checkbox.is_active() { 1.0 } else { 0.5 });
        self.checkbox.extract_contents(gui_graphics, i, j, f);
    }
}
