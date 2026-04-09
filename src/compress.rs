use crate::error::ShrinkError;
use crate::keys::Key;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub enum CompressOutcome {
    Ok(Vec<u8>),
}

pub const DEFAULT_API_BASE: &str = "https://api.tinify.com";

pub fn api_base() -> String {
    std::env::var("TINIFY_API_BASE").unwrap_or_else(|_| DEFAULT_API_BASE.to_string())
}

static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(30))
        .user_agent(concat!("tinypng-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build reqwest client")
});

pub async fn call_tinypng(file: &Path, key: &Key) -> Result<CompressOutcome, ShrinkError> {
    let base = api_base();
    call_tinypng_with_base(file, key, &base).await
}

pub async fn call_tinypng_with_base(
    file: &Path,
    key: &Key,
    base_url: &str,
) -> Result<CompressOutcome, ShrinkError> {
    let bytes = tokio::fs::read(file).await?;
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(ShrinkError::FileTooLarge {
            size: bytes.len() as u64,
        });
    }

    // POST /shrink
    let url = format!("{}/shrink", base_url.trim_end_matches('/'));
    let resp = HTTP_CLIENT
        .post(&url)
        .basic_auth("api", Some(&key.value))
        .body(bytes)
        .send()
        .await?;

    let status = resp.status().as_u16();
    match status {
        201 => {
            let location = resp
                .headers()
                .get("location")
                .ok_or(ShrinkError::ProtocolError)?
                .to_str()
                .map_err(|_| ShrinkError::ProtocolError)?
                .to_string();

            let download = HTTP_CLIENT
                .get(&location)
                .basic_auth("api", Some(&key.value))
                .send()
                .await?;
            if !download.status().is_success() {
                return Err(ShrinkError::ProtocolError);
            }
            let body = download.bytes().await?.to_vec();
            Ok(CompressOutcome::Ok(body))
        }
        400 => Err(ShrinkError::InvalidImage("HTTP 400".into())),
        401 => Err(ShrinkError::KeyInvalid),
        415 => Err(ShrinkError::UnsupportedFormat("HTTP 415".into())),
        429 => Err(ShrinkError::KeyExhausted),
        500..=599 => Err(ShrinkError::ServerError(status)),
        _ => Err(ShrinkError::ProtocolError),
    }
}
