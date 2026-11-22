//! 1:1 Horizon Rust port of Nicer Skies (ZtereoHYPE, LGPL-3.0-only).
//! Upstream: https://github.com/ZtereoHYPE/nicer-skies v1.5.0+26.2.
//! Every Java class maps to one module below, same names, same math,
//! same defaults, same control flow.
//!
//! The mod API wires the original's config as settings so Horizon's
//! Options > Mods page holds it, same defaults. The sky renderer itself
//! (star buffers, nebula skybox, twinkling) waits on the sky-render hook
//! surface: the pinned mod API has no hook into the vanilla sky pass
//! (buildStars / renderStars / renderSunMoonAndStars), so the ported
//! renderer's draw path cannot be attached. See horizon-rust/horizon
//! issue #19623 for the gap; the core compiles and its config math is
//! covered by tests.

use mod_api::{RegistrationContext, Registrar, SettingSpec};

pub mod seams;

pub mod client_level_accessor;
pub mod core;
pub mod config;

pub mod sky;
pub mod mixin;
pub mod gui;

use std::sync::{Mutex, OnceLock};

/// java: NicerSkies implements ClientModInitializer. Static instance, config
/// loaded from nicerskies.json, renderer constructed on init.
pub const MOD_ID: &str = "nicer-skies";

pub struct NicerSkies {
    pub config: config::Config,
    pub renderer: sky::nicer_skies_renderer::NicerSkiesRenderer,
}

static INSTANCE: OnceLock<Mutex<Option<NicerSkies>>> = OnceLock::new();
static LOGGER: OnceLock<crate::seams::Logger> = OnceLock::new();

pub fn logger() -> &'static crate::seams::Logger {
    LOGGER.get_or_init(crate::seams::Logger::new)
}

/// java: getInstance(). Horizon entry point: onInitializeClient runs once.
pub fn instance() -> Option<&'static Mutex<Option<NicerSkies>>> {
    Some(INSTANCE.get_or_init(|| Mutex::new(None)))
}

/// java: NicerSkies#onInitializeClient().
pub fn on_initialize_client() {
    let fabric_loader_config_dir = crate::seams::fabric_config_dir();
    let file = fabric_loader_config_dir.join("nicerskies.json");

    let config = config::Config::from_file(&file);
    let renderer = sky::nicer_skies_renderer::NicerSkiesRenderer::new();

    let instance = INSTANCE.get_or_init(|| Mutex::new(None));
    *instance.lock().unwrap() = Some(NicerSkies { config, renderer });
}

/// java: NicerSkies.getInstance().
pub fn get_instance() -> &'static Mutex<Option<NicerSkies>> {
    INSTANCE.get_or_init(|| Mutex::new(None))
}

/// The mod's id: its settings' namespace.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {
    /// The original's config fields registered as settings, same defaults.
    /// The config screen's own slider bounds carry over: both nebula
    /// sliders map 0..1 (shown as %), the background-strength slider maps
    /// 0..255, and the nebula-scale slider maps through
    /// gui/config_screen.rs's map_value_to_scale onto the stored value.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        let cfg = config::ConfigData::default();
        for (key, label, value) in [
            ("twinkling_stars", "setting.nicer_skies.twinkling_stars", cfg.twinkling_stars),
            ("render_nebulas", "setting.nicer_skies.render_nebulas", cfg.render_nebulas),
            ("render_during_day", "setting.nicer_skies.render_during_day", cfg.nebula_config.render_during_day),
        ] {
            let _ = registrar.register_setting(SettingSpec::toggle(key, label, value));
        }
        // Nebula Transparency: the screen's slider shows the raw value as %.
        let _ = registrar.register_setting(SettingSpec::range(
            "nebula_strength",
            "setting.nicer_skies.nebula_transparency",
            0.0,
            1.0,
            0.01,
            cfg.nebula_config.nebula_strength as f64,
        ));
        // Nebula Amount: same mapping.
        let _ = registrar.register_setting(SettingSpec::range(
            "nebula_noise_amount",
            "setting.nicer_skies.nebula_amount",
            0.0,
            1.0,
            0.01,
            cfg.nebula_config.nebula_noise_amount as f64,
        ));
        // Background Strength: the screen's slider divides by 255.
        let _ = registrar.register_setting(SettingSpec::range(
            "base_colour_amount",
            "setting.nicer_skies.background_strength",
            0.0,
            255.0,
            1.0,
            cfg.nebula_config.base_colour_amount as f64,
        ));
        // Nebula Scale: the screen's slider holds the mapped value
        // (map_value_to_scale: ((1/v) - 0.5) / 1.5, so the stored value
        // 2.0..0.667 sits at the slider's ends); the setting carries the
        // stored value over the same span, same default 1.0.
        let _ = registrar.register_setting(SettingSpec::range(
            "nebula_noise_scale",
            "setting.nicer_skies.nebula_scale",
            0.67,
            2.0,
            0.01,
            cfg.nebula_config.nebula_noise_scale as f64,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// java: ConfigData's defaults (twinklingStars, nebulas, nebulaStrength,
    /// nebulaNoiseAmount, nebulaNoiseScale, baseColourAmount, renderDuringDay).
    #[test]
    fn config_defaults_match_upstream() {
        let cfg = config::ConfigData::default();
        assert!(cfg.twinkling_stars);
        assert!(cfg.render_nebulas);
        assert_eq!(cfg.nebula_config.nebula_strength, 1.0);
        assert_eq!(cfg.nebula_config.nebula_noise_amount, 0.5);
        assert_eq!(cfg.nebula_config.nebula_noise_scale, 1.0);
        assert_eq!(cfg.nebula_config.base_colour_amount, 128);
        assert!(!cfg.nebula_config.render_during_day);
    }

    /// java: areTwinlkingStarsEnabled()/areNebulasEnabled() read the flags.
    #[test]
    fn config_toggles_read_flags() {
        let cfg = config::Config::from_config_data(config::ConfigData::default(), None);
        assert!(cfg.are_twinlking_stars_enabled());
        assert!(cfg.are_nebulas_enabled());
    }

    /// gui/config_screen.rs scale mapping round-trips: the slider's 0..1
    /// value maps to the stored scale and back.
    #[test]
    fn scale_mapping_round_trips() {
        let v = gui::config_screen::ConfigScreen::map_scale_to_value(0.5);
        assert_eq!(gui::config_screen::ConfigScreen::map_value_to_scale(v), 0.5);
    }
}
