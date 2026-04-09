use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0x99, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x01, 0x5B, 0xFA, 0x04, 0x4B, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn cmd(home: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("tinypng").unwrap();
    c.env_remove("TINIFY_KEY");
    c.env_remove("TINIFY_KEYS");
    c.env("HOME", home);
    c
}

fn isolated_home() -> TempDir {
    TempDir::new().unwrap()
}

#[test]
fn help_exits_zero() {
    let h = isolated_home();
    cmd(h.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("tinypng"));
}

#[test]
fn no_keys_exits_3() {
    let h = isolated_home();
    let target = TempDir::new().unwrap();
    cmd(h.path())
        .arg(target.path())
        .assert()
        .code(3)
        .stderr(predicate::str::contains("No TinyPNG API keys configured"));
}

#[test]
fn bad_path_exits_2() {
    let h = isolated_home();
    cmd(h.path())
        .env("TINIFY_KEY", "dummy_key_for_test")
        .arg("/definitely/does/not/exist/12345")
        .assert()
        .code(2);
}

#[test]
fn dry_run_with_no_files_succeeds() {
    let h = isolated_home();
    let tmp = TempDir::new().unwrap();
    cmd(h.path())
        .env("TINIFY_KEY", "dummy_key_for_test")
        .args(["--dry-run", "--json"])
        .arg(tmp.path())
        .assert()
        .success();
}

#[test]
fn dry_run_emits_ndjson_summary() {
    let h = isolated_home();
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("a.png");
    let mut bytes = PNG_1X1.to_vec();
    bytes.resize(20_000, 0);
    fs::write(&p, bytes).unwrap();

    let output = cmd(h.path())
        .env("TINIFY_KEY", "dummy_key_for_test")
        .args(["--dry-run", "--json"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let last = stdout.lines().last().unwrap();
    let v: serde_json::Value = serde_json::from_str(last).unwrap();
    assert_eq!(v["type"], "summary");
    assert_eq!(v["dry_run"], true);
}

#[test]
fn keys_list_shows_env_key() {
    let h = isolated_home();
    cmd(h.path())
        .env("TINIFY_KEY", "some_test_key")
        .args(["keys", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("env"));
}
