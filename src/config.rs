use std::{
    fs,
    path::PathBuf,
};

use anyhow::{
    Context,
    Result,
};
use directories::ProjectDirs;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub mods_dir: Option<PathBuf>,
    pub update_mirror: Option<String>,
    pub gamebanana_mirror: Option<String>,
}

impl Config {
    fn get_config_path() -> Result<PathBuf> {
        ProjectDirs::from("com.github", "EnderHane", "evemoddl")
            .map(|proj_dirs| proj_dirs.config_dir().join("config.toml"))
            .context("Could not determine configuration directory")
    }

    pub fn load() -> Self {
        let path = match Self::get_config_path() {
            Ok(p) => p,
            Err(_) => return Config::default(),
        };

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Config::default(),
        };

        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let content = toml::to_string(self).context("Failed to serialize config to TOML")?;
        fs::write(path, content).context("Failed to write config file")?;
        Ok(())
    }
}
