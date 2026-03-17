//! Kino Desktop - Native GStreamer Video Player
//!
//! Native desktop video player with:
//! - Hardware-accelerated decoding (VA-API, VideoToolbox, NVDEC)
//! - HLS/DASH adaptive streaming
//! - DRM support via Widevine CDM
//! - Low-latency playback
//!
//! # Example
//!
//! ```rust,no_run
//! use kino_desktop::{DesktopPlayer, DesktopPlayerConfig};
//!
//! let config = DesktopPlayerConfig::default();
//! let mut player = DesktopPlayer::new(config).unwrap();
//! player.load("https://example.com/stream.m3u8").unwrap();
//! player.play();
//! ```

pub mod controls;
pub mod player;
pub mod window;

pub use player::{
    check_gstreamer_installation, DesktopPlayer, DesktopPlayerConfig, GStreamerInfo,
    HardwareBackend,
};
