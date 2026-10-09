use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::dictionary::Source;

/// Overrides the data directory; useful for tests and portable installs.
pub const HOME_ENV: &str = "STEALTHLINGO_HOME";

pub const DEFAULT_FREE_DICTIONARY_URL: &str = "https://api.dictionaryapi.dev/api/v2/entries/en";

/// Locations of everything StealthLingo stores on disk.
#[derive(Debug, Clone)]
pub struct Paths {
    pub data_dir: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self> {
        if let Some(home) = std::env::var_os(HOME_ENV).filter(|v| !v.is_empty()) {
            return Ok(Self::at(PathBuf::from(home)));
        }
        let dirs = ProjectDirs::from("", "", "stealthlingo").context(
            "could not determine a data directory for this platform; set STEALTHLINGO_HOME",
        )?;
        Ok(Self::at(dirs.data_dir().to_path_buf()))
    }

    pub fn at(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn ensure_exists(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir).with_context(|| {
            format!(
                "could not create data directory {}",
                self.data_dir.display()
            )
        })
    }

    pub fn database(&self) -> PathBuf {
        self.data_dir.join("stealthlingo.db")
    }

    pub fn config_file(&self) -> PathBuf {
        self.data_dir.join("config.json")
    }

    pub fn audio_dir(&self) -> PathBuf {
        self.data_dir.join("audio")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Maximum number of never-studied words introduced per local day.
    pub daily_new_limit: u32,
    /// Session length used when neither --minutes nor --count is given.
    pub default_minutes: u32,
    pub dictionary_source: Source,
    #[serde(alias = "api_base_url")]
    pub free_dictionary_url: String,
    pub merriam_webster_key: Option<String>,
    pub http_timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            daily_new_limit: 10,
            default_minutes: 5,
            dictionary_source: Source::default(),
            free_dictionary_url: DEFAULT_FREE_DICTIONARY_URL.to_string(),
            merriam_webster_key: None,
            http_timeout_secs: 10,
        }
    }
}

pub const CONFIG_KEYS: &[&str] = &[
    "daily_new_limit",
    "default_minutes",
    "dictionary_source",
    "free_dictionary_url",
    "merriam_webster_key",
    "http_timeout_secs",
];

impl Config {
    /// Loads the config file, falling back to defaults when it does not exist.
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(text.trim_start_matches('\u{feff}'))
                .with_context(|| format!("config file {} is not valid JSON", path.display())),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err).with_context(|| format!("could not read {}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text + "\n").with_context(|| format!("could not write {}", path.display()))
    }

    /// Updates one setting. Returns the canonical key name that was changed.
    pub fn set(&mut self, key: &str, value: &str) -> Result<&'static str> {
        let value = value.trim();
        let key = match key {
            "daily_new_limit" => {
                self.daily_new_limit = parse_number(key, value)?;
                "daily_new_limit"
            }
            "default_minutes" => {
                let minutes: u32 = parse_number(key, value)?;
                if minutes == 0 {
                    bail!("default_minutes must be at least 1");
                }
                self.default_minutes = minutes;
                "default_minutes"
            }
            "dictionary_source" | "source" => {
                self.dictionary_source = Source::parse(value).with_context(|| {
                    format!(
                        "unknown dictionary source \"{value}\"; choose one of: {}",
                        Source::names()
                    )
                })?;
                "dictionary_source"
            }
            "free_dictionary_url" | "api_base_url" => {
                if !(value.starts_with("http://") || value.starts_with("https://")) {
                    bail!("free_dictionary_url must start with http:// or https://");
                }
                self.free_dictionary_url = value.trim_end_matches('/').to_string();
                "free_dictionary_url"
            }
            "merriam_webster_key" => {
                self.merriam_webster_key = match value {
                    "" | "none" | "-" => None,
                    key => Some(key.to_string()),
                };
                "merriam_webster_key"
            }
            "http_timeout_secs" => {
                let secs: u64 = parse_number(key, value)?;
                if secs == 0 {
                    bail!("http_timeout_secs must be at least 1");
                }
                self.http_timeout_secs = secs;
                "http_timeout_secs"
            }
            other => bail!(
                "unknown config key \"{other}\"; valid keys: {}",
                CONFIG_KEYS.join(", ")
            ),
        };
        Ok(key)
    }

    /// Settings for display. The Merriam-Webster key is partially masked.
    pub fn entries(&self) -> Vec<(&'static str, String)> {
        let key = match &self.merriam_webster_key {
            Some(key) => {
                let visible: String = key.chars().take(4).collect();
                format!("{visible}**** (set)")
            }
            None => "(not set)".to_string(),
        };
        vec![
            ("daily_new_limit", self.daily_new_limit.to_string()),
            ("default_minutes", self.default_minutes.to_string()),
            (
                "dictionary_source",
                self.dictionary_source.as_str().to_string(),
            ),
            ("free_dictionary_url", self.free_dictionary_url.clone()),
            ("merriam_webster_key", key),
            ("http_timeout_secs", self.http_timeout_secs.to_string()),
        ]
    }
}

fn parse_number<T: std::str::FromStr>(key: &str, value: &str) -> Result<T> {
    value
        .parse()
        .ok()
        .with_context(|| format!("{key} must be a non-negative whole number, got \"{value}\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults_and_save_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = Config::load(&path).unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(config.dictionary_source, Source::Wiktionary);

        config.set("daily_new_limit", "3").unwrap();
        config.set("dictionary_source", "Free_Dictionary").unwrap();
        config.set("merriam_webster_key", "abcd-1234").unwrap();
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.daily_new_limit, 3);
        assert_eq!(loaded.dictionary_source, Source::FreeDictionary);
        assert_eq!(loaded.merriam_webster_key.as_deref(), Some("abcd-1234"));
        assert!(loaded
            .entries()
            .contains(&("merriam_webster_key", "abcd**** (set)".to_string())));
    }

    #[test]
    fn reads_legacy_api_base_url_key() {
        let config: Config =
            serde_json::from_str(r#"{"api_base_url": "http://localhost:1/x"}"#).unwrap();
        assert_eq!(config.free_dictionary_url, "http://localhost:1/x");
        assert_eq!(config.dictionary_source, Source::Wiktionary);
    }

    #[test]
    fn rejects_bad_values() {
        let mut config = Config::default();
        assert!(config.set("daily_new_limit", "-1").is_err());
        assert!(config.set("default_minutes", "0").is_err());
        assert!(config.set("free_dictionary_url", "ftp://x").is_err());
        assert!(config.set("dictionary_source", "oxford").is_err());
        assert!(config.set("nope", "1").is_err());
        config.set("merriam_webster_key", "k").unwrap();
        config.set("merriam_webster_key", "none").unwrap();
        assert!(config.merriam_webster_key.is_none());
    }
}
