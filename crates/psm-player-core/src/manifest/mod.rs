//! Manifest parsing for HLS and DASH

mod hls;
mod dash;

pub use hls::HlsParser;
pub use dash::DashParser;

use crate::{Result, Rendition, Segment};
use async_trait::async_trait;
use url::Url;

/// Manifest types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestType {
    Hls,
    Dash,
}

/// Parsed manifest data
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Type of manifest
    pub manifest_type: ManifestType,
    /// Available renditions/variants
    pub renditions: Vec<Rendition>,
    /// Is this a live stream
    pub is_live: bool,
    /// Total duration (for VOD)
    pub duration: Option<std::time::Duration>,
    /// Target segment duration
    pub target_duration: std::time::Duration,
    /// Base URL for resolving relative URIs
    pub base_url: Url,
}

/// Trait for manifest parsers
#[async_trait]
pub trait ManifestParser: Send + Sync {
    /// Parse a manifest from URL
    async fn parse(&self, url: &Url) -> Result<Manifest>;

    /// Parse variant/representation playlist
    async fn parse_variant(&self, url: &Url) -> Result<Vec<Segment>>;

    /// Get the latest segments (for live)
    async fn get_latest_segments(&self, url: &Url, last_sequence: u64) -> Result<Vec<Segment>>;
}

/// Detect manifest type from URL or content
pub fn detect_manifest_type(url: &Url, content: Option<&str>) -> ManifestType {
    // Check URL extension first
    let path = url.path().to_lowercase();
    if path.ends_with(".m3u8") || path.ends_with(".m3u") {
        return ManifestType::Hls;
    }
    if path.ends_with(".mpd") {
        return ManifestType::Dash;
    }

    // Check content if available
    if let Some(content) = content {
        if content.contains("#EXTM3U") {
            return ManifestType::Hls;
        }
        if content.contains("<MPD") || content.contains("urn:mpeg:dash") {
            return ManifestType::Dash;
        }
    }

    // Default to HLS
    ManifestType::Hls
}

/// Create appropriate parser for URL
pub fn create_parser(url: &Url) -> Box<dyn ManifestParser> {
    match detect_manifest_type(url, None) {
        ManifestType::Hls => Box::new(HlsParser::new()),
        ManifestType::Dash => Box::new(DashParser::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_hls() {
        let url = Url::parse("https://example.com/master.m3u8").unwrap();
        assert_eq!(detect_manifest_type(&url, None), ManifestType::Hls);
    }

    #[test]
    fn test_detect_dash() {
        let url = Url::parse("https://example.com/manifest.mpd").unwrap();
        assert_eq!(detect_manifest_type(&url, None), ManifestType::Dash);
    }
}
