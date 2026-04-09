use super::Key;
use crate::error::ShrinkError;
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyStatus {
    Healthy,
    Exhausted,
    Invalid,
}

pub struct KeyPool {
    keys: Vec<Key>,
    status: Mutex<Vec<KeyStatus>>,
    cursor: AtomicUsize,
}

impl KeyPool {
    pub fn new(keys: Vec<Key>) -> Self {
        let n = keys.len();
        Self {
            keys,
            status: Mutex::new(vec![KeyStatus::Healthy; n]),
            cursor: AtomicUsize::new(0),
        }
    }

    pub fn total(&self) -> usize {
        self.keys.len()
    }

    /// Return the next healthy key, advancing the cursor.
    /// Returns `AllKeysExhausted` if every key is dead.
    pub fn next_healthy(&self) -> Result<Key, ShrinkError> {
        let n = self.keys.len();
        if n == 0 {
            return Err(ShrinkError::NoKeysConfigured);
        }
        let status = self.status.lock().unwrap();
        let start = self.cursor.fetch_add(1, Ordering::Relaxed) % n;
        for offset in 0..n {
            let idx = (start + offset) % n;
            if status[idx] == KeyStatus::Healthy {
                return Ok(self.keys[idx].clone());
            }
        }
        Err(ShrinkError::AllKeysExhausted)
    }

    pub fn mark_exhausted(&self, hash: &str) {
        self.mark(hash, KeyStatus::Exhausted);
    }

    pub fn mark_invalid(&self, hash: &str) {
        self.mark(hash, KeyStatus::Invalid);
    }

    fn mark(&self, hash: &str, new_status: KeyStatus) {
        let mut status = self.status.lock().unwrap();
        for (i, k) in self.keys.iter().enumerate() {
            if k.hash == hash {
                status[i] = new_status;
            }
        }
    }

    /// Snapshot of counts for diagnostics / `keys list`.
    pub fn snapshot(&self) -> KeyPoolSnapshot {
        let status = self.status.lock().unwrap();
        let mut healthy = 0;
        let mut exhausted = 0;
        let mut invalid = 0;
        for s in status.iter() {
            match s {
                KeyStatus::Healthy => healthy += 1,
                KeyStatus::Exhausted => exhausted += 1,
                KeyStatus::Invalid => invalid += 1,
            }
        }
        KeyPoolSnapshot {
            total: self.keys.len(),
            healthy,
            exhausted,
            invalid,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct KeyPoolSnapshot {
    pub total: usize,
    pub healthy: usize,
    pub exhausted: usize,
    pub invalid: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{Key, KeySource};

    fn mk(values: &[&str]) -> Vec<Key> {
        values
            .iter()
            .enumerate()
            .map(|(i, v)| Key {
                value: v.to_string(),
                source: KeySource::Env,
                index: i,
                hash: crate::keys::hash_key(v),
            })
            .collect()
    }

    #[test]
    fn next_healthy_rotates() {
        let pool = KeyPool::new(mk(&["a", "b", "c"]));
        let mut seen = std::collections::HashSet::new();
        for _ in 0..6 {
            seen.insert(pool.next_healthy().unwrap().value);
        }
        assert_eq!(seen.len(), 3);
    }

    #[test]
    fn marking_exhausted_skips_key() {
        let pool = KeyPool::new(mk(&["a", "b"]));
        let a_hash = crate::keys::hash_key("a");
        pool.mark_exhausted(&a_hash);
        for _ in 0..5 {
            let k = pool.next_healthy().unwrap();
            assert_eq!(k.value, "b");
        }
    }

    #[test]
    fn all_dead_returns_error() {
        let pool = KeyPool::new(mk(&["a", "b"]));
        pool.mark_exhausted(&crate::keys::hash_key("a"));
        pool.mark_invalid(&crate::keys::hash_key("b"));
        let err = pool.next_healthy().unwrap_err();
        assert_eq!(err.code(), "all_keys_exhausted");
    }

    #[test]
    fn snapshot_counts() {
        let pool = KeyPool::new(mk(&["a", "b", "c"]));
        pool.mark_exhausted(&crate::keys::hash_key("a"));
        pool.mark_invalid(&crate::keys::hash_key("b"));
        let snap = pool.snapshot();
        assert_eq!(snap.total, 3);
        assert_eq!(snap.healthy, 1);
        assert_eq!(snap.exhausted, 1);
        assert_eq!(snap.invalid, 1);
    }

    #[test]
    fn empty_pool_errors() {
        let pool = KeyPool::new(vec![]);
        assert_eq!(
            pool.next_healthy().unwrap_err().code(),
            "no_keys_configured"
        );
    }
}
