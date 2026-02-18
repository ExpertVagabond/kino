//! Adaptive Bitrate (ABR) Engine
//!
//! Implements multiple ABR algorithms:
//! - Throughput-based: Simple bandwidth estimation
//! - BOLA: Buffer Occupancy based Lyapunov Algorithm
//! - Hybrid: Combines throughput and buffer metrics

use crate::types::*;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, instrument};

/// ABR algorithm trait
pub trait AbrAlgorithm: Send + Sync {
    /// Select the best rendition given current conditions
    fn select_rendition<'a>(
        &self,
        renditions: &'a [Rendition],
        context: &AbrContext,
    ) -> Option<&'a Rendition>;

    /// Update algorithm state with new measurement
    fn update(&mut self, measurement: &BandwidthMeasurement);

    /// Get algorithm name
    fn name(&self) -> &'static str;
}

/// Context for ABR decisions
#[derive(Debug, Clone, Default)]
pub struct AbrContext {
    /// Current buffer level in seconds
    pub buffer_level: f64,
    /// Target buffer level
    pub target_buffer: f64,
    /// Current playback rate (1.0 = normal)
    pub playback_rate: f64,
    /// Is stream live
    pub is_live: bool,
    /// Screen width for resolution capping
    pub screen_width: Option<u32>,
    /// Maximum allowed bitrate (0 = unlimited)
    pub max_bitrate: u64,
    /// Network info
    pub network: NetworkInfo,
}

/// Bandwidth measurement sample
#[derive(Debug, Clone)]
pub struct BandwidthMeasurement {
    /// Bytes downloaded
    pub bytes: usize,
    /// Time taken
    pub duration: Duration,
    /// Timestamp
    pub timestamp: Instant,
}

impl BandwidthMeasurement {
    /// Calculate throughput in bits per second
    pub fn throughput_bps(&self) -> u64 {
        if self.duration.as_secs_f64() > 0.0 {
            ((self.bytes as f64 * 8.0) / self.duration.as_secs_f64()) as u64
        } else {
            0
        }
    }
}

/// ABR Engine combining multiple algorithms
pub struct AbrEngine {
    /// Active algorithm
    algorithm: Box<dyn AbrAlgorithm>,
    /// Bandwidth history
    bandwidth_history: VecDeque<BandwidthMeasurement>,
    /// Maximum history size
    max_history: usize,
    /// Current bandwidth estimate
    bandwidth_estimate: u64,
    /// Last selected rendition index
    last_selection: Option<usize>,
    /// Stability counter (prevent oscillation)
    stability_counter: u32,
}

impl AbrEngine {
    /// Create new ABR engine with specified algorithm
    pub fn new(algorithm_type: AbrAlgorithmType) -> Self {
        let algorithm: Box<dyn AbrAlgorithm> = match algorithm_type {
            AbrAlgorithmType::Throughput => Box::new(ThroughputAlgorithm::new()),
            AbrAlgorithmType::Bola => Box::new(BolaAlgorithm::new()),
            AbrAlgorithmType::Hybrid => Box::new(HybridAlgorithm::new()),
            AbrAlgorithmType::Ml => Box::new(ThroughputAlgorithm::new()), // Fallback
        };

        Self {
            algorithm,
            bandwidth_history: VecDeque::with_capacity(20),
            max_history: 20,
            bandwidth_estimate: 0,
            last_selection: None,
            stability_counter: 0,
        }
    }

    /// Record a bandwidth measurement
    #[instrument(skip(self))]
    pub fn record_measurement(&mut self, bytes: usize, duration: Duration) {
        let measurement = BandwidthMeasurement {
            bytes,
            duration,
            timestamp: Instant::now(),
        };

        // Update history
        if self.bandwidth_history.len() >= self.max_history {
            self.bandwidth_history.pop_front();
        }
        self.bandwidth_history.push_back(measurement.clone());

        // Update estimate using EWMA
        let sample = measurement.throughput_bps();
        if self.bandwidth_estimate == 0 {
            self.bandwidth_estimate = sample;
        } else {
            // EWMA with alpha = 0.2
            self.bandwidth_estimate =
                ((self.bandwidth_estimate as f64 * 0.8) + (sample as f64 * 0.2)) as u64;
        }

        // Update algorithm
        self.algorithm.update(&measurement);

        debug!(
            bytes = bytes,
            duration_ms = duration.as_millis(),
            throughput_mbps = sample as f64 / 1_000_000.0,
            estimate_mbps = self.bandwidth_estimate as f64 / 1_000_000.0,
            "Bandwidth measurement recorded"
        );
    }

    /// Select best rendition
    #[instrument(skip(self, renditions))]
    pub fn select_rendition<'a>(
        &mut self,
        renditions: &'a [Rendition],
        context: &AbrContext,
    ) -> Option<&'a Rendition> {
        if renditions.is_empty() {
            return None;
        }

        // Get algorithm recommendation
        let selected = self.algorithm.select_rendition(renditions, context)?;

        // Find index
        let new_index = renditions.iter().position(|r| r.id == selected.id)?;

        // Apply stability filter to prevent oscillation
        if let Some(last) = self.last_selection {
            if new_index != last {
                self.stability_counter += 1;
                if self.stability_counter < 3 {
                    // Don't switch yet
                    return renditions.get(last);
                }
            }
            self.stability_counter = 0;
        }

        self.last_selection = Some(new_index);

        debug!(
            selected_id = %selected.id,
            bandwidth = selected.bandwidth,
            resolution = ?selected.resolution,
            "Rendition selected"
        );

        Some(selected)
    }

    /// Get current bandwidth estimate
    pub fn bandwidth_estimate(&self) -> u64 {
        self.bandwidth_estimate
    }

    /// Get algorithm name
    pub fn algorithm_name(&self) -> &'static str {
        self.algorithm.name()
    }

    /// Force switch algorithm
    pub fn set_algorithm(&mut self, algorithm_type: AbrAlgorithmType) {
        self.algorithm = match algorithm_type {
            AbrAlgorithmType::Throughput => Box::new(ThroughputAlgorithm::new()),
            AbrAlgorithmType::Bola => Box::new(BolaAlgorithm::new()),
            AbrAlgorithmType::Hybrid => Box::new(HybridAlgorithm::new()),
            AbrAlgorithmType::Ml => Box::new(ThroughputAlgorithm::new()),
        };
    }
}

/// Throughput-based ABR algorithm
pub struct ThroughputAlgorithm {
    /// Safety factor (0.0-1.0)
    safety_factor: f64,
    /// Estimated throughput
    throughput_estimate: u64,
}

impl ThroughputAlgorithm {
    pub fn new() -> Self {
        Self {
            safety_factor: 0.8, // Use 80% of estimated bandwidth
            throughput_estimate: 0,
        }
    }
}

impl Default for ThroughputAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}

impl AbrAlgorithm for ThroughputAlgorithm {
    fn select_rendition<'a>(
        &self,
        renditions: &'a [Rendition],
        context: &AbrContext,
    ) -> Option<&'a Rendition> {
        let available_bandwidth =
            (context.network.bandwidth_estimate as f64 * self.safety_factor) as u64;

        // Filter by max bitrate if set
        let max_bitrate = if context.max_bitrate > 0 {
            context.max_bitrate.min(available_bandwidth)
        } else {
            available_bandwidth
        };

        // Select highest quality that fits in bandwidth
        renditions
            .iter()
            .filter(|r| r.bandwidth <= max_bitrate)
            .filter(|r| {
                // Filter by screen resolution if available
                if let (Some(res), Some(screen_w)) = (&r.resolution, context.screen_width) {
                    res.width <= screen_w
                } else {
                    true
                }
            })
            .max_by_key(|r| r.bandwidth)
    }

    fn update(&mut self, measurement: &BandwidthMeasurement) {
        let sample = measurement.throughput_bps();
        if self.throughput_estimate == 0 {
            self.throughput_estimate = sample;
        } else {
            self.throughput_estimate =
                ((self.throughput_estimate as f64 * 0.7) + (sample as f64 * 0.3)) as u64;
        }
    }

    fn name(&self) -> &'static str {
        "throughput"
    }
}

/// BOLA (Buffer Occupancy based Lyapunov Algorithm)
/// Paper: https://arxiv.org/abs/1601.06748
pub struct BolaAlgorithm {
    /// Minimum buffer (seconds)
    buffer_min: f64,
    /// Maximum buffer (seconds)
    buffer_max: f64,
    /// BOLA parameter V
    v: f64,
    /// BOLA parameter gamma
    gamma: f64,
}

impl BolaAlgorithm {
    pub fn new() -> Self {
        Self {
            buffer_min: 5.0,
            buffer_max: 30.0,
            v: 0.93,
            gamma: 5.0,
        }
    }

    /// Calculate utility for a rendition
    fn utility(&self, rendition: &Rendition) -> f64 {
        // Logarithmic utility function
        (rendition.bandwidth as f64).ln()
    }
}

impl Default for BolaAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}

impl AbrAlgorithm for BolaAlgorithm {
    fn select_rendition<'a>(
        &self,
        renditions: &'a [Rendition],
        context: &AbrContext,
    ) -> Option<&'a Rendition> {
        if renditions.is_empty() {
            return None;
        }

        let buffer = context.buffer_level;

        // BOLA formula: maximize (V * utility - buffer_level) / (bitrate + gamma)
        let mut best: Option<&Rendition> = None;
        let mut best_score = f64::NEG_INFINITY;

        for rendition in renditions {
            // Skip if over max bitrate
            if context.max_bitrate > 0 && rendition.bandwidth > context.max_bitrate {
                continue;
            }

            let utility = self.utility(rendition);
            let size = rendition.bandwidth as f64;

            // BOLA objective function
            let score = (self.v * utility - buffer) / (size / 1_000_000.0 + self.gamma);

            if score > best_score {
                best_score = score;
                best = Some(rendition);
            }
        }

        // Safety: if buffer is very low, pick lowest quality
        if buffer < self.buffer_min {
            return renditions.first();
        }

        best
    }

    fn update(&mut self, _measurement: &BandwidthMeasurement) {
        // BOLA doesn't use throughput measurements directly
    }

    fn name(&self) -> &'static str {
        "bola"
    }
}

/// Hybrid algorithm combining throughput and buffer metrics
pub struct HybridAlgorithm {
    throughput: ThroughputAlgorithm,
    bola: BolaAlgorithm,
    /// Weight for throughput (0.0-1.0)
    throughput_weight: f64,
}

impl HybridAlgorithm {
    pub fn new() -> Self {
        Self {
            throughput: ThroughputAlgorithm::new(),
            bola: BolaAlgorithm::new(),
            throughput_weight: 0.5,
        }
    }
}

impl Default for HybridAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}

impl AbrAlgorithm for HybridAlgorithm {
    fn select_rendition<'a>(
        &self,
        renditions: &'a [Rendition],
        context: &AbrContext,
    ) -> Option<&'a Rendition> {
        let throughput_pick = self.throughput.select_rendition(renditions, context);
        let bola_pick = self.bola.select_rendition(renditions, context);

        match (throughput_pick, bola_pick) {
            (Some(t), Some(b)) => {
                // If buffer is low, prefer BOLA (more conservative)
                if context.buffer_level < 10.0 {
                    Some(b)
                } else if t.bandwidth <= b.bandwidth {
                    Some(t)
                } else {
                    // Average the two
                    let t_idx = renditions.iter().position(|r| r.id == t.id).unwrap_or(0);
                    let b_idx = renditions.iter().position(|r| r.id == b.id).unwrap_or(0);
                    let avg_idx = (t_idx + b_idx) / 2;
                    renditions.get(avg_idx)
                }
            }
            (Some(t), None) => Some(t),
            (None, Some(b)) => Some(b),
            (None, None) => renditions.first(),
        }
    }

    fn update(&mut self, measurement: &BandwidthMeasurement) {
        self.throughput.update(measurement);
        self.bola.update(measurement);
    }

    fn name(&self) -> &'static str {
        "hybrid"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn create_test_renditions() -> Vec<Rendition> {
        vec![
            Rendition {
                id: "360p".to_string(),
                bandwidth: 800_000,
                resolution: Some(Resolution::new(640, 360)),
                frame_rate: None,
                video_codec: Some(VideoCodec::H264),
                audio_codec: Some(AudioCodec::Aac),
                uri: Url::parse("https://example.com/360p.m3u8").unwrap(),
                hdr: None,
                language: None,
                name: None,
            },
            Rendition {
                id: "720p".to_string(),
                bandwidth: 2_800_000,
                resolution: Some(Resolution::new(1280, 720)),
                frame_rate: None,
                video_codec: Some(VideoCodec::H264),
                audio_codec: Some(AudioCodec::Aac),
                uri: Url::parse("https://example.com/720p.m3u8").unwrap(),
                hdr: None,
                language: None,
                name: None,
            },
            Rendition {
                id: "1080p".to_string(),
                bandwidth: 5_000_000,
                resolution: Some(Resolution::new(1920, 1080)),
                frame_rate: None,
                video_codec: Some(VideoCodec::H264),
                audio_codec: Some(AudioCodec::Aac),
                uri: Url::parse("https://example.com/1080p.m3u8").unwrap(),
                hdr: None,
                language: None,
                name: None,
            },
        ]
    }

    #[test]
    fn test_throughput_selection() {
        let renditions = create_test_renditions();
        let algorithm = ThroughputAlgorithm::new();

        // High bandwidth - should select 1080p
        let context = AbrContext {
            buffer_level: 20.0,
            network: NetworkInfo {
                bandwidth_estimate: 10_000_000,
                ..Default::default()
            },
            ..Default::default()
        };

        let selected = algorithm.select_rendition(&renditions, &context);
        assert_eq!(selected.map(|r| &r.id), Some(&"1080p".to_string()));

        // Low bandwidth - should select 360p
        let context = AbrContext {
            buffer_level: 20.0,
            network: NetworkInfo {
                bandwidth_estimate: 1_000_000,
                ..Default::default()
            },
            ..Default::default()
        };

        let selected = algorithm.select_rendition(&renditions, &context);
        assert_eq!(selected.map(|r| &r.id), Some(&"360p".to_string()));
    }

    #[test]
    fn test_bola_low_buffer() {
        let renditions = create_test_renditions();
        let algorithm = BolaAlgorithm::new();

        // Low buffer - should select lowest quality
        let context = AbrContext {
            buffer_level: 2.0,
            ..Default::default()
        };

        let selected = algorithm.select_rendition(&renditions, &context);
        assert_eq!(selected.map(|r| &r.id), Some(&"360p".to_string()));
    }
}
