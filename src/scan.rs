use crate::config::RunConfig;
use crate::error::ShrinkError;
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize)]
pub struct FileTask {
    pub uuid: String,
    pub path: PathBuf,
    pub original_size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    BelowMinSize,
    AlreadyCompressed,
    UnsupportedExtension,
}

#[derive(Debug, Clone)]
pub enum ScanEntry {
    Task(FileTask),
    Skipped {
        path: PathBuf,
        reason: SkipReason,
        original_size: u64,
    },
}

pub fn scan_paths(paths: &[PathBuf], cfg: &RunConfig) -> Result<Vec<FileTask>, ShrinkError> {
    let entries = scan_with_skips(paths, cfg)?;
    Ok(entries
        .into_iter()
        .filter_map(|e| match e {
            ScanEntry::Task(t) => Some(t),
            ScanEntry::Skipped { .. } => None,
        })
        .collect())
}

pub fn scan_with_skips(paths: &[PathBuf], cfg: &RunConfig) -> Result<Vec<ScanEntry>, ShrinkError> {
    let mut out = Vec::new();
    for p in paths {
        if !p.exists() {
            return Err(ShrinkError::BadArgument(format!(
                "path does not exist: {}",
                p.display()
            )));
        }
        if p.is_file() {
            push_file(p, cfg, &mut out)?;
        } else if p.is_dir() {
            for entry in WalkDir::new(p).follow_links(false) {
                let entry = entry.map_err(|e| ShrinkError::IoError(e.to_string()))?;
                if entry.file_type().is_file() {
                    push_file(entry.path(), cfg, &mut out)?;
                }
            }
        }
    }
    Ok(out)
}

fn push_file(path: &Path, cfg: &RunConfig, out: &mut Vec<ScanEntry>) -> Result<(), ShrinkError> {
    let meta = std::fs::metadata(path)?;
    let size = meta.len();

    // extension check
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if !cfg.extensions.contains(&ext) {
        out.push(ScanEntry::Skipped {
            path: path.to_path_buf(),
            reason: SkipReason::UnsupportedExtension,
            original_size: size,
        });
        return Ok(());
    }

    // skip compressed_ prefix
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if cfg.skip_compressed && file_name.starts_with("compressed_") {
        out.push(ScanEntry::Skipped {
            path: path.to_path_buf(),
            reason: SkipReason::AlreadyCompressed,
            original_size: size,
        });
        return Ok(());
    }

    // min size
    if size < cfg.min_size {
        out.push(ScanEntry::Skipped {
            path: path.to_path_buf(),
            reason: SkipReason::BelowMinSize,
            original_size: size,
        });
        return Ok(());
    }

    out.push(ScanEntry::Task(FileTask {
        uuid: Uuid::new_v4().to_string(),
        path: path.to_path_buf(),
        original_size: size,
    }));
    Ok(())
}
