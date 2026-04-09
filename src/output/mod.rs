pub mod human;
pub mod json;

use crate::runner::CompressResult;
use crate::scan::SkipReason;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Start {
        version: String,
        build: &'static str,
        paths: Vec<PathBuf>,
        concurrency: usize,
        dry_run: bool,
        overwrite: bool,
        output_dir: Option<PathBuf>,
        total_files: usize,
        total_bytes: u64,
        ts: DateTime<Utc>,
    },
    File(FileEvent),
    Progress {
        processed: usize,
        total: usize,
        success: usize,
        fail: usize,
        skipped: usize,
        bytes_saved: u64,
        ts: DateTime<Utc>,
    },
    Log {
        level: String,
        event: String,
        detail: serde_json::Value,
        ts: DateTime<Utc>,
    },
    Summary {
        total: usize,
        success: usize,
        fail: usize,
        skipped: usize,
        original_total_bytes: u64,
        compressed_total_bytes: u64,
        saved_bytes: u64,
        saved_percent: f64,
        duration_ms: u64,
        keys_used: usize,
        keys_exhausted: usize,
        exit_code: i32,
        dry_run: bool,
        ts: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FileEvent {
    Success {
        uuid: String,
        path: PathBuf,
        original_size: u64,
        compressed_size: u64,
        ratio: f64,
        saved_bytes: u64,
        output_path: PathBuf,
        key_hash: String,
        duration_ms: u64,
        ts: DateTime<Utc>,
    },
    Fail {
        uuid: String,
        path: PathBuf,
        original_size: u64,
        error: String,
        error_message: String,
        attempted_keys: Vec<String>,
        duration_ms: u64,
        ts: DateTime<Utc>,
    },
    Skipped {
        uuid: String,
        path: PathBuf,
        original_size: u64,
        reason: SkipReason,
        ts: DateTime<Utc>,
    },
    DryRun {
        uuid: String,
        path: PathBuf,
        original_size: u64,
        estimated_saved_bytes: u64,
        ts: DateTime<Utc>,
    },
}

pub fn build() -> &'static str {
    "internal"
}

pub fn file_event_from_result(r: &CompressResult) -> FileEvent {
    let now = Utc::now();
    match r {
        CompressResult::Success {
            task,
            compressed_size,
            output_path,
            key_hash,
            duration_ms,
        } => {
            let saved = task.original_size.saturating_sub(*compressed_size);
            FileEvent::Success {
                uuid: task.uuid.clone(),
                path: task.path.clone(),
                original_size: task.original_size,
                compressed_size: *compressed_size,
                ratio: *compressed_size as f64 / task.original_size.max(1) as f64,
                saved_bytes: saved,
                output_path: output_path.clone(),
                key_hash: key_hash.clone(),
                duration_ms: *duration_ms,
                ts: now,
            }
        }
        CompressResult::Failure {
            task,
            error,
            attempted_keys,
            duration_ms,
        } => FileEvent::Fail {
            uuid: task.uuid.clone(),
            path: task.path.clone(),
            original_size: task.original_size,
            error: error.code.clone(),
            error_message: error.message.clone(),
            attempted_keys: attempted_keys.clone(),
            duration_ms: *duration_ms,
            ts: now,
        },
    }
}

/// Sink receives events in order during a run.
pub trait OutputSink: Send {
    fn emit(&mut self, event: &Event);
    fn finish(&mut self);
}
