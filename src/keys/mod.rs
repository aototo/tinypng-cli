pub mod rotation;

use crate::config::Config;
use crate::error::ShrinkError;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeySource {
    Env,
    Config,
}

#[derive(Debug, Clone)]
pub struct Key {
    pub value: String,
    pub source: KeySource,
    pub index: usize,
    pub hash: String, // sha256(value)[0..8]
}

pub fn hash_key(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(&digest[..4]) // 4 bytes = 8 hex chars
}

fn make_key(value: String, source: KeySource, index: usize) -> Key {
    let hash = hash_key(&value);
    Key {
        value,
        source,
        index,
        hash,
    }
}

/// Load keys from all sources: env > config.
/// Both sources are merged into one pool (not fallback).
pub fn load_all_keys(config: &Config) -> Result<Vec<Key>, ShrinkError> {
    let mut keys: Vec<Key> = Vec::new();

    // 1. Env
    if let Ok(single) = std::env::var("TINIFY_KEY") {
        if !single.is_empty() {
            let idx = keys.len();
            keys.push(make_key(single, KeySource::Env, idx));
        }
    }
    if let Ok(multi) = std::env::var("TINIFY_KEYS") {
        for k in multi.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let idx = keys.len();
            keys.push(make_key(k.to_string(), KeySource::Env, idx));
        }
    }

    // 2. Config file
    for k in &config.keys.values {
        if k.is_empty() {
            continue;
        }
        let idx = keys.len();
        keys.push(make_key(k.clone(), KeySource::Config, idx));
    }

    dedup_by_value(&mut keys);

    if keys.is_empty() {
        return Err(ShrinkError::NoKeysConfigured);
    }
    Ok(keys)
}

fn dedup_by_value(keys: &mut Vec<Key>) {
    let mut seen = std::collections::HashSet::new();
    keys.retain(|k| seen.insert(k.value.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_8_hex_chars() {
        let h = hash_key("mykey123");
        assert_eq!(h.len(), 8);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_key("same"), hash_key("same"));
        assert_ne!(hash_key("a"), hash_key("b"));
    }

    #[test]
    fn no_keys_returns_error() {
        let _guard = EnvGuard::set(&[]);
        let config = Config::default();
        let err = load_all_keys(&config).unwrap_err();
        assert_eq!(err.code(), "no_keys_configured");
    }

    #[test]
    fn env_single_key_loads() {
        let _guard = EnvGuard::set(&[("TINIFY_KEY", "abc123")]);
        let config = Config::default();
        let keys = load_all_keys(&config).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].value, "abc123");
        assert_eq!(keys[0].source, KeySource::Env);
    }

    #[test]
    fn env_multi_keys_parsed() {
        let _guard = EnvGuard::set(&[("TINIFY_KEYS", "a,b,c")]);
        let config = Config::default();
        let keys = load_all_keys(&config).unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].value, "a");
        assert_eq!(keys[2].value, "c");
    }

    #[test]
    fn env_and_config_merge() {
        let _guard = EnvGuard::set(&[("TINIFY_KEY", "env_key")]);
        let mut config = Config::default();
        config.keys.values = vec!["config_key".into()];
        let keys = load_all_keys(&config).unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].source, KeySource::Env);
        assert_eq!(keys[1].source, KeySource::Config);
    }

    #[test]
    fn dedup_removes_duplicates_across_sources() {
        let _guard = EnvGuard::set(&[("TINIFY_KEY", "same_key")]);
        let mut config = Config::default();
        config.keys.values = vec!["same_key".into()];
        let keys = load_all_keys(&config).unwrap();
        assert_eq!(keys.len(), 1, "duplicate key should be deduped to 1");
    }

    // ---- test helper ----
    /// Takes a serialized lock so key-env tests never race.
    struct EnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        keys: Vec<String>,
        original: Vec<(String, Option<String>)>,
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    impl EnvGuard {
        fn set(pairs: &[(&str, &str)]) -> Self {
            let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let all = ["TINIFY_KEY", "TINIFY_KEYS"];
            let mut original = Vec::new();
            for k in all {
                original.push((k.to_string(), std::env::var(k).ok()));
                // SAFETY: tests hold ENV_LOCK mutex, serializing env mutation.
                unsafe { std::env::remove_var(k) };
            }
            for (k, v) in pairs {
                unsafe { std::env::set_var(k, v) };
            }
            Self {
                _lock: lock,
                keys: all.iter().map(|s| s.to_string()).collect(),
                original,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in &self.original {
                match v {
                    Some(val) => unsafe { std::env::set_var(k, val) },
                    None => unsafe { std::env::remove_var(k) },
                }
            }
            let _ = &self.keys;
        }
    }
}
