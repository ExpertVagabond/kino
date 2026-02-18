//! Buffer Controller - Intelligent buffer management for WASM
//!
//! Provides buffer strategy recommendations to complement MSE/hls.js.

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

/// Buffer state information
#[wasm_bindgen]
#[derive(Clone, Serialize, Deserialize)]
pub struct BufferState {
    /// Current buffer level in seconds
    pub level: f64,
    /// Target buffer level
    pub target: f64,
    /// Is buffer healthy for playback
    pub healthy: bool,
    /// Should fetch more data
    pub needs_data: bool,
    /// Recommended action
    action: String,
}

#[wasm_bindgen]
impl BufferState {
    #[wasm_bindgen(getter)]
    pub fn action(&self) -> String {
        self.action.clone()
    }
}

/// Buffer management controller
#[wasm_bindgen]
pub struct KinoBufferController {
    /// Minimum buffer before playback starts
    min_buffer: f64,
    /// Target buffer level
    target_buffer: f64,
    /// Maximum buffer level
    max_buffer: f64,
    /// Rebuffer threshold
    rebuffer_threshold: f64,
    /// Current playback position
    position: f64,
    /// Content duration
    duration: f64,
    /// Is live content
    is_live: bool,
    /// Stall count
    stall_count: u32,
}

#[wasm_bindgen]
impl KinoBufferController {
    /// Create a new buffer controller
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            min_buffer: 10.0,
            target_buffer: 20.0,
            max_buffer: 30.0,
            rebuffer_threshold: 2.0,
            position: 0.0,
            duration: 0.0,
            is_live: false,
            stall_count: 0,
        }
    }

    /// Configure for VOD content
    #[wasm_bindgen]
    pub fn configure_vod(&mut self, duration: f64) {
        self.duration = duration;
        self.is_live = false;
        self.min_buffer = 15.0;
        self.target_buffer = 30.0;
        self.max_buffer = 60.0;
    }

    /// Configure for live content
    #[wasm_bindgen]
    pub fn configure_live(&mut self, target_latency: f64) {
        self.is_live = true;
        self.min_buffer = target_latency * 0.5;
        self.target_buffer = target_latency;
        self.max_buffer = target_latency * 2.0;
    }

    /// Update current playback position
    #[wasm_bindgen]
    pub fn update_position(&mut self, position: f64) {
        self.position = position;
    }

    /// Get buffer state and recommendations
    #[wasm_bindgen]
    pub fn get_state(&self, buffer_level: f64) -> BufferState {
        let healthy = buffer_level >= self.rebuffer_threshold;
        let needs_data = buffer_level < self.max_buffer;

        let action = if buffer_level < self.rebuffer_threshold {
            "pause_and_buffer".to_string()
        } else if buffer_level < self.min_buffer {
            "buffer_aggressively".to_string()
        } else if buffer_level < self.target_buffer {
            "buffer_normal".to_string()
        } else if buffer_level >= self.max_buffer {
            "stop_buffering".to_string()
        } else {
            "maintain".to_string()
        };

        BufferState {
            level: buffer_level,
            target: self.target_buffer,
            healthy,
            needs_data,
            action,
        }
    }

    /// Check if playback can start
    #[wasm_bindgen]
    pub fn can_start_playback(&self, buffer_level: f64) -> bool {
        buffer_level >= self.min_buffer
    }

    /// Report a stall event
    #[wasm_bindgen]
    pub fn report_stall(&mut self) {
        self.stall_count += 1;

        // Adaptive: increase buffer thresholds after stalls
        if self.stall_count >= 2 {
            self.min_buffer = (self.min_buffer * 1.2).min(20.0);
            self.rebuffer_threshold = (self.rebuffer_threshold * 1.1).min(5.0);
        }
    }

    /// Get number of stalls
    #[wasm_bindgen]
    pub fn get_stall_count(&self) -> u32 {
        self.stall_count
    }

    /// Calculate optimal prefetch count
    #[wasm_bindgen]
    pub fn get_prefetch_count(&self, segment_duration: f64) -> u32 {
        let segments_to_target = (self.target_buffer / segment_duration).ceil() as u32;
        segments_to_target.min(5) // Cap at 5 segments ahead
    }

    /// Get time until buffer depletes at current rate
    #[wasm_bindgen]
    pub fn time_to_empty(&self, buffer_level: f64, playback_rate: f64) -> f64 {
        if playback_rate <= 0.0 {
            return f64::INFINITY;
        }
        buffer_level / playback_rate
    }

    /// Reset controller state
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.position = 0.0;
        self.stall_count = 0;
    }
}

impl Default for KinoBufferController {
    fn default() -> Self {
        Self::new()
    }
}
