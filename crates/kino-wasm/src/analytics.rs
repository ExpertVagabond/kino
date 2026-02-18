//! Analytics - QoE metrics and event tracking for WASM
//!
//! Collects and aggregates playback metrics for quality analysis.

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

/// Quality of Experience breakdown
#[wasm_bindgen]
#[derive(Clone, Serialize, Deserialize)]
pub struct QoeMetrics {
    /// Overall QoE score (0-100)
    pub score: f64,
    /// Time to first frame (startup delay)
    pub startup_time_ms: f64,
    /// Number of rebuffer events
    pub rebuffer_count: u32,
    /// Total rebuffer duration in seconds
    pub rebuffer_duration: f64,
    /// Number of quality switches
    pub quality_switches: u32,
    /// Average bitrate played
    pub avg_bitrate: u32,
    /// Percentage of time spent at highest quality
    pub high_quality_ratio: f64,
    /// Total watch time in seconds
    pub watch_time: f64,
}

#[wasm_bindgen]
impl QoeMetrics {
    /// Convert to JSON string
    #[wasm_bindgen]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Bitrate sample for averaging
#[derive(Clone)]
struct BitrateSample {
    bitrate: u32,
    duration: f64,
}

/// Analytics collector and QoE calculator
#[wasm_bindgen]
pub struct KinoAnalytics {
    /// Session start time
    session_start: f64,
    /// Time to first frame
    startup_time_ms: Option<f64>,
    /// Rebuffer events
    rebuffer_count: u32,
    /// Total rebuffer duration
    rebuffer_duration: f64,
    /// Current rebuffer start (if rebuffering)
    rebuffer_start: Option<f64>,
    /// Quality switches
    quality_switches: u32,
    /// Last bitrate
    last_bitrate: Option<u32>,
    /// Bitrate samples for averaging
    bitrate_samples: VecDeque<BitrateSample>,
    /// Highest available bitrate
    max_available_bitrate: u32,
    /// Time spent at max quality
    max_quality_time: f64,
    /// Total playback time
    total_play_time: f64,
    /// Last position update
    last_position: f64,
    /// Is currently playing
    is_playing: bool,
    /// Event log
    events: VecDeque<AnalyticsEvent>,
    /// Max events to keep
    max_events: usize,
}

#[derive(Clone, Serialize, Deserialize)]
struct AnalyticsEvent {
    event_type: String,
    timestamp: f64,
    data: serde_json::Value,
}

#[wasm_bindgen]
impl KinoAnalytics {
    /// Create a new analytics collector
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            session_start: js_sys::Date::now(),
            startup_time_ms: None,
            rebuffer_count: 0,
            rebuffer_duration: 0.0,
            rebuffer_start: None,
            quality_switches: 0,
            last_bitrate: None,
            bitrate_samples: VecDeque::new(),
            max_available_bitrate: 0,
            max_quality_time: 0.0,
            total_play_time: 0.0,
            last_position: 0.0,
            is_playing: false,
            events: VecDeque::new(),
            max_events: 1000,
        }
    }

    /// Report first frame rendered (startup complete)
    #[wasm_bindgen]
    pub fn report_first_frame(&mut self) {
        if self.startup_time_ms.is_none() {
            self.startup_time_ms = Some(js_sys::Date::now() - self.session_start);
            self.log_event("first_frame", serde_json::json!({
                "startup_ms": self.startup_time_ms
            }));
        }
    }

    /// Report play event
    #[wasm_bindgen]
    pub fn report_play(&mut self, position: f64) {
        self.is_playing = true;
        self.last_position = position;
        self.log_event("play", serde_json::json!({ "position": position }));
    }

    /// Report pause event
    #[wasm_bindgen]
    pub fn report_pause(&mut self, position: f64) {
        self.is_playing = false;
        self.log_event("pause", serde_json::json!({ "position": position }));
    }

    /// Report rebuffer start
    #[wasm_bindgen]
    pub fn report_rebuffer_start(&mut self, position: f64) {
        if self.rebuffer_start.is_none() {
            self.rebuffer_start = Some(js_sys::Date::now());
            self.rebuffer_count += 1;
            self.log_event("rebuffer_start", serde_json::json!({ "position": position }));
        }
    }

    /// Report rebuffer end
    #[wasm_bindgen]
    pub fn report_rebuffer_end(&mut self, position: f64) {
        if let Some(start) = self.rebuffer_start.take() {
            let duration = (js_sys::Date::now() - start) / 1000.0;
            self.rebuffer_duration += duration;
            self.log_event("rebuffer_end", serde_json::json!({
                "position": position,
                "duration_s": duration
            }));
        }
    }

    /// Report quality change
    #[wasm_bindgen]
    pub fn report_quality_change(&mut self, new_bitrate: u32, position: f64) {
        if let Some(old) = self.last_bitrate {
            if old != new_bitrate {
                self.quality_switches += 1;
                self.log_event("quality_change", serde_json::json!({
                    "from": old,
                    "to": new_bitrate,
                    "position": position
                }));
            }
        }
        self.last_bitrate = Some(new_bitrate);
    }

    /// Report current bitrate (for averaging)
    #[wasm_bindgen]
    pub fn report_bitrate_sample(&mut self, bitrate: u32, duration: f64) {
        self.bitrate_samples.push_back(BitrateSample { bitrate, duration });

        // Update max quality time
        if bitrate >= self.max_available_bitrate && self.max_available_bitrate > 0 {
            self.max_quality_time += duration;
        }
        self.total_play_time += duration;
    }

    /// Set the available quality levels (for high quality ratio)
    #[wasm_bindgen]
    pub fn set_available_qualities(&mut self, max_bitrate: u32) {
        self.max_available_bitrate = max_bitrate;
    }

    /// Report seek event
    #[wasm_bindgen]
    pub fn report_seek(&mut self, from: f64, to: f64) {
        self.log_event("seek", serde_json::json!({
            "from": from,
            "to": to
        }));
    }

    /// Report error
    #[wasm_bindgen]
    pub fn report_error(&mut self, code: &str, message: &str, fatal: bool) {
        self.log_event("error", serde_json::json!({
            "code": code,
            "message": message,
            "fatal": fatal
        }));
    }

    /// Calculate and return QoE metrics
    #[wasm_bindgen]
    pub fn get_qoe(&self) -> QoeMetrics {
        let avg_bitrate = self.calculate_avg_bitrate();
        let watch_time = (js_sys::Date::now() - self.session_start) / 1000.0;
        let high_quality_ratio = if self.total_play_time > 0.0 {
            self.max_quality_time / self.total_play_time
        } else {
            0.0
        };

        let score = self.calculate_qoe_score(watch_time, avg_bitrate);

        QoeMetrics {
            score,
            startup_time_ms: self.startup_time_ms.unwrap_or(0.0),
            rebuffer_count: self.rebuffer_count,
            rebuffer_duration: self.rebuffer_duration,
            quality_switches: self.quality_switches,
            avg_bitrate,
            high_quality_ratio,
            watch_time,
        }
    }

    /// Get events as JSON array
    #[wasm_bindgen]
    pub fn get_events_json(&self) -> String {
        serde_json::to_string(&self.events.iter().collect::<Vec<_>>()).unwrap_or("[]".to_string())
    }

    /// Get event count
    #[wasm_bindgen]
    pub fn get_event_count(&self) -> usize {
        self.events.len()
    }

    /// Reset analytics
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl KinoAnalytics {
    fn calculate_avg_bitrate(&self) -> u32 {
        if self.bitrate_samples.is_empty() {
            return 0;
        }

        let total_duration: f64 = self.bitrate_samples.iter().map(|s| s.duration).sum();
        if total_duration <= 0.0 {
            return 0;
        }

        let weighted_sum: f64 = self.bitrate_samples
            .iter()
            .map(|s| s.bitrate as f64 * s.duration)
            .sum();

        (weighted_sum / total_duration) as u32
    }

    fn calculate_qoe_score(&self, _watch_time: f64, avg_bitrate: u32) -> f64 {
        let mut score = 100.0;

        // Penalize startup time (target < 2s)
        if let Some(startup) = self.startup_time_ms {
            if startup > 2000.0 {
                score -= ((startup - 2000.0) / 1000.0) * 5.0;
            }
        }

        // Penalize rebuffers heavily (-10 per rebuffer)
        score -= self.rebuffer_count as f64 * 10.0;

        // Penalize rebuffer duration (-5 per second)
        score -= self.rebuffer_duration * 5.0;

        // Penalize quality switches (-2 per switch)
        score -= self.quality_switches as f64 * 2.0;

        // Bonus for high average bitrate
        if avg_bitrate > 5_000_000 {
            score += 5.0;
        } else if avg_bitrate > 2_500_000 {
            score += 2.0;
        }

        score.clamp(0.0, 100.0)
    }

    fn log_event(&mut self, event_type: &str, data: serde_json::Value) {
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }

        self.events.push_back(AnalyticsEvent {
            event_type: event_type.to_string(),
            timestamp: js_sys::Date::now(),
            data,
        });
    }
}

impl Default for KinoAnalytics {
    fn default() -> Self {
        Self::new()
    }
}
