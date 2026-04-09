use tinypng_cli::cli::ConfigAction;
use tinypng_cli::config::Config;
use tinypng_cli::error::ShrinkError;

pub fn execute(action: &ConfigAction) -> Result<i32, ShrinkError> {
    match action {
        ConfigAction::Get { key } => get(key.as_deref()),
        ConfigAction::Set { key, value } => set(key, value),
        ConfigAction::Path => {
            println!("{}", Config::path().display());
            Ok(0)
        }
        ConfigAction::Edit => edit(),
    }
}

fn get(key: Option<&str>) -> Result<i32, ShrinkError> {
    let config = Config::load()?;
    match key {
        None => {
            let s = toml::to_string_pretty(&config)
                .map_err(|e| ShrinkError::ConfigParse(e.to_string()))?;
            println!("{s}");
        }
        Some(k) => match k {
            "concurrency" => println!("{:?}", config.concurrency),
            "overwrite" => println!("{:?}", config.overwrite),
            "min_size" => println!("{:?}", config.min_size),
            "extensions" => println!("{:?}", config.extensions),
            "skip_compressed" => println!("{:?}", config.skip_compressed),
            _ => return Err(ShrinkError::BadArgument(format!("unknown key: {k}"))),
        },
    }
    Ok(0)
}

fn set(key: &str, value: &str) -> Result<i32, ShrinkError> {
    let mut config = Config::load()?;
    match key {
        "concurrency" => {
            config.concurrency = Some(
                value
                    .parse()
                    .map_err(|_| ShrinkError::BadArgument("concurrency must be integer".into()))?,
            )
        }
        "overwrite" => {
            config.overwrite = Some(
                value
                    .parse()
                    .map_err(|_| ShrinkError::BadArgument("overwrite must be bool".into()))?,
            )
        }
        "min_size" => config.min_size = Some(value.to_string()),
        "skip_compressed" => {
            config.skip_compressed = Some(
                value
                    .parse()
                    .map_err(|_| ShrinkError::BadArgument("skip_compressed must be bool".into()))?,
            )
        }
        "extensions" => {
            config.extensions = Some(value.split(',').map(|s| s.trim().to_lowercase()).collect());
        }
        _ => return Err(ShrinkError::BadArgument(format!("unknown key: {key}"))),
    }
    config.save()?;
    println!("ok");
    Ok(0)
}

fn edit() -> Result<i32, ShrinkError> {
    let path = Config::path();
    if !path.exists() {
        Config::default().save()?;
    }
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|e| ShrinkError::IoError(format!("launch editor: {e}")))?;
    Ok(if status.success() { 0 } else { 1 })
}
