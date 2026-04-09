use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tinypng_cli::cli::RunArgs;
use tinypng_cli::config::{Config, RunConfig};
use tinypng_cli::error::ShrinkError;
use tinypng_cli::keys::load_all_keys;
use tinypng_cli::keys::rotation::KeyPool;
use tinypng_cli::output::{
    build, file_event_from_result, human::HumanSink, json::JsonSink, Event, FileEvent, OutputSink,
};
use tinypng_cli::runner::{run as run_stream, CompressResult, RunSummary, RunnerInput};
use tinypng_cli::scan::{scan_with_skips, ScanEntry};

pub async fn execute(args: &RunArgs, paths: &[PathBuf]) -> Result<i32, ShrinkError> {
    if paths.is_empty() {
        return Err(ShrinkError::BadArgument(
            "no path given; try `tinypng --help`".into(),
        ));
    }

    let file_config = Config::load()?;
    let cfg = RunConfig::resolve(args, paths, &file_config)?;

    let keys = load_all_keys(&file_config)?;
    let pool = Arc::new(KeyPool::new(keys));

    let scan_entries = scan_with_skips(paths, &cfg)?;
    let mut tasks = Vec::new();
    let mut skipped = Vec::new();
    for e in scan_entries {
        match e {
            ScanEntry::Task(t) => tasks.push(t),
            ScanEntry::Skipped {
                path,
                reason,
                original_size,
            } => skipped.push((path, reason, original_size)),
        }
    }

    let total_bytes: u64 = tasks.iter().map(|t| t.original_size).sum();
    let total_files = tasks.len() + skipped.len();

    let mut sink: Box<dyn OutputSink> = if cfg.json {
        Box::new(JsonSink::new(std::io::stdout()))
    } else {
        Box::new(HumanSink::new(cfg.quiet))
    };

    sink.emit(&Event::Start {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build: build(),
        paths: paths.to_vec(),
        concurrency: cfg.concurrency,
        dry_run: cfg.dry_run,
        overwrite: cfg.overwrite,
        output_dir: cfg.output_dir.clone(),
        total_files,
        total_bytes,
        ts: Utc::now(),
    });

    for (path, reason, original_size) in &skipped {
        sink.emit(&Event::File(FileEvent::Skipped {
            uuid: uuid::Uuid::new_v4().to_string(),
            path: path.clone(),
            original_size: *original_size,
            reason: reason.clone(),
            ts: Utc::now(),
        }));
    }

    if cfg.dry_run {
        let mut summary = RunSummary {
            total: total_files,
            skipped: skipped.len(),
            ..Default::default()
        };
        for t in &tasks {
            let est = (t.original_size as f64 * 0.5) as u64;
            summary.original_total_bytes += t.original_size;
            summary.compressed_total_bytes += t.original_size - est;
            summary.saved_bytes += est;
            sink.emit(&Event::File(FileEvent::DryRun {
                uuid: t.uuid.clone(),
                path: t.path.clone(),
                original_size: t.original_size,
                estimated_saved_bytes: est,
                ts: Utc::now(),
            }));
        }
        let snap = pool.snapshot();
        emit_summary(&mut *sink, &summary, 0, snap.exhausted + snap.invalid, true);
        sink.finish();
        return Ok(0);
    }

    let input = RunnerInput {
        tasks,
        pool: pool.clone(),
        cfg: Arc::new(cfg.clone()),
    };
    let (results, mut summary) = run_stream(input).await;
    summary.skipped = skipped.len();
    summary.total = total_files;

    for r in &results {
        sink.emit(&Event::File(file_event_from_result(r)));
    }

    let keys_used = {
        let mut hashes = std::collections::HashSet::new();
        for r in &results {
            if let CompressResult::Success { key_hash, .. } = r {
                hashes.insert(key_hash.clone());
            }
        }
        hashes.len()
    };
    let snap = pool.snapshot();
    emit_summary(
        &mut *sink,
        &summary,
        keys_used,
        snap.exhausted + snap.invalid,
        false,
    );
    sink.finish();

    Ok(summary.exit_code())
}

fn emit_summary(
    sink: &mut dyn OutputSink,
    summary: &RunSummary,
    keys_used: usize,
    keys_exhausted: usize,
    dry_run: bool,
) {
    sink.emit(&Event::Summary {
        total: summary.total,
        success: summary.success,
        fail: summary.fail,
        skipped: summary.skipped,
        original_total_bytes: summary.original_total_bytes,
        compressed_total_bytes: summary.compressed_total_bytes,
        saved_bytes: summary.saved_bytes,
        saved_percent: summary.saved_percent(),
        duration_ms: summary.duration_ms,
        keys_used,
        keys_exhausted,
        exit_code: if dry_run { 0 } else { summary.exit_code() },
        dry_run,
        ts: Utc::now(),
    });
}
