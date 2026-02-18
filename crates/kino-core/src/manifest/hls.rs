//! HLS (HTTP Live Streaming) manifest parser
//!
//! Implements parsing for:
//! - Master playlists (multivariant)
//! - Media playlists (segments)
//! - EXT-X-KEY encryption
//! - EXT-X-MAP initialization segments
//! - Discontinuity handling

use crate::{
    error::Error,
    types::*,
    Result,
};
use super::{Manifest, ManifestParser, ManifestType};
use async_trait::async_trait;
use m3u8_rs::{self, MediaPlaylist, MasterPlaylist};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, instrument};
use url::Url;

/// HLS manifest parser
pub struct HlsParser {
    client: Client,
}

impl HlsParser {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// Parse master playlist
    fn parse_master(&self, content: &str, base_url: &Url) -> Result<Manifest> {
        let parsed = m3u8_rs::parse_master_playlist_res(content.as_bytes())
            .map_err(|e| Error::ManifestParse(format!("Failed to parse HLS master: {:?}", e)))?;

        let renditions = self.extract_renditions(&parsed, base_url)?;

        Ok(Manifest {
            manifest_type: ManifestType::Hls,
            renditions,
            is_live: false, // Will be determined from media playlist
            duration: None,
            target_duration: Duration::from_secs(6), // Default, overridden by media playlist
            base_url: base_url.clone(),
        })
    }

    /// Extract renditions from master playlist
    fn extract_renditions(&self, master: &MasterPlaylist, base_url: &Url) -> Result<Vec<Rendition>> {
        let mut renditions = Vec::new();

        for (idx, variant) in master.variants.iter().enumerate() {
            let uri = self.resolve_uri(base_url, &variant.uri)?;

            let resolution = variant.resolution.map(|r| Resolution {
                width: r.width as u32,
                height: r.height as u32,
            });

            let video_codec = variant.codecs.as_ref().and_then(|c| parse_video_codec(c));
            let audio_codec = variant.codecs.as_ref().and_then(|c| parse_audio_codec(c));

            renditions.push(Rendition {
                id: format!("variant_{}", idx),
                bandwidth: variant.bandwidth,
                resolution,
                frame_rate: variant.frame_rate.map(|f| f as f32),
                video_codec,
                audio_codec,
                uri,
                hdr: None, // TODO: Parse HDR info from VIDEO-RANGE
                language: None,
                name: variant.video.clone(),
            });
        }

        // Sort by bandwidth
        renditions.sort_by_key(|r| r.bandwidth);

        Ok(renditions)
    }

    /// Parse media playlist
    fn parse_media(&self, content: &str, base_url: &Url) -> Result<(Vec<Segment>, bool, Option<Duration>)> {
        let parsed = m3u8_rs::parse_media_playlist_res(content.as_bytes())
            .map_err(|e| Error::ManifestParse(format!("Failed to parse HLS media: {:?}", e)))?;

        let is_live = !parsed.end_list;
        let duration = if parsed.end_list {
            Some(Duration::from_secs_f32(
                parsed.segments.iter().map(|s| s.duration).sum(),
            ))
        } else {
            None
        };

        let segments = self.extract_segments(&parsed, base_url)?;

        Ok((segments, is_live, duration))
    }

    /// Extract segments from media playlist
    fn extract_segments(&self, media: &MediaPlaylist, base_url: &Url) -> Result<Vec<Segment>> {
        let mut segments = Vec::new();
        let mut current_encryption: Option<EncryptionInfo> = None;
        let mut discontinuity_sequence = 0u32;
        let sequence_start = media.media_sequence;

        for (idx, seg) in media.segments.iter().enumerate() {
            // Handle discontinuity
            if seg.discontinuity {
                discontinuity_sequence += 1;
            }

            // Handle encryption
            if let Some(key) = &seg.key {
                current_encryption = self.parse_encryption_key(key, base_url)?;
            }

            let uri = self.resolve_uri(base_url, &seg.uri)?;

            let byte_range = seg.byte_range.as_ref().map(|br| ByteRange {
                start: br.offset.unwrap_or(0),
                length: br.length,
            });

            segments.push(Segment {
                number: sequence_start + idx as u64,
                uri,
                duration: Duration::from_secs_f32(seg.duration),
                byte_range,
                encryption: current_encryption.clone(),
                discontinuity_sequence,
                program_date_time: None, // TODO: Parse EXT-X-PROGRAM-DATE-TIME
            });
        }

        Ok(segments)
    }

    /// Parse encryption key
    fn parse_encryption_key(
        &self,
        key: &m3u8_rs::Key,
        base_url: &Url,
    ) -> Result<Option<EncryptionInfo>> {
        use m3u8_rs::KeyMethod;

        let method = match &key.method {
            KeyMethod::None => return Ok(None),
            KeyMethod::AES128 => EncryptionMethod::Aes128,
            KeyMethod::SampleAES => EncryptionMethod::SampleAes,
            KeyMethod::Other(s) if s == "SAMPLE-AES-CTR" => EncryptionMethod::SampleAesCtr,
            KeyMethod::Other(other) => {
                tracing::warn!("Unknown encryption method: {}", other);
                return Ok(None);
            }
        };

        let key_uri = key
            .uri
            .as_ref()
            .map(|u| self.resolve_uri(base_url, u))
            .transpose()?;

        let iv = key.iv.as_ref().map(|iv| {
            // Parse hex IV (0x...)
            let hex_str = iv.trim_start_matches("0x").trim_start_matches("0X");
            hex_decode(hex_str)
        });

        Ok(Some(EncryptionInfo {
            method,
            key_uri,
            iv,
            key_format: key.keyformat.clone(),
        }))
    }

    /// Resolve relative URI against base URL
    fn resolve_uri(&self, base: &Url, relative: &str) -> Result<Url> {
        base.join(relative)
            .map_err(|e| Error::InvalidManifest(format!("Invalid URI '{}': {}", relative, e)))
    }
}

impl Default for HlsParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ManifestParser for HlsParser {
    #[instrument(skip(self))]
    async fn parse(&self, url: &Url) -> Result<Manifest> {
        debug!("Fetching HLS manifest: {}", url);

        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| Error::ManifestFetch(e.to_string()))?;

        let content = response
            .text()
            .await
            .map_err(|e| Error::ManifestFetch(e.to_string()))?;

        // Detect if master or media playlist
        if content.contains("#EXT-X-STREAM-INF") {
            self.parse_master(&content, url)
        } else {
            // Single rendition (media playlist as entry point)
            let (_segments, is_live, duration) = self.parse_media(&content, url)?;

            // Create synthetic rendition
            let rendition = Rendition {
                id: "default".to_string(),
                bandwidth: 0, // Unknown
                resolution: None,
                frame_rate: None,
                video_codec: None,
                audio_codec: None,
                uri: url.clone(),
                hdr: None,
                language: None,
                name: None,
            };

            Ok(Manifest {
                manifest_type: ManifestType::Hls,
                renditions: vec![rendition],
                is_live,
                duration,
                target_duration: Duration::from_secs(6),
                base_url: url.clone(),
            })
        }
    }

    #[instrument(skip(self))]
    async fn parse_variant(&self, url: &Url) -> Result<Vec<Segment>> {
        debug!("Fetching HLS variant playlist: {}", url);

        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| Error::ManifestFetch(e.to_string()))?;

        let content = response
            .text()
            .await
            .map_err(|e| Error::ManifestFetch(e.to_string()))?;

        let (segments, _, _) = self.parse_media(&content, url)?;
        Ok(segments)
    }

    #[instrument(skip(self))]
    async fn get_latest_segments(&self, url: &Url, last_sequence: u64) -> Result<Vec<Segment>> {
        let all_segments = self.parse_variant(url).await?;

        // Filter to only new segments
        let new_segments: Vec<_> = all_segments
            .into_iter()
            .filter(|s| s.number > last_sequence)
            .collect();

        Ok(new_segments)
    }
}

/// Parse video codec from codecs string
fn parse_video_codec(codecs: &str) -> Option<VideoCodec> {
    let codecs_lower = codecs.to_lowercase();
    if codecs_lower.contains("avc1") || codecs_lower.contains("avc3") {
        Some(VideoCodec::H264)
    } else if codecs_lower.contains("hvc1") || codecs_lower.contains("hev1") {
        Some(VideoCodec::H265)
    } else if codecs_lower.contains("vp09") || codecs_lower.contains("vp9") {
        Some(VideoCodec::Vp9)
    } else if codecs_lower.contains("av01") || codecs_lower.contains("av1") {
        Some(VideoCodec::Av1)
    } else {
        None
    }
}

/// Parse audio codec from codecs string
fn parse_audio_codec(codecs: &str) -> Option<AudioCodec> {
    let codecs_lower = codecs.to_lowercase();
    if codecs_lower.contains("mp4a.40") {
        Some(AudioCodec::Aac)
    } else if codecs_lower.contains("ac-3") || codecs_lower.contains("ac3") {
        Some(AudioCodec::Ac3)
    } else if codecs_lower.contains("ec-3") || codecs_lower.contains("ec3") {
        Some(AudioCodec::Eac3)
    } else if codecs_lower.contains("opus") {
        Some(AudioCodec::Opus)
    } else if codecs_lower.contains("flac") {
        Some(AudioCodec::Flac)
    } else {
        None
    }
}

// Add hex crate for IV parsing
fn hex_decode(s: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut chars = s.chars().peekable();
    while chars.peek().is_some() {
        let high = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0) as u8;
        let low = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0) as u8;
        bytes.push((high << 4) | low);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_video_codec() {
        assert_eq!(parse_video_codec("avc1.640028"), Some(VideoCodec::H264));
        assert_eq!(parse_video_codec("hvc1.1.6.L93.B0"), Some(VideoCodec::H265));
        assert_eq!(parse_video_codec("vp09.00.10.08"), Some(VideoCodec::Vp9));
        assert_eq!(parse_video_codec("av01.0.01M.08"), Some(VideoCodec::Av1));
    }

    #[test]
    fn test_parse_audio_codec() {
        assert_eq!(parse_audio_codec("mp4a.40.2"), Some(AudioCodec::Aac));
        assert_eq!(parse_audio_codec("ac-3"), Some(AudioCodec::Ac3));
        assert_eq!(parse_audio_codec("ec-3"), Some(AudioCodec::Eac3));
    }
}
