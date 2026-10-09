use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Overrides the data directory; useful for tests and portable installs.
pub const HOME_ENV: &str = "STEALTHLINGO_HOME";

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

/// Settings from `config.json`. Keys from older versions (such as
/// `dictionary_source`) are ignored and dropped on the next save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Maximum number of never-studied words introduced per local day.
    pub daily_new_limit: u32,
    /// Session length used when neither --minutes nor --count is given.
    pub default_minutes: u32,
    pub http_timeout_secs: u64,
    /// Pronunciation played by default: "UK" or "US". Words recorded in only
    /// one accent play that one.
    pub accent: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            daily_new_limit: 10,
            default_minutes: 5,
            http_timeout_secs: 10,
            accent: "UK".to_string(),
        }
    }
}

pub const CONFIG_KEYS: &[&str] = &[
    "daily_new_limit",
    "default_minutes",
    "http_timeout_secs",
    "accent",
];

/// "UK" or "US" for the spellings people commonly use for them.
pub fn parse_accent(value: &str) -> Result<&'static str> {
    match crate::dictionary::accent_name(value) {
        Some(accent @ ("UK" | "US")) => Ok(accent),
        _ => bail!("accent must be uk or us, got \"{}\"", value.trim()),
    }
}

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
            "http_timeout_secs" => {
                let secs: u64 = parse_number(key, value)?;
                if secs == 0 {
                    bail!("http_timeout_secs must be at least 1");
                }
                self.http_timeout_secs = secs;
                "http_timeout_secs"
            }
            "accent" => {
                self.accent = parse_accent(value)?.to_string();
                "accent"
            }
            "dictionary_source"
            | "source"
            | "free_dictionary_url"
            | "api_base_url"
            | "merriam_webster_key" => {
                bail!("\"{key}\" is no longer a setting: StealthLingo now always uses Wiktionary")
            }
            other => bail!(
                "unknown config key \"{other}\"; valid keys: {}",
                CONFIG_KEYS.join(", ")
            ),
        };
        Ok(key)
    }

    /// Settings for display.
    pub fn entries(&self) -> Vec<(&'static str, String)> {
        vec![
            ("daily_new_limit", self.daily_new_limit.to_string()),
            ("default_minutes", self.default_minutes.to_string()),
            ("http_timeout_secs", self.http_timeout_secs.to_string()),
            ("accent", self.accent.clone()),
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

        config.set("daily_new_limit", "3").unwrap();
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.daily_new_limit, 3);
    }

    #[test]
    fn ignores_settings_from_older_versions() {
        let config: Config = serde_json::from_str(
            r#"{"daily_new_limit": 4, "dictionary_source": "merriam-webster",
                "free_dictionary_url": "http://localhost:1/x", "merriam_webster_key": "k",
                "api_base_url": "http://localhost:1/y"}"#,
        )
        .unwrap();
        assert_eq!(config.daily_new_limit, 4);
        let saved = serde_json::to_string(&config).unwrap();
        assert!(!saved.contains("dictionary"), "{saved}");
    }

    #[test]
    fn rejects_bad_values() {
        let mut config = Config::default();
        assert!(config.set("daily_new_limit", "-1").is_err());
        assert!(config.set("default_minutes", "0").is_err());
        assert!(config.set("http_timeout_secs", "0").is_err());
        assert!(config.set("nope", "1").is_err());
        assert!(config.set("accent", "india").is_err());
        config.set("accent", "us").unwrap();
        assert_eq!(config.accent, "US");
        config.set("accent", "British").unwrap();
        assert_eq!(config.accent, "UK");
        let removed = config
            .set("dictionary_source", "free-dictionary")
            .unwrap_err();
        assert!(removed.to_string().contains("Wiktionary"), "{removed}");
    }
}
