//! Purple Squirrel Media - Frequency Analysis Library
//!
//! This crate provides audio frequency analysis capabilities for:
//! - **Audio Fingerprinting**: Cryptographic content verification using spectral peaks
//! - **AI Auto-Tagging**: Content classification based on frequency signatures
//! - **Thumbnail Generation**: Optimal frame selection using FFT-based quality metrics
//! - **Recommendations**: Content similarity matching via frequency signatures
//!
//! # Architecture
//!
//! The frequency analysis pipeline integrates with the Kino player ecosystem:
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │  Video Upload   │───▶│  Audio Extract   │───▶│  FFT Analysis   │
//! └─────────────────┘    └──────────────────┘    └────────┬────────┘
//!                                                         │
//!         ┌───────────────────────────────────────────────┼───────────────────────┐
//!         │                                               │                       │
//!         ▼                                               ▼                       ▼
//! ┌───────────────┐                              ┌────────────────┐       ┌───────────────┐
//! │ Fingerprint   │                              │  Auto-Tagging  │       │ Recommendations│
//! │ (SHA-256)     │                              │  (ML Model)    │       │ (Similarity)   │
//! └───────┬───────┘                              └────────┬───────┘       └───────┬───────┘
//!         │                                               │                       │
//!         ▼                                               ▼                       ▼
//! ┌───────────────┐                              ┌────────────────┐       ┌───────────────┐
//! │ Solana Chain  │                              │  Content Tags  │       │ Similar Items │
//! │ (Verification)│                              │  (Metadata)    │       │ (API Response)│
//! └───────────────┘                              └────────────────┘       └───────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use kino_frequency::{
//!     AudioAnalyzer,
//!     fingerprint::Fingerprinter,
//!     tagging::ContentTagger,
//!     thumbnail::ThumbnailSelector,
//!     recommend::RecommendationEngine,
//! };
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize the analyzer
//!     let analyzer = AudioAnalyzer::new(44100);
//!
//!     // Extract audio from video
//!     let audio = analyzer.extract_audio("video.mp4").await?;
//!
//!     // Generate fingerprint
//!     let fingerprint = Fingerprinter::new().fingerprint(&audio)?;
//!     println!("Content hash: {}", fingerprint.hash);
//!
//!     // Auto-tag content
//!     let tags = ContentTagger::new().predict(&audio)?;
//!     println!("Tags: {:?}", tags);
//!
//!     Ok(())
//! }
//! ```

#![warn(clippy::all)]
#![warn(missing_docs)]

pub mod fft;
pub mod types;

#[cfg(feature = "fingerprint")]
pub mod fingerprint;

#[cfg(feature = "tagging")]
pub mod tagging;

#[cfg(feature = "thumbnail")]
pub mod thumbnail;

#[cfg(feature = "recommend")]
pub mod recommend;

#[cfg(feature = "solana")]
pub mod solana;

pub mod streaming;

use std::path::Path;
use std::process::Command;
use anyhow::{Context, Result, bail};
use tracing::{info, debug, warn};

pub use types::*;
pub use fft::FrequencyAnalyzer;

#[cfg(feature = "fingerprint")]
pub use fingerprint::Fingerprinter;

#[cfg(feature = "tagging")]
pub use tagging::ContentTagger;

#[cfg(feature = "thumbnail")]
pub use thumbnail::ThumbnailSelector;

#[cfg(feature = "recommend")]
pub use recommend::RecommendationEngine;

/// Main audio analyzer that coordinates all frequency analysis operations.
pub struct AudioAnalyzer {
    sample_rate: u32,
    fft_size: usize,
    hop_size: usize,
}

impl AudioAnalyzer {
    /// Create a new audio analyzer with the specified sample rate.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            fft_size: 4096,
            hop_size: 2048,
        }
    }

    /// Create an analyzer with custom FFT parameters.
    pub fn with_fft_params(sample_rate: u32, fft_size: usize, hop_size: usize) -> Self {
        Self {
            sample_rate,
            fft_size,
            hop_size,
        }
    }

    /// Extract audio from a video file using FFmpeg.
    pub async fn extract_audio(&self, video_path: impl AsRef<Path>) -> Result<AudioData> {
        let video_path = video_path.as_ref();

        info!("Extracting audio from: {}", video_path.display());

        // Create temporary WAV file
        let temp_dir = std::env::temp_dir();
        let temp_wav = temp_dir.join(format!("kino_audio_{}.wav", uuid::Uuid::new_v4()));

        // Run FFmpeg to extract audio
        let output = Command::new("ffmpeg")
            .args([
                "-i", &video_path.to_string_lossy(),
                "-vn",                          // No video
                "-acodec", "pcm_s16le",         // 16-bit PCM
                "-ar", &self.sample_rate.to_string(),  // Sample rate
                "-ac", "1",                     // Mono
                "-y",                           // Overwrite
                &temp_wav.to_string_lossy(),
            ])
            .output()
            .context("FFmpeg not found. Please install FFmpeg.")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("FFmpeg audio extraction failed: {}", stderr);
        }

        // Read the WAV file
        let reader = hound::WavReader::open(&temp_wav)
            .context("Failed to open extracted audio")?;

        let spec = reader.spec();
        debug!("Audio spec: {:?}", spec);

        let samples: Vec<f32> = reader
            .into_samples::<i16>()
            .filter_map(|s| s.ok())
            .map(|s| s as f32 / 32768.0)
            .collect();

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_wav);

        info!("Extracted {} samples at {}Hz", samples.len(), spec.sample_rate);

        Ok(AudioData {
            samples,
            sample_rate: spec.sample_rate,
            channels: spec.channels as u32,
            duration_secs: 0.0, // Will be calculated
        })
    }

    /// Perform complete frequency analysis on audio data.
    pub fn analyze(&self, audio: &AudioData) -> Result<FrequencyAnalysis> {
        let analyzer = FrequencyAnalyzer::new(self.fft_size, self.hop_size);
        analyzer.analyze(&audio.samples, audio.sample_rate)
    }

    /// Get the dominant frequencies from audio.
    pub fn dominant_frequencies(&self, audio: &AudioData, top_k: usize) -> Result<Vec<DominantFrequency>> {
        let analyzer = FrequencyAnalyzer::new(self.fft_size, self.hop_size);
        analyzer.dominant_frequencies(&audio.samples, audio.sample_rate, top_k)
    }

    /// Compute frequency signature for similarity matching.
    pub fn compute_signature(&self, audio: &AudioData) -> Result<FrequencySignature> {
        let analyzer = FrequencyAnalyzer::new(self.fft_size, self.hop_size);
        analyzer.compute_signature(&audio.samples, audio.sample_rate)
    }
}

/// Process a video file through the complete frequency analysis pipeline.
pub async fn process_video(
    video_path: impl AsRef<Path>,
    config: ProcessingConfig,
) -> Result<ProcessingResult> {
    let video_path = video_path.as_ref();
    info!("Processing video: {}", video_path.display());

    let analyzer = AudioAnalyzer::new(config.sample_rate);
    let audio = analyzer.extract_audio(video_path).await?;

    let mut result = ProcessingResult {
        content_id: uuid::Uuid::new_v4().to_string(),
        fingerprint: None,
        tags: Vec::new(),
        thumbnail_timestamp: None,
        signature: None,
        dominant_frequencies: Vec::new(),
    };

    // Fingerprint
    #[cfg(feature = "fingerprint")]
    if config.enable_fingerprint {
        let fingerprinter = Fingerprinter::new();
        result.fingerprint = Some(fingerprinter.fingerprint(&audio)?);
    }

    // Auto-tagging
    #[cfg(feature = "tagging")]
    if config.enable_tagging {
        let tagger = ContentTagger::new();
        result.tags = tagger.predict(&audio)?;
    }

    // Thumbnail selection
    #[cfg(feature = "thumbnail")]
    if config.enable_thumbnail {
        let selector = ThumbnailSelector::new();
        if let Ok(timestamp) = selector.find_best_timestamp(video_path, &audio) {
            result.thumbnail_timestamp = Some(timestamp);
        }
    }

    // Frequency signature for recommendations
    if config.enable_signature {
        result.signature = Some(analyzer.compute_signature(&audio)?);
    }

    // Dominant frequencies
    result.dominant_frequencies = analyzer.dominant_frequencies(&audio, 10)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = AudioAnalyzer::new(44100);
        assert_eq!(analyzer.sample_rate, 44100);
        assert_eq!(analyzer.fft_size, 4096);
    }

    #[test]
    fn test_custom_fft_params() {
        let analyzer = AudioAnalyzer::with_fft_params(48000, 8192, 4096);
        assert_eq!(analyzer.sample_rate, 48000);
        assert_eq!(analyzer.fft_size, 8192);
        assert_eq!(analyzer.hop_size, 4096);
    }
}
