//! 1:1 port of NicerSkiesRenderer.

use crate::config::Config;
use crate::core::gradient::Gradient;
use crate::core::hashed_seed_manager;
use crate::sky::nebula::nebula_skybox_painter::NebulaSkyboxPainter;
use crate::sky::nebula::skybox::Skybox;
use crate::sky::nebula::skybox_painter::SkyboxPainter;
use crate::sky::star::starbox::{Starbox, StarboxRef};
use crate::NicerSkies;
use crate::seams;

pub struct NicerSkiesRenderer {
    pub starbox: StarboxRef,
    pub skybox: Option<Skybox>,

    pub ticks: i32,

    pub star_gradient: Gradient,
    pub nebula_gradient: Gradient,
}

impl NicerSkiesRenderer {
    /// java: instance initializer blocks on starGradient/nebulaGradient.
    pub fn new() -> Self {
        let mut star_gradient = Gradient::new();
        star_gradient.add(0.0, 255, 179, 97);
        star_gradient.add(0.2, 255, 249, 253);
        star_gradient.add(1.0, 175, 199, 255);

        let mut nebula_gradient = Gradient::new();
        nebula_gradient.add(0.2, 41, 83, 146);
        nebula_gradient.add(0.5, 120, 47, 93);
        nebula_gradient.add(0.7, 209, 58, 103);
        nebula_gradient.add(0.8, 255, 160, 123);
        nebula_gradient.add(1.0, 253, 194, 220);

        NicerSkiesRenderer {
            starbox: StarboxRef::None,
            skybox: None,
            ticks: 42,
            star_gradient,
            nebula_gradient,
        }
    }

    /// java: Lombok @Getter starbox -> StarboxRef; getStarbox() reads
    /// StarboxRef::Some. java: hasBuiltBuffers().
    pub fn has_built_buffers(&self) -> bool {
        self.starbox.is_built() && self.skybox.is_some()
    }

    /// java: getStarbox().
    pub fn get_starbox(&self) -> Option<&Starbox> {
        self.starbox.as_star()
    }

    /// java: getSkybox().
    pub fn get_skybox(&self) -> Option<&Skybox> {
        self.skybox.as_ref()
    }

    pub fn build_buffers(&mut self) {
        self.starbox = StarboxRef::Some(Box::new(Starbox::new()));
        self.skybox = Some(Skybox::new());
    }

    pub fn generate_sky(&mut self, seed: i64) {
        let mut random_source = seams::RandomSource::create(seed);

        let cm: Config = match (crate::get_instance().lock().unwrap()).as_ref() {
            Some(inst) => inst.config.clone_config(),
            None => return,
        };

        if cm.are_nebulas_enabled() {
            let perlin_noise = seams::PerlinNoise::create(random_source.fork(), [1, 2, 3, 4, 5, 6, 7]);
            let noise_scale = cm.get_nebula_noise_scale();
            let noise_amount = cm.get_nebula_noise_amount();
            let bg_strength = cm.get_nebula_base_colour_amount();

            let painter = NebulaSkyboxPainter::new(
                perlin_noise,
                &self.nebula_gradient,
                noise_scale,
                noise_amount,
                bg_strength,
            );
            if let Some(skybox) = self.skybox.as_mut() {
                skybox.paint(&painter);
            }
        }

        if cm.are_twinlking_stars_enabled() {
            if let Some(starbox) = self.starbox.as_mut() {
                starbox.generate_stars(&mut random_source, &self.star_gradient);
            }
        }
    }

    pub fn tick(&mut self) {
        self.ticks += 1;
        if let Some(starbox) = self.starbox.as_mut() {
            starbox.update_stars(self.ticks);
        }
    }

    /// java: private generateSky helper the ConfigScreen calls after apply.
    pub fn regenerate(&mut self, seed: i64) {
        self.generate_sky(seed);
    }

    pub fn generate_sky_from_manager(&mut self) {
        if hashed_seed_manager::can_generate_sky() {
            self.generate_sky(hashed_seed_manager::get_seed());
        }
    }
}
