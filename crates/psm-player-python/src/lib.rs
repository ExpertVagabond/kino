//! Python bindings for PSM Player frequency analysis.
//!
//! This module provides Python bindings using PyO3 for:
//! - Audio frequency analysis
//! - Fingerprint generation
//! - Auto-tagging
//! - Recommendation similarity
//!
//! ## Installation
//!
//! ```bash
//! pip install psm-frequency
//! ```
//!
//! ## Usage
//!
//! ```python
//! import numpy as np
//! from psm_frequency import FrequencyAnalyzer, Fingerprinter, ContentTagger
//!
//! # Load audio samples
//! samples = np.array([...], dtype=np.float32)
//! sample_rate = 44100
//!
//! # Analyze frequencies
//! analyzer = FrequencyAnalyzer(sample_rate)
//! result = analyzer.analyze(samples)
//!
//! print(f"Dominant frequency: {result.dominant_frequencies[0].frequency_hz} Hz")
//! print(f"Spectral centroid: {result.spectral_centroid} Hz")
//!
//! # Generate fingerprint
//! fingerprinter = Fingerprinter()
//! fingerprint = fingerprinter.fingerprint(samples, sample_rate)
//! print(f"Fingerprint hash: {fingerprint.hash}")
//!
//! # Auto-tag content
//! tagger = ContentTagger()
//! tags = tagger.predict(samples, sample_rate)
//! for tag in tags:
//!     print(f"{tag.label}: {tag.confidence:.2%}")
//! ```

use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use pyo3::types::PyDict;

// ============================================================================
// Data Types
// ============================================================================

/// Dominant frequency result
#[pyclass]
#[derive(Clone)]
pub struct DominantFrequency {
    #[pyo3(get)]
    pub frequency_hz: f32,
    #[pyo3(get)]
    pub magnitude: f32,
    #[pyo3(get)]
    pub rank: usize,
}

/// Band energy distribution
#[pyclass]
#[derive(Clone)]
pub struct BandEnergies {
    #[pyo3(get)]
    pub sub_bass: f32,
    #[pyo3(get)]
    pub bass: f32,
    #[pyo3(get)]
    pub low_mid: f32,
    #[pyo3(get)]
    pub mid: f32,
    #[pyo3(get)]
    pub high_mid: f32,
    #[pyo3(get)]
    pub high: f32,
}

#[pymethods]
impl BandEnergies {
    fn to_list(&self) -> Vec<f32> {
        vec![
            self.sub_bass,
            self.bass,
            self.low_mid,
            self.mid,
            self.high_mid,
            self.high,
        ]
    }

    fn __repr__(&self) -> String {
        format!(
            "BandEnergies(sub_bass={:.3}, bass={:.3}, low_mid={:.3}, mid={:.3}, high_mid={:.3}, high={:.3})",
            self.sub_bass, self.bass, self.low_mid, self.mid, self.high_mid, self.high
        )
    }
}

/// Frequency analysis result
#[pyclass]
pub struct AnalysisResult {
    #[pyo3(get)]
    pub dominant_frequencies: Vec<DominantFrequency>,
    #[pyo3(get)]
    pub spectral_centroid: f32,
    #[pyo3(get)]
    pub spectral_rolloff: f32,
    #[pyo3(get)]
    pub spectral_flatness: f32,
    #[pyo3(get)]
    pub zero_crossing_rate: f32,
    #[pyo3(get)]
    pub band_energies: BandEnergies,
}

/// Audio fingerprint
#[pyclass]
#[derive(Clone)]
pub struct Fingerprint {
    #[pyo3(get)]
    pub hash: String,
    #[pyo3(get)]
    pub version: u32,
    #[pyo3(get)]
    pub duration_secs: f64,
    #[pyo3(get)]
    pub num_points: usize,
}

#[pymethods]
impl Fingerprint {
    fn __repr__(&self) -> String {
        format!(
            "Fingerprint(hash='{}...', duration={:.2}s, points={})",
            &self.hash[..16.min(self.hash.len())],
            self.duration_secs,
            self.num_points
        )
    }
}

/// Content tag
#[pyclass]
#[derive(Clone)]
pub struct ContentTag {
    #[pyo3(get)]
    pub label: String,
    #[pyo3(get)]
    pub confidence: f32,
}

#[pymethods]
impl ContentTag {
    fn __repr__(&self) -> String {
        format!("ContentTag('{}', confidence={:.2})", self.label, self.confidence)
    }
}

/// Frequency signature for similarity
#[pyclass]
#[derive(Clone)]
pub struct FrequencySignature {
    features: Vec<f32>,
    #[pyo3(get)]
    pub centroid: f32,
    #[pyo3(get)]
    pub flatness: f32,
}

#[pymethods]
impl FrequencySignature {
    /// Get the feature vector as a numpy array
    fn get_features<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        PyArray1::from_slice_bound(py, &self.features)
    }

    /// Compute similarity with another signature
    fn similarity(&self, other: &FrequencySignature) -> f32 {
        if self.features.len() != other.features.len() {
            return 0.0;
        }

        let dot: f32 = self.features.iter()
            .zip(other.features.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f32 = self.features.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.features.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }
}

// ============================================================================
// Main Classes
// ============================================================================

/// Frequency analyzer for audio data
#[pyclass]
pub struct FrequencyAnalyzer {
    sample_rate: u32,
    fft_size: usize,
    hop_size: usize,
}

#[pymethods]
impl FrequencyAnalyzer {
    /// Create a new frequency analyzer
    #[new]
    #[pyo3(signature = (sample_rate, fft_size=4096, hop_size=2048))]
    pub fn new(sample_rate: u32, fft_size: usize, hop_size: usize) -> Self {
        Self {
            sample_rate,
            fft_size,
            hop_size,
        }
    }

    /// Analyze audio samples
    pub fn analyze(&self, samples: PyReadonlyArray1<f32>) -> PyResult<AnalysisResult> {
        let samples_slice = samples.as_slice()?;

        if samples_slice.len() < self.fft_size {
            return Err(pyo3::exceptions::PyValueError::new_err(
                format!("Need at least {} samples, got {}", self.fft_size, samples_slice.len())
            ));
        }

        // Compute spectrum using simple DFT (in production, use proper FFT)
        let spectrum = self.compute_spectrum(samples_slice);
        let freq_resolution = self.sample_rate as f32 / self.fft_size as f32;

        // Find dominant frequencies
        let mut indexed: Vec<(usize, f32)> = spectrum.iter()
            .enumerate()
            .map(|(i, &m)| (i, m))
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let max_mag = indexed.first().map(|(_, m)| *m).unwrap_or(1.0);
        let dominant_frequencies: Vec<DominantFrequency> = indexed.iter()
            .take(10)
            .enumerate()
            .map(|(rank, (idx, mag))| DominantFrequency {
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
        let zero_crossing_rate = self.compute_zcr(samples_slice);
        let band_energies = self.compute_band_energies(&spectrum, &frequencies);

        Ok(AnalysisResult {
            dominant_frequencies,
            spectral_centroid,
            spectral_rolloff,
            spectral_flatness,
            zero_crossing_rate,
            band_energies,
        })
    }

    /// Get dominant frequencies
    #[pyo3(signature = (samples, top_k=10))]
    pub fn dominant_frequencies(
        &self,
        samples: PyReadonlyArray1<f32>,
        top_k: usize,
    ) -> PyResult<Vec<DominantFrequency>> {
        let result = self.analyze(samples)?;
        Ok(result.dominant_frequencies.into_iter().take(top_k).collect())
    }

    /// Compute frequency signature
    pub fn compute_signature(&self, samples: PyReadonlyArray1<f32>) -> PyResult<FrequencySignature> {
        let samples_slice = samples.as_slice()?;
        let spectrum = self.compute_spectrum(samples_slice);
        let freq_resolution = self.sample_rate as f32 / self.fft_size as f32;

        // Create 128-dimensional signature
        let num_features = 128;
        let min_freq = 20.0f32;
        let max_freq = (self.sample_rate / 2) as f32;

        let bin_edges: Vec<f32> = (0..=num_features)
            .map(|i| {
                let t = i as f32 / num_features as f32;
                min_freq * (max_freq / min_freq).powf(t)
            })
            .collect();

        let mut features = vec![0.0f32; num_features];

        for i in 0..num_features {
            let low = bin_edges[i];
            let high = bin_edges[i + 1];

            let mut energy = 0.0f32;
            let mut count = 0;

            for (j, &mag) in spectrum.iter().enumerate() {
                let freq = j as f32 * freq_resolution;
                if freq >= low && freq < high {
                    energy += mag;
                    count += 1;
                }
            }

            if count > 0 {
                features[i] = energy / count as f32;
            }
        }

        // Normalize
        let max_f = features.iter().cloned().fold(0.0f32, f32::max);
        if max_f > 0.0 {
            for f in &mut features {
                *f /= max_f;
            }
        }

        let frequencies: Vec<f32> = (0..spectrum.len())
            .map(|i| i as f32 * freq_resolution)
            .collect();

        Ok(FrequencySignature {
            features,
            centroid: self.compute_centroid(&spectrum, &frequencies),
            flatness: self.compute_flatness(&spectrum),
        })
    }
}

// Private helper methods (not exposed to Python)
impl FrequencyAnalyzer {
    fn compute_spectrum(&self, samples: &[f32]) -> Vec<f32> {
        let n = self.fft_size.min(samples.len());

        // Simple DFT (in production, use rustfft)
        let mut spectrum = vec![0.0f32; n / 2];

        for k in 0..n / 2 {
            let mut real = 0.0f32;
            let mut imag = 0.0f32;

            for (i, &sample) in samples.iter().take(n).enumerate() {
                let angle = 2.0 * std::f32::consts::PI * k as f32 * i as f32 / n as f32;
                // Apply Hann window
                let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos());
                let windowed = sample * window;
                real += windowed * angle.cos();
                imag -= windowed * angle.sin();
            }

            spectrum[k] = (real * real + imag * imag).sqrt() * 2.0 / n as f32;
        }

        spectrum
    }

    fn compute_centroid(&self, spectrum: &[f32], frequencies: &[f32]) -> f32 {
        let weighted: f32 = spectrum.iter().zip(frequencies.iter())
            .map(|(&m, &f)| m * f).sum();
        let total: f32 = spectrum.iter().sum();
        if total > 0.0 { weighted / total } else { 0.0 }
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

        if arithmetic_mean > 0.0 { geometric_mean / arithmetic_mean } else { 0.0 }
    }

    fn compute_zcr(&self, samples: &[f32]) -> f32 {
        let crossings: usize = samples.windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();
        crossings as f32 / samples.len() as f32
    }

    fn compute_band_energies(&self, spectrum: &[f32], frequencies: &[f32]) -> BandEnergies {
        let bands = [
            (20.0, 60.0), (60.0, 250.0), (250.0, 500.0),
            (500.0, 2000.0), (2000.0, 4000.0), (4000.0, 20000.0),
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

/// Audio fingerprinter
#[pyclass]
pub struct Fingerprinter {
    fft_size: usize,
    hop_size: usize,
}

#[pymethods]
impl Fingerprinter {
    #[new]
    #[pyo3(signature = (fft_size=4096, hop_size=2048))]
    pub fn new(fft_size: usize, hop_size: usize) -> Self {
        Self { fft_size, hop_size }
    }

    /// Generate fingerprint from audio samples
    pub fn fingerprint(
        &self,
        samples: PyReadonlyArray1<f32>,
        sample_rate: u32,
    ) -> PyResult<Fingerprint> {
        let samples_slice = samples.as_slice()?;

        if samples_slice.len() < self.fft_size {
            return Err(pyo3::exceptions::PyValueError::new_err("Not enough samples"));
        }

        // Generate hash based on spectral peaks (simplified)
        let mut hash_data = Vec::new();
        let num_frames = (samples_slice.len() - self.fft_size) / self.hop_size + 1;

        for frame_idx in 0..num_frames.min(100) {
            let start = frame_idx * self.hop_size;
            let frame = &samples_slice[start..start + self.fft_size];

            // Simple energy-based hash
            let energy: f32 = frame.iter().map(|&s| s * s).sum();
            hash_data.push((energy * 255.0).min(255.0) as u8);
        }

        // Generate hash
        let hash: u64 = hash_data.iter()
            .enumerate()
            .fold(0u64, |acc, (i, &b)| {
                acc.wrapping_add((b as u64).wrapping_mul(31u64.pow((i % 16) as u32)))
            });

        let duration_secs = samples_slice.len() as f64 / sample_rate as f64;

        Ok(Fingerprint {
            hash: format!("{:016x}", hash),
            version: 1,
            duration_secs,
            num_points: hash_data.len(),
        })
    }

    /// Verify audio against a known hash
    pub fn verify(
        &self,
        samples: PyReadonlyArray1<f32>,
        sample_rate: u32,
        expected_hash: &str,
    ) -> PyResult<bool> {
        let fp = self.fingerprint(samples, sample_rate)?;
        Ok(fp.hash == expected_hash)
    }
}

/// Content tagger
#[pyclass]
pub struct ContentTagger {
    min_confidence: f32,
}

#[pymethods]
impl ContentTagger {
    #[new]
    #[pyo3(signature = (min_confidence=0.3))]
    pub fn new(min_confidence: f32) -> Self {
        Self { min_confidence }
    }

    /// Predict content tags from audio
    pub fn predict(
        &self,
        samples: PyReadonlyArray1<f32>,
        _sample_rate: u32,
    ) -> PyResult<Vec<ContentTag>> {
        let samples_slice = samples.as_slice()?;

        // Simplified rule-based tagging
        let mut tags = Vec::new();

        // Analyze spectrum characteristics
        let n = samples_slice.len().min(4096);
        let energy: f32 = samples_slice.iter().take(n).map(|&s| s * s).sum::<f32>() / n as f32;

        // ZCR for speech/music detection
        let zcr: usize = samples_slice.windows(2)
            .take(n - 1)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();
        let zcr_rate = zcr as f32 / n as f32;

        if zcr_rate < 0.05 {
            tags.push(ContentTag { label: "music".to_string(), confidence: 0.7 });
        } else if zcr_rate < 0.1 {
            tags.push(ContentTag { label: "speech".to_string(), confidence: 0.65 });
        }

        if energy > 0.1 {
            tags.push(ContentTag { label: "energetic".to_string(), confidence: 0.6 });
        } else if energy < 0.01 {
            tags.push(ContentTag { label: "ambient".to_string(), confidence: 0.5 });
        }

        // Filter by confidence
        tags.retain(|t| t.confidence >= self.min_confidence);
        tags.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        Ok(tags)
    }
}

// ============================================================================
// Module Definition
// ============================================================================

/// PSM Frequency Analysis Python Module
#[pymodule]
fn psm_frequency(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FrequencyAnalyzer>()?;
    m.add_class::<Fingerprinter>()?;
    m.add_class::<ContentTagger>()?;
    m.add_class::<DominantFrequency>()?;
    m.add_class::<BandEnergies>()?;
    m.add_class::<AnalysisResult>()?;
    m.add_class::<Fingerprint>()?;
    m.add_class::<ContentTag>()?;
    m.add_class::<FrequencySignature>()?;

    // Add version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
