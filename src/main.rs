use clap::Parser;
use tinypng_cli::cli::{Cli, Command};
use tinypng_cli::error::ShrinkError;

mod commands;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let code = match dispatch(cli).await {
        Ok(c) => c,
        Err(e) => exit_code_for_error(&e),
    };
    std::process::exit(code);
}

async fn dispatch(cli: Cli) -> Result<i32, ShrinkError> {
    match cli.command {
        Some(Command::Keys { action }) => commands::keys::execute(&action).await,
        Some(Command::Config { action }) => commands::config::execute(&action),
        None => commands::run::execute(&cli.run, &cli.paths).await,
    }
}

fn exit_code_for_error(e: &ShrinkError) -> i32 {
    match e {
        ShrinkError::BadArgument(msg) => {
            eprintln!("error: {msg}");
            2
        }
        ShrinkError::NoKeysConfigured | ShrinkError::ConfigParse(_) => {
            eprintln!();
            eprintln!("✗ No TinyPNG API keys configured.");
            eprintln!();
            eprintln!("To use tinypng, you need a free TinyPNG API key:");
            eprintln!("  1. Get one at https://tinypng.com/developers");
            eprintln!("  2. Run: tinypng keys add <your-key>");
            eprintln!("     or set: export TINIFY_KEY=<your-key>");
            eprintln!("     or edit: ~/.config/tinypng/config.toml");
            eprintln!();
            eprintln!("Run 'tinypng keys --help' for more options.");
            3
        }
        ShrinkError::AllKeysExhausted
        | ShrinkError::NetworkTimeout
        | ShrinkError::ServerError(_) => {
            eprintln!("error: {e}");
            4
        }
        _ => {
            eprintln!("error: {e}");
            1
        }
    }
}
