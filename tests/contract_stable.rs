use tinypng_cli::error::ShrinkError;

/// The list of stable error codes. CHANGING THIS LIST IS A BREAKING CHANGE.
/// Skill files and AI prompts reference these exact strings.
const STABLE_ERROR_CODES: &[&str] = &[
    "invalid_image",
    "unsupported_format",
    "file_too_large",
    "key_exhausted",
    "key_invalid",
    "no_keys_configured",
    "all_keys_exhausted",
    "network_timeout",
    "server_error",
    "io_error",
    "permission_denied",
    "max_retries_exceeded",
    "config_parse",
    "bad_argument",
    "protocol_error",
];

#[test]
fn every_stable_code_is_reachable() {
    let cases: Vec<ShrinkError> = vec![
        ShrinkError::InvalidImage("x".into()),
        ShrinkError::UnsupportedFormat("x".into()),
        ShrinkError::FileTooLarge { size: 1 },
        ShrinkError::KeyExhausted,
        ShrinkError::KeyInvalid,
        ShrinkError::NoKeysConfigured,
        ShrinkError::AllKeysExhausted,
        ShrinkError::NetworkTimeout,
        ShrinkError::ServerError(500),
        ShrinkError::IoError("x".into()),
        ShrinkError::PermissionDenied("x".into()),
        ShrinkError::MaxRetriesExceeded,
        ShrinkError::ConfigParse("x".into()),
        ShrinkError::BadArgument("x".into()),
        ShrinkError::ProtocolError,
    ];
    let codes: Vec<&str> = cases.iter().map(|e| e.code()).collect();
    assert_eq!(codes.len(), STABLE_ERROR_CODES.len());
    for expected in STABLE_ERROR_CODES {
        assert!(codes.contains(expected), "missing stable code: {expected}");
    }
}

#[test]
fn ndjson_event_type_field_names_are_frozen() {
    use chrono::Utc;
    use std::path::PathBuf;
    use tinypng_cli::output::{Event, FileEvent};
    use tinypng_cli::scan::SkipReason;

    let events = [
        Event::Start {
            version: "x".into(),
            build: "public",
            paths: vec![],
            concurrency: 1,
            dry_run: false,
            overwrite: false,
            output_dir: None,
            total_files: 0,
            total_bytes: 0,
            ts: Utc::now(),
        },
        Event::File(FileEvent::Success {
            uuid: "u".into(),
            path: PathBuf::from("a"),
            original_size: 1,
            compressed_size: 1,
            ratio: 1.0,
            saved_bytes: 0,
            output_path: PathBuf::from("a"),
            key_hash: "aa11bb22".into(),
            duration_ms: 0,
            ts: Utc::now(),
        }),
        Event::File(FileEvent::Skipped {
            uuid: "u".into(),
            path: PathBuf::from("a"),
            original_size: 1,
            reason: SkipReason::BelowMinSize,
            ts: Utc::now(),
        }),
        Event::Summary {
            total: 0,
            success: 0,
            fail: 0,
            skipped: 0,
            original_total_bytes: 0,
            compressed_total_bytes: 0,
            saved_bytes: 0,
            saved_percent: 0.0,
            duration_ms: 0,
            keys_used: 0,
            keys_exhausted: 0,
            exit_code: 0,
            dry_run: false,
            ts: Utc::now(),
        },
    ];
    let expected_types = ["start", "file", "file", "summary"];
    for (e, expected) in events.iter().zip(expected_types) {
        let v: serde_json::Value = serde_json::to_value(e).unwrap();
        assert_eq!(v["type"], expected);
    }
}
