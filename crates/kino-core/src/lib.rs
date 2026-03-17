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

pub mod abr;
pub mod analytics;
pub mod branding;
pub mod buffer;
pub mod captions;
pub mod drm;
pub mod error;
pub mod manifest;
pub mod session;
pub mod types;

pub use abr::{AbrAlgorithm, AbrEngine};
pub use analytics::{AnalyticsEmitter, AnalyticsEvent};
pub use branding::{CssVariables, JsTheme, KinoColors, KinoTheme};
pub use buffer::BufferManager;
pub use captions::{SrtParser, WebVttParser};
pub use drm::{DrmConfig, DrmManager, DrmSession, PsshBox};
pub use error::{Error, Result};
pub use manifest::{DashParser, HlsParser, ManifestParser};
pub use session::PlayerSession;
pub use types::*;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the player library with default configuration
pub fn init() {
    tracing::info!(version = VERSION, "Kino Core initialized");
}
