//! Real-time streaming frequency analysis module.
//!
//! This module provides capabilities for live audio analysis:
//! - Frame-by-frame processing for low latency
//! - Rolling window statistics
//! - Event-driven analysis callbacks
//! - Integration with media streaming pipelines
//!
//! # Usage
//!
//! ```rust,ignore
//! use psm_player_frequency::streaming::{StreamAnalyzer, AnalysisEvent};
//!
//! let mut analyzer = StreamAnalyzer::new(44100, 2048);
//!
//! // Register event handler
//! analyzer.on_event(|event| {
//!     match event {
//!         AnalysisEvent::DominantChange { old, new, .. } => {
//!             println!("Dominant frequency changed: {} -> {}", old, new);
//!         }
//!         AnalysisEvent::BeatDetected { timestamp, strength } => {
//!             println!("Beat at {:.2}s (strength: {:.2})", timestamp, strength);
//!         }
//!         _ => {}
//!     }
//! });
//!
//! // Feed audio data
//! let samples: Vec<f32> = vec![0.0; 2048]; // Your audio samples
//! let frames = analyzer.process(&samples);
//! // Use frames for visualization
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tracing::{debug, trace};

use crate::fft::FrequencyAnalyzer;
use crate::types::*;

/// Events emitted during streaming analysis.
#[derive(Debug, Clone)]
pub enum AnalysisEvent {
    /// Dominant frequency changed significantly
    DominantChange {
        old: f32,
        new: f32,
        timestamp: f64,
    },
    /// Beat/onset detected
    BeatDetected {
        timestamp: f64,
        strength: f32,
    },
    /// Spectral shift detected (e.g., song section change)
    SpectralShift {
        timestamp: f64,
        magnitude: f32,
    },
    /// Silence detected
    SilenceStart {
        timestamp: f64,
    },
    /// Silence ended
    SilenceEnd {
        timestamp: f64,
        duration: f64,
    },
    /// New frame analyzed
    FrameAnalyzed {
        timestamp: f64,
        frame: AnalysisFrame,
    },
}

/// Single frame of analysis data.
#[derive(Debug, Clone)]
pub struct AnalysisFrame {
    /// Frame timestamp in seconds
    pub timestamp: f64,
    /// Current dominant frequency
    pub dominant_frequency: f32,
    /// Dominant magnitude (0-1)
    pub dominant_magnitude: f32,
    /// Spectral centroid
    pub spectral_centroid: f32,
    /// Band energies
    pub band_energies: BandEnergies,
    /// RMS energy level
    pub rms_energy: f32,
    /// Zero crossing rate
    pub zcr: f32,
}

/// Configuration for streaming analyzer.
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// FFT window size
    pub fft_size: usize,
    /// Hop size between frames
    pub hop_size: usize,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// History length for rolling statistics (in frames)
    pub history_length: usize,
    /// Silence threshold (RMS energy below this is silence)
    pub silence_threshold: f32,
    /// Beat detection threshold
    pub beat_threshold: f32,
    /// Minimum frequency change to trigger DominantChange event
    pub frequency_change_threshold: f32,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            hop_size: 512,
            sample_rate: 44100,
            history_length: 100,
            silence_threshold: 0.01,
            beat_threshold: 1.5,
            frequency_change_threshold: 50.0, // Hz
        }
    }
}

/// Event callback type.
pub type EventCallback = Box<dyn Fn(AnalysisEvent) + Send + Sync>;

/// Real-time streaming frequency analyzer.
pub struct StreamAnalyzer {
    config: StreamConfig,
    analyzer: FrequencyAnalyzer,
    /// Audio sample buffer
    buffer: VecDeque<f32>,
    /// History of analysis frames
    history: VecDeque<AnalysisFrame>,
    /// Current timestamp in seconds
    current_time: f64,
    /// Previous dominant frequency for change detection
    prev_dominant: f32,
    /// Rolling energy history for beat detection
    energy_history: VecDeque<f32>,
    /// Whether currently in silence
    in_silence: bool,
    /// Silence start timestamp
    silence_start: f64,
    /// Event callbacks
    callbacks: Vec<EventCallback>,
}

impl StreamAnalyzer {
    /// Create a new streaming analyzer with default configuration.
    pub fn new(sample_rate: u32, fft_size: usize) -> Self {
        let config = StreamConfig {
            sample_rate,
            fft_size,
            hop_size: fft_size / 4,
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Create analyzer with custom configuration.
    pub fn with_config(config: StreamConfig) -> Self {
        let analyzer = FrequencyAnalyzer::new(config.fft_size, config.hop_size);

        Self {
            config: config.clone(),
            analyzer,
            buffer: VecDeque::with_capacity(config.fft_size * 2),
            history: VecDeque::with_capacity(config.history_length),
            current_time: 0.0,
            prev_dominant: 0.0,
            energy_history: VecDeque::with_capacity(config.history_length),
            in_silence: false,
            silence_start: 0.0,
            callbacks: Vec::new(),
        }
    }

    /// Register an event callback.
    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(AnalysisEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    /// Process incoming audio samples.
    /// Returns analysis frames if any were generated.
    pub fn process(&mut self, samples: &[f32]) -> Vec<AnalysisFrame> {
        self.buffer.extend(samples);

        let mut frames = Vec::new();

        // Process complete frames
        while self.buffer.len() >= self.config.fft_size {
            // Extract frame
            let frame_samples: Vec<f32> = self.buffer
                .iter()
                .take(self.config.fft_size)
                .copied()
                .collect();

            // Analyze frame
            if let Some(frame) = self.analyze_frame(&frame_samples) {
                self.detect_events(&frame);
                self.update_history(&frame);
                frames.push(frame);
            }

            // Advance buffer by hop size
            for _ in 0..self.config.hop_size {
                self.buffer.pop_front();
            }

            // Update timestamp
            self.current_time += self.config.hop_size as f64 / self.config.sample_rate as f64;
        }

        frames
    }

    /// Analyze a single frame of audio.
    fn analyze_frame(&self, samples: &[f32]) -> Option<AnalysisFrame> {
        let analysis = self.analyzer.analyze(samples, self.config.sample_rate).ok()?;

        // Find dominant frequency
        let (dominant_idx, dominant_mag) = analysis.spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))?;

        let freq_resolution = self.config.sample_rate as f32 / self.config.fft_size as f32;
        let dominant_frequency = dominant_idx as f32 * freq_resolution;

        // Compute RMS energy
        let rms_energy = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

        Some(AnalysisFrame {
            timestamp: self.current_time,
            dominant_frequency,
            dominant_magnitude: *dominant_mag,
            spectral_centroid: analysis.spectral_centroid,
            band_energies: analysis.band_energies,
            rms_energy,
            zcr: analysis.zero_crossing_rate,
        })
    }

    /// Detect events based on frame analysis.
    fn detect_events(&mut self, frame: &AnalysisFrame) {
        // Dominant frequency change
        let freq_diff = (frame.dominant_frequency - self.prev_dominant).abs();
        if freq_diff > self.config.frequency_change_threshold && self.prev_dominant > 0.0 {
            self.emit_event(AnalysisEvent::DominantChange {
                old: self.prev_dominant,
                new: frame.dominant_frequency,
                timestamp: frame.timestamp,
            });
        }
        self.prev_dominant = frame.dominant_frequency;

        // Beat detection
        self.energy_history.push_back(frame.rms_energy);
        if self.energy_history.len() > self.config.history_length {
            self.energy_history.pop_front();
        }

        if self.energy_history.len() >= 10 {
            let avg_energy: f32 = self.energy_history.iter().sum::<f32>() / self.energy_history.len() as f32;
            if frame.rms_energy > avg_energy * self.config.beat_threshold {
                self.emit_event(AnalysisEvent::BeatDetected {
                    timestamp: frame.timestamp,
                    strength: frame.rms_energy / avg_energy,
                });
            }
        }

        // Silence detection
        if frame.rms_energy < self.config.silence_threshold {
            if !self.in_silence {
                self.in_silence = true;
                self.silence_start = frame.timestamp;
                self.emit_event(AnalysisEvent::SilenceStart {
                    timestamp: frame.timestamp,
                });
            }
        } else if self.in_silence {
            self.in_silence = false;
            let duration = frame.timestamp - self.silence_start;
            self.emit_event(AnalysisEvent::SilenceEnd {
                timestamp: frame.timestamp,
                duration,
            });
        }

        // Frame analyzed event
        self.emit_event(AnalysisEvent::FrameAnalyzed {
            timestamp: frame.timestamp,
            frame: frame.clone(),
        });
    }

    /// Update history with new frame.
    fn update_history(&mut self, frame: &AnalysisFrame) {
        self.history.push_back(frame.clone());
        if self.history.len() > self.config.history_length {
            self.history.pop_front();
        }
    }

    /// Emit an event to all registered callbacks.
    fn emit_event(&self, event: AnalysisEvent) {
        trace!("Emitting event: {:?}", event);
        for callback in &self.callbacks {
            callback(event.clone());
        }
    }

    /// Get rolling statistics over the history window.
    pub fn get_statistics(&self) -> StreamStatistics {
        if self.history.is_empty() {
            return StreamStatistics::default();
        }

        let n = self.history.len() as f32;

        // Average dominant frequency
        let avg_dominant: f32 = self.history.iter()
            .map(|f| f.dominant_frequency)
            .sum::<f32>() / n;

        // Average centroid
        let avg_centroid: f32 = self.history.iter()
            .map(|f| f.spectral_centroid)
            .sum::<f32>() / n;

        // Average RMS
        let avg_rms: f32 = self.history.iter()
            .map(|f| f.rms_energy)
            .sum::<f32>() / n;

        // RMS variance
        let rms_variance: f32 = self.history.iter()
            .map(|f| (f.rms_energy - avg_rms).powi(2))
            .sum::<f32>() / n;

        // Dominant frequency variance
        let freq_variance: f32 = self.history.iter()
            .map(|f| (f.dominant_frequency - avg_dominant).powi(2))
            .sum::<f32>() / n;

        // Average band energies
        let mut avg_bands = BandEnergies {
            sub_bass: 0.0, bass: 0.0, low_mid: 0.0,
            mid: 0.0, high_mid: 0.0, high: 0.0,
        };
        for frame in &self.history {
            avg_bands.sub_bass += frame.band_energies.sub_bass;
            avg_bands.bass += frame.band_energies.bass;
            avg_bands.low_mid += frame.band_energies.low_mid;
            avg_bands.mid += frame.band_energies.mid;
            avg_bands.high_mid += frame.band_energies.high_mid;
            avg_bands.high += frame.band_energies.high;
        }
        avg_bands.sub_bass /= n;
        avg_bands.bass /= n;
        avg_bands.low_mid /= n;
        avg_bands.mid /= n;
        avg_bands.high_mid /= n;
        avg_bands.high /= n;

        StreamStatistics {
            window_duration: self.config.history_length as f64
                * self.config.hop_size as f64
                / self.config.sample_rate as f64,
            avg_dominant_frequency: avg_dominant,
            avg_spectral_centroid: avg_centroid,
            avg_rms_energy: avg_rms,
            rms_variance,
            frequency_variance: freq_variance,
            avg_band_energies: avg_bands,
            frame_count: self.history.len(),
        }
    }

    /// Get the most recent frame.
    pub fn current_frame(&self) -> Option<&AnalysisFrame> {
        self.history.back()
    }

    /// Get the current timestamp.
    pub fn current_time(&self) -> f64 {
        self.current_time
    }

    /// Reset the analyzer state.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.history.clear();
        self.energy_history.clear();
        self.current_time = 0.0;
        self.prev_dominant = 0.0;
        self.in_silence = false;
    }
}

/// Rolling statistics over the analysis window.
#[derive(Debug, Clone, Default)]
pub struct StreamStatistics {
    /// Duration of the statistics window in seconds
    pub window_duration: f64,
    /// Average dominant frequency
    pub avg_dominant_frequency: f32,
    /// Average spectral centroid
    pub avg_spectral_centroid: f32,
    /// Average RMS energy
    pub avg_rms_energy: f32,
    /// RMS energy variance
    pub rms_variance: f32,
    /// Dominant frequency variance
    pub frequency_variance: f32,
    /// Average band energies
    pub avg_band_energies: BandEnergies,
    /// Number of frames in the window
    pub frame_count: usize,
}

/// Thread-safe streaming analyzer for async contexts.
pub struct AsyncStreamAnalyzer {
    inner: Arc<Mutex<StreamAnalyzer>>,
}

impl AsyncStreamAnalyzer {
    /// Create a new async streaming analyzer.
    pub fn new(sample_rate: u32, fft_size: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StreamAnalyzer::new(sample_rate, fft_size))),
        }
    }

    /// Process samples asynchronously.
    pub async fn process(&self, samples: &[f32]) -> Vec<AnalysisFrame> {
        let mut analyzer = self.inner.lock().unwrap();
        analyzer.process(samples)
    }

    /// Get current statistics.
    pub fn get_statistics(&self) -> StreamStatistics {
        let analyzer = self.inner.lock().unwrap();
        analyzer.get_statistics()
    }

    /// Reset the analyzer.
    pub fn reset(&self) {
        let mut analyzer = self.inner.lock().unwrap();
        analyzer.reset();
    }
}

impl Clone for AsyncStreamAnalyzer {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn generate_sine(freq: f32, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
        let n = (sample_rate as f32 * duration_secs) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin()
            })
            .collect()
    }

    #[test]
    fn test_stream_analyzer_basic() {
        let mut analyzer = StreamAnalyzer::new(44100, 2048);
        let samples = generate_sine(440.0, 44100, 0.5);

        let frames = analyzer.process(&samples);

        assert!(!frames.is_empty());
        assert!(frames[0].dominant_frequency > 400.0 && frames[0].dominant_frequency < 480.0);
    }

    #[test]
    fn test_event_callbacks() {
        let event_count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&event_count);

        let mut analyzer = StreamAnalyzer::new(44100, 2048);
        analyzer.on_event(move |_event| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let samples = generate_sine(440.0, 44100, 0.5);
        let _ = analyzer.process(&samples);

        assert!(event_count.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn test_statistics() {
        let mut analyzer = StreamAnalyzer::new(44100, 2048);
        let samples = generate_sine(440.0, 44100, 1.0);

        let _ = analyzer.process(&samples);
        let stats = analyzer.get_statistics();

        assert!(stats.frame_count > 0);
        assert!(stats.avg_dominant_frequency > 400.0);
    }

    #[test]
    fn test_silence_detection() {
        let config = StreamConfig {
            sample_rate: 44100,
            fft_size: 1024,
            hop_size: 256,
            silence_threshold: 0.01,
            ..Default::default()
        };

        let mut analyzer = StreamAnalyzer::with_config(config);

        let silence_detected = Arc::new(AtomicUsize::new(0));
        let sd_clone = Arc::clone(&silence_detected);

        analyzer.on_event(move |event| {
            if matches!(event, AnalysisEvent::SilenceStart { .. }) {
                sd_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        // Generate silence (very low amplitude)
        let silence: Vec<f32> = vec![0.0001; 44100];
        let _ = analyzer.process(&silence);

        assert!(silence_detected.load(Ordering::SeqCst) > 0);
    }
}
