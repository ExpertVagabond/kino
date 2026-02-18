//! Core types for PSM Player

use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;
use uuid::Uuid;

/// Unique identifier for a playback session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Video codec types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    Vp9,
    Av1,
    Unknown,
}

impl std::fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoCodec::H264 => write!(f, "H.264/AVC"),
            VideoCodec::H265 => write!(f, "H.265/HEVC"),
            VideoCodec::Vp9 => write!(f, "VP9"),
            VideoCodec::Av1 => write!(f, "AV1"),
            VideoCodec::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Audio codec types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioCodec {
    Aac,
    Ac3,
    Eac3,
    Opus,
    Flac,
    Unknown,
}

impl std::fmt::Display for AudioCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioCodec::Aac => write!(f, "AAC"),
            AudioCodec::Ac3 => write!(f, "AC-3"),
            AudioCodec::Eac3 => write!(f, "E-AC-3"),
            AudioCodec::Opus => write!(f, "Opus"),
            AudioCodec::Flac => write!(f, "FLAC"),
            AudioCodec::Unknown => write!(f, "Unknown"),
        }
    }
}

/// DRM system types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DrmSystem {
    Widevine,
    FairPlay,
    PlayReady,
    ClearKey,
}

impl DrmSystem {
    /// Returns the system ID (UUID) for PSSH box
    pub fn system_id(&self) -> &'static str {
        match self {
            DrmSystem::Widevine => "edef8ba9-79d6-4ace-a3c8-27dcd51d21ed",
            DrmSystem::FairPlay => "94ce86fb-07ff-4f43-adb8-93d2fa968ca2",
            DrmSystem::PlayReady => "9a04f079-9840-4286-ab92-e65be0885f95",
            DrmSystem::ClearKey => "1077efec-c0b2-4d02-ace3-3c1e52e2fb4b",
        }
    }
}

/// Video resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns quality tier name
    pub fn quality_name(&self) -> &'static str {
        match self.height {
            0..=240 => "240p",
            241..=360 => "360p",
            361..=480 => "480p",
            481..=720 => "720p",
            721..=1080 => "1080p",
            1081..=1440 => "1440p",
            _ => "4K",
        }
    }

    /// Common resolutions
    pub const SD_480P: Resolution = Resolution { width: 854, height: 480 };
    pub const HD_720P: Resolution = Resolution { width: 1280, height: 720 };
    pub const FHD_1080P: Resolution = Resolution { width: 1920, height: 1080 };
    pub const QHD_1440P: Resolution = Resolution { width: 2560, height: 1440 };
    pub const UHD_4K: Resolution = Resolution { width: 3840, height: 2160 };
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

/// Represents a video/audio rendition in the ABR ladder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rendition {
    /// Unique identifier for this rendition
    pub id: String,
    /// Bandwidth in bits per second
    pub bandwidth: u64,
    /// Video resolution (if video track)
    pub resolution: Option<Resolution>,
    /// Frame rate (if video track)
    pub frame_rate: Option<f32>,
    /// Video codec
    pub video_codec: Option<VideoCodec>,
    /// Audio codec
    pub audio_codec: Option<AudioCodec>,
    /// URI to the variant playlist (HLS) or representation (DASH)
    pub uri: Url,
    /// HDR format if applicable
    pub hdr: Option<HdrFormat>,
    /// Language code (for audio/subtitle tracks)
    pub language: Option<String>,
    /// Human-readable name
    pub name: Option<String>,
}

impl Rendition {
    /// Estimated quality score (0-100) for ABR decisions
    pub fn quality_score(&self) -> u32 {
        let base = match self.resolution {
            Some(r) if r.height >= 2160 => 100,
            Some(r) if r.height >= 1440 => 85,
            Some(r) if r.height >= 1080 => 70,
            Some(r) if r.height >= 720 => 55,
            Some(r) if r.height >= 480 => 40,
            Some(r) if r.height >= 360 => 25,
            _ => 10,
        };

        // Boost for HDR
        let hdr_boost = if self.hdr.is_some() { 5 } else { 0 };

        // Boost for high frame rate
        let fps_boost = match self.frame_rate {
            Some(f) if f >= 60.0 => 5,
            Some(f) if f >= 50.0 => 3,
            _ => 0,
        };

        (base + hdr_boost + fps_boost).min(100)
    }
}

/// HDR format types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HdrFormat {
    Hdr10,
    Hdr10Plus,
    DolbyVision,
    Hlg,
}

/// Segment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Segment number/index
    pub number: u64,
    /// URI to fetch the segment
    pub uri: Url,
    /// Duration of this segment
    pub duration: Duration,
    /// Byte range (if applicable)
    pub byte_range: Option<ByteRange>,
    /// Encryption key info (if encrypted)
    pub encryption: Option<EncryptionInfo>,
    /// Discontinuity sequence number
    pub discontinuity_sequence: u32,
    /// Program date/time (if available)
    pub program_date_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Byte range for partial segment requests
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ByteRange {
    pub start: u64,
    pub length: u64,
}

impl ByteRange {
    pub fn end(&self) -> u64 {
        self.start + self.length - 1
    }
}

/// Encryption information for a segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionInfo {
    pub method: EncryptionMethod,
    pub key_uri: Option<Url>,
    pub iv: Option<Vec<u8>>,
    pub key_format: Option<String>,
}

/// Encryption methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionMethod {
    None,
    Aes128,
    SampleAes,
    SampleAesCtr,
}

/// Player state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerState {
    /// Initial state, no content loaded
    Idle,
    /// Loading manifest
    Loading,
    /// Buffering content
    Buffering,
    /// Content is playing
    Playing,
    /// Playback paused
    Paused,
    /// Seeking to new position
    Seeking,
    /// Playback ended
    Ended,
    /// Error occurred
    Error,
}

impl PlayerState {
    /// Check if transition to target state is valid
    pub fn can_transition_to(&self, target: PlayerState) -> bool {
        use PlayerState::*;
        matches!(
            (self, target),
            // From Idle
            (Idle, Loading) |
            // From Loading
            (Loading, Buffering) | (Loading, Error) |
            // From Buffering
            (Buffering, Playing) | (Buffering, Paused) | (Buffering, Error) |
            // From Playing
            (Playing, Paused) | (Playing, Buffering) | (Playing, Seeking) | (Playing, Ended) | (Playing, Error) |
            // From Paused
            (Paused, Playing) | (Paused, Seeking) | (Paused, Idle) |
            // From Seeking
            (Seeking, Buffering) | (Seeking, Playing) | (Seeking, Error) |
            // From Ended
            (Ended, Idle) | (Ended, Seeking) |
            // From Error
            (Error, Idle) | (Error, Loading)
        )
    }
}

impl std::fmt::Display for PlayerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerState::Idle => write!(f, "idle"),
            PlayerState::Loading => write!(f, "loading"),
            PlayerState::Buffering => write!(f, "buffering"),
            PlayerState::Playing => write!(f, "playing"),
            PlayerState::Paused => write!(f, "paused"),
            PlayerState::Seeking => write!(f, "seeking"),
            PlayerState::Ended => write!(f, "ended"),
            PlayerState::Error => write!(f, "error"),
        }
    }
}

/// Playback quality metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Current bitrate in bps
    pub bitrate: u64,
    /// Current resolution
    pub resolution: Option<Resolution>,
    /// Frames dropped
    pub dropped_frames: u64,
    /// Total frames decoded
    pub decoded_frames: u64,
    /// Current buffer level in seconds
    pub buffer_level: f64,
    /// Number of buffer stalls
    pub stall_count: u32,
    /// Total time spent stalled in seconds
    pub stall_duration: f64,
    /// Number of quality switches
    pub quality_switches: u32,
    /// Average throughput in bps
    pub throughput: u64,
}

impl QualityMetrics {
    /// Calculate Quality of Experience score (0-100)
    pub fn qoe_score(&self) -> f64 {
        // Simple QoE model based on:
        // - Resolution quality
        // - Stall frequency
        // - Quality stability

        let resolution_score = match &self.resolution {
            Some(r) if r.height >= 1080 => 100.0,
            Some(r) if r.height >= 720 => 80.0,
            Some(r) if r.height >= 480 => 60.0,
            _ => 40.0,
        };

        // Penalize stalls heavily
        let stall_penalty = (self.stall_count as f64 * 10.0).min(50.0);

        // Penalize quality switches
        let switch_penalty = (self.quality_switches as f64 * 2.0).min(20.0);

        (resolution_score - stall_penalty - switch_penalty).max(0.0)
    }
}

/// Network information for ABR decisions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Estimated bandwidth in bps
    pub bandwidth_estimate: u64,
    /// RTT in milliseconds
    pub rtt_ms: u32,
    /// Connection type (if known)
    pub connection_type: Option<ConnectionType>,
    /// Is connection metered
    pub metered: bool,
}

/// Connection type for network-aware ABR
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    Ethernet,
    Wifi,
    Cellular4G,
    Cellular5G,
    Cellular3G,
    Unknown,
}

/// Player configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    /// Minimum buffer level before playback starts (seconds)
    pub min_buffer_time: f64,
    /// Maximum buffer level (seconds)
    pub max_buffer_time: f64,
    /// Buffer level to start rebuffering (seconds)
    pub rebuffer_threshold: f64,
    /// ABR algorithm to use
    pub abr_algorithm: AbrAlgorithmType,
    /// Maximum bitrate cap (0 = no cap)
    pub max_bitrate: u64,
    /// Start at lowest quality
    pub start_at_lowest: bool,
    /// Enable prefetch of next segment
    pub prefetch_enabled: bool,
    /// Retry attempts for failed requests
    pub retry_attempts: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Enable analytics
    pub analytics_enabled: bool,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            min_buffer_time: 10.0,
            max_buffer_time: 30.0,
            rebuffer_threshold: 2.0,
            abr_algorithm: AbrAlgorithmType::Bola,
            max_bitrate: 0,
            start_at_lowest: false,
            prefetch_enabled: true,
            retry_attempts: 3,
            retry_delay_ms: 1000,
            request_timeout_ms: 10000,
            analytics_enabled: true,
        }
    }
}

/// ABR algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbrAlgorithmType {
    /// Throughput-based selection
    Throughput,
    /// Buffer-based BOLA algorithm
    Bola,
    /// Hybrid throughput + buffer
    Hybrid,
    /// Machine learning based (requires model)
    Ml,
}

// =============================================================================
// Chapter and Caption Types
// =============================================================================

/// Video chapter/marker for navigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    /// Unique identifier
    pub id: String,
    /// Chapter title
    pub title: String,
    /// Start time in seconds
    pub start_time: f64,
    /// End time in seconds
    pub end_time: f64,
    /// Optional thumbnail URL
    pub thumbnail: Option<Url>,
    /// Optional description
    pub description: Option<String>,
}

impl Chapter {
    /// Create a new chapter
    pub fn new(id: impl Into<String>, title: impl Into<String>, start_time: f64, end_time: f64) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            start_time,
            end_time,
            thumbnail: None,
            description: None,
        }
    }

    /// Duration of this chapter in seconds
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// Check if a given time falls within this chapter
    pub fn contains_time(&self, time: f64) -> bool {
        time >= self.start_time && time < self.end_time
    }
}

/// Text track type (captions, subtitles, descriptions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextTrackKind {
    /// Closed captions (includes speaker identification, sound effects)
    Captions,
    /// Subtitles (dialogue translation)
    Subtitles,
    /// Audio descriptions for visually impaired
    Descriptions,
    /// Chapter titles
    Chapters,
    /// Metadata track
    Metadata,
}

impl std::fmt::Display for TextTrackKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextTrackKind::Captions => write!(f, "captions"),
            TextTrackKind::Subtitles => write!(f, "subtitles"),
            TextTrackKind::Descriptions => write!(f, "descriptions"),
            TextTrackKind::Chapters => write!(f, "chapters"),
            TextTrackKind::Metadata => write!(f, "metadata"),
        }
    }
}

/// Text track format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextTrackFormat {
    /// WebVTT format
    WebVtt,
    /// TTML/DFXP format
    Ttml,
    /// SRT format
    Srt,
    /// CEA-608 embedded captions
    Cea608,
    /// CEA-708 embedded captions
    Cea708,
}

impl TextTrackFormat {
    /// Get MIME type for format
    pub fn mime_type(&self) -> &'static str {
        match self {
            TextTrackFormat::WebVtt => "text/vtt",
            TextTrackFormat::Ttml => "application/ttml+xml",
            TextTrackFormat::Srt => "text/plain",
            TextTrackFormat::Cea608 => "application/cea-608",
            TextTrackFormat::Cea708 => "application/cea-708",
        }
    }

    /// Get file extension
    pub fn extension(&self) -> &'static str {
        match self {
            TextTrackFormat::WebVtt => "vtt",
            TextTrackFormat::Ttml => "ttml",
            TextTrackFormat::Srt => "srt",
            TextTrackFormat::Cea608 | TextTrackFormat::Cea708 => "",
        }
    }
}

/// Text track (captions, subtitles, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextTrack {
    /// Unique identifier
    pub id: String,
    /// Track kind
    pub kind: TextTrackKind,
    /// BCP-47 language code (e.g., "en", "es", "fr")
    pub language: String,
    /// Human-readable label (e.g., "English", "Español")
    pub label: String,
    /// URL to the track file
    pub url: Url,
    /// Track format
    pub format: TextTrackFormat,
    /// Is this the default track
    pub is_default: bool,
    /// Is this an auto-generated track (e.g., ASR)
    pub is_auto_generated: bool,
    /// For forced subtitles (foreign language parts)
    pub is_forced: bool,
}

impl TextTrack {
    /// Create a new text track
    pub fn new(
        id: impl Into<String>,
        kind: TextTrackKind,
        language: impl Into<String>,
        label: impl Into<String>,
        url: Url,
        format: TextTrackFormat,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            language: language.into(),
            label: label.into(),
            url,
            format,
            is_default: false,
            is_auto_generated: false,
            is_forced: false,
        }
    }

    /// Create a captions track
    pub fn captions(
        language: impl Into<String>,
        label: impl Into<String>,
        url: Url,
    ) -> Self {
        let lang = language.into();
        Self {
            id: format!("cc-{}", lang),
            kind: TextTrackKind::Captions,
            language: lang,
            label: label.into(),
            url,
            format: TextTrackFormat::WebVtt,
            is_default: false,
            is_auto_generated: false,
            is_forced: false,
        }
    }

    /// Create a subtitles track
    pub fn subtitles(
        language: impl Into<String>,
        label: impl Into<String>,
        url: Url,
    ) -> Self {
        let lang = language.into();
        Self {
            id: format!("sub-{}", lang),
            kind: TextTrackKind::Subtitles,
            language: lang,
            label: label.into(),
            url,
            format: TextTrackFormat::WebVtt,
            is_default: false,
            is_auto_generated: false,
            is_forced: false,
        }
    }

    /// Set as default track
    pub fn with_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    /// Mark as auto-generated
    pub fn with_auto_generated(mut self, is_auto: bool) -> Self {
        self.is_auto_generated = is_auto;
        self
    }
}

/// Individual cue within a text track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextCue {
    /// Cue identifier
    pub id: String,
    /// Start time in seconds
    pub start_time: f64,
    /// End time in seconds
    pub end_time: f64,
    /// Cue text content (may contain markup)
    pub text: String,
    /// Cue settings (position, alignment, etc.)
    pub settings: Option<CueSettings>,
}

impl TextCue {
    /// Create a new text cue
    pub fn new(
        id: impl Into<String>,
        start_time: f64,
        end_time: f64,
        text: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            start_time,
            end_time,
            text: text.into(),
            settings: None,
        }
    }

    /// Duration of this cue in seconds
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// Check if cue should be displayed at given time
    pub fn is_active_at(&self, time: f64) -> bool {
        time >= self.start_time && time < self.end_time
    }
}

/// Cue positioning and styling settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CueSettings {
    /// Vertical positioning ("" = horizontal, "rl" = right-to-left, "lr" = left-to-right)
    pub vertical: Option<String>,
    /// Line position (-1 = auto)
    pub line: Option<f64>,
    /// Text position (0-100%)
    pub position: Option<f64>,
    /// Cue size (0-100%)
    pub size: Option<f64>,
    /// Text alignment
    pub align: Option<CueAlignment>,
}

/// Text alignment for cues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CueAlignment {
    Start,
    Center,
    End,
    Left,
    Right,
}

/// Container for all tracks in a media asset
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MediaTracks {
    /// Video renditions (ABR ladder)
    pub video: Vec<Rendition>,
    /// Audio tracks
    pub audio: Vec<AudioTrack>,
    /// Text tracks (captions, subtitles)
    pub text: Vec<TextTrack>,
    /// Chapters
    pub chapters: Vec<Chapter>,
}

impl MediaTracks {
    /// Create empty media tracks
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a text track
    pub fn add_text_track(&mut self, track: TextTrack) {
        self.text.push(track);
    }

    /// Add a chapter
    pub fn add_chapter(&mut self, chapter: Chapter) {
        self.chapters.push(chapter);
    }

    /// Get chapter at given time
    pub fn chapter_at(&self, time: f64) -> Option<&Chapter> {
        self.chapters.iter().find(|c| c.contains_time(time))
    }

    /// Get all text tracks of a specific kind
    pub fn text_tracks_by_kind(&self, kind: TextTrackKind) -> Vec<&TextTrack> {
        self.text.iter().filter(|t| t.kind == kind).collect()
    }

    /// Get text tracks by language
    pub fn text_tracks_by_language(&self, language: &str) -> Vec<&TextTrack> {
        self.text.iter().filter(|t| t.language == language).collect()
    }

    /// Get default text track of a kind
    pub fn default_text_track(&self, kind: TextTrackKind) -> Option<&TextTrack> {
        self.text
            .iter()
            .filter(|t| t.kind == kind)
            .find(|t| t.is_default)
            .or_else(|| self.text.iter().find(|t| t.kind == kind))
    }
}

/// Audio track information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    /// Unique identifier
    pub id: String,
    /// BCP-47 language code
    pub language: String,
    /// Human-readable label
    pub label: String,
    /// Audio codec
    pub codec: Option<AudioCodec>,
    /// Number of channels
    pub channels: Option<u8>,
    /// Bitrate in bps
    pub bitrate: Option<u64>,
    /// Is this the default track
    pub is_default: bool,
    /// Audio description track for accessibility
    pub is_audio_description: bool,
    /// URL to audio variant (if separate from video)
    pub url: Option<Url>,
}
