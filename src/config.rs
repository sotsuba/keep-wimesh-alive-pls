//! Configuration management
//!
//! This module handles loading and validating configuration from TOML files.
//! The config supports multiple portal types with their specific settings.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

/// Root configuration structure
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Global settings
    #[serde(default)]
    pub global: GlobalConfig,
    
    /// HTTP client settings
    #[serde(default)]
    pub http: HttpConfig,
    
    /// Logging settings
    #[serde(default)]
    pub logging: LoggingConfig,
    
    /// Portal configurations (multiple portals supported)
    #[serde(default)]
    pub portals: Vec<PortalConfig>,
}

/// Global daemon settings
#[derive(Debug, Deserialize, Clone)]
pub struct GlobalConfig {
    /// Check interval in seconds for daemon mode
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            check_interval: default_check_interval(),
        }
    }
}

/// Configuration for a single portal
#[derive(Debug, Deserialize, Clone)]
pub struct PortalConfig {
    /// Human-readable name for this portal
    pub name: String,
    
    /// Portal type: "awing", "fpt", etc.
    #[serde(rename = "type")]
    pub portal_type: String,
    
    /// SSIDs that this portal handles
    pub ssids: Vec<String>,
    
    /// MAC address for authentication (optional, auto-detect if empty)
    #[serde(default)]
    pub mac_address: String,
    
    /// Additional portal-specific settings (for future extensibility)
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, toml::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpConfig {
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Connection timeout in seconds
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,

    /// Maximum number of retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            connect_timeout: default_connect_timeout(),
            max_retries: default_max_retries(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Optional log file path
    #[serde(default)]
    pub log_file: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            log_file: String::new(),
        }
    }
}

// Default value functions
fn default_check_interval() -> u64 {
    5
}

fn default_timeout() -> u64 {
    10
}

fn default_connect_timeout() -> u64 {
    5
}

fn default_max_retries() -> u32 {
    3
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    /// Load configuration from file, or use defaults if not found
    pub fn load() -> Result<Self> {
        let config_paths = vec![
            PathBuf::from("config.toml"),
            PathBuf::from("wimesh-rs/config.toml"),
            PathBuf::from("/etc/wimesh/config.toml"),
            dirs::home_dir()
                .map(|h| h.join(".config/wimesh/config.toml"))
                .unwrap_or_default(),
        ];

        // Try to find config file
        for path in &config_paths {
            if path.exists() {
                tracing::debug!("Loading config from: {}", path.display());
                let contents = std::fs::read_to_string(path)
                    .context("Failed to read config file")?;
                
                let config: Config = toml::from_str(&contents)
                    .context("Failed to parse config file")?;
                
                return Ok(config);
            }
        }

        // No config file found, use defaults
        tracing::debug!("No config file found, using defaults");
        Ok(Self::default())
    }

    /// Get all SSIDs from all configured portals
    pub fn all_ssids(&self) -> Vec<&str> {
        self.portals
            .iter()
            .flat_map(|p| p.ssids.iter().map(|s| s.as_str()))
            .collect()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            http: HttpConfig::default(),
            logging: LoggingConfig::default(),
            portals: vec![PortalConfig {
                name: "KTX Khu B".to_string(),
                portal_type: "awing".to_string(),
                ssids: vec!["1.Free Wi-MESH".to_string()],
                mac_address: String::new(),
                extra: std::collections::HashMap::new(),
            }],
        }
    }
}
