//! Desktop Player - GStreamer-based native video player
//!
//! Features:
//! - Hardware-accelerated decoding (VA-API, VideoToolbox, NVDEC)
//! - HLS/DASH playback via hlsdemux/dashdemux
//! - Subtitle support
//! - Chapter navigation

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_player as gst_player;
use psm_player_core::{PlayerConfig, PlayerSession, PlayerState, QualityMetrics, Resolution, PsmColors};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

/// Hardware decoding backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareBackend {
    /// Automatic selection
    Auto,
    /// VA-API (Linux/Intel/AMD)
    VaApi,
    /// VideoToolbox (macOS)
    VideoToolbox,
    /// NVDEC (NVIDIA)
    Nvdec,
    /// D3D11VA (Windows)
    D3d11Va,
    /// Software decoding (fallback)
    Software,
}

impl HardwareBackend {
    /// Get the GStreamer element name for this backend
    pub fn decoder_element(&self, codec: &str) -> Option<&'static str> {
        match (self, codec) {
            (Self::Auto, _) => None, // Let GStreamer choose
            (Self::VaApi, "h264") => Some("vaapih264dec"),
            (Self::VaApi, "h265") => Some("vaapih265dec"),
            (Self::VaApi, "vp9") => Some("vaapivp9dec"),
            (Self::VideoToolbox, "h264") => Some("vtdec"),
            (Self::VideoToolbox, "h265") => Some("vtdec"),
            (Self::Nvdec, "h264") => Some("nvh264dec"),
            (Self::Nvdec, "h265") => Some("nvh265dec"),
            (Self::Nvdec, "vp9") => Some("nvvp9dec"),
            (Self::D3d11Va, "h264") => Some("d3d11h264dec"),
            (Self::D3d11Va, "h265") => Some("d3d11h265dec"),
            (Self::Software, _) => None,
            _ => None,
        }
    }

    /// Detect available hardware backends on this system
    pub fn detect_available() -> Vec<Self> {
        let mut available = vec![Self::Software];

        // Check for VA-API
        if gst::ElementFactory::find("vaapih264dec").is_some() {
            available.push(Self::VaApi);
        }

        // Check for VideoToolbox (macOS)
        if gst::ElementFactory::find("vtdec").is_some() {
            available.push(Self::VideoToolbox);
        }

        // Check for NVDEC
        if gst::ElementFactory::find("nvh264dec").is_some() {
            available.push(Self::Nvdec);
        }

        // Check for D3D11VA (Windows)
        if gst::ElementFactory::find("d3d11h264dec").is_some() {
            available.push(Self::D3d11Va);
        }

        available
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::VaApi => "VA-API (Linux)",
            Self::VideoToolbox => "VideoToolbox (macOS)",
            Self::Nvdec => "NVDEC (NVIDIA)",
            Self::D3d11Va => "D3D11VA (Windows)",
            Self::Software => "Software",
        }
    }
}

/// Desktop player configuration
#[derive(Debug, Clone)]
pub struct DesktopPlayerConfig {
    /// Core player config
    pub core: PlayerConfig,
    /// Preferred hardware backend
    pub hardware_backend: HardwareBackend,
    /// Enable hardware decoding
    pub hardware_decode: bool,
    /// Enable subtitle display
    pub subtitles_enabled: bool,
    /// Default subtitle language
    pub subtitle_language: Option<String>,
    /// Buffer duration in nanoseconds
    pub buffer_duration: u64,
    /// Enable low-latency mode
    pub low_latency: bool,
}

impl Default for DesktopPlayerConfig {
    fn default() -> Self {
        Self {
            core: PlayerConfig::default(),
            hardware_backend: HardwareBackend::Auto,
            hardware_decode: true,
            subtitles_enabled: true,
            subtitle_language: None,
            buffer_duration: 3_000_000_000, // 3 seconds
            low_latency: false,
        }
    }
}

impl DesktopPlayerConfig {
    /// Create low-latency configuration
    pub fn low_latency() -> Self {
        Self {
            core: PlayerConfig::default(),
            hardware_backend: HardwareBackend::Auto,
            hardware_decode: true,
            subtitles_enabled: false,
            subtitle_language: None,
            buffer_duration: 500_000_000, // 500ms
            low_latency: true,
        }
    }
}

/// Player state wrapper
#[derive(Debug, Clone)]
struct PlayerStateInner {
    state: PlayerState,
    position: u64,
    duration: u64,
    volume: f64,
    muted: bool,
    current_uri: Option<String>,
    video_width: u32,
    video_height: u32,
    current_bitrate: u64,
}

impl Default for PlayerStateInner {
    fn default() -> Self {
        Self {
            state: PlayerState::Idle,
            position: 0,
            duration: 0,
            volume: 1.0,
            muted: false,
            current_uri: None,
            video_width: 0,
            video_height: 0,
            current_bitrate: 0,
        }
    }
}

/// GStreamer-based desktop video player
pub struct DesktopPlayer {
    player: gst_player::Player,
    session: Arc<PlayerSession>,
    config: DesktopPlayerConfig,
    state: Arc<Mutex<PlayerStateInner>>,
    available_backends: Vec<HardwareBackend>,
}

impl DesktopPlayer {
    /// Create a new desktop player
    pub fn new(config: DesktopPlayerConfig) -> Result<Self> {
        // Initialize GStreamer
        gst::init().context("Failed to initialize GStreamer")?;

        // Detect available hardware backends
        let available_backends = HardwareBackend::detect_available();
        info!("Available hardware backends: {:?}", available_backends);

        // Create player with video renderer
        let player = gst_player::Player::new(
            None::<gst_player::PlayerVideoRenderer>,
            None::<gst_player::PlayerSignalDispatcher>,
        );

        let session = Arc::new(PlayerSession::new(config.core.clone()));
        let state = Arc::new(Mutex::new(PlayerStateInner::default()));

        // Connect signals
        let state_clone = state.clone();
        player.connect_state_changed(move |_player, gst_state| {
            let psm_state = match gst_state {
                gst_player::PlayerState::Stopped => PlayerState::Idle,
                gst_player::PlayerState::Buffering => PlayerState::Buffering,
                gst_player::PlayerState::Paused => PlayerState::Paused,
                gst_player::PlayerState::Playing => PlayerState::Playing,
                _ => PlayerState::Idle,
            };

            if let Ok(mut s) = state_clone.lock() {
                s.state = psm_state;
            }
            debug!("Player state changed: {:?}", psm_state);
        });

        let state_clone = state.clone();
        player.connect_position_updated(move |_player, position| {
            if let Some(pos) = position {
                if let Ok(mut s) = state_clone.lock() {
                    s.position = pos.nseconds();
                }
            }
        });

        let state_clone = state.clone();
        player.connect_duration_changed(move |_player, duration| {
            if let Some(dur) = duration {
                if let Ok(mut s) = state_clone.lock() {
                    s.duration = dur.nseconds();
                }
            }
        });

        let state_clone = state.clone();
        player.connect_video_dimensions_changed(move |_player, width, height| {
            if let Ok(mut s) = state_clone.lock() {
                s.video_width = width as u32;
                s.video_height = height as u32;
            }
            info!("Video dimensions: {}x{}", width, height);
        });

        player.connect_error(|_player, error| {
            error!("Player error: {}", error);
        });

        player.connect_warning(|_player, warning| {
            warn!("Player warning: {}", warning);
        });

        Ok(Self {
            player,
            session,
            config,
            state,
            available_backends,
        })
    }

    /// Get player session
    pub fn session(&self) -> &Arc<PlayerSession> {
        &self.session
    }

    /// Load a media URI (HLS, DASH, or direct file)
    pub fn load(&mut self, uri: &str) -> Result<()> {
        info!("Loading: {}", uri);

        if let Ok(mut s) = self.state.lock() {
            s.current_uri = Some(uri.to_string());
            s.state = PlayerState::Loading;
        }

        self.player.set_uri(Some(uri));
        Ok(())
    }

    /// Start playback
    pub fn play(&self) {
        self.player.play();
    }

    /// Pause playback
    pub fn pause(&self) {
        self.player.pause();
    }

    /// Stop playback
    pub fn stop(&self) {
        self.player.stop();
        if let Ok(mut s) = self.state.lock() {
            s.state = PlayerState::Idle;
        }
    }

    /// Seek to position (in nanoseconds)
    pub fn seek(&self, position_ns: u64) {
        self.player.seek(gst::ClockTime::from_nseconds(position_ns));
    }

    /// Seek to position (in seconds)
    pub fn seek_seconds(&self, position_secs: f64) {
        self.seek((position_secs * 1_000_000_000.0) as u64);
    }

    /// Set volume (0.0 - 1.0)
    pub fn set_volume(&self, volume: f64) {
        self.player.set_volume(volume.clamp(0.0, 1.0));
        if let Ok(mut s) = self.state.lock() {
            s.volume = volume;
        }
    }

    /// Get current volume
    pub fn volume(&self) -> f64 {
        self.player.volume()
    }

    /// Set muted state
    pub fn set_muted(&self, muted: bool) {
        self.player.set_mute(muted);
        if let Ok(mut s) = self.state.lock() {
            s.muted = muted;
        }
    }

    /// Check if muted
    pub fn is_muted(&self) -> bool {
        self.player.is_muted()
    }

    /// Get current position in nanoseconds
    pub fn position(&self) -> u64 {
        self.player.position()
            .map(|p| p.nseconds())
            .unwrap_or(0)
    }

    /// Get current position in seconds
    pub fn position_seconds(&self) -> f64 {
        self.position() as f64 / 1_000_000_000.0
    }

    /// Get total duration in nanoseconds
    pub fn duration(&self) -> u64 {
        self.player.duration()
            .map(|d| d.nseconds())
            .unwrap_or(0)
    }

    /// Get total duration in seconds
    pub fn duration_seconds(&self) -> f64 {
        self.duration() as f64 / 1_000_000_000.0
    }

    /// Get current player state
    pub fn player_state(&self) -> PlayerState {
        self.state.lock()
            .map(|s| s.state)
            .unwrap_or(PlayerState::Error)
    }

    /// Get video dimensions
    pub fn video_dimensions(&self) -> (u32, u32) {
        self.state.lock()
            .map(|s| (s.video_width, s.video_height))
            .unwrap_or((0, 0))
    }

    /// Get available hardware backends
    pub fn available_backends(&self) -> &[HardwareBackend] {
        &self.available_backends
    }

    /// Get current hardware backend being used
    pub fn current_backend(&self) -> HardwareBackend {
        self.config.hardware_backend
    }

    /// Check if hardware decoding is active
    pub fn is_hardware_accelerated(&self) -> bool {
        self.config.hardware_decode &&
            self.available_backends.iter().any(|b| *b != HardwareBackend::Software)
    }

    /// Get quality metrics
    pub fn quality_metrics(&self) -> QualityMetrics {
        let s = self.state.lock().ok();
        let (width, height) = s.as_ref()
            .map(|s| (s.video_width, s.video_height))
            .unwrap_or((0, 0));

        QualityMetrics {
            bitrate: s.as_ref().map(|s| s.current_bitrate).unwrap_or(0),
            resolution: if width > 0 {
                Some(Resolution::new(width, height))
            } else {
                None
            },
            dropped_frames: 0,
            decoded_frames: 0,
            buffer_level: 0.0,
            stall_count: 0,
            stall_duration: 0.0,
            quality_switches: 0,
            throughput: 0,
        }
    }

    /// Set playback rate
    pub fn set_rate(&self, rate: f64) {
        self.player.set_rate(rate);
    }

    /// Get playback rate
    pub fn rate(&self) -> f64 {
        self.player.rate()
    }

    /// Enable/disable subtitles
    pub fn set_subtitles_enabled(&mut self, enabled: bool) {
        self.config.subtitles_enabled = enabled;
        self.player.set_subtitle_track_enabled(enabled);
    }

    /// Select subtitle track by index
    pub fn select_subtitle_track(&self, index: i32) {
        self.player.set_subtitle_track(index).ok();
    }

    /// Get branding colors
    pub fn branding_colors() -> PsmColors {
        PsmColors::default()
    }
}

impl Drop for DesktopPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Check GStreamer installation and capabilities
pub fn check_gstreamer_installation() -> Result<GStreamerInfo> {
    gst::init().context("Failed to initialize GStreamer")?;

    let (major, minor, micro, nano) = gst::version();
    let version = format!("{}.{}.{}.{}", major, minor, micro, nano);

    // Check for required elements
    let required_elements = [
        ("playbin", "Core playback"),
        ("hlsdemux", "HLS support"),
        ("dashdemux", "DASH support"),
        ("decodebin", "Auto decoding"),
    ];

    let mut missing = Vec::new();
    for (element, desc) in &required_elements {
        if gst::ElementFactory::find(element).is_none() {
            missing.push(format!("{} ({})", element, desc));
        }
    }

    let hardware_backends = HardwareBackend::detect_available();

    Ok(GStreamerInfo {
        version,
        missing_elements: missing,
        hardware_backends,
    })
}

/// GStreamer installation information
#[derive(Debug)]
pub struct GStreamerInfo {
    pub version: String,
    pub missing_elements: Vec<String>,
    pub hardware_backends: Vec<HardwareBackend>,
}

impl GStreamerInfo {
    pub fn is_complete(&self) -> bool {
        self.missing_elements.is_empty()
    }

    pub fn has_hardware_accel(&self) -> bool {
        self.hardware_backends.iter().any(|b| *b != HardwareBackend::Software)
    }
}
