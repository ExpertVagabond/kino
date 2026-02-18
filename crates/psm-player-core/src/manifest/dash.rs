//! DASH (Dynamic Adaptive Streaming over HTTP) manifest parser
//!
//! Implements parsing for:
//! - MPD (Media Presentation Description)
//! - SegmentTemplate and SegmentList
//! - AdaptationSets and Representations
//! - Period handling

use crate::{
    error::Error,
    types::*,
    Result,
};
use super::{Manifest, ManifestParser, ManifestType};
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, instrument};
use url::Url;

/// DASH MPD parser
pub struct DashParser {
    client: Client,
}

impl DashParser {
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

    /// Parse MPD content
    fn parse_mpd(&self, content: &str, base_url: &Url) -> Result<Manifest> {
        // Simple MPD parsing using string operations
        // For production, use a proper XML parser like quick-xml

        let is_live = content.contains("type=\"dynamic\"");

        let duration = self.parse_duration_attr(content, "mediaPresentationDuration");
        let target_duration = self
            .parse_duration_attr(content, "maxSegmentDuration")
            .unwrap_or(Duration::from_secs(4));

        let renditions = self.extract_representations(content, base_url)?;

        Ok(Manifest {
            manifest_type: ManifestType::Dash,
            renditions,
            is_live,
            duration,
            target_duration,
            base_url: base_url.clone(),
        })
    }

    /// Extract representations from MPD
    fn extract_representations(&self, content: &str, base_url: &Url) -> Result<Vec<Rendition>> {
        let mut renditions = Vec::new();
        let mut idx = 0;

        // Find all Representation elements
        for rep_match in content.split("<Representation").skip(1) {
            if let Some(end) = rep_match.find('>') {
                let attrs = &rep_match[..end];

                let bandwidth = self.extract_attr(attrs, "bandwidth")
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                let width = self.extract_attr(attrs, "width")
                    .and_then(|s| s.parse::<u32>().ok());
                let height = self.extract_attr(attrs, "height")
                    .and_then(|s| s.parse::<u32>().ok());

                let resolution = match (width, height) {
                    (Some(w), Some(h)) => Some(Resolution::new(w, h)),
                    _ => None,
                };

                let codecs = self.extract_attr(attrs, "codecs");
                let video_codec = codecs.as_ref().and_then(|c| parse_dash_video_codec(c));
                let audio_codec = codecs.as_ref().and_then(|c| parse_dash_audio_codec(c));

                let frame_rate = self.extract_attr(attrs, "frameRate")
                    .and_then(|s| {
                        if s.contains('/') {
                            let parts: Vec<_> = s.split('/').collect();
                            if parts.len() == 2 {
                                let num: f32 = parts[0].parse().ok()?;
                                let den: f32 = parts[1].parse().ok()?;
                                Some(num / den)
                            } else {
                                None
                            }
                        } else {
                            s.parse().ok()
                        }
                    });

                // Get BaseURL or construct from template
                let uri = self.extract_base_url(rep_match, base_url)?;

                renditions.push(Rendition {
                    id: self.extract_attr(attrs, "id").unwrap_or_else(|| format!("rep_{}", idx)),
                    bandwidth,
                    resolution,
                    frame_rate,
                    video_codec,
                    audio_codec,
                    uri,
                    hdr: None,
                    language: None,
                    name: None,
                });

                idx += 1;
            }
        }

        // Sort by bandwidth
        renditions.sort_by_key(|r| r.bandwidth);

        if renditions.is_empty() {
            return Err(Error::InvalidManifest("No representations found in MPD".to_string()));
        }

        Ok(renditions)
    }

    /// Extract attribute value from XML attributes string
    fn extract_attr(&self, attrs: &str, name: &str) -> Option<String> {
        let pattern = format!("{}=\"", name);
        if let Some(start) = attrs.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = attrs[value_start..].find('"') {
                return Some(attrs[value_start..value_start + end].to_string());
            }
        }
        None
    }

    /// Parse ISO 8601 duration (PT...S format)
    fn parse_duration_attr(&self, content: &str, attr_name: &str) -> Option<Duration> {
        let pattern = format!("{}=\"", attr_name);
        if let Some(start) = content.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = content[value_start..].find('"') {
                let duration_str = &content[value_start..value_start + end];
                return parse_iso8601_duration(duration_str);
            }
        }
        None
    }

    /// Extract BaseURL for representation
    fn extract_base_url(&self, rep_content: &str, base_url: &Url) -> Result<Url> {
        // Look for BaseURL element
        if let Some(start) = rep_content.find("<BaseURL>") {
            if let Some(end) = rep_content[start..].find("</BaseURL>") {
                let url_str = &rep_content[start + 9..start + end];
                return base_url.join(url_str)
                    .map_err(|e| Error::InvalidManifest(format!("Invalid BaseURL: {}", e)));
            }
        }

        // No BaseURL, use manifest URL
        Ok(base_url.clone())
    }

    /// Generate segment URLs from SegmentTemplate
    fn generate_segment_urls(
        &self,
        template: &str,
        representation_id: &str,
        start: u64,
        count: u64,
        base_url: &Url,
    ) -> Result<Vec<Url>> {
        let mut urls = Vec::new();

        for i in start..start + count {
            let url_str = template
                .replace("$RepresentationID$", representation_id)
                .replace("$Number$", &i.to_string())
                .replace("$Time$", &(i * 4000).to_string()); // Assume 4s segments

            let url = base_url.join(&url_str)
                .map_err(|e| Error::InvalidManifest(format!("Invalid segment URL: {}", e)))?;
            urls.push(url);
        }

        Ok(urls)
    }
}

impl Default for DashParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ManifestParser for DashParser {
    #[instrument(skip(self))]
    async fn parse(&self, url: &Url) -> Result<Manifest> {
        debug!("Fetching DASH manifest: {}", url);

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

        self.parse_mpd(&content, url)
    }

    #[instrument(skip(self))]
    async fn parse_variant(&self, url: &Url) -> Result<Vec<Segment>> {
        // For DASH, we need to parse the MPD and generate segments
        // based on SegmentTemplate or SegmentList

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

        self.parse_segments(&content, url)
    }

    #[instrument(skip(self))]
    async fn get_latest_segments(&self, url: &Url, last_sequence: u64) -> Result<Vec<Segment>> {
        let all_segments = self.parse_variant(url).await?;

        let new_segments: Vec<_> = all_segments
            .into_iter()
            .filter(|s| s.number > last_sequence)
            .collect();

        Ok(new_segments)
    }
}

impl DashParser {
    /// Parse segments from MPD content
    fn parse_segments(&self, content: &str, base_url: &Url) -> Result<Vec<Segment>> {
        let mut segments = Vec::new();

        // Look for SegmentTemplate
        if let Some(template_start) = content.find("<SegmentTemplate") {
            if let Some(template_end) = content[template_start..].find('>') {
                let template_attrs = &content[template_start..template_start + template_end];

                let media_template = self.extract_attr(template_attrs, "media");
                let timescale: u64 = self.extract_attr(template_attrs, "timescale")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
                let duration: u64 = self.extract_attr(template_attrs, "duration")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(timescale * 4);

                let segment_duration = Duration::from_secs_f64(duration as f64 / timescale as f64);

                // Generate segments (simplified - assumes 100 segments for VOD)
                let segment_count = 100;

                if let Some(template) = media_template {
                    for i in 1..=segment_count {
                        let url_str = template
                            .replace("$Number$", &i.to_string())
                            .replace("$Time$", &((i - 1) * duration).to_string());

                        let url = base_url.join(&url_str)
                            .map_err(|e| Error::InvalidManifest(format!("Invalid segment URL: {}", e)))?;

                        segments.push(Segment {
                            number: i as u64,
                            uri: url,
                            duration: segment_duration,
                            byte_range: None,
                            encryption: None,
                            discontinuity_sequence: 0,
                            program_date_time: None,
                        });
                    }
                }
            }
        }

        // Look for SegmentList
        if let Some(list_start) = content.find("<SegmentList") {
            // Parse SegmentURL elements
            for segment_match in content[list_start..].split("<SegmentURL").skip(1) {
                if let Some(end) = segment_match.find('>') {
                    let attrs = &segment_match[..end];

                    if let Some(media) = self.extract_attr(attrs, "media") {
                        let url = base_url.join(&media)
                            .map_err(|e| Error::InvalidManifest(format!("Invalid segment URL: {}", e)))?;

                        segments.push(Segment {
                            number: segments.len() as u64 + 1,
                            uri: url,
                            duration: Duration::from_secs(4), // Default
                            byte_range: None,
                            encryption: None,
                            discontinuity_sequence: 0,
                            program_date_time: None,
                        });
                    }
                }
            }
        }

        if segments.is_empty() {
            return Err(Error::InvalidManifest("No segments found in MPD".to_string()));
        }

        Ok(segments)
    }
}

/// Parse ISO 8601 duration (PT1H2M3.4S format)
fn parse_iso8601_duration(s: &str) -> Option<Duration> {
    let s = s.trim_start_matches("PT").trim_start_matches("P");

    let mut total_seconds = 0.0;
    let mut current = String::new();

    for c in s.chars() {
        match c {
            'H' => {
                total_seconds += current.parse::<f64>().unwrap_or(0.0) * 3600.0;
                current.clear();
            }
            'M' => {
                total_seconds += current.parse::<f64>().unwrap_or(0.0) * 60.0;
                current.clear();
            }
            'S' => {
                total_seconds += current.parse::<f64>().unwrap_or(0.0);
                current.clear();
            }
            _ => current.push(c),
        }
    }

    if total_seconds > 0.0 {
        Some(Duration::from_secs_f64(total_seconds))
    } else {
        None
    }
}

/// Parse video codec from DASH codecs attribute
fn parse_dash_video_codec(codecs: &str) -> Option<VideoCodec> {
    let codecs_lower = codecs.to_lowercase();
    if codecs_lower.contains("avc1") || codecs_lower.contains("avc3") {
        Some(VideoCodec::H264)
    } else if codecs_lower.contains("hvc1") || codecs_lower.contains("hev1") {
        Some(VideoCodec::H265)
    } else if codecs_lower.contains("vp09") || codecs_lower.contains("vp9") {
        Some(VideoCodec::Vp9)
    } else if codecs_lower.contains("av01") {
        Some(VideoCodec::Av1)
    } else {
        None
    }
}

/// Parse audio codec from DASH codecs attribute
fn parse_dash_audio_codec(codecs: &str) -> Option<AudioCodec> {
    let codecs_lower = codecs.to_lowercase();
    if codecs_lower.contains("mp4a") {
        Some(AudioCodec::Aac)
    } else if codecs_lower.contains("ac-3") {
        Some(AudioCodec::Ac3)
    } else if codecs_lower.contains("ec-3") {
        Some(AudioCodec::Eac3)
    } else if codecs_lower.contains("opus") {
        Some(AudioCodec::Opus)
    } else if codecs_lower.contains("flac") {
        Some(AudioCodec::Flac)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso8601_duration() {
        assert_eq!(
            parse_iso8601_duration("PT1H30M"),
            Some(Duration::from_secs(5400))
        );
        assert_eq!(
            parse_iso8601_duration("PT45.5S"),
            Some(Duration::from_secs_f64(45.5))
        );
        assert_eq!(
            parse_iso8601_duration("PT2H5M10S"),
            Some(Duration::from_secs(7510))
        );
    }
}
