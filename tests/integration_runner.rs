use std::sync::Arc;
use tempfile::TempDir;
use tinypng_cli::config::RunConfig;
use tinypng_cli::keys::{rotation::KeyPool, Key, KeySource};
use tinypng_cli::runner::{run, CompressResult, RunnerInput};
use tinypng_cli::scan::FileTask;
use wiremock::matchers::{basic_auth, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn mk_pool(vals: &[&str]) -> Arc<KeyPool> {
    let keys: Vec<Key> = vals
        .iter()
        .enumerate()
        .map(|(i, v)| Key {
            value: v.to_string(),
            source: KeySource::Env,
            index: i,
            hash: tinypng_cli::keys::hash_key(v),
        })
        .collect();
    Arc::new(KeyPool::new(keys))
}

fn write_task(dir: &std::path::Path, name: &str, size: usize) -> FileTask {
    let p = dir.join(name);
    std::fs::write(&p, vec![0u8; size]).unwrap();
    FileTask {
        uuid: uuid::Uuid::new_v4().to_string(),
        path: p,
        original_size: size as u64,
    }
}

/// Serialize tests that mutate TINIFY_API_BASE.
static API_BASE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn exhausted_key_switches_to_next_key() {
    let _guard = API_BASE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let server = MockServer::start().await;
    let location = format!("{}/output/ok", server.uri());

    Mock::given(method("POST"))
        .and(path("/shrink"))
        .and(basic_auth("api", "a"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/shrink"))
        .and(basic_auth("api", "b"))
        .respond_with(ResponseTemplate::new(201).insert_header("location", location.as_str()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/output/ok"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![9u8; 100]))
        .mount(&server)
        .await;

    unsafe { std::env::set_var("TINIFY_API_BASE", server.uri()) };

    let tmp = TempDir::new().unwrap();
    let task = write_task(tmp.path(), "a.png", 1000);
    let pool = mk_pool(&["a", "b"]);
    let cfg = RunConfig::default();

    let input = RunnerInput {
        tasks: vec![task],
        pool: pool.clone(),
        cfg: Arc::new(cfg),
    };
    let (results, summary) = run(input).await;

    assert_eq!(summary.success, 1);
    assert_eq!(summary.fail, 0);
    match &results[0] {
        CompressResult::Success { key_hash, .. } => {
            assert_eq!(key_hash, &tinypng_cli::keys::hash_key("b"));
        }
        _ => panic!("expected success"),
    }
    let snap = pool.snapshot();
    assert_eq!(snap.exhausted, 1);
    assert_eq!(snap.healthy, 1);

    unsafe { std::env::remove_var("TINIFY_API_BASE") };
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn invalid_image_does_not_rotate_key() {
    let _guard = API_BASE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/shrink"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;

    unsafe { std::env::set_var("TINIFY_API_BASE", server.uri()) };

    let tmp = TempDir::new().unwrap();
    let task = write_task(tmp.path(), "bad.png", 1000);
    let pool = mk_pool(&["a", "b", "c"]);
    let cfg = RunConfig::default();

    let input = RunnerInput {
        tasks: vec![task],
        pool: pool.clone(),
        cfg: Arc::new(cfg),
    };
    let (results, summary) = run(input).await;

    assert_eq!(summary.fail, 1);
    match &results[0] {
        CompressResult::Failure {
            error,
            attempted_keys,
            ..
        } => {
            assert_eq!(error.code, "invalid_image");
            assert_eq!(attempted_keys.len(), 1);
        }
        _ => panic!("expected failure"),
    }
    let snap = pool.snapshot();
    assert_eq!(snap.healthy, 3);

    unsafe { std::env::remove_var("TINIFY_API_BASE") };
}
