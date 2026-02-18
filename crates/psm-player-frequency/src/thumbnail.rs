//! Intelligent thumbnail selection using frequency analysis.
//!
//! This module selects optimal video frames for thumbnails by analyzing:
//! - **Image sharpness** via 2D FFT high-frequency content
//! - **Audio energy** to find visually interesting moments
//! - **Motion detection** to avoid blurry transitional frames
//! - **Contrast analysis** for visually appealing frames

use std::path::Path;
use std::process::Command;
use anyhow::{Result, bail, Context};
use image::GrayImage;
use rustfft::{FftPlanner, num_complex::Complex};
use tracing::{debug, info, warn};

use crate::types::*;

/// Configuration for thumbnail selection.
#[derive(Debug, Clone)]
pub struct ThumbnailConfig {
    /// Number of candidate frames to extract
    pub num_candidates: usize,
    /// Skip first N seconds (avoid intro/logos)
    pub skip_start_secs: f64,
    /// Skip last N seconds (avoid outros)
    pub skip_end_secs: f64,
    /// Minimum sharpness threshold (0-1)
    pub min_sharpness: f32,
    /// Weight for sharpness in scoring
    pub sharpness_weight: f32,
    /// Weight for contrast in scoring
    pub contrast_weight: f32,
    /// Weight for audio energy correlation
    pub audio_weight: f32,
    /// Target thumbnail width
    pub output_width: u32,
    /// Target thumbnail height
    pub output_height: u32,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            num_candidates: 30,
            skip_start_secs: 2.0,
            skip_end_secs: 2.0,
            min_sharpness: 0.3,
            sharpness_weight: 0.4,
            contrast_weight: 0.3,
            audio_weight: 0.3,
            output_width: 1280,
            output_height: 720,
        }
    }
}

/// Thumbnail selector using frequency-based frame analysis.
pub struct ThumbnailSelector {
    config: ThumbnailConfig,
}

impl ThumbnailSelector {
    /// Create a new thumbnail selector with default configuration.
    pub fn new() -> Self {
        Self::with_config(ThumbnailConfig::default())
    }

    /// Create a selector with custom configuration.
    pub fn with_config(config: ThumbnailConfig) -> Self {
        Self { config }
    }

    /// Find the best timestamp for a thumbnail.
    pub fn find_best_timestamp(
        &self,
        video_path: impl AsRef<Path>,
        audio: &AudioData,
    ) -> Result<f64> {
        let video_path = video_path.as_ref();
        info!("Finding best thumbnail timestamp for: {}", video_path.display());

        // Get video duration
        let duration = self.get_video_duration(video_path)?;
        debug!("Video duration: {:.2}s", duration);

        // Calculate valid range
        let start_time = self.config.skip_start_secs;
        let end_time = (duration - self.config.skip_end_secs).max(start_time + 1.0);

        // Generate candidate timestamps
        let step = (end_time - start_time) / self.config.num_candidates as f64;
        let timestamps: Vec<f64> = (0..self.config.num_candidates)
            .map(|i| start_time + i as f64 * step)
            .collect();

        // Analyze audio energy at each timestamp
        let audio_energies = self.compute_audio_energies(audio, &timestamps);

        // Score each candidate
        let mut candidates: Vec<(f64, f32)> = Vec::new();

        for (i, &timestamp) in timestamps.iter().enumerate() {
            // Extract frame at timestamp
            match self.extract_frame(video_path, timestamp) {
                Ok(frame) => {
                    let quality = self.analyze_frame_quality(&frame);

                    // Combine scores
                    let audio_score = audio_energies.get(i).copied().unwrap_or(0.5);
                    let total_score = quality.sharpness * self.config.sharpness_weight
                        + quality.contrast * self.config.contrast_weight
                        + audio_score * self.config.audio_weight;

                    if quality.sharpness >= self.config.min_sharpness {
                        candidates.push((timestamp, total_score));
                        debug!(
                            "Frame at {:.2}s: sharpness={:.3}, contrast={:.3}, audio={:.3}, total={:.3}",
                            timestamp, quality.sharpness, quality.contrast, audio_score, total_score
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to extract frame at {:.2}s: {}", timestamp, e);
                }
            }
        }

        // Find best candidate
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((best_timestamp, best_score)) = candidates.first() {
            info!("Best thumbnail at {:.2}s with score {:.3}", best_timestamp, best_score);
            Ok(*best_timestamp)
        } else {
            // Fallback to middle of video
            let fallback = (start_time + end_time) / 2.0;
            warn!("No suitable frames found, using fallback at {:.2}s", fallback);
            Ok(fallback)
        }
    }

    /// Find multiple thumbnail candidates ranked by quality.
    pub fn find_candidates(
        &self,
        video_path: impl AsRef<Path>,
        audio: &AudioData,
        num_results: usize,
    ) -> Result<Vec<ThumbnailCandidate>> {
        let video_path = video_path.as_ref();

        // Get video duration
        let duration = self.get_video_duration(video_path)?;

        // Calculate valid range
        let start_time = self.config.skip_start_secs;
        let end_time = (duration - self.config.skip_end_secs).max(start_time + 1.0);

        // Generate more candidates than requested
        let num_samples = self.config.num_candidates.max(num_results * 3);
        let step = (end_time - start_time) / num_samples as f64;
        let timestamps: Vec<f64> = (0..num_samples)
            .map(|i| start_time + i as f64 * step)
            .collect();

        // Analyze audio energy
        let audio_energies = self.compute_audio_energies(audio, &timestamps);

        // Analyze each frame
        let mut candidates: Vec<ThumbnailCandidate> = Vec::new();

        for (i, &timestamp) in timestamps.iter().enumerate() {
            if let Ok(frame) = self.extract_frame(video_path, timestamp) {
                let quality = self.analyze_frame_quality(&frame);

                let audio_score = audio_energies.get(i).copied().unwrap_or(0.5);
                let total_score = quality.sharpness * self.config.sharpness_weight
                    + quality.contrast * self.config.contrast_weight
                    + audio_score * self.config.audio_weight;

                candidates.push(ThumbnailCandidate {
                    timestamp,
                    sharpness: quality.sharpness,
                    contrast: quality.contrast,
                    audio_energy: audio_score,
                    total_score,
                });
            }
        }

        // Sort by total score
        candidates.sort_by(|a, b| b.total_score.partial_cmp(&a.total_score).unwrap_or(std::cmp::Ordering::Equal));

        // Diversify results (avoid clustering)
        let mut diversified = Vec::new();
        let min_gap = (end_time - start_time) / (num_results as f64 * 2.0);

        for candidate in candidates {
            let too_close = diversified.iter().any(|c: &ThumbnailCandidate| {
                (c.timestamp - candidate.timestamp).abs() < min_gap
            });

            if !too_close {
                diversified.push(candidate);
                if diversified.len() >= num_results {
                    break;
                }
            }
        }

        Ok(diversified)
    }

    /// Extract a thumbnail at the specified timestamp.
    pub fn extract_thumbnail(
        &self,
        video_path: impl AsRef<Path>,
        timestamp: f64,
        output_path: impl AsRef<Path>,
    ) -> Result<()> {
        let video_path = video_path.as_ref();
        let output_path = output_path.as_ref();

        let output = Command::new("ffmpeg")
            .args([
                "-ss", &format!("{:.3}", timestamp),
                "-i", &video_path.to_string_lossy(),
                "-vframes", "1",
                "-vf", &format!("scale={}:{}", self.config.output_width, self.config.output_height),
                "-y",
                &output_path.to_string_lossy(),
            ])
            .output()
            .context("FFmpeg not found")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("FFmpeg thumbnail extraction failed: {}", stderr);
        }

        info!("Extracted thumbnail to: {}", output_path.display());
        Ok(())
    }

    /// Get video duration using ffprobe.
    fn get_video_duration(&self, video_path: &Path) -> Result<f64> {
        let output = Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                &video_path.to_string_lossy(),
            ])
            .output()
            .context("FFprobe not found")?;

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .context("Failed to parse ffprobe output")?;

        let duration = json["format"]["duration"]
            .as_str()
            .and_then(|d| d.parse::<f64>().ok())
            .ok_or_else(|| anyhow::anyhow!("Could not determine video duration"))?;

        Ok(duration)
    }

    /// Extract a single frame as grayscale image.
    fn extract_frame(&self, video_path: &Path, timestamp: f64) -> Result<GrayImage> {
        // Extract frame to raw grayscale
        let output = Command::new("ffmpeg")
            .args([
                "-ss", &format!("{:.3}", timestamp),
                "-i", &video_path.to_string_lossy(),
                "-vframes", "1",
                "-vf", "scale=320:180,format=gray",  // Small for analysis
                "-f", "rawvideo",
                "-pix_fmt", "gray",
                "pipe:1",
            ])
            .output()
            .context("FFmpeg frame extraction failed")?;

        if !output.status.success() || output.stdout.is_empty() {
            bail!("Failed to extract frame at {:.2}s", timestamp);
        }

        let width = 320;
        let height = 180;

        if output.stdout.len() < width * height {
            bail!("Incomplete frame data");
        }

        let img = GrayImage::from_raw(width as u32, height as u32, output.stdout[..width * height].to_vec())
            .ok_or_else(|| anyhow::anyhow!("Failed to create image from raw data"))?;

        Ok(img)
    }

    /// Analyze frame quality using 2D FFT.
    fn analyze_frame_quality(&self, frame: &GrayImage) -> ImageQuality {
        let (width, height) = frame.dimensions();
        let pixels: Vec<f32> = frame.pixels()
            .map(|p| p.0[0] as f32 / 255.0)
            .collect();

        // Compute 2D FFT for sharpness analysis
        let sharpness = self.compute_2d_fft_sharpness(&pixels, width as usize, height as usize);

        // Compute contrast (standard deviation of pixel values)
        let mean: f32 = pixels.iter().sum::<f32>() / pixels.len() as f32;
        let variance: f32 = pixels.iter()
            .map(|&p| (p - mean) * (p - mean))
            .sum::<f32>() / pixels.len() as f32;
        let contrast = variance.sqrt();

        // Normalize contrast to 0-1 range
        let contrast_normalized = (contrast * 4.0).min(1.0);

        ImageQuality {
            sharpness,
            contrast: contrast_normalized,
        }
    }

    /// Compute image sharpness using 2D FFT high-frequency content.
    fn compute_2d_fft_sharpness(&self, pixels: &[f32], width: usize, height: usize) -> f32 {
        // Pad to power of 2 for efficient FFT
        let fft_width = width.next_power_of_two();
        let fft_height = height.next_power_of_two();

        let mut planner = FftPlanner::new();

        // FFT along rows
        let row_fft = planner.plan_fft_forward(fft_width);
        let mut row_data: Vec<Vec<Complex<f32>>> = (0..height)
            .map(|y| {
                let mut row: Vec<Complex<f32>> = (0..fft_width)
                    .map(|x| {
                        if x < width {
                            Complex::new(pixels[y * width + x], 0.0)
                        } else {
                            Complex::new(0.0, 0.0)
                        }
                    })
                    .collect();
                row_fft.process(&mut row);
                row
            })
            .collect();

        // FFT along columns
        let col_fft = planner.plan_fft_forward(fft_height);
        for x in 0..fft_width {
            let mut col: Vec<Complex<f32>> = (0..fft_height)
                .map(|y| {
                    if y < height {
                        row_data[y][x]
                    } else {
                        Complex::new(0.0, 0.0)
                    }
                })
                .collect();
            col_fft.process(&mut col);
            for y in 0..height {
                row_data[y][x] = col[y];
            }
        }

        // Compute magnitude and analyze high-frequency content
        let center_x = fft_width / 2;
        let center_y = height / 2;
        let radius = (fft_width.min(height) / 4) as f32;

        let mut high_freq_energy = 0.0f32;
        let mut total_energy = 0.0f32;

        for y in 0..height {
            for x in 0..fft_width {
                let magnitude = (row_data[y][x].re.powi(2) + row_data[y][x].im.powi(2)).sqrt();
                total_energy += magnitude;

                // Distance from center (DC component)
                let dx = (x as i32 - center_x as i32).abs() as f32;
                let dy = (y as i32 - center_y as i32).abs() as f32;
                let dist = (dx * dx + dy * dy).sqrt();

                // High frequency = far from center
                if dist > radius {
                    high_freq_energy += magnitude;
                }
            }
        }

        // Sharpness = ratio of high-frequency energy
        if total_energy > 0.0 {
            (high_freq_energy / total_energy).min(1.0)
        } else {
            0.0
        }
    }

    /// Compute audio energy at each candidate timestamp.
    fn compute_audio_energies(&self, audio: &AudioData, timestamps: &[f64]) -> Vec<f32> {
        let window_secs = 0.5; // Look at 0.5 second window around each timestamp
        let window_samples = (audio.sample_rate as f64 * window_secs) as usize;

        let mut energies: Vec<f32> = timestamps.iter()
            .map(|&t| {
                let center_sample = (t * audio.sample_rate as f64) as usize;
                let start = center_sample.saturating_sub(window_samples / 2);
                let end = (start + window_samples).min(audio.samples.len());

                if start >= audio.samples.len() {
                    return 0.0;
                }

                let energy: f32 = audio.samples[start..end]
                    .iter()
                    .map(|&s| s * s)
                    .sum::<f32>() / (end - start) as f32;

                energy.sqrt()
            })
            .collect();

        // Normalize energies to 0-1
        let max_energy = energies.iter().cloned().fold(0.0f32, f32::max);
        if max_energy > 0.0 {
            for e in &mut energies {
                *e /= max_energy;
            }
        }

        energies
    }
}

impl Default for ThumbnailSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Image quality metrics.
#[derive(Debug, Clone)]
struct ImageQuality {
    /// Sharpness score (0-1)
    sharpness: f32,
    /// Contrast score (0-1)
    contrast: f32,
}

/// Thumbnail candidate with quality scores.
#[derive(Debug, Clone)]
pub struct ThumbnailCandidate {
    /// Timestamp in seconds
    pub timestamp: f64,
    /// Sharpness score (0-1)
    pub sharpness: f32,
    /// Contrast score (0-1)
    pub contrast: f32,
    /// Audio energy at this moment (0-1)
    pub audio_energy: f32,
    /// Combined quality score
    pub total_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ThumbnailConfig::default();
        assert_eq!(config.num_candidates, 30);
        assert_eq!(config.skip_start_secs, 2.0);
        assert_eq!(config.output_width, 1280);
    }

    #[test]
    fn test_image_quality_analysis() {
        // Create a test image with clear edges (high frequency content)
        let width = 320;
        let height = 180;
        let mut pixels = vec![0u8; width * height];

        // Add vertical stripes (high frequency)
        for y in 0..height {
            for x in 0..width {
                pixels[y * width + x] = if x % 4 < 2 { 255 } else { 0 };
            }
        }

        let img = GrayImage::from_raw(width as u32, height as u32, pixels).unwrap();
        let selector = ThumbnailSelector::new();
        let quality = selector.analyze_frame_quality(&img);

        // High frequency stripes should give high sharpness
        assert!(quality.sharpness > 0.1);
        // Stripes have high contrast
        assert!(quality.contrast > 0.3);
    }

    #[test]
    fn test_audio_energy_computation() {
        let sample_rate = 44100;
        let duration = 10.0;
        let num_samples = (sample_rate as f64 * duration) as usize;

        // Create audio with peak energy at 5 seconds
        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                let envelope = (-(t - 5.0).powi(2) / 2.0).exp() as f32;
                envelope * (2.0 * std::f32::consts::PI * 440.0 * t as f32).sin()
            })
            .collect();

        let audio = AudioData::new(samples, sample_rate);
        let selector = ThumbnailSelector::new();

        let timestamps: Vec<f64> = (0..10).map(|i| i as f64 + 0.5).collect();
        let energies = selector.compute_audio_energies(&audio, &timestamps);

        // Energy should peak around 5 seconds (index 4 or 5)
        let max_idx = energies.iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        assert!(max_idx >= 3 && max_idx <= 6);
    }
}
