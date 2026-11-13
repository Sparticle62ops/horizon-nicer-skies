//! 1:1 port of DebugSkyboxPainter.

use super::skybox_painter::SkyboxPainter;
use crate::seams;

pub struct DebugSkyboxPainter;

impl SkyboxPainter for DebugSkyboxPainter {
    fn get_texel_colour(&self, _x: f32, _y: f32, _z: f32) -> i32 {
        seams::argb_color(255, 0, 0, 255)
    }
}
