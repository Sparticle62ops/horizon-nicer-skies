//! 1:1 port of SkyboxPainter (abstract).

use crate::seams;

pub trait SkyboxPainter {
    /// java: package-private abstract getTexelColour.
    fn get_texel_colour(&self, x: f32, y: f32, z: f32) -> i32;

    /// java: public projectOnSphere.
    fn project_on_sphere(&self, x: f32, y: f32, z: f32) -> [f32; 3] {
        let inv_distance = seams::mth_inv_sqrt(x * x + y * y + z * z);

        // divide by distance to get projection on sphere (shorten the vector)
        [x * inv_distance, y * inv_distance, z * inv_distance]
    }
}
