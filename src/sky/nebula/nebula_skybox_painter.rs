//! 1:1 port of NebulaSkyboxPainter.

use super::skybox_painter::SkyboxPainter;
use crate::core::gradient::Gradient;
use crate::seams;

pub struct NebulaSkyboxPainter<'a> {
    noise: seams::PerlinNoise,
    nebula_gradient: &'a Gradient,
    scaling_factor: f32,
    /// the amount of base noise to keep
    noise_amount: f32,
    base_colour_strength: i32,
}

impl<'a> NebulaSkyboxPainter<'a> {
    pub fn new(noise: seams::PerlinNoise, nebula_gradient: &'a Gradient, scaling_factor: f32, noise_amount: f32, base_colour_strength: i32) -> Self {
        NebulaSkyboxPainter { noise, nebula_gradient, scaling_factor, noise_amount, base_colour_strength }
    }
}

impl SkyboxPainter for NebulaSkyboxPainter<'_> {
    fn get_texel_colour(&self, x: f32, y: f32, z: f32) -> i32 {
        // Get projection
        let proj_coords = self.project_on_sphere(x, y, z);
        let mut x = proj_coords[0];
        let mut y = proj_coords[1];
        let mut z = proj_coords[2];

        // Get offset
        let offset = self.noise.get_value(
            (x * self.scaling_factor * 3.0) as f64,
            (y * self.scaling_factor * 3.0) as f64,
            (z * self.scaling_factor * 3.0) as f64,
        ) as f32;
        x = seams::mth_clamp_f32(x + offset / 5.0, -1.0, 1.0);
        y = seams::mth_clamp_f32(y + offset / 5.0, -1.0, 1.0);
        z = seams::mth_clamp_f32(z + offset / 5.0, -1.0, 1.0);

        // Value of noise at coord, 0..1
        let noise_value = seams::mth_clamp_f64(
            self.noise.get_value(
                (x * self.scaling_factor) as f64,
                (y * self.scaling_factor) as f64,
                (z * self.scaling_factor) as f64,
            ) + 0.5,
            0.0,
            1.0,
        );

        // Get the derivatives of the first (largest) octave of noise to shift the colour around with.
        let mut ds = [0.0f64; 3];
        self.noise.get_octave_noise(0).noise_with_derivative(
            (x * self.scaling_factor) as f64,
            (y * self.scaling_factor) as f64,
            (z * self.scaling_factor) as f64,
            &mut ds,
        );

        // Find a base background colour to use (xyz interpoaltion across sky, gamer mode)
        let base_b = ((x / 2.0 + 0.5) * self.base_colour_strength as f32) as i32;
        let base_g = ((y / 2.0 + 0.5) * self.base_colour_strength as f32) as i32;
        let base_r = ((z / 2.0 + 0.5) * self.base_colour_strength as f32) as i32;

        // Turn off nebula rendering if noiseAmount is 0. (user expected behaviour)
        let nebula_factor: f64;
        if self.noise_amount != 0.0 {
            nebula_factor = seams::mth_clamp_f64(
                noise_value * (1.0 / self.noise_amount as f64) - (1.0 / self.noise_amount as f64 - 1.0),
                0.0,
                0.99,
            );
        } else {
            nebula_factor = 0.0;
        }

        // Get the colour of the nebula and the amount of background colour to show at the current factor.
        let nebula = self.nebula_gradient.get_at(nebula_factor);
        let bg_factor = seams::mth_clamp_f64((-nebula_factor + 1.0).log10() + 1.0, 0.0, 1.0);

        // Merge everything:
        // colour = nebulaFactor * [colour of nebula in that pixel] - [noise derivative in that pixel] * nebulaFactor + background colour * bgFactor
        let r = seams::mth_clamp_i32(
            (nebula_factor * nebula[0] as f64 - ds[0] * nebula_factor * 128.0 + base_r as f64 * bg_factor) as i32,
            0,
            255,
        );
        let g = seams::mth_clamp_i32(
            (nebula_factor * nebula[1] as f64 - ds[1] * nebula_factor * 64.0 + base_g as f64 * bg_factor) as i32,
            0,
            255,
        );
        let b = seams::mth_clamp_i32(
            (nebula_factor * nebula[2] as f64 - ds[2] * nebula_factor * 128.0 + base_b as f64 * bg_factor) as i32,
            0,
            255,
        );

        // Get the alpha depending on the background factor
        let alpha = seams::mth_clamp_i32(((1.0 - bg_factor) * 255.0) as i32, 50, 255);

        seams::argb_color(alpha, b, g, r)
    }
}
