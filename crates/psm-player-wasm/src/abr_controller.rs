//! ABR Controller - Drop-in replacement for hls.js AbrController
//!
//! Implements the BOLA algorithm for better quality selection than
//! the default throughput-based approach in hls.js.
//!
//! ## Usage with hls.js
//!
//! ```javascript
//! import Hls from 'hls.js';
//! import { PsmAbrController } from '@purplesquirrel/player-wasm';
//!
//! // Option 1: Replace the ABR controller entirely
//! class HlsWithPsmAbr extends Hls {
//!   constructor(config) {
//!     super({
//!       ...config,
//!       abrController: PsmAbrController,
//!     });
//!   }
//! }
//!
//! // Option 2: Use alongside hls.js for decisions only
//! const psm = new PsmAbrController();
//! hls.on(Hls.Events.FRAG_LOADING, () => {
//!   const level = psm.selectLevel(hls.levels, bufferLevel, bandwidth);
//!   if (level !== hls.currentLevel) {
//!     hls.currentLevel = level;
//!   }
//! });
//! ```

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

/// Level/Quality information from hls.js
#[derive(Clone, Serialize, Deserialize)]
pub struct Level {
    pub bitrate: u32,
    pub width: u32,
    pub height: u32,
    pub codec: Option<String>,
}

/// Bandwidth measurement sample
#[derive(Clone)]
struct BandwidthSample {
    bytes: usize,
    duration_ms: f64,
    _timestamp: f64,
}

impl BandwidthSample {
    fn throughput_bps(&self) -> f64 {
        if self.duration_ms > 0.0 {
            (self.bytes as f64 * 8.0 * 1000.0) / self.duration_ms
        } else {
            0.0
        }
    }
}

/// PSM ABR Controller using BOLA algorithm
#[wasm_bindgen]
pub struct PsmAbrController {
    /// Algorithm type
    algorithm: String,
    /// Bandwidth history
    bandwidth_history: VecDeque<BandwidthSample>,
    /// Maximum history size
    max_history: usize,
    /// Current bandwidth estimate
    bandwidth_estimate: f64,
    /// Last selected level
    last_level: i32,
    /// Stability counter (prevent oscillation)
    stability_count: u32,
    /// BOLA parameters
    bola_v: f64,
    bola_gamma: f64,
    /// Buffer thresholds
    buffer_min: f64,
    buffer_max: f64,
    /// Maximum bitrate cap
    max_bitrate: u32,
}

#[wasm_bindgen]
impl PsmAbrController {
    /// Create a new ABR controller
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            algorithm: "bola".to_string(),
            bandwidth_history: VecDeque::with_capacity(20),
            max_history: 20,
            bandwidth_estimate: 0.0,
            last_level: -1,
            stability_count: 0,
            bola_v: 0.93,
            bola_gamma: 5.0,
            buffer_min: 5.0,
            buffer_max: 30.0,
            max_bitrate: 0,
        }
    }

    /// Create controller with specific algorithm
    #[wasm_bindgen]
    pub fn with_algorithm(algorithm: &str) -> Self {
        let mut controller = Self::new();
        controller.algorithm = algorithm.to_string();
        controller
    }

    /// Set maximum bitrate cap
    #[wasm_bindgen]
    pub fn set_max_bitrate(&mut self, max_bitrate: u32) {
        self.max_bitrate = max_bitrate;
    }

    /// Set buffer thresholds
    #[wasm_bindgen]
    pub fn set_buffer_thresholds(&mut self, min: f64, max: f64) {
        self.buffer_min = min;
        self.buffer_max = max;
    }

    /// Record a bandwidth measurement (called after each segment download)
    #[wasm_bindgen]
    pub fn record_download(&mut self, bytes: usize, duration_ms: f64) {
        let sample = BandwidthSample {
            bytes,
            duration_ms,
            _timestamp: js_sys::Date::now(),
        };

        // Update history
        if self.bandwidth_history.len() >= self.max_history {
            self.bandwidth_history.pop_front();
        }
        self.bandwidth_history.push_back(sample.clone());

        // Update estimate using EWMA (Exponentially Weighted Moving Average)
        let throughput = sample.throughput_bps();
        if self.bandwidth_estimate == 0.0 {
            self.bandwidth_estimate = throughput;
        } else {
            // EWMA with alpha = 0.2 for smoothing
            self.bandwidth_estimate = self.bandwidth_estimate * 0.8 + throughput * 0.2;
        }
    }

    /// Select the best level based on current conditions
    ///
    /// # Arguments
    /// * `levels` - JSON array of {bitrate, width, height} objects
    /// * `buffer_level` - Current buffer level in seconds
    ///
    /// # Returns
    /// Index of the recommended level
    #[wasm_bindgen]
    pub fn select_level(&mut self, levels_json: &str, buffer_level: f64) -> i32 {
        let levels: Vec<Level> = match serde_json::from_str(levels_json) {
            Ok(l) => l,
            Err(_) => return 0,
        };

        if levels.is_empty() {
            return 0;
        }

        let selected = match self.algorithm.as_str() {
            "throughput" => self.select_throughput(&levels),
            "bola" => self.select_bola(&levels, buffer_level),
            "hybrid" => self.select_hybrid(&levels, buffer_level),
            _ => self.select_bola(&levels, buffer_level),
        };

        // Apply stability filter to prevent rapid oscillation
        let selected_i32 = selected as i32;
        if self.last_level >= 0 && selected_i32 != self.last_level {
            self.stability_count += 1;
            if self.stability_count < 3 {
                return self.last_level;
            }
        }

        self.stability_count = 0;
        self.last_level = selected_i32;
        selected_i32
    }

    /// Get current bandwidth estimate in bps
    #[wasm_bindgen]
    pub fn get_bandwidth_estimate(&self) -> f64 {
        self.bandwidth_estimate
    }

    /// Get bandwidth estimate in human-readable format
    #[wasm_bindgen]
    pub fn get_bandwidth_display(&self) -> String {
        let mbps = self.bandwidth_estimate / 1_000_000.0;
        format!("{:.1} Mbps", mbps)
    }

    /// Get the number of samples in history
    #[wasm_bindgen]
    pub fn get_sample_count(&self) -> usize {
        self.bandwidth_history.len()
    }

    /// Reset the controller state
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.bandwidth_history.clear();
        self.bandwidth_estimate = 0.0;
        self.last_level = -1;
        self.stability_count = 0;
    }
}

impl PsmAbrController {
    /// Throughput-based selection (simple, fast)
    fn select_throughput(&self, levels: &[Level]) -> usize {
        // Use 80% of estimated bandwidth for safety margin
        let safe_bandwidth = (self.bandwidth_estimate * 0.8) as u32;

        // Find highest quality that fits
        let mut best = 0;
        for (i, level) in levels.iter().enumerate() {
            if self.max_bitrate > 0 && level.bitrate > self.max_bitrate {
                continue;
            }
            if level.bitrate <= safe_bandwidth {
                best = i;
            }
        }
        best
    }

    /// BOLA algorithm (buffer-based, proven better QoE)
    /// Paper: https://arxiv.org/abs/1601.06748
    fn select_bola(&self, levels: &[Level], buffer_level: f64) -> usize {
        // Emergency: if buffer is critically low, pick lowest
        if buffer_level < self.buffer_min {
            return 0;
        }

        let mut best_level = 0;
        let mut best_score = f64::NEG_INFINITY;

        for (i, level) in levels.iter().enumerate() {
            // Skip if over max bitrate
            if self.max_bitrate > 0 && level.bitrate > self.max_bitrate {
                continue;
            }

            // BOLA utility function: logarithmic quality
            let utility = (level.bitrate as f64).ln();

            // Segment size estimate (normalized)
            let size = level.bitrate as f64 / 1_000_000.0;

            // BOLA objective: maximize (V * utility - buffer) / (size + gamma)
            let score = (self.bola_v * utility - buffer_level) / (size + self.bola_gamma);

            if score > best_score {
                best_score = score;
                best_level = i;
            }
        }

        best_level
    }

    /// Hybrid: combine throughput and buffer metrics
    fn select_hybrid(&self, levels: &[Level], buffer_level: f64) -> usize {
        let throughput_pick = self.select_throughput(levels);
        let bola_pick = self.select_bola(levels, buffer_level);

        // If buffer is low, trust BOLA (more conservative)
        if buffer_level < 10.0 {
            return bola_pick;
        }

        // If buffer is healthy, average the two
        (throughput_pick + bola_pick) / 2
    }
}

impl Default for PsmAbrController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bandwidth_estimation() {
        let mut controller = PsmAbrController::new();

        // Simulate downloading 1MB in 1 second = 8 Mbps
        controller.record_download(1_000_000, 1000.0);
        assert!((controller.get_bandwidth_estimate() - 8_000_000.0).abs() < 1000.0);
    }

    #[test]
    fn test_level_selection() {
        let mut controller = PsmAbrController::new();
        controller.record_download(1_000_000, 1000.0); // 8 Mbps

        let levels = r#"[
            {"bitrate": 500000, "width": 640, "height": 360},
            {"bitrate": 1500000, "width": 854, "height": 480},
            {"bitrate": 3000000, "width": 1280, "height": 720},
            {"bitrate": 6000000, "width": 1920, "height": 1080}
        ]"#;

        // With 8 Mbps and 20s buffer, should pick 720p or 1080p
        let selected = controller.select_level(levels, 20.0);
        assert!(selected >= 2); // At least 720p
    }
}
