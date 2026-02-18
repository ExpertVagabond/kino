//! Error types for Kino Core

use thiserror::Error;

/// Result type alias for player operations
pub type Result<T> = std::result::Result<T, Error>;

/// Player error types
#[derive(Error, Debug)]
pub enum Error {
    // Manifest errors
    #[error("Failed to fetch manifest: {0}")]
    ManifestFetch(String),

    #[error("Failed to parse manifest: {0}")]
    ManifestParse(String),

    #[error("Invalid manifest format: {0}")]
    InvalidManifest(String),

    #[error("No suitable rendition found")]
    NoSuitableRendition,

    // Segment errors
    #[error("Failed to fetch segment: {url}")]
    SegmentFetch { url: String, source: reqwest::Error },

    #[error("Segment timeout: {url}")]
    SegmentTimeout { url: String },

    #[error("Segment decryption failed")]
    SegmentDecryption,

    // Buffer errors
    #[error("Buffer underrun")]
    BufferUnderrun,

    #[error("Buffer overflow")]
    BufferOverflow,

    #[error("Buffer seek failed: position {position}s not buffered")]
    BufferSeekFailed { position: f64 },

    // DRM errors
    #[error("DRM not supported: {system}")]
    DrmNotSupported { system: String },

    #[error("License acquisition failed: {0}")]
    LicenseAcquisition(String),

    #[error("License expired")]
    LicenseExpired,

    #[error("Content key not found")]
    ContentKeyNotFound,

    // Playback errors
    #[error("Playback stalled")]
    PlaybackStalled,

    #[error("Invalid playback state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Codec not supported: {codec}")]
    CodecNotSupported { codec: String },

    // Network errors
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Connection timeout")]
    ConnectionTimeout,

    // Configuration errors
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    // Internal errors
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Create a DRM error
    pub fn drm(msg: impl Into<String>) -> Self {
        Error::LicenseAcquisition(msg.into())
    }

    /// Returns true if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Error::SegmentFetch { .. }
                | Error::SegmentTimeout { .. }
                | Error::BufferUnderrun
                | Error::Network(_)
                | Error::ConnectionTimeout
        )
    }

    /// Returns the error code for analytics
    pub fn error_code(&self) -> &'static str {
        match self {
            Error::ManifestFetch(_) => "MANIFEST_FETCH",
            Error::ManifestParse(_) => "MANIFEST_PARSE",
            Error::InvalidManifest(_) => "INVALID_MANIFEST",
            Error::NoSuitableRendition => "NO_RENDITION",
            Error::SegmentFetch { .. } => "SEGMENT_FETCH",
            Error::SegmentTimeout { .. } => "SEGMENT_TIMEOUT",
            Error::SegmentDecryption => "SEGMENT_DECRYPT",
            Error::BufferUnderrun => "BUFFER_UNDERRUN",
            Error::BufferOverflow => "BUFFER_OVERFLOW",
            Error::BufferSeekFailed { .. } => "BUFFER_SEEK",
            Error::DrmNotSupported { .. } => "DRM_UNSUPPORTED",
            Error::LicenseAcquisition(_) => "LICENSE_ACQUIRE",
            Error::LicenseExpired => "LICENSE_EXPIRED",
            Error::ContentKeyNotFound => "KEY_NOT_FOUND",
            Error::PlaybackStalled => "PLAYBACK_STALLED",
            Error::InvalidStateTransition { .. } => "INVALID_STATE",
            Error::CodecNotSupported { .. } => "CODEC_UNSUPPORTED",
            Error::Network(_) => "NETWORK",
            Error::ConnectionTimeout => "TIMEOUT",
            Error::InvalidConfig(_) => "INVALID_CONFIG",
            Error::Internal(_) => "INTERNAL",
            Error::Io(_) => "IO",
        }
    }
}
