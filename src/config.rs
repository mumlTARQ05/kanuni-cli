use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_endpoint: String,
    pub api_key: Option<String>, // Legacy, will be removed
    pub user_email: Option<String>,
    pub default_format: String,
    pub color_output: bool,
    pub verbose: bool,
    #[serde(default)]
    pub websocket: WebSocketConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub url: Option<String>,  // Override default WS endpoint
    pub reconnect_max_attempts: u32,
    pub reconnect_delay_ms: u64,
    pub ping_interval_secs: u64,
    pub enable_progress: bool,  // Enable/disable progress streaming
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: None, // Will derive from api_endpoint
            reconnect_max_attempts: 5,
            reconnect_delay_ms: 1000,
            ping_interval_secs: 30,
            enable_progress: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_endpoint: "http://localhost:8080/api/v1".to_string(),
            api_key: None,
            user_email: None,
            default_format: "text".to_string(),
            color_output: true,
            verbose: false,
            websocket: WebSocketConfig::default(),
        }
    }
}

impl Config {
    pub fn get_websocket_url(&self) -> String {
        if let Some(url) = &self.websocket.url {
            return url.clone();
        }

        // Derive from API endpoint
        let base_url = self.api_endpoint
            .replace("http://", "ws://")
            .replace("https://", "wss://");

        // Remove /api/v1 suffix if present and add /ws
        if base_url.ends_with("/api/v1") {
            format!("{}/ws", base_url)
        } else if base_url.contains("/api/v1") {
            base_url.replace("/api/v1", "/api/v1/ws")
        } else {
            format!("{}/api/v1/ws", base_url.trim_end_matches('/'))
        }
    }

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
