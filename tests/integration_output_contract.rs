use chrono::Utc;
use std::path::PathBuf;
use tinypng_cli::output::json::JsonSink;
use tinypng_cli::output::{Event, FileEvent, OutputSink};
use tinypng_cli::scan::SkipReason;

fn parse_line(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("valid json")
}

#[test]
fn emits_valid_ndjson_per_event() {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut sink = JsonSink::new(&mut buf);

        sink.emit(&Event::Start {
            version: "0.1.0".into(),
            build: "public",
            paths: vec![PathBuf::from("./x")],
            concurrency: 4,
            dry_run: false,
            overwrite: false,
            output_dir: None,
            total_files: 2,
            total_bytes: 200,
            ts: Utc::now(),
        });

        sink.emit(&Event::File(FileEvent::Success {
            uuid: "u1".into(),
            path: PathBuf::from("./a.png"),
            original_size: 100,
            compressed_size: 40,
            ratio: 0.4,
            saved_bytes: 60,
            output_path: PathBuf::from("./compressed_a.png"),
            key_hash: "aa11bb22".into(),
            duration_ms: 123,
            ts: Utc::now(),
        }));

        sink.emit(&Event::File(FileEvent::Skipped {
            uuid: "u2".into(),
            path: PathBuf::from("./b.png"),
            original_size: 500,
            reason: SkipReason::BelowMinSize,
            ts: Utc::now(),
        }));

        sink.emit(&Event::Summary {
            total: 2,
            success: 1,
            fail: 0,
            skipped: 1,
            original_total_bytes: 100,
            compressed_total_bytes: 40,
            saved_bytes: 60,
            saved_percent: 60.0,
            duration_ms: 1000,
            keys_used: 1,
            keys_exhausted: 0,
            exit_code: 0,
            dry_run: false,
            ts: Utc::now(),
        });

        sink.finish();
    }

    let text = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4);

    let start = parse_line(lines[0]);
    assert_eq!(start["type"], "start");
    assert_eq!(start["total_files"], 2);

    let file_ok = parse_line(lines[1]);
    assert_eq!(file_ok["type"], "file");
    assert_eq!(file_ok["status"], "success");
    assert_eq!(file_ok["key_hash"], "aa11bb22");

    let file_skip = parse_line(lines[2]);
    assert_eq!(file_skip["type"], "file");
    assert_eq!(file_skip["status"], "skipped");
    assert_eq!(file_skip["reason"], "below_min_size");

    let summary = parse_line(lines[3]);
    assert_eq!(summary["type"], "summary");
    assert_eq!(summary["exit_code"], 0);
    assert_eq!(summary["saved_percent"], 60.0);
}

#[test]
fn fail_event_has_error_code() {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut sink = JsonSink::new(&mut buf);
        sink.emit(&Event::File(FileEvent::Fail {
            uuid: "u".into(),
            path: PathBuf::from("./bad.png"),
            original_size: 1000,
            error: "invalid_image".into(),
            error_message: "corrupt header".into(),
            attempted_keys: vec!["aa11bb22".into()],
            duration_ms: 50,
            ts: Utc::now(),
        }));
    }
    let text = String::from_utf8(buf).unwrap();
    let v = parse_line(text.lines().next().unwrap());
    assert_eq!(v["status"], "fail");
    assert_eq!(v["error"], "invalid_image");
    assert_eq!(v["attempted_keys"][0], "aa11bb22");
}
