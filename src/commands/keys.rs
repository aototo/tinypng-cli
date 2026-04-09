use futures::stream::{self, StreamExt};
use serde_json::json;
use tinypng_cli::cli::KeysAction;
use tinypng_cli::config::Config;
use tinypng_cli::error::ShrinkError;
use tinypng_cli::keys::{hash_key, load_all_keys, KeySource};

pub async fn execute(action: &KeysAction) -> Result<i32, ShrinkError> {
    match action {
        KeysAction::List { json: as_json } => list(*as_json),
        KeysAction::Test {
            json: as_json,
            concurrency,
        } => test(*as_json, *concurrency).await,
        KeysAction::Add { key } => add(key),
        KeysAction::Remove { identifier } => remove(identifier),
    }
}

fn list(as_json: bool) -> Result<i32, ShrinkError> {
    let config = Config::load()?;
    let keys = load_all_keys(&config).unwrap_or_default();
    let mut env_count = 0;
    let mut config_count = 0;
    for k in &keys {
        match k.source {
            KeySource::Env => env_count += 1,
            KeySource::Config => config_count += 1,
        }
    }
    if as_json {
        let v = json!({
            "sources": [
                {"name":"env","count":env_count,"active":env_count},
                {"name":"config","count":config_count,"active":config_count}
            ],
            "total": keys.len(),
            "priority": ["env","config"]
        });
        println!("{v}");
    } else {
        println!("key pool:");
        println!("  source              count");
        println!("  env (TINIFY_KEY*)   {env_count}");
        println!("  config file         {config_count}");
        println!("  ───────────────────────");
        println!("  total               {}", keys.len());
        println!();
        println!("priority: env > config");
    }
    Ok(if keys.is_empty() { 3 } else { 0 })
}

async fn test(as_json: bool, concurrency: usize) -> Result<i32, ShrinkError> {
    let config = Config::load()?;
    let keys = load_all_keys(&config)?;
    let base = tinypng_cli::compress::api_base();

    let total = keys.len();
    let results: Vec<serde_json::Value> = stream::iter(keys)
        .map(|k| {
            let base = base.clone();
            async move { probe(&k, &base).await }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    if as_json {
        for r in &results {
            println!("{r}");
        }
    } else {
        for r in &results {
            let source = r["source"].as_str().unwrap_or("?");
            let hash = r["hash"].as_str().unwrap_or("");
            let status = r["status"].as_str().unwrap_or("?");
            let remaining = r["remaining"].as_u64();
            match remaining {
                Some(n) => println!("  {source:<8} {hash}  {status:<10} {n} remaining"),
                None => println!("  {source:<8} {hash}  {status}"),
            }
        }
    }

    let alive = results.iter().filter(|r| r["status"] == "ok").count();
    let exhausted = results
        .iter()
        .filter(|r| r["status"] == "exhausted")
        .count();
    let invalid = results.iter().filter(|r| r["status"] == "invalid").count();

    if as_json {
        let summary = json!({
            "type":"summary",
            "alive": alive,
            "exhausted": exhausted,
            "invalid": invalid,
            "total": total,
        });
        println!("{summary}");
    } else {
        println!("summary: {alive} alive / {exhausted} exhausted / {invalid} invalid");
    }

    Ok(if alive == 0 { 4 } else { 0 })
}

async fn probe(key: &tinypng_cli::keys::Key, base: &str) -> serde_json::Value {
    let url = format!("{}/shrink", base.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .basic_auth("api", Some(&key.value))
        .body(Vec::<u8>::new())
        .send()
        .await;
    let source = match key.source {
        KeySource::Env => "env",
        KeySource::Config => "config",
    };
    match resp {
        Ok(r) => {
            let used: Option<u32> = r
                .headers()
                .get("compression-count")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());
            let status_str = match r.status().as_u16() {
                400 => "ok",
                401 => "invalid",
                429 => "exhausted",
                _ => "unknown",
            };
            json!({
                "type": "key",
                "source": source,
                "index": key.index,
                "hash": key.hash,
                "status": status_str,
                "remaining": used.map(|u| 500u32.saturating_sub(u)),
            })
        }
        Err(_) => json!({
            "type": "key",
            "source": source,
            "index": key.index,
            "hash": key.hash,
            "status": "network_error",
        }),
    }
}

fn add(key: &str) -> Result<i32, ShrinkError> {
    let mut config = Config::load()?;
    let looks_valid = key.len() == 32 && key.chars().all(|c| c.is_ascii_alphanumeric());
    if !looks_valid {
        eprintln!(
            "warning: '{}' does not look like a TinyPNG key (expected 32 alphanumeric chars). Adding anyway.",
            &key[..key.len().min(8)]
        );
    }
    if config.keys.values.iter().any(|k| k == key) {
        eprintln!("already present");
        return Ok(0);
    }
    config.keys.values.push(key.to_string());
    config.save()?;
    println!("added. config now has {} key(s)", config.keys.values.len());
    Ok(0)
}

fn remove(identifier: &str) -> Result<i32, ShrinkError> {
    let mut config = Config::load()?;
    let before = config.keys.values.len();
    config.keys.values.retain(|k| {
        if k == identifier {
            return false;
        }
        let h = hash_key(k);
        !(identifier.len() >= 4 && h.starts_with(identifier))
    });
    config.save()?;
    let removed = before - config.keys.values.len();
    println!("removed {removed} key(s)");
    Ok(if removed == 0 { 1 } else { 0 })
}
