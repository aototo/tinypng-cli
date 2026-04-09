use crate::compress::{call_tinypng, CompressOutcome};
use crate::config::RunConfig;
use crate::error::{SerializedError, ShrinkError};
use crate::keys::rotation::KeyPool;
use crate::scan::FileTask;
use futures::stream::{self, StreamExt};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

const MAX_KEY_SWITCHES: usize = 5;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CompressResult {
    Success {
        task: FileTask,
        compressed_size: u64,
        output_path: PathBuf,
        key_hash: String,
        duration_ms: u64,
    },
    Failure {
        task: FileTask,
        error: SerializedError,
        attempted_keys: Vec<String>,
        duration_ms: u64,
    },
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct RunSummary {
    pub total: usize,
    pub success: usize,
    pub fail: usize,
    pub skipped: usize,
    pub original_total_bytes: u64,
    pub compressed_total_bytes: u64,
    pub saved_bytes: u64,
    pub duration_ms: u64,
}

impl RunSummary {
    pub fn saved_percent(&self) -> f64 {
        if self.original_total_bytes == 0 {
            0.0
        } else {
            100.0 * self.saved_bytes as f64 / self.original_total_bytes as f64
        }
    }
    pub fn exit_code(&self) -> i32 {
        if self.fail == 0 {
            0
        } else if self.success > 0 {
            1
        } else {
            4
        }
    }
}

pub struct RunnerInput {
    pub tasks: Vec<FileTask>,
    pub pool: Arc<KeyPool>,
    pub cfg: Arc<RunConfig>,
}

pub async fn run(input: RunnerInput) -> (Vec<CompressResult>, RunSummary) {
    let RunnerInput { tasks, pool, cfg } = input;
    let total = tasks.len();
    let mut summary = RunSummary {
        total,
        ..Default::default()
    };
    let start = Instant::now();

    let concurrency = cfg.concurrency;
    let results: Vec<CompressResult> = stream::iter(tasks)
        .map(|task| {
            let pool = pool.clone();
            let cfg = cfg.clone();
            async move { compress_one(task, pool, cfg).await }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    for r in &results {
        match r {
            CompressResult::Success {
                task,
                compressed_size,
                ..
            } => {
                summary.success += 1;
                summary.original_total_bytes += task.original_size;
                summary.compressed_total_bytes += *compressed_size;
            }
            CompressResult::Failure { task, .. } => {
                summary.fail += 1;
                summary.original_total_bytes += task.original_size;
                summary.compressed_total_bytes += task.original_size;
            }
        }
    }
    summary.saved_bytes = summary
        .original_total_bytes
        .saturating_sub(summary.compressed_total_bytes);
    summary.duration_ms = start.elapsed().as_millis() as u64;
    (results, summary)
}

async fn compress_one(task: FileTask, pool: Arc<KeyPool>, cfg: Arc<RunConfig>) -> CompressResult {
    let start = Instant::now();
    let mut attempted_keys: Vec<String> = Vec::new();

    for _ in 0..MAX_KEY_SWITCHES {
        let key = match pool.next_healthy() {
            Ok(k) => k,
            Err(e) => {
                return CompressResult::Failure {
                    task,
                    error: (&e).into(),
                    attempted_keys,
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
        };
        attempted_keys.push(key.hash.clone());

        let call_result = call_tinypng(&task.path, &key).await;
        match call_result {
            Ok(CompressOutcome::Ok(bytes)) => match write_result(&task, &bytes, &cfg) {
                Ok(output_path) => {
                    return CompressResult::Success {
                        compressed_size: bytes.len() as u64,
                        output_path,
                        key_hash: key.hash.clone(),
                        duration_ms: start.elapsed().as_millis() as u64,
                        task,
                    };
                }
                Err(e) => {
                    return CompressResult::Failure {
                        task,
                        error: (&e).into(),
                        attempted_keys,
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }
            },
            Err(e) => {
                // Fatal: don't rotate, file itself is bad.
                if matches!(
                    e,
                    ShrinkError::InvalidImage(_)
                        | ShrinkError::UnsupportedFormat(_)
                        | ShrinkError::FileTooLarge { .. }
                ) {
                    return CompressResult::Failure {
                        task,
                        error: (&e).into(),
                        attempted_keys,
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }

                // Key-dead: mark and rotate.
                if e.is_key_dead() {
                    match e {
                        ShrinkError::KeyExhausted => pool.mark_exhausted(&key.hash),
                        ShrinkError::KeyInvalid => pool.mark_invalid(&key.hash),
                        _ => {}
                    }
                    continue;
                }

                // Transient: rotate to next key.
                if e.is_transient() {
                    continue;
                }

                // Other: fail the file.
                return CompressResult::Failure {
                    task,
                    error: (&e).into(),
                    attempted_keys,
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
        }
    }

    CompressResult::Failure {
        task,
        error: (&ShrinkError::MaxRetriesExceeded).into(),
        attempted_keys,
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

fn write_result(task: &FileTask, bytes: &[u8], cfg: &RunConfig) -> Result<PathBuf, ShrinkError> {
    let output_path: PathBuf = match (&cfg.output_dir, cfg.overwrite) {
        (Some(dir), _) => {
            std::fs::create_dir_all(dir)?;
            dir.join(task.path.file_name().unwrap())
        }
        (None, true) => task.path.clone(),
        (None, false) => generate_compressed_name(&task.path),
    };

    let tmp = output_path.with_extension("tinypng.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, &output_path)?;
    Ok(output_path)
}

fn generate_compressed_name(path: &std::path::Path) -> PathBuf {
    let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let new_name = if ext.is_empty() {
        format!("compressed_{stem}")
    } else {
        format!("compressed_{stem}.{ext}")
    };
    dir.join(new_name)
}
