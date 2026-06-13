use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConnectionMode {
    Cemu,
    WiiU,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NetworkMode {
    Pretendo,
    Spacebar,
}

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub connection_mode: ConnectionMode,
    pub network_mode: NetworkMode,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            connection_mode: ConnectionMode::Cemu,
            network_mode: NetworkMode::Pretendo,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "jerrysm64", "squidmod")
        .map(|dirs| dirs.config_dir().join("settings.json"))
}

pub fn load_config() -> AppConfig {
    config_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &AppConfig) {
    if let Some(path) = config_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(config) {
            let _ = fs::write(path, json);
        }
    }
}
