use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use serde::Serialize;

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

/// Settings from `config.json`. Every key is described in [`SETTINGS`]. Keys
/// from older versions (such as `dictionary_source`) are ignored and dropped
/// on the next save.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Config {
    /// Different words to practise per local day; 0 turns the goal off.
    pub daily_goal: u32,
    /// Maximum number of never-studied words introduced per local day.
    pub daily_new_limit: u32,
    /// Session length used when neither --minutes nor --count is given.
    pub default_minutes: u32,
    pub http_timeout_secs: u64,
    /// Pronunciation played by default: "UK" or "US". Words recorded in only
    /// one accent play that one.
    pub accent: String,
    /// Words an agent may save through MCP per local day; 0 turns it off.
    pub mcp_daily_add_limit: u32,
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Self {
            daily_goal: 0,
            daily_new_limit: 0,
            default_minutes: 0,
            http_timeout_secs: 0,
            accent: String::new(),
            mcp_daily_add_limit: 0,
        };
        for setting in SETTINGS {
            setting.reset(&mut config);
        }
        config
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Study,
    Audio,
    Agent,
    Network,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Self::Study => "Study",
            Self::Audio => "Audio",
            Self::Agent => "Agent (MCP)",
            Self::Network => "Network",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Number {
        min: u64,
        max: u64,
        default: u64,
        /// What 0 means, when it has a special meaning.
        zero: Option<&'static str>,
        get: fn(&Config) -> u64,
        put: fn(&mut Config, u64),
    },
    Accent,
}

/// One user setting: its key in `config.json`, how it is shown and which
/// values it accepts. The settings screen, `config set` / `reset` / `show`
/// and loading `config.json` all follow this table.
#[derive(Debug, Clone, Copy)]
pub struct Setting {
    pub key: &'static str,
    pub label: &'static str,
    pub group: Group,
    pub help: &'static str,
    pub kind: Kind,
}

const DEFAULT_ACCENT: &str = "UK";

/// All settings, in the order the settings screen lists them.
pub const SETTINGS: &[Setting] = &[
    Setting {
        key: "daily_goal",
        label: "Daily goal (words)",
        group: Group::Study,
        help: "Words practised today, each counted once.",
        kind: Kind::Number {
            min: 0,
            max: 500,
            default: 10,
            zero: Some("off"),
            get: |c| c.daily_goal.into(),
            put: |c, v| c.daily_goal = v as u32,
        },
    },
    Setting {
        key: "daily_new_limit",
        label: "New words per day",
        group: Group::Study,
        help: "Most never-studied words introduced per day.",
        kind: Kind::Number {
            min: 0,
            max: 200,
            default: 10,
            zero: Some("review only"),
            get: |c| c.daily_new_limit.into(),
            put: |c, v| c.daily_new_limit = v as u32,
        },
    },
    Setting {
        key: "default_minutes",
        label: "Default session (min)",
        group: Group::Study,
        help: "Session length without --minutes or --count.",
        kind: Kind::Number {
            min: 1,
            max: 120,
            default: 5,
            zero: None,
            get: |c| c.default_minutes.into(),
            put: |c, v| c.default_minutes = v as u32,
        },
    },
    Setting {
        key: "accent",
        label: "Default accent",
        group: Group::Audio,
        help: "Pronunciation played first.",
        kind: Kind::Accent,
    },
    Setting {
        key: "mcp_daily_add_limit",
        label: "Agent adds per day",
        group: Group::Agent,
        help: "Words an AI agent may save per day through MCP.",
        kind: Kind::Number {
            min: 0,
            max: 200,
            default: 30,
            zero: Some("off"),
            get: |c| c.mcp_daily_add_limit.into(),
            put: |c, v| c.mcp_daily_add_limit = v as u32,
        },
    },
    Setting {
        key: "http_timeout_secs",
        label: "Lookup timeout (s)",
        group: Group::Network,
        help: "Time limit for one dictionary lookup.",
        kind: Kind::Number {
            min: 1,
            max: 120,
            default: 10,
            zero: None,
            get: |c| c.http_timeout_secs,
            put: |c, v| c.http_timeout_secs = v,
        },
    },
];

/// Keys of settings that older versions had.
const REMOVED_KEYS: &[&str] = &[
    "dictionary_source",
    "source",
    "free_dictionary_url",
    "api_base_url",
    "merriam_webster_key",
];

/// The setting called `key`.
pub fn setting(key: &str) -> Result<&'static Setting> {
    if let Some(setting) = SETTINGS.iter().find(|s| s.key == key) {
        return Ok(setting);
    }
    if REMOVED_KEYS.contains(&key) {
        bail!("\"{key}\" is no longer a setting: StealthLingo now always uses Wiktionary");
    }
    bail!(
        "unknown config key \"{key}\"; valid keys: {}",
        SETTINGS
            .iter()
            .map(|s| s.key)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// "UK" or "US" for the spellings people commonly use for them.
pub fn parse_accent(value: &str) -> Result<&'static str> {
    match crate::dictionary::accent_name(value) {
        Some(accent @ ("UK" | "US")) => Ok(accent),
        _ => bail!("accent must be uk or us, got \"{}\"", value.trim()),
    }
}

impl Setting {
    /// The current value as shown to the user.
    pub fn value(&self, config: &Config) -> String {
        match self.kind {
            Kind::Number { get, .. } => get(config).to_string(),
            Kind::Accent => config.accent.clone(),
        }
    }

    pub fn default_value(&self) -> String {
        match self.kind {
            Kind::Number { default, .. } => default.to_string(),
            Kind::Accent => DEFAULT_ACCENT.to_string(),
        }
    }

    /// Allowed values, e.g. "0–500, 0 = off".
    pub fn range(&self) -> String {
        match self.kind {
            Kind::Number { min, max, zero, .. } => match zero {
                Some(zero) => format!("{min}–{max}, 0 = {zero}"),
                None => format!("{min}–{max}"),
            },
            Kind::Accent => "UK / US".to_string(),
        }
    }

    pub fn is_default(&self, config: &Config) -> bool {
        self.value(config) == self.default_value()
    }

    pub fn reset(&self, config: &mut Config) {
        match self.kind {
            Kind::Number { default, put, .. } => put(config, default),
            Kind::Accent => config.accent = DEFAULT_ACCENT.to_string(),
        }
    }

    /// Validates and stores a value typed by the user.
    pub fn set(&self, config: &mut Config, value: &str) -> Result<()> {
        let value = value.trim();
        match self.kind {
            Kind::Number {
                min,
                max,
                put,
                zero,
                ..
            } => {
                let zero = zero.map_or(String::new(), |z| format!("; 0 = {z}"));
                let number = value
                    .parse::<u64>()
                    .ok()
                    .filter(|n| (min..=max).contains(n))
                    .with_context(|| {
                        format!(
                            "{} must be a whole number from {min} to {max}{zero}, got \"{value}\"",
                            self.key
                        )
                    })?;
                put(config, number);
            }
            Kind::Accent => config.accent = parse_accent(value)?.to_string(),
        }
        Ok(())
    }

    /// Moves a number by `delta`, stopping at the ends of its range; switches
    /// the accent. Returns false when nothing changed.
    pub fn step(&self, config: &mut Config, delta: i64) -> bool {
        match self.kind {
            Kind::Number {
                min, max, get, put, ..
            } => {
                let old = get(config);
                let new = old.saturating_add_signed(delta).clamp(min, max);
                put(config, new);
                new != old
            }
            Kind::Accent => {
                config.accent = if config.accent == "UK" { "US" } else { "UK" }.to_string();
                true
            }
        }
    }

    /// Reads this setting from a parsed `config.json`. Out-of-range numbers
    /// are pulled to the nearest limit; other bad values fall back to the
    /// default. Returns a warning when the file's value is not used as is.
    fn load(&self, config: &mut Config, value: &serde_json::Value) -> Option<String> {
        match self.kind {
            Kind::Number {
                min,
                max,
                default,
                put,
                ..
            } => {
                let number = value
                    .as_u64()
                    .map(i128::from)
                    .or_else(|| value.as_i64().map(i128::from));
                match number {
                    Some(n) => {
                        let used = n.clamp(i128::from(min), i128::from(max)) as u64;
                        put(config, used);
                        (i128::from(used) != n).then(|| {
                            format!(
                                "{} = {n} in config.json is outside {min}–{max}; using {used}",
                                self.key
                            )
                        })
                    }
                    None => {
                        put(config, default);
                        Some(format!(
                            "{} = {value} in config.json is not a whole number; using {default}",
                            self.key
                        ))
                    }
                }
            }
            Kind::Accent => match value.as_str().map(parse_accent) {
                Some(Ok(accent)) => {
                    config.accent = accent.to_string();
                    None
                }
                _ => {
                    config.accent = DEFAULT_ACCENT.to_string();
                    Some(format!(
                        "accent = {value} in config.json is not uk or us; using {DEFAULT_ACCENT}"
                    ))
                }
            },
        }
    }
}

impl Config {
    /// Loads the config file, falling back to defaults when it does not
    /// exist. Also returns a warning for every value that could not be used
    /// as written; the file itself is left alone.
    pub fn load(path: &Path) -> Result<(Self, Vec<String>)> {
        match fs::read_to_string(path) {
            Ok(text) => Self::from_json(&text)
                .with_context(|| format!("config file {} is not valid", path.display())),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Ok((Self::default(), Vec::new()))
            }
            Err(err) => Err(err).with_context(|| format!("could not read {}", path.display())),
        }
    }

    pub fn from_json(text: &str) -> Result<(Self, Vec<String>)> {
        let value: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .context("it is not valid JSON")?;
        let Some(map) = value.as_object() else {
            bail!("it must contain a JSON object");
        };
        let mut config = Self::default();
        let warnings = SETTINGS
            .iter()
            .filter_map(|s| map.get(s.key).and_then(|v| s.load(&mut config, v)))
            .collect();
        Ok((config, warnings))
    }

    /// Writes the file through a temporary file, so an interrupted save
    /// cannot leave a half-written config behind.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)? + "\n";
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, text).with_context(|| format!("could not write {}", temp.display()))?;
        if let Err(err) = fs::rename(&temp, path) {
            let _ = fs::remove_file(&temp);
            return Err(err).with_context(|| format!("could not replace {}", path.display()));
        }
        Ok(())
    }

    /// Updates one setting. Returns the canonical key name that was changed.
    pub fn set(&mut self, key: &str, value: &str) -> Result<&'static str> {
        let setting = setting(key)?;
        setting.set(self, value)?;
        Ok(setting.key)
    }

    /// Restores one setting to its default.
    pub fn reset(&mut self, key: &str) -> Result<&'static str> {
        let setting = setting(key)?;
        setting.reset(self);
        Ok(setting.key)
    }

    /// Settings for display.
    pub fn entries(&self) -> Vec<(&'static str, String)> {
        SETTINGS.iter().map(|s| (s.key, s.value(self))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults_and_save_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let (mut config, warnings) = Config::load(&path).unwrap();
        assert_eq!(config, Config::default());
        assert!(warnings.is_empty());
        assert_eq!(config.daily_goal, 10);
        assert_eq!(config.mcp_daily_add_limit, 30);

        config.set("daily_new_limit", "3").unwrap();
        config.save(&path).unwrap();
        let (loaded, _) = Config::load(&path).unwrap();
        assert_eq!(loaded.daily_new_limit, 3);
        assert!(!dir.path().join("config.json.tmp").exists());
    }

    #[test]
    fn ignores_settings_from_older_versions() {
        let (config, warnings) = Config::from_json(
            r#"{"daily_new_limit": 4, "dictionary_source": "merriam-webster",
                "free_dictionary_url": "http://localhost:1/x", "merriam_webster_key": "k",
                "api_base_url": "http://localhost:1/y"}"#,
        )
        .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(config.daily_new_limit, 4);
        let saved = serde_json::to_string(&config).unwrap();
        assert!(!saved.contains("dictionary"), "{saved}");
    }

    #[test]
    fn out_of_range_values_use_the_nearest_limit() {
        let (config, warnings) = Config::from_json(
            r#"{"daily_new_limit": 300, "default_minutes": 0, "daily_goal": -4,
                "http_timeout_secs": "fast", "accent": "india"}"#,
        )
        .unwrap();
        assert_eq!(config.daily_new_limit, 200);
        assert_eq!(config.default_minutes, 1);
        assert_eq!(config.daily_goal, 0);
        assert_eq!(config.http_timeout_secs, 10);
        assert_eq!(config.accent, "UK");
        assert_eq!(warnings.len(), 5, "{warnings:?}");
        assert!(warnings[0].contains("daily_goal = -4"), "{warnings:?}");
    }

    #[test]
    fn rejects_bad_values() {
        let mut config = Config::default();
        assert!(config.set("daily_new_limit", "-1").is_err());
        assert!(config.set("daily_new_limit", "201").is_err());
        assert!(config.set("daily_goal", "501").is_err());
        assert!(config.set("default_minutes", "0").is_err());
        assert!(config.set("http_timeout_secs", "0").is_err());
        assert!(config.set("nope", "1").is_err());
        assert!(config.set("accent", "india").is_err());
        assert_eq!(config, Config::default());
        config.set("daily_goal", "0").unwrap();
        assert_eq!(config.daily_goal, 0);
        config.set("accent", "us").unwrap();
        assert_eq!(config.accent, "US");
        config.set("accent", "British").unwrap();
        assert_eq!(config.accent, "UK");
        let removed = config
            .set("dictionary_source", "free-dictionary")
            .unwrap_err();
        assert!(removed.to_string().contains("Wiktionary"), "{removed}");
    }

    #[test]
    fn steps_stop_at_the_limits_and_reset_restores_defaults() {
        let mut config = Config::default();
        let goal = setting("daily_goal").unwrap();
        assert!(goal.step(&mut config, -10));
        assert!(!goal.step(&mut config, -1));
        assert_eq!(config.daily_goal, 0);
        goal.step(&mut config, 1000);
        assert_eq!(config.daily_goal, 500);
        let accent = setting("accent").unwrap();
        accent.step(&mut config, 1);
        assert_eq!(config.accent, "US");
        config.reset("daily_goal").unwrap();
        config.reset("accent").unwrap();
        assert_eq!(config, Config::default());
    }
}
