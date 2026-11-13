//! 1:1 port of ModMenuSettingsApi: ModMenu config screen factory that hands
//! the engine's settings entry the ConfigScreen constructor.

use super::config_screen::ConfigScreen;

pub fn get_mod_config_screen_factory() -> fn(crate::seams::ScreenHandle) -> ConfigScreen {
    ConfigScreen::new
}
