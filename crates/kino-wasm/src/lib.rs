//! Kino WASM - WebAssembly Video Player Library
//!
//! Provides high-performance streaming logic for browser integration:
//! - BOLA ABR algorithm (proven 10-25% QoE improvement)
//! - Buffer strategy optimization
//! - Analytics aggregation
//! - Kino Branding
//!
//! ## Integration with hls.js
//!
//! ```javascript
//! import init, { KinoAbrController, KinoBranding } from '@kino/wasm';
//!
//! await init();
//! const abr = new KinoAbrController();
//! ```

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

mod abr_controller;
mod buffer_controller;
mod analytics;
mod branding;
mod frequency;

pub use abr_controller::KinoAbrController;
pub use buffer_controller::KinoBufferController;
pub use analytics::KinoAnalytics;
pub use branding::KinoBranding;
pub use frequency::{
    KinoFrequencyAnalyzer,
    KinoFingerprinter,
    KinoStreamingAnalyzer,
    FrequencyResult,
    RealtimeFrequencyData,
};

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"[Kino WASM] Initialized".into());
}

/// Library version
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Configuration for the WASM player
#[wasm_bindgen]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    /// ABR algorithm: "throughput", "bola", or "hybrid"
    abr_algorithm: String,
    /// Minimum buffer before playback (seconds)
    pub min_buffer_time: f64,
    /// Maximum buffer level (seconds)
    pub max_buffer_time: f64,
    /// Enable analytics collection
    pub analytics_enabled: bool,
    /// Maximum bitrate cap (0 = unlimited)
    pub max_bitrate: u32,
    /// Start at lowest quality
    pub start_at_lowest: bool,
}

#[wasm_bindgen]
impl WasmConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            abr_algorithm: "bola".to_string(),
            min_buffer_time: 10.0,
            max_buffer_time: 30.0,
            analytics_enabled: true,
            max_bitrate: 0,
            start_at_lowest: false,
        }
    }

    /// Get ABR algorithm
    #[wasm_bindgen(getter)]
    pub fn abr_algorithm(&self) -> String {
        self.abr_algorithm.clone()
    }

    /// Set ABR algorithm
    #[wasm_bindgen(setter)]
    pub fn set_abr_algorithm(&mut self, algorithm: String) {
        self.abr_algorithm = algorithm;
    }

    /// Create config optimized for low-latency live streaming
    #[wasm_bindgen]
    pub fn low_latency() -> Self {
        Self {
            abr_algorithm: "throughput".to_string(),
            min_buffer_time: 2.0,
            max_buffer_time: 6.0,
            analytics_enabled: true,
            max_bitrate: 0,
            start_at_lowest: true,
        }
    }

    /// Create config optimized for VOD
    #[wasm_bindgen]
    pub fn vod() -> Self {
        Self {
            abr_algorithm: "bola".to_string(),
            min_buffer_time: 15.0,
            max_buffer_time: 60.0,
            analytics_enabled: true,
            max_bitrate: 0,
            start_at_lowest: false,
        }
    }
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self::new()
    }
}
