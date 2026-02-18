//! WebAssembly bindings for frequency analysis
//!
//! Provides high-performance FFT analysis in the browser:
//! - Real-time frequency analysis
//! - Dominant frequency detection
//! - Spectral feature extraction
//! - Audio fingerprinting
//!
//! ## JavaScript Integration
//!
//! ```javascript
//! import { KinoFrequencyAnalyzer } from '@kino/wasm';
//!
//! const analyzer = new KinoFrequencyAnalyzer(2048);
//!
//! // Analyze audio samples
//! const samples = new Float32Array([...]);
//! const result = analyzer.analyze(samples, 44100);
//!
//! console.log('Dominant:', result.dominant_frequencies);
//! console.log('Centroid:', result.spectral_centroid);
//! ```

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use js_sys::{Float32Array, Array};

// ============================================================================
// Core FFT Implementation (no Tokio - WASM compatible)
// ============================================================================

/// FFT Analyzer for WASM
struct FftAnalyzer {
    fft_size: usize,
    window: Vec<f32>,
}

impl FftAnalyzer {
    fn new(fft_size: usize) -> Self {
        // Generate Hann window
        let window: Vec<f32> = (0..fft_size)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos())
            })
            .collect();

        Self { fft_size, window }
    }

    fn compute_spectrum(&self, samples: &[f32]) -> Vec<f32> {
        if samples.len() < self.fft_size {
            return vec![0.0; self.fft_size / 2];
        }

        // Apply window
        let windowed: Vec<f32> = samples.iter()
            .take(self.fft_size)
            .zip(self.window.iter())
            .map(|(&s, &w)| s * w)
            .collect();

        // Simple DFT (for WASM we avoid complex FFT library dependencies)
        // In production, use web-sys AudioContext.createAnalyser()
        let mut spectrum = vec![0.0f32; self.fft_size / 2];
        let n = self.fft_size as f32;

        for k in 0..self.fft_size / 2 {
            let mut real = 0.0f32;
            let mut imag = 0.0f32;

            for (i, &sample) in windowed.iter().enumerate() {
                let angle = 2.0 * std::f32::consts::PI * k as f32 * i as f32 / n;
                real += sample * angle.cos();
                imag -= sample * angle.sin();
            }

            spectrum[k] = (real * real + imag * imag).sqrt() * 2.0 / n;
        }

        spectrum
    }
}

// ============================================================================
// WASM Bindings
// ============================================================================

/// Frequency analysis result
#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct FrequencyResult {
    dominant_frequencies: Vec<DominantFreq>,
    spectral_centroid: f32,
    spectral_rolloff: f32,
    spectral_flatness: f32,
    band_energies: BandEnergies,
}

#[derive(Serialize, Deserialize, Clone)]
struct DominantFreq {
    frequency_hz: f32,
    magnitude: f32,
    rank: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct BandEnergies {
    sub_bass: f32,
    bass: f32,
    low_mid: f32,
    mid: f32,
    high_mid: f32,
    high: f32,
}

#[wasm_bindgen]
impl FrequencyResult {
    #[wasm_bindgen(getter)]
    pub fn spectral_centroid(&self) -> f32 {
        self.spectral_centroid
    }

    #[wasm_bindgen(getter)]
    pub fn spectral_rolloff(&self) -> f32 {
        self.spectral_rolloff
    }

    #[wasm_bindgen(getter)]
    pub fn spectral_flatness(&self) -> f32 {
        self.spectral_flatness
    }

    /// Get dominant frequencies as JSON
    #[wasm_bindgen]
    pub fn get_dominant_json(&self) -> String {
        serde_json::to_string(&self.dominant_frequencies).unwrap_or_default()
    }

    /// Get band energies as JSON
    #[wasm_bindgen]
    pub fn get_band_energies_json(&self) -> String {
        serde_json::to_string(&self.band_energies).unwrap_or_default()
    }
}

/// High-performance frequency analyzer for WASM
#[wasm_bindgen]
pub struct KinoFrequencyAnalyzer {
    fft_size: usize,
    analyzer: FftAnalyzer,
}

#[wasm_bindgen]
impl KinoFrequencyAnalyzer {
    /// Create a new frequency analyzer
    #[wasm_bindgen(constructor)]
    pub fn new(fft_size: usize) -> Self {
        let fft_size = fft_size.max(256).min(8192);
        Self {
            fft_size,
            analyzer: FftAnalyzer::new(fft_size),
        }
    }

    /// Analyze audio samples and return frequency data
    #[wasm_bindgen]
    pub fn analyze(&self, samples: &Float32Array, sample_rate: u32) -> FrequencyResult {
        let samples_vec: Vec<f32> = samples.to_vec();

        if samples_vec.len() < self.fft_size {
            return FrequencyResult {
                dominant_frequencies: Vec::new(),
                spectral_centroid: 0.0,
                spectral_rolloff: 0.0,
                spectral_flatness: 0.0,
                band_energies: BandEnergies {
                    sub_bass: 0.0,
                    bass: 0.0,
                    low_mid: 0.0,
                    mid: 0.0,
                    high_mid: 0.0,
                    high: 0.0,
                },
            };
        }

        let spectrum = self.analyzer.compute_spectrum(&samples_vec);
        let freq_resolution = sample_rate as f32 / self.fft_size as f32;

        // Find dominant frequencies
        let mut indexed: Vec<(usize, f32)> = spectrum.iter()
            .enumerate()
            .map(|(i, &m)| (i, m))
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let max_mag = indexed.first().map(|(_, m)| *m).unwrap_or(1.0);
        let dominant_frequencies: Vec<DominantFreq> = indexed.iter()
            .take(10)
            .enumerate()
            .map(|(rank, (idx, mag))| DominantFreq {
                frequency_hz: *idx as f32 * freq_resolution,
                magnitude: mag / max_mag,
                rank: rank + 1,
            })
            .collect();

        // Compute spectral features
        let frequencies: Vec<f32> = (0..spectrum.len())
            .map(|i| i as f32 * freq_resolution)
            .collect();

        let spectral_centroid = self.compute_centroid(&spectrum, &frequencies);
        let spectral_rolloff = self.compute_rolloff(&spectrum, &frequencies, 0.95);
        let spectral_flatness = self.compute_flatness(&spectrum);
        let band_energies = self.compute_band_energies(&spectrum, &frequencies);

        FrequencyResult {
            dominant_frequencies,
            spectral_centroid,
            spectral_rolloff,
            spectral_flatness,
            band_energies,
        }
    }

    /// Get the magnitude spectrum as a Float32Array
    #[wasm_bindgen]
    pub fn get_spectrum(&self, samples: &Float32Array) -> Float32Array {
        let samples_vec: Vec<f32> = samples.to_vec();
        let spectrum = self.analyzer.compute_spectrum(&samples_vec);
        Float32Array::from(&spectrum[..])
    }

    /// Get dominant frequencies as JavaScript array
    #[wasm_bindgen]
    pub fn get_dominant(&self, samples: &Float32Array, sample_rate: u32, top_k: usize) -> Array {
        let result = self.analyze(samples, sample_rate);
        let array = Array::new();

        for freq in result.dominant_frequencies.iter().take(top_k) {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"frequencyHz".into(), &freq.frequency_hz.into()).ok();
            js_sys::Reflect::set(&obj, &"magnitude".into(), &freq.magnitude.into()).ok();
            js_sys::Reflect::set(&obj, &"rank".into(), &(freq.rank as u32).into()).ok();
            array.push(&obj);
        }

        array
    }

    fn compute_centroid(&self, spectrum: &[f32], frequencies: &[f32]) -> f32 {
        let weighted_sum: f32 = spectrum.iter()
            .zip(frequencies.iter())
            .map(|(&m, &f)| m * f)
            .sum();
        let total: f32 = spectrum.iter().sum();
        if total > 0.0 { weighted_sum / total } else { 0.0 }
    }

    fn compute_rolloff(&self, spectrum: &[f32], frequencies: &[f32], threshold: f32) -> f32 {
        let total: f32 = spectrum.iter().sum();
        let target = total * threshold;
        let mut cumulative = 0.0f32;

        for (i, &mag) in spectrum.iter().enumerate() {
            cumulative += mag;
            if cumulative >= target {
                return frequencies.get(i).copied().unwrap_or(0.0);
            }
        }

        frequencies.last().copied().unwrap_or(0.0)
    }

    fn compute_flatness(&self, spectrum: &[f32]) -> f32 {
        let n = spectrum.len() as f32;
        let log_sum: f32 = spectrum.iter()
            .map(|&x| (x + 1e-10).ln())
            .sum();
        let geometric_mean = (log_sum / n).exp();
        let arithmetic_mean: f32 = spectrum.iter().sum::<f32>() / n;

        if arithmetic_mean > 0.0 {
            geometric_mean / arithmetic_mean
        } else {
            0.0
        }
    }

    fn compute_band_energies(&self, spectrum: &[f32], frequencies: &[f32]) -> BandEnergies {
        let bands = [
            (20.0, 60.0),
            (60.0, 250.0),
            (250.0, 500.0),
            (500.0, 2000.0),
            (2000.0, 4000.0),
            (4000.0, 20000.0),
        ];

        let mut energies = [0.0f32; 6];

        for (i, (low, high)) in bands.iter().enumerate() {
            for (j, &freq) in frequencies.iter().enumerate() {
                if freq >= *low && freq < *high {
                    energies[i] += spectrum[j];
                }
            }
        }

        let total: f32 = energies.iter().sum();
        if total > 0.0 {
            for e in &mut energies {
                *e /= total;
            }
        }

        BandEnergies {
            sub_bass: energies[0],
            bass: energies[1],
            low_mid: energies[2],
            mid: energies[3],
            high_mid: energies[4],
            high: energies[5],
        }
    }
}

/// Fingerprint generator for WASM
#[wasm_bindgen]
pub struct KinoFingerprinter {
    fft_size: usize,
    hop_size: usize,
}

#[wasm_bindgen]
impl KinoFingerprinter {
    /// Create a new fingerprinter
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            fft_size: 4096,
            hop_size: 2048,
        }
    }

    /// Generate a fingerprint hash from audio samples
    #[wasm_bindgen]
    pub fn fingerprint(&self, samples: &Float32Array, _sample_rate: u32) -> String {
        let samples_vec: Vec<f32> = samples.to_vec();

        if samples_vec.len() < self.fft_size {
            return String::new();
        }

        // Simple hash based on spectral peaks
        let analyzer = FftAnalyzer::new(self.fft_size);
        let mut hash_data = Vec::new();

        let num_frames = (samples_vec.len() - self.fft_size) / self.hop_size + 1;

        for frame_idx in 0..num_frames.min(100) {
            let start = frame_idx * self.hop_size;
            let frame = &samples_vec[start..start + self.fft_size];
            let spectrum = analyzer.compute_spectrum(frame);

            // Find peaks in 6 bands
            let bands = [0, 10, 20, 40, 80, 160, 256];
            for b in 0..6 {
                let band_start = bands[b];
                let band_end = bands[b + 1].min(spectrum.len());
                if band_start < band_end {
                    let peak_idx = spectrum[band_start..band_end]
                        .iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    hash_data.push((band_start + peak_idx) as u8);
                }
            }
        }

        // Simple hash (in production, use proper SHA-256)
        let hash: u64 = hash_data.iter()
            .enumerate()
            .fold(0u64, |acc, (i, &b)| {
                acc.wrapping_add((b as u64).wrapping_mul(31u64.pow(i as u32)))
            });

        format!("{:016x}", hash)
    }

    /// Compare two fingerprints for similarity
    #[wasm_bindgen]
    pub fn compare(&self, hash1: &str, hash2: &str) -> f32 {
        if hash1.len() != hash2.len() {
            return 0.0;
        }

        let matching: usize = hash1.chars()
            .zip(hash2.chars())
            .filter(|(a, b)| a == b)
            .count();

        matching as f32 / hash1.len() as f32
    }
}

impl Default for KinoFingerprinter {
    fn default() -> Self {
        Self::new()
    }
}

/// Real-time frequency data for visualization
#[wasm_bindgen]
pub struct RealtimeFrequencyData {
    spectrum: Vec<f32>,
    band_energies: [f32; 6],
    dominant_freq: f32,
    centroid: f32,
}

#[wasm_bindgen]
impl RealtimeFrequencyData {
    #[wasm_bindgen(getter)]
    pub fn dominant_frequency(&self) -> f32 {
        self.dominant_freq
    }

    #[wasm_bindgen(getter)]
    pub fn spectral_centroid(&self) -> f32 {
        self.centroid
    }

    #[wasm_bindgen]
    pub fn get_spectrum(&self) -> Float32Array {
        Float32Array::from(&self.spectrum[..])
    }

    #[wasm_bindgen]
    pub fn get_band_energy(&self, band: usize) -> f32 {
        self.band_energies.get(band).copied().unwrap_or(0.0)
    }
}

/// Streaming analyzer for real-time use
#[wasm_bindgen]
pub struct KinoStreamingAnalyzer {
    fft_size: usize,
    buffer: Vec<f32>,
    analyzer: FftAnalyzer,
    sample_rate: u32,
}

#[wasm_bindgen]
impl KinoStreamingAnalyzer {
    #[wasm_bindgen(constructor)]
    pub fn new(fft_size: usize, sample_rate: u32) -> Self {
        let fft_size = fft_size.max(256).min(8192);
        Self {
            fft_size,
            buffer: Vec::with_capacity(fft_size * 2),
            analyzer: FftAnalyzer::new(fft_size),
            sample_rate,
        }
    }

    /// Push samples and get analysis if ready
    #[wasm_bindgen]
    pub fn push(&mut self, samples: &Float32Array) -> Option<RealtimeFrequencyData> {
        self.buffer.extend(samples.to_vec());

        if self.buffer.len() >= self.fft_size {
            let spectrum = self.analyzer.compute_spectrum(&self.buffer[..self.fft_size]);

            // Compute features
            let freq_resolution = self.sample_rate as f32 / self.fft_size as f32;
            let frequencies: Vec<f32> = (0..spectrum.len())
                .map(|i| i as f32 * freq_resolution)
                .collect();

            // Dominant frequency
            let dominant_idx = spectrum.iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);
            let dominant_freq = dominant_idx as f32 * freq_resolution;

            // Centroid
            let weighted: f32 = spectrum.iter().zip(frequencies.iter())
                .map(|(&m, &f)| m * f).sum();
            let total: f32 = spectrum.iter().sum();
            let centroid = if total > 0.0 { weighted / total } else { 0.0 };

            // Band energies
            let bands = [
                (20.0, 60.0), (60.0, 250.0), (250.0, 500.0),
                (500.0, 2000.0), (2000.0, 4000.0), (4000.0, 20000.0),
            ];
            let mut band_energies = [0.0f32; 6];
            for (i, (low, high)) in bands.iter().enumerate() {
                for (j, &freq) in frequencies.iter().enumerate() {
                    if freq >= *low && freq < *high {
                        band_energies[i] += spectrum[j];
                    }
                }
            }
            let band_total: f32 = band_energies.iter().sum();
            if band_total > 0.0 {
                for e in &mut band_energies {
                    *e /= band_total;
                }
            }

            // Keep overlap
            let drain = self.buffer.len() - self.fft_size / 2;
            self.buffer.drain(0..drain);

            Some(RealtimeFrequencyData {
                spectrum,
                band_energies,
                dominant_freq,
                centroid,
            })
        } else {
            None
        }
    }

    /// Reset the analyzer buffer
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}
