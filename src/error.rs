use serde::Serialize;
use std::io;
use thiserror::Error;

/// Stable error codes exposed via `--json` output.
/// CHANGING ANY STRING HERE IS A BREAKING CHANGE (see contract_stable.rs).
#[derive(Debug, Error)]
pub enum ShrinkError {
    #[error("invalid image: {0}")]
    InvalidImage(String),

    #[error("unsupported image format: {0}")]
    UnsupportedFormat(String),

    #[error("file too large: {size} bytes (TinyPNG limit is 5 MB)")]
    FileTooLarge { size: u64 },

    #[error("TinyPNG API key exhausted")]
    KeyExhausted,

    #[error("TinyPNG API key invalid")]
    KeyInvalid,

    #[error("no TinyPNG API keys configured")]
    NoKeysConfigured,

    #[error("all configured keys are exhausted or invalid")]
    AllKeysExhausted,

    #[error("network timeout")]
    NetworkTimeout,

    #[error("TinyPNG server error (HTTP {0})")]
    ServerError(u16),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("maximum retries exceeded")]
    MaxRetriesExceeded,

    #[error("configuration error: {0}")]
    ConfigParse(String),

    #[error("argument error: {0}")]
    BadArgument(String),

    #[error("protocol error (unexpected TinyPNG response)")]
    ProtocolError,
}

impl ShrinkError {
    /// Stable snake_case code used in NDJSON `error` field.
    /// This is a PUBLIC API. Changes require a major version bump.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidImage(_) => "invalid_image",
            Self::UnsupportedFormat(_) => "unsupported_format",
            Self::FileTooLarge { .. } => "file_too_large",
            Self::KeyExhausted => "key_exhausted",
            Self::KeyInvalid => "key_invalid",
            Self::NoKeysConfigured => "no_keys_configured",
            Self::AllKeysExhausted => "all_keys_exhausted",
            Self::NetworkTimeout => "network_timeout",
            Self::ServerError(_) => "server_error",
            Self::IoError(_) => "io_error",
            Self::PermissionDenied(_) => "permission_denied",
            Self::MaxRetriesExceeded => "max_retries_exceeded",
            Self::ConfigParse(_) => "config_parse",
            Self::BadArgument(_) => "bad_argument",
            Self::ProtocolError => "protocol_error",
        }
    }

    /// Whether the error is retryable with the same key.
    pub fn is_transient(&self) -> bool {
        matches!(self, Self::NetworkTimeout | Self::ServerError(_))
    }

    /// Whether the error indicates the current key is dead (should rotate).
    pub fn is_key_dead(&self) -> bool {
        matches!(self, Self::KeyExhausted | Self::KeyInvalid)
    }
}

impl From<io::Error> for ShrinkError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::PermissionDenied {
            Self::PermissionDenied(e.to_string())
        } else {
            Self::IoError(e.to_string())
        }
    }
}

impl From<reqwest::Error> for ShrinkError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::NetworkTimeout
        } else {
            Self::IoError(e.to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerializedError {
    pub code: String,
    pub message: String,
}

impl From<&ShrinkError> for SerializedError {
    fn from(e: &ShrinkError) -> Self {
        Self {
            code: e.code().to_string(),
            message: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable_snake_case() {
        let cases = [
            (ShrinkError::InvalidImage("x".into()), "invalid_image"),
            (
                ShrinkError::UnsupportedFormat("gif".into()),
                "unsupported_format",
            ),
            (ShrinkError::FileTooLarge { size: 1 }, "file_too_large"),
            (ShrinkError::KeyExhausted, "key_exhausted"),
            (ShrinkError::KeyInvalid, "key_invalid"),
            (ShrinkError::NoKeysConfigured, "no_keys_configured"),
            (ShrinkError::AllKeysExhausted, "all_keys_exhausted"),
            (ShrinkError::NetworkTimeout, "network_timeout"),
            (ShrinkError::ServerError(500), "server_error"),
            (ShrinkError::IoError("x".into()), "io_error"),
            (
                ShrinkError::PermissionDenied("x".into()),
                "permission_denied",
            ),
            (ShrinkError::MaxRetriesExceeded, "max_retries_exceeded"),
            (ShrinkError::ConfigParse("x".into()), "config_parse"),
            (ShrinkError::BadArgument("x".into()), "bad_argument"),
            (ShrinkError::ProtocolError, "protocol_error"),
        ];
        for (err, code) in cases {
            assert_eq!(err.code(), code);
        }
    }

    #[test]
    fn transient_classification() {
        assert!(ShrinkError::NetworkTimeout.is_transient());
        assert!(ShrinkError::ServerError(503).is_transient());
        assert!(!ShrinkError::KeyExhausted.is_transient());
        assert!(!ShrinkError::InvalidImage("x".into()).is_transient());
    }

    #[test]
    fn key_dead_classification() {
        assert!(ShrinkError::KeyExhausted.is_key_dead());
        assert!(ShrinkError::KeyInvalid.is_key_dead());
        assert!(!ShrinkError::NetworkTimeout.is_key_dead());
    }
}
