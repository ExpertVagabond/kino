//! Kino Core - Video Player Library for Kino
//!
//! This crate provides the core functionality for video playback:
//! - HLS manifest parsing and segment management
//! - DASH MPD parsing and adaptation
//! - Adaptive bitrate (ABR) algorithms
//! - Buffer management with prefetching
//! - Analytics event emission
//! - DRM license acquisition (optional)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Kino Core                            │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
//! │  │   Manifest   │  │    Buffer    │  │     ABR      │          │
//! │  │    Parser    │  │   Manager    │  │   Engine     │          │
//! │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
//! │         │                 │                 │                   │
//! │         └─────────────────┼─────────────────┘                   │
//! │                           │                                     │
//! │                    ┌──────┴──────┐                              │
//! │                    │   Player    │                              │
//! │                    │   Session   │                              │
//! │                    └──────┬──────┘                              │
//! │                           │                                     │
//! │  ┌──────────────┐  ┌──────┴──────┐  ┌──────────────┐           │
//! │  │   Analytics  │  │    Event    │  │     DRM      │           │
//! │  │   Emitter    │  │     Bus     │  │   Manager    │           │
//! │  └──────────────┘  └─────────────┘  └──────────────┘           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod error;
pub mod types;
pub mod manifest;
pub mod buffer;
pub mod abr;
pub mod session;
pub mod analytics;
pub mod branding;
pub mod drm;
pub mod captions;

pub use error::{Error, Result};
pub use types::*;
pub use manifest::{ManifestParser, HlsParser, DashParser};
pub use buffer::BufferManager;
pub use abr::{AbrEngine, AbrAlgorithm};
pub use session::PlayerSession;
pub use analytics::{AnalyticsEvent, AnalyticsEmitter};
pub use branding::{KinoColors, KinoTheme, JsTheme, CssVariables};
pub use drm::{DrmConfig, DrmManager, DrmSession, PsshBox};
pub use captions::{WebVttParser, SrtParser};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the player library with default configuration
pub fn init() {
    tracing::info!(version = VERSION, "Kino Core initialized");
}
