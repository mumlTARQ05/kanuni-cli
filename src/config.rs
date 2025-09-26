use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_endpoint: String,
    pub api_key: Option<String>,
    pub default_format: String,
    pub color_output: bool,
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_endpoint: "https://api.vlawyer.com/v1".to_string(),
            api_key: None,
            default_format: "text".to_string(),
            color_output: true,
            verbose: false,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        let config_dir = config_path.parent().unwrap();

        if !config_dir.exists() {
            fs::create_dir_all(config_dir)?;
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(config_path, contents)?;

        Ok(())
    }

    pub fn reset() -> Result<()> {
        let config = Config::default();
        config.save()?;
        Ok(())
    }

    pub fn get_config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "vlawyer", "kanuni")
            .ok_or_else(|| anyhow::anyhow!("Unable to determine config directory"))?;

        Ok(proj_dirs.config_dir().join("config.toml"))
    }
}