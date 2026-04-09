use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Fast batch image compression powered by TinyPNG, built for humans and AI.
#[derive(Debug, Parser)]
#[command(name = "tinypng", version, about, long_about = None)]
pub struct Cli {
    /// Files or directories to compress (recursive). Omit when using a subcommand.
    pub paths: Vec<PathBuf>,

    #[command(flatten)]
    pub run: RunArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Write compressed files to this directory (default: in-place).
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Overwrite originals instead of writing compressed_* alongside.
    #[arg(short = 'w', long)]
    pub overwrite: bool,

    /// Comma-separated extensions (default: png,jpg,jpeg,webp).
    #[arg(short, long)]
    pub ext: Option<String>,

    /// Skip files smaller than this (e.g. 10k, 1m) [default: 10k].
    #[arg(long)]
    pub min_size: Option<String>,

    /// Do not skip files prefixed with "compressed_".
    #[arg(long)]
    pub no_skip_compressed: bool,

    /// Parallel compression workers [default: 4].
    #[arg(short, long)]
    pub concurrency: Option<usize>,

    /// Scan and report, don't compress.
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// NDJSON machine-readable output.
    #[arg(long)]
    pub json: bool,

    /// Suppress progress; print only summary.
    #[arg(short, long)]
    pub quiet: bool,

    /// Verbose logging to stderr.
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage TinyPNG API keys.
    Keys {
        #[command(subcommand)]
        action: KeysAction,
    },
    /// Manage local configuration.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum KeysAction {
    /// Show configured key sources and counts (hashes only).
    List {
        #[arg(long)]
        json: bool,
    },
    /// Probe each key for health and quota.
    Test {
        #[arg(long)]
        json: bool,
        #[arg(short, long, default_value_t = 4)]
        concurrency: usize,
    },
    /// Append a key to the config file.
    Add { key: String },
    /// Remove a key by value or 8-char hash prefix.
    Remove { identifier: String },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Show current config.
    Get { key: Option<String> },
    /// Set a config value.
    Set { key: String, value: String },
    /// Print config file path.
    Path,
    /// Open config in $EDITOR.
    Edit,
}
