use crate::error::ShrinkError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// On-disk TOML schema. All fields optional so missing fields use defaults.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overwrite: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_compressed: Option<bool>,
    #[serde(default)]
    pub keys: KeysConfig,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct KeysConfig {
    #[serde(default)]
    pub values: Vec<String>,
}

impl Config {
    /// Config file path: forced XDG convention (`~/.config/tinypng/config.toml` on Unix).
    pub fn path() -> PathBuf {
        if let Some(home) = dirs_home() {
            home.join(".config").join("tinypng").join("config.toml")
        } else {
            PathBuf::from("tinypng_config.toml")
        }
    }

    pub fn load() -> Result<Self, ShrinkError> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)
            .map_err(|e| ShrinkError::ConfigParse(format!("read {}: {}", path.display(), e)))?;
        toml::from_str(&raw).map_err(|e| ShrinkError::ConfigParse(e.to_string()))
    }

    pub fn save(&self) -> Result<(), ShrinkError> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw =
            toml::to_string_pretty(self).map_err(|e| ShrinkError::ConfigParse(e.to_string()))?;
        fs::write(&path, raw)?;
        Ok(())
    }
}

fn dirs_home() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())
}

/// Parse human-readable byte size: "10k", "1.5m", "500", "2G".
pub fn parse_size(s: &str) -> Result<u64, ShrinkError> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return Err(ShrinkError::BadArgument("empty size".into()));
    }
    let last = s.chars().last().unwrap();
    let (num_part, suffix): (&str, Option<char>) = if last.is_ascii_alphabetic() {
        (&s[..s.len() - 1], Some(last))
    } else {
        (&s[..], None)
    };

    let num: f64 = num_part
        .parse()
        .map_err(|_| ShrinkError::BadArgument(format!("invalid size: {s}")))?;
    if num < 0.0 {
        return Err(ShrinkError::BadArgument(format!("negative size: {s}")));
    }

    let mult: u64 = match suffix {
        None | Some('b') => 1,
        Some('k') => 1_024,
        Some('m') => 1_024 * 1_024,
        Some('g') => 1_024 * 1_024 * 1_024,
        Some(c) => {
            return Err(ShrinkError::BadArgument(format!(
                "unknown size suffix: {c}"
            )))
        }
    };

    Ok((num * mult as f64) as u64)
}

/// Final runtime config: CLI flags > config file > built-in defaults.
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub concurrency: usize,
    pub overwrite: bool,
    pub min_size: u64,
    pub extensions: HashSet<String>,
    pub skip_compressed: bool,
    pub output_dir: Option<PathBuf>,
    pub dry_run: bool,
    pub json: bool,
    pub quiet: bool,
    pub verbose: bool,
}

impl RunConfig {
    pub fn resolve(
        cli: &crate::cli::RunArgs,
        paths: &[std::path::PathBuf],
        config: &Config,
    ) -> Result<Self, ShrinkError> {
        let defaults = Self::default();

        let concurrency = cli
            .concurrency
            .or(config.concurrency)
            .unwrap_or(defaults.concurrency);

        let overwrite = if cli.overwrite {
            true
        } else {
            config.overwrite.unwrap_or(defaults.overwrite)
        };

        let min_size_str = cli.min_size.clone().or(config.min_size.clone());
        let min_size = match min_size_str {
            Some(s) => parse_size(&s)?,
            None => defaults.min_size,
        };

        let ext_source = cli
            .ext
            .clone()
            .map(|s| {
                s.split(',')
                    .map(|x| x.trim().to_lowercase())
                    .collect::<Vec<_>>()
            })
            .or_else(|| {
                config
                    .extensions
                    .clone()
                    .map(|v| v.into_iter().map(|s| s.to_lowercase()).collect())
            });
        let extensions: HashSet<String> = match ext_source {
            Some(v) => v.into_iter().collect(),
            None => defaults.extensions.clone(),
        };

        let skip_compressed = if cli.no_skip_compressed {
            false
        } else {
            config.skip_compressed.unwrap_or(defaults.skip_compressed)
        };

        for p in paths {
            if !p.exists() {
                return Err(ShrinkError::BadArgument(format!(
                    "path does not exist: {}",
                    p.display()
                )));
            }
        }

        Ok(Self {
            concurrency,
            overwrite,
            min_size,
            extensions,
            skip_compressed,
            output_dir: cli.output.clone(),
            dry_run: cli.dry_run,
            json: cli.json,
            quiet: cli.quiet,
            verbose: cli.verbose,
        })
    }
}

impl Default for RunConfig {
    fn default() -> Self {
        let mut ext = HashSet::new();
        for e in ["png", "jpg", "jpeg", "webp"] {
            ext.insert(e.to_string());
        }
        Self {
            concurrency: 4,
            overwrite: false,
            min_size: 10 * 1024,
            extensions: ext,
            skip_compressed: true,
            output_dir: None,
            dry_run: false,
            json: false,
            quiet: false,
            verbose: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_size_ok() {
        assert_eq!(parse_size("500").unwrap(), 500);
        assert_eq!(parse_size("10k").unwrap(), 10 * 1024);
        assert_eq!(parse_size("1M").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("2g").unwrap(), 2 * 1024 * 1024 * 1024);
        assert_eq!(parse_size("1.5m").unwrap(), (1.5 * 1024.0 * 1024.0) as u64);
    }

    #[test]
    fn parse_size_invalid() {
        assert!(parse_size("").is_err());
        assert!(parse_size("abc").is_err());
        assert!(parse_size("-1k").is_err());
        assert!(parse_size("1x").is_err());
    }

    #[test]
    fn config_defaults_all_none() {
        let c = Config::default();
        assert!(c.concurrency.is_none());
        assert!(c.overwrite.is_none());
        assert!(c.keys.values.is_empty());
    }

    #[test]
    fn config_toml_roundtrip() {
        let c = Config {
            concurrency: Some(8),
            overwrite: Some(true),
            min_size: Some("20k".into()),
            extensions: Some(vec!["png".into(), "jpg".into()]),
            skip_compressed: Some(false),
            keys: KeysConfig {
                values: vec!["abc".into(), "def".into()],
            },
        };
        let s = toml::to_string(&c).unwrap();
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(back.concurrency, Some(8));
        assert_eq!(back.keys.values.len(), 2);
    }

    #[test]
    fn resolve_cli_overrides_config() {
        use crate::cli::RunArgs;
        let args = RunArgs {
            output: None,
            overwrite: true,
            ext: Some("png,jpg".into()),
            min_size: Some("50k".into()),
            no_skip_compressed: false,
            concurrency: Some(16),
            dry_run: false,
            json: true,
            quiet: false,
            verbose: false,
        };
        let config = Config {
            concurrency: Some(2),
            min_size: Some("5k".into()),
            ..Config::default()
        };

        let tmp = tempfile::tempdir().unwrap();
        let r = RunConfig::resolve(&args, &[tmp.path().to_path_buf()], &config).unwrap();
        assert_eq!(r.concurrency, 16);
        assert_eq!(r.min_size, 50 * 1024);
        assert!(r.overwrite);
        assert!(r.extensions.contains("png"));
        assert!(!r.extensions.contains("webp"));
    }

    #[test]
    fn run_config_defaults() {
        let r = RunConfig::default();
        assert_eq!(r.concurrency, 4);
        assert_eq!(r.min_size, 10 * 1024);
        assert!(r.extensions.contains("png"));
        assert!(!r.extensions.contains("gif"));
    }
}
