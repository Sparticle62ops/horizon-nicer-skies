//! 1:1 Horizon Rust port of Nicer Skies (ZtereoHYPE, LGPL-3.0-only).
//! Upstream: https://github.com/ZtereoHYPE/nicer-skies v1.5.0+26.2.
//! Every Java class maps to one module below, same names, same math,
//! same defaults, same control flow.

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
