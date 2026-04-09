use std::path::PathBuf;
use tempfile::TempDir;
use tinypng_cli::compress::{call_tinypng_with_base, CompressOutcome};
use tinypng_cli::keys::{Key, KeySource};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn mk_key(v: &str) -> Key {
    Key {
        value: v.to_string(),
        source: KeySource::Env,
        index: 0,
        hash: tinypng_cli::keys::hash_key(v),
    }
}

fn write_png(dir: &std::path::Path) -> PathBuf {
    let p = dir.join("t.png");
    let mut bytes = vec![0x89, 0x50, 0x4E, 0x47];
    bytes.resize(1024, 0);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[tokio::test]
async fn success_flow_201_then_get() {
    let server = MockServer::start().await;
    let location = format!("{}/output/xyz", server.uri());

    Mock::given(method("POST"))
        .and(path("/shrink"))
        .respond_with(ResponseTemplate::new(201).insert_header("location", location.as_str()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/output/xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1u8, 2, 3, 4]))
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let file = write_png(tmp.path());
    let out = call_tinypng_with_base(&file, &mk_key("k"), &server.uri())
        .await
        .unwrap();
    match out {
        CompressOutcome::Ok(bytes) => assert_eq!(bytes, vec![1, 2, 3, 4]),
    }
}

#[tokio::test]
async fn http_401_maps_to_key_invalid() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/shrink"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let file = write_png(tmp.path());
    let err = call_tinypng_with_base(&file, &mk_key("k"), &server.uri())
        .await
        .unwrap_err();
    assert_eq!(err.code(), "key_invalid");
}

#[tokio::test]
async fn http_429_maps_to_key_exhausted() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/shrink"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let file = write_png(tmp.path());
    let err = call_tinypng_with_base(&file, &mk_key("k"), &server.uri())
        .await
        .unwrap_err();
    assert_eq!(err.code(), "key_exhausted");
}

#[tokio::test]
async fn http_400_maps_to_invalid_image() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/shrink"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let file = write_png(tmp.path());
    let err = call_tinypng_with_base(&file, &mk_key("k"), &server.uri())
        .await
        .unwrap_err();
    assert_eq!(err.code(), "invalid_image");
}

#[tokio::test]
async fn http_503_maps_to_server_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/shrink"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let tmp = TempDir::new().unwrap();
    let file = write_png(tmp.path());
    let err = call_tinypng_with_base(&file, &mk_key("k"), &server.uri())
        .await
        .unwrap_err();
    assert_eq!(err.code(), "server_error");
}
