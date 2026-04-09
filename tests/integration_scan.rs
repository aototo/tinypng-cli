use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tinypng_cli::config::RunConfig;
use tinypng_cli::scan::scan_paths;

/// Minimum valid 1x1 PNG, 67 bytes.
const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0x99, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x01, 0x5B, 0xFA, 0x04, 0x4B, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn write_fixture(dir: &std::path::Path, name: &str, size: usize) -> PathBuf {
    let p = dir.join(name);
    let mut bytes = PNG_1X1.to_vec();
    bytes.resize(size.max(PNG_1X1.len()), 0); // pad to requested size
    fs::write(&p, &bytes).unwrap();
    p
}

fn names(tasks: &[tinypng_cli::scan::FileTask]) -> HashSet<String> {
    tasks
        .iter()
        .map(|t| t.path.file_name().unwrap().to_str().unwrap().to_string())
        .collect()
}

#[test]
fn scan_filters_by_min_size() {
    let tmp = TempDir::new().unwrap();
    write_fixture(tmp.path(), "tiny.png", 100);
    write_fixture(tmp.path(), "big.png", 20_000);

    let cfg = RunConfig {
        min_size: 10 * 1024,
        ..RunConfig::default()
    };

    let tasks = scan_paths(&[tmp.path().to_path_buf()], &cfg).unwrap();
    let n = names(&tasks);
    assert!(n.contains("big.png"));
    assert!(!n.contains("tiny.png"));
}

#[test]
fn scan_filters_by_extension() {
    let tmp = TempDir::new().unwrap();
    write_fixture(tmp.path(), "a.png", 20_000);
    write_fixture(tmp.path(), "b.gif", 20_000);

    let cfg = RunConfig::default();
    let tasks = scan_paths(&[tmp.path().to_path_buf()], &cfg).unwrap();
    let n = names(&tasks);
    assert!(n.contains("a.png"));
    assert!(!n.contains("b.gif"));
}

#[test]
fn scan_skips_compressed_prefix() {
    let tmp = TempDir::new().unwrap();
    write_fixture(tmp.path(), "compressed_foo.png", 20_000);
    write_fixture(tmp.path(), "foo.png", 20_000);

    let cfg = RunConfig::default();
    let tasks = scan_paths(&[tmp.path().to_path_buf()], &cfg).unwrap();
    let n = names(&tasks);
    assert!(n.contains("foo.png"));
    assert!(!n.contains("compressed_foo.png"));
}

#[test]
fn scan_recurses_subdirectories() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("sub")).unwrap();
    write_fixture(&tmp.path().join("sub"), "nested.png", 20_000);

    let cfg = RunConfig::default();
    let tasks = scan_paths(&[tmp.path().to_path_buf()], &cfg).unwrap();
    assert_eq!(tasks.len(), 1);
    assert!(tasks[0].path.ends_with("sub/nested.png"));
}

#[test]
fn scan_accepts_file_path_directly() {
    let tmp = TempDir::new().unwrap();
    let file = write_fixture(tmp.path(), "a.png", 20_000);
    let cfg = RunConfig::default();
    let tasks = scan_paths(&[file.clone()], &cfg).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].path, file);
}

#[test]
fn scan_nonexistent_path_errors() {
    let cfg = RunConfig::default();
    let err = scan_paths(&[PathBuf::from("/definitely/does/not/exist")], &cfg).unwrap_err();
    assert_eq!(err.code(), "bad_argument");
}
