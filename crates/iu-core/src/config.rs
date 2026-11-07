//! `config.json`: user settings with defaults.
//!
//! Every field has a default, so a missing file, a missing field or an
//! older file all load cleanly. Unknown fields are ignored so a config
//! written by a newer version still loads.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    /// Follow the OS language.
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "zh-TW")]
    ZhTw,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "en")]
    En,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub language: Language,
    pub capture: CaptureConfig,
    pub server: ServerConfig,
    /// `tracing` filter, e.g. `info` or `iu_recog=debug,info`.
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: Language::default(),
            capture: CaptureConfig::default(),
            server: ServerConfig::default(),
            log_level: "info".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    /// Spout sender name set in the OBS Spout Filter.
    pub spout_sender: String,
    pub fps: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            spout_sender: "Spout_OBS_Filter".to_owned(),
            fps: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// WebSocket for OBS browser sources.
    pub ws_port: u16,
    /// HTTP server for phones on the LAN.
    pub mobile_port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ws_port: 8767,
            mobile_port: 8787,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read or write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{path} is not valid config JSON: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl Config {
    /// Loads the config from `path`. A missing file yields the defaults.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(ConfigError::Io {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        serde_json::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })
    }

    /// Like [`Config::load`], but a broken file is moved aside to
    /// `config.json.broken` and the defaults are used, so a bad edit
    /// never stops the app from starting.
    pub fn load_or_recover(path: &Path) -> Self {
        match Self::load(path) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!("{e}; falling back to defaults");
                if matches!(e, ConfigError::Parse { .. }) {
                    let backup = path.with_extension("json.broken");
                    if let Err(e) = fs::rename(path, &backup) {
                        tracing::warn!("could not move broken config aside: {e}");
                    }
                }
                Self::default()
            }
        }
    }

    /// Writes the config atomically: a temp file next to it, then rename.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let io_err = |source| ConfigError::Io {
            path: path.to_owned(),
            source,
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(io_err)?;
        }
        let json = serde_json::to_string_pretty(self).expect("Config always serializes");
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(io_err)?;
        fs::rename(&tmp, path).map_err(io_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(&dir.path().join(CONFIG_FILE_NAME)).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(CONFIG_FILE_NAME);
        let config = Config {
            language: Language::Ja,
            capture: CaptureConfig {
                fps: 60,
                ..Default::default()
            },
            ..Default::default()
        };
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), config);
    }

    #[test]
    fn missing_fields_take_defaults_and_unknown_fields_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE_NAME);
        fs::write(
            &path,
            r#"{ "language": "zh-TW", "capture": { "fps": 20 }, "from_the_future": 1 }"#,
        )
        .unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.language, Language::ZhTw);
        assert_eq!(config.capture.fps, 20);
        assert_eq!(config.capture.spout_sender, "Spout_OBS_Filter");
        assert_eq!(config.server, ServerConfig::default());
    }

    #[test]
    fn broken_file_is_moved_aside() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE_NAME);
        fs::write(&path, "{ not json").unwrap();
        assert!(matches!(
            Config::load(&path),
            Err(ConfigError::Parse { .. })
        ));
        assert_eq!(Config::load_or_recover(&path), Config::default());
        assert!(!path.exists());
        assert!(dir.path().join("config.json.broken").exists());
    }
}
