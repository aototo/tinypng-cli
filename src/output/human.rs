use super::{Event, FileEvent, OutputSink};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;

pub struct HumanSink {
    progress: Option<ProgressBar>,
    quiet: bool,
}

impl HumanSink {
    pub fn new(quiet: bool) -> Self {
        Self {
            progress: None,
            quiet,
        }
    }
}

fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.2} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.2} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KB", n as f64 / KB as f64)
    } else {
        format!("{n} B")
    }
}

impl OutputSink for HumanSink {
    fn emit(&mut self, event: &Event) {
        match event {
            Event::Start {
                total_files,
                total_bytes,
                dry_run,
                ..
            } => {
                if !self.quiet {
                    let pb = ProgressBar::new(*total_files as u64);
                    pb.set_style(
                        ProgressStyle::with_template(
                            "{spinner:.green} [{bar:30.cyan/blue}] {pos}/{len} {msg}",
                        )
                        .unwrap()
                        .progress_chars("=> "),
                    );
                    let tag = if *dry_run { " [dry-run]" } else { "" };
                    pb.set_message(format!(
                        "{} files, {}{}",
                        total_files,
                        format_bytes(*total_bytes),
                        tag
                    ));
                    self.progress = Some(pb);
                }
            }
            Event::File(f) => {
                if let Some(pb) = &self.progress {
                    pb.inc(1);
                    match f {
                        FileEvent::Success {
                            path,
                            saved_bytes,
                            original_size,
                            ..
                        } => {
                            let pct = if *original_size > 0 {
                                100.0 * *saved_bytes as f64 / *original_size as f64
                            } else {
                                0.0
                            };
                            pb.set_message(format!(
                                "{} {}  -{:.0}%",
                                "✓".green(),
                                path.display(),
                                pct
                            ));
                        }
                        FileEvent::Fail { path, error, .. } => {
                            pb.set_message(format!(
                                "{} {}  {}",
                                "✗".red(),
                                path.display(),
                                error.yellow()
                            ));
                        }
                        FileEvent::Skipped { path, .. } => {
                            pb.set_message(format!("{} {}", "-".dimmed(), path.display()));
                        }
                        FileEvent::DryRun { path, .. } => {
                            pb.set_message(format!("{} {}", "?".blue(), path.display()));
                        }
                    }
                }
            }
            Event::Progress { .. } | Event::Log { .. } => {}
            Event::Summary {
                total,
                success,
                fail,
                skipped,
                original_total_bytes,
                compressed_total_bytes,
                saved_bytes,
                saved_percent,
                duration_ms,
                keys_used,
                ..
            } => {
                if let Some(pb) = self.progress.take() {
                    pb.finish_and_clear();
                }
                let secs = *duration_ms as f64 / 1000.0;
                println!();
                println!("{}", "tinypng summary".bold());
                println!("  total:    {total}");
                println!("  success:  {}", success.to_string().green());
                if *fail > 0 {
                    println!("  failed:   {}", fail.to_string().red());
                }
                if *skipped > 0 {
                    println!("  skipped:  {}", skipped.to_string().dimmed());
                }
                println!("  before:   {}", format_bytes(*original_total_bytes));
                println!("  after:    {}", format_bytes(*compressed_total_bytes));
                println!(
                    "  saved:    {}  ({:.1}%)",
                    format_bytes(*saved_bytes).green(),
                    saved_percent
                );
                println!("  duration: {secs:.1}s");
                println!("  keys:     {keys_used} used");
            }
        }
    }
    fn finish(&mut self) {
        if let Some(pb) = self.progress.take() {
            pb.finish_and_clear();
        }
    }
}
