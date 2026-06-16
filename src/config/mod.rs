use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration loaded from ~/.config/sqliteforge/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_mode")]
    pub mode: String,

    #[serde(default = "default_true")]
    pub headers: bool,

    #[serde(default = "default_true")]
    pub history: bool,

    #[serde(default = "default_nullvalue")]
    pub nullvalue: String,
}

fn default_theme() -> String {
    "catppuccin".to_string()
}

fn default_mode() -> String {
    "box".to_string()
}

fn default_true() -> bool {
    true
}

fn default_nullvalue() -> String {
    String::new()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            mode: default_mode(),
            headers: default_true(),
            history: default_true(),
            nullvalue: default_nullvalue(),
        }
    }
}

impl Config {
    /// Returns the path to the configuration file
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("sqliteforge")
            .join("config.toml")
    }

    /// Load configuration from file, falling back to defaults
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse config: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read config: {}", e);
                }
            }
        }
        Self::default()
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
