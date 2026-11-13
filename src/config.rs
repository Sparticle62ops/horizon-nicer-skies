//! 1:1 port of Config (Gson -> serde_json). ConfigData/NebulaConfigData keep
//! the exact java field names and defaults; Lombok getters map to methods.

use crate::NicerSkies;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NebulaConfigData {
    #[serde(rename = "nebulaStrength")]
    pub nebula_strength: f32,
    #[serde(rename = "nebulaNoiseAmount")]
    pub nebula_noise_amount: f32,
    #[serde(rename = "nebulaNoiseScale")]
    pub nebula_noise_scale: f32,
    #[serde(rename = "baseColourAmount")]
    pub base_colour_amount: i32,
    #[serde(rename = "renderDuringDay")]
    pub render_during_day: bool,
}

impl NebulaConfigData {
    pub fn new(nebula_strength: f32, nebula_noise_amount: f32, nebula_noise_scale: f32, base_colour_amount: i32, render_during_day: bool) -> Self {
        NebulaConfigData { nebula_strength, nebula_noise_amount, nebula_noise_scale, base_colour_amount, render_during_day }
    }
}

impl Default for NebulaConfigData {
    fn default() -> Self {
        NebulaConfigData::new(1.0, 0.5, 1.0, 128, false)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigData {
    #[serde(rename = "twinklingStars")]
    pub twinkling_stars: bool,
    #[serde(rename = "renderNebulas")]
    pub render_nebulas: bool,

    #[serde(rename = "nebulaConfig")]
    pub nebula_config: NebulaConfigData,
}

impl ConfigData {
    /// java: package-private constructor (twinklingStars, nebulas,
    /// nebulaStrength, nebulaNoiseAmount, nebulaNoiseScale, baseColourAmount,
    /// renderDuringDay).
    pub fn new(twinkling_stars: bool, nebulas: bool, nebula_strength: f32, nebula_noise_amount: f32, nebula_noise_scale: f32, base_colour_amount: i32, render_during_day: bool) -> Self {
        ConfigData {
            twinkling_stars,
            render_nebulas: nebulas,
            nebula_config: NebulaConfigData::new(nebula_strength, nebula_noise_amount, nebula_noise_scale, base_colour_amount, render_during_day),
        }
    }
}

impl Default for ConfigData {
    fn default() -> Self {
        ConfigData::new(true, true, 1.0, 0.5, 1.0, 128, false)
    }
}

pub struct Config {
    pub(crate) config_data: ConfigData,
    file: Option<std::path::PathBuf>,
}

impl Config {
    /// java: DEFAULT_CONFIG.
    pub const DEFAULT_CONFIG: ConfigData = ConfigData {
        twinkling_stars: true,
        render_nebulas: true,
        nebula_config: NebulaConfigData {
            nebula_strength: 1.0,
            nebula_noise_amount: 0.5,
            nebula_noise_scale: 1.0,
            base_colour_amount: 128,
            render_during_day: false,
        },
    };

    /// java: static Config fromFile(File file).
    pub fn from_file(file: &std::path::Path) -> Config {
        let config_data = match std::fs::read_to_string(file) {
            Ok(text) => match serde_json::from_str::<ConfigData>(&text) {
                Ok(c) => c,
                Err(_) => {
                    crate::logger().warn("Failed to read config file, falling back to default config.");
                    ConfigData::default()
                }
            },
            Err(_) => {
                // java: !file.exists() -> mkdirs, createNewFile, write DEFAULT_CONFIG.
                if let Some(parent) = file.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::File::create(file);
                ConfigData::default()
            }
        };

        let config = Config { config_data, file: Some(file.to_path_buf()) };
        config.save_file();
        config
    }

    /// java: private constructor also calls save(file).
    pub fn from_config_data(config_data: ConfigData, file: Option<std::path::PathBuf>) -> Config {
        let config = Config { config_data, file };
        config.save_file();
        config
    }

    pub fn get_config_data(&self) -> &ConfigData {
        &self.config_data
    }

    // java: Lombok-style accessors on Config.
    pub fn are_twinlking_stars_enabled(&self) -> bool {
        self.config_data.twinkling_stars
    }

    pub fn are_nebulas_enabled(&self) -> bool {
        self.config_data.render_nebulas
    }

    pub fn get_nebula_strength(&self) -> f32 {
        self.config_data.nebula_config.nebula_strength
    }

    pub fn get_nebula_noise_amount(&self) -> f32 {
        self.config_data.nebula_config.nebula_noise_amount
    }

    pub fn get_nebula_noise_scale(&self) -> f32 {
        self.config_data.nebula_config.nebula_noise_scale
    }

    pub fn get_nebula_base_colour_amount(&self) -> i32 {
        self.config_data.nebula_config.base_colour_amount
    }

    pub fn get_render_during_day(&self) -> bool {
        self.config_data.nebula_config.render_during_day
    }

    pub fn update_config(&mut self, config_data: ConfigData) {
        self.config_data = config_data.clone();
        self.save_file();
    }

    pub fn save_file(&self) {
        if let Some(file) = &self.file {
            let json = serde_json::to_string(&self.config_data).unwrap_or_else(|_| "null".to_string());
            if let Err(_e) = std::fs::write(file, json) {
                crate::logger().warn("Failed to save config file!");
            }
        }
    }
}

impl Config {
    /// java: Config is shared by reference; the renderer/screen clone the
    /// current ConfigData into their own Config. Mirrors the effective
    /// behavior of holding the same ConfigData.
    pub fn clone_config(&self) -> Config {
        Config { config_data: self.config_data.clone(), file: None }
    }

    /// java: getConfigData().toBuilder().build() deep-copy used by the screen.
    pub fn to_builder_build(&self) -> ConfigData {
        self.config_data.clone()
    }
}
