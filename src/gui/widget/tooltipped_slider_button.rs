//! 1:1 port of TooltippedSliderButton.

use crate::seams;

pub struct TooltippedSliderButton {
    pub button: seams::AbstractSliderButton,
    slider_moved: seams::SliderMovedFn,
    slider_released: seams::SliderReleasedFn,
}

impl TooltippedSliderButton {
    pub fn new(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        text: seams::Component,
        value: f64,
        slider_moved: seams::SliderMovedFn,
        slider_released: seams::SliderReleasedFn,
        tooltip: Option<seams::Tooltip>,
    ) -> Self {
        let button = seams::AbstractSliderButton::new(x, y, width, height, text, value);

        TooltippedSliderButton {
            button,
            slider_moved,
            slider_released,
        }
        .with_tooltip(tooltip)
    }

    fn with_tooltip(self, tooltip: Option<seams::Tooltip>) -> Self {
        self.button.set_tooltip(tooltip);
        self
    }

    /// java: protected updateMessage().
    pub fn update_message(&mut self) {
        let value = self.button.value;
        let message = (self.slider_moved.0)(value);
        self.button.set_message(message);
    }

    /// java: protected applyValue().
    pub fn apply_value(&mut self) {
        let value = self.button.value;
        (self.slider_released.0)(value);
    }
}
