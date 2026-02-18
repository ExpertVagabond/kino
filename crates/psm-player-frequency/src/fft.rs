//! FFT-based frequency analysis core.
//!
//! This module provides the fundamental frequency analysis operations
//! used throughout the PSM frequency analysis system.

use std::sync::Arc;
use anyhow::{Result, bail};
use rustfft::{FftPlanner, num_complex::Complex};
use tracing::debug;

use crate::types::*;

/// Core frequency analyzer using FFT.
pub struct FrequencyAnalyzer {
    fft_size: usize,
    hop_size: usize,
    window: Vec<f32>,
}

impl FrequencyAnalyzer {
    /// Create a new frequency analyzer.
    pub fn new(fft_size: usize, hop_size: usize) -> Self {
        // Generate Hann window
        let window: Vec<f32> = (0..fft_size)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos())
            })
            .collect();

        Self {
            fft_size,
            hop_size,
            window,
        }
    }

    /// Perform complete frequency analysis on audio samples.
    pub fn analyze(&self, samples: &[f32], sample_rate: u32) -> Result<FrequencyAnalysis> {
        if samples.len() < self.fft_size {
            bail!("Not enough samples for FFT analysis. Need at least {} samples.", self.fft_size);
        }

        // Compute average spectrum across all frames
        let spectrogram = self.compute_spectrogram(samples)?;

        // Average spectrum
        let num_frames = spectrogram.len();
        let spectrum_size = spectrogram[0].len();
        let mut spectrum = vec![0.0f32; spectrum_size];

        for frame in &spectrogram {
            for (i, &mag) in frame.iter().enumerate() {
                spectrum[i] += mag;
            }
        }
        for mag in &mut spectrum {
            *mag /= num_frames as f32;
        }

        // Compute frequency bins
        let frequencies: Vec<f32> = (0..spectrum_size)
            .map(|i| i as f32 * sample_rate as f32 / self.fft_size as f32)
            .collect();

        // Compute spectral features
        let spectral_centroid = self.compute_spectral_centroid(&spectrum, &frequencies);
        let spectral_rolloff = self.compute_spectral_rolloff(&spectrum, &frequencies, 0.95);
        let spectral_flatness = self.compute_spectral_flatness(&spectrum);
        let band_energies = BandEnergies::from_spectrum(&spectrum, &frequencies);
        let zero_crossing_rate = self.compute_zcr(samples);

        Ok(FrequencyAnalysis {
            spectrum,
            frequencies,
            spectral_centroid,
            spectral_rolloff,
            spectral_flatness,
            band_energies,
            zero_crossing_rate,
        })
    }

    /// Compute spectrogram (time-frequency representation).
    pub fn compute_spectrogram(&self, samples: &[f32]) -> Result<Vec<Vec<f32>>> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.fft_size);

        let num_frames = (samples.len() - self.fft_size) / self.hop_size + 1;
        let mut spectrogram = Vec::with_capacity(num_frames);

        for frame_idx in 0..num_frames {
            let start = frame_idx * self.hop_size;
            let frame_samples = &samples[start..start + self.fft_size];

            // Apply window and convert to complex
            let mut buffer: Vec<Complex<f32>> = frame_samples
                .iter()
                .zip(self.window.iter())
                .map(|(&s, &w)| Complex::new(s * w, 0.0))
                .collect();

            // Perform FFT
            fft.process(&mut buffer);

            // Compute magnitude spectrum (only positive frequencies)
            let magnitude: Vec<f32> = buffer[..self.fft_size / 2]
                .iter()
                .map(|c| (c.re * c.re + c.im * c.im).sqrt() * 2.0 / self.fft_size as f32)
                .collect();

            spectrogram.push(magnitude);
        }

        Ok(spectrogram)
    }

    /// Find dominant frequencies in the audio.
    pub fn dominant_frequencies(
        &self,
        samples: &[f32],
        sample_rate: u32,
        top_k: usize,
    ) -> Result<Vec<DominantFrequency>> {
        let analysis = self.analyze(samples, sample_rate)?;

        // Find peaks in spectrum
        let mut indexed: Vec<(usize, f32)> = analysis.spectrum
            .iter()
            .enumerate()
            .map(|(i, &mag)| (i, mag))
            .collect();

        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Normalize magnitudes
        let max_mag = indexed.first().map(|(_, m)| *m).unwrap_or(1.0);

        let dominant: Vec<DominantFrequency> = indexed
            .into_iter()
            .take(top_k)
            .enumerate()
            .map(|(rank, (idx, mag))| DominantFrequency {
                frequency_hz: analysis.frequencies[idx],
                magnitude: mag / max_mag,
                rank: rank + 1,
            })
            .collect();

        Ok(dominant)
    }

    /// Compute a compact frequency signature for similarity matching.
    pub fn compute_signature(&self, samples: &[f32], sample_rate: u32) -> Result<FrequencySignature> {
        let analysis = self.analyze(samples, sample_rate)?;

        // Create mel-scale inspired binning (128 features)
        let num_features = 128;
        let min_freq = 20.0f32;
        let max_freq = (sample_rate / 2) as f32;

        // Log-spaced frequency bins
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

            for (j, &freq) in analysis.frequencies.iter().enumerate() {
                if freq >= low && freq < high {
                    energy += analysis.spectrum[j];
                    count += 1;
                }
            }

            if count > 0 {
                features[i] = energy / count as f32;
            }
        }

        // Normalize features
        let max_feature = features.iter().cloned().fold(0.0f32, f32::max);
        if max_feature > 0.0 {
            for f in &mut features {
                *f /= max_feature;
            }
        }

        Ok(FrequencySignature {
            features,
            band_energies: analysis.band_energies,
            centroid: analysis.spectral_centroid,
            flatness: analysis.spectral_flatness,
        })
    }

    /// Compute spectral centroid (center of mass of spectrum).
    fn compute_spectral_centroid(&self, spectrum: &[f32], frequencies: &[f32]) -> f32 {
        let weighted_sum: f32 = spectrum.iter()
            .zip(frequencies.iter())
            .map(|(&mag, &freq)| mag * freq)
            .sum();

        let total_mag: f32 = spectrum.iter().sum();

        if total_mag > 0.0 {
            weighted_sum / total_mag
        } else {
            0.0
        }
    }

    /// Compute spectral rolloff (frequency below which N% of energy lies).
    fn compute_spectral_rolloff(&self, spectrum: &[f32], frequencies: &[f32], percentage: f32) -> f32 {
        let total_energy: f32 = spectrum.iter().sum();
        let threshold = total_energy * percentage;

        let mut cumulative = 0.0f32;
        for (i, &mag) in spectrum.iter().enumerate() {
            cumulative += mag;
            if cumulative >= threshold {
                return frequencies[i];
            }
        }

        *frequencies.last().unwrap_or(&0.0)
    }

    /// Compute spectral flatness (tonality measure).
    fn compute_spectral_flatness(&self, spectrum: &[f32]) -> f32 {
        let n = spectrum.len() as f32;

        // Geometric mean
        let log_sum: f32 = spectrum.iter()
            .map(|&x| (x + 1e-10).ln())
            .sum();
        let geometric_mean = (log_sum / n).exp();

        // Arithmetic mean
        let arithmetic_mean: f32 = spectrum.iter().sum::<f32>() / n;

        if arithmetic_mean > 0.0 {
            geometric_mean / arithmetic_mean
        } else {
            0.0
        }
    }

    /// Compute zero crossing rate.
    fn compute_zcr(&self, samples: &[f32]) -> f32 {
        let crossings: usize = samples.windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();

        crossings as f32 / samples.len() as f32
    }

    /// Apply a bandpass filter to extract specific frequency range.
    pub fn bandpass_filter(
        &self,
        samples: &[f32],
        sample_rate: u32,
        low_freq: f32,
        high_freq: f32,
    ) -> Result<Vec<f32>> {
        let mut planner = FftPlanner::new();
        let fft_forward = planner.plan_fft_forward(samples.len());
        let fft_inverse = planner.plan_fft_inverse(samples.len());

        // Forward FFT
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .map(|&s| Complex::new(s, 0.0))
            .collect();
        fft_forward.process(&mut buffer);

        // Apply bandpass filter in frequency domain
        let freq_resolution = sample_rate as f32 / samples.len() as f32;
        for (i, c) in buffer.iter_mut().enumerate() {
            let freq = if i <= samples.len() / 2 {
                i as f32 * freq_resolution
            } else {
                (samples.len() - i) as f32 * freq_resolution
            };

            if freq < low_freq || freq > high_freq {
                *c = Complex::new(0.0, 0.0);
            }
        }

        // Inverse FFT
        fft_inverse.process(&mut buffer);

        // Normalize and extract real part
        let scale = 1.0 / samples.len() as f32;
        Ok(buffer.iter().map(|c| c.re * scale).collect())
    }

    /// Project signal onto top-K dominant frequencies.
    pub fn project_to_dominant(
        &self,
        samples: &[f32],
        sample_rate: u32,
        top_k: usize,
    ) -> Result<Vec<f32>> {
        let dominant = self.dominant_frequencies(samples, sample_rate, top_k)?;

        let mut planner = FftPlanner::new();
        let fft_forward = planner.plan_fft_forward(samples.len());
        let fft_inverse = planner.plan_fft_inverse(samples.len());

        // Forward FFT
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .map(|&s| Complex::new(s, 0.0))
            .collect();
        fft_forward.process(&mut buffer);

        // Keep only dominant frequency bins
        let freq_resolution = sample_rate as f32 / samples.len() as f32;
        let dominant_freqs: Vec<f32> = dominant.iter().map(|d| d.frequency_hz).collect();

        let mut mask = vec![false; buffer.len()];
        for (i, _) in buffer.iter().enumerate() {
            let freq = if i <= samples.len() / 2 {
                i as f32 * freq_resolution
            } else {
                (samples.len() - i) as f32 * freq_resolution
            };

            // Check if this bin is close to a dominant frequency
            for &dom_freq in &dominant_freqs {
                if (freq - dom_freq).abs() < freq_resolution {
                    mask[i] = true;
                    break;
                }
            }
        }

        // Zero out non-dominant frequencies
        for (i, c) in buffer.iter_mut().enumerate() {
            if !mask[i] {
                *c = Complex::new(0.0, 0.0);
            }
        }

        // Inverse FFT
        fft_inverse.process(&mut buffer);

        let scale = 1.0 / samples.len() as f32;
        Ok(buffer.iter().map(|c| c.re * scale).collect())
    }
}

/// Real-time frequency analyzer for streaming applications.
pub struct RealtimeAnalyzer {
    analyzer: FrequencyAnalyzer,
    buffer: Vec<f32>,
    sample_rate: u32,
}

impl RealtimeAnalyzer {
    /// Create a new real-time analyzer.
    pub fn new(fft_size: usize, sample_rate: u32) -> Self {
        Self {
            analyzer: FrequencyAnalyzer::new(fft_size, fft_size / 2),
            buffer: Vec::with_capacity(fft_size),
            sample_rate,
        }
    }

    /// Push samples and get analysis if enough data is available.
    pub fn push(&mut self, samples: &[f32]) -> Option<FrequencyAnalysis> {
        self.buffer.extend_from_slice(samples);

        if self.buffer.len() >= self.analyzer.fft_size {
            let analysis = self.analyzer.analyze(&self.buffer, self.sample_rate).ok();

            // Keep overlap for next frame
            let drain_amount = self.buffer.len() - self.analyzer.fft_size / 2;
            self.buffer.drain(0..drain_amount);

            analysis
        } else {
            None
        }
    }

    /// Reset the analyzer state.
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_sine_wave(freq: f32, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin()
            })
            .collect()
    }

    #[test]
    fn test_dominant_frequency_detection() {
        let sample_rate = 44100;
        let samples = generate_sine_wave(440.0, sample_rate, 1.0);

        let analyzer = FrequencyAnalyzer::new(4096, 2048);
        let dominant = analyzer.dominant_frequencies(&samples, sample_rate, 5).unwrap();

        // First dominant frequency should be close to 440 Hz
        assert!((dominant[0].frequency_hz - 440.0).abs() < 20.0);
    }

    #[test]
    fn test_spectral_centroid() {
        let sample_rate = 44100;

        // Low frequency signal
        let low_freq = generate_sine_wave(100.0, sample_rate, 1.0);
        // High frequency signal
        let high_freq = generate_sine_wave(5000.0, sample_rate, 1.0);

        let analyzer = FrequencyAnalyzer::new(4096, 2048);

        let low_analysis = analyzer.analyze(&low_freq, sample_rate).unwrap();
        let high_analysis = analyzer.analyze(&high_freq, sample_rate).unwrap();

        // High frequency signal should have higher centroid
        assert!(high_analysis.spectral_centroid > low_analysis.spectral_centroid);
    }

    #[test]
    fn test_frequency_signature_similarity() {
        let sample_rate = 44100;

        // Two similar signals (same frequency)
        let signal1 = generate_sine_wave(440.0, sample_rate, 1.0);
        let signal2 = generate_sine_wave(440.0, sample_rate, 1.0);
        // Different signal
        let signal3 = generate_sine_wave(1000.0, sample_rate, 1.0);

        let analyzer = FrequencyAnalyzer::new(4096, 2048);

        let sig1 = analyzer.compute_signature(&signal1, sample_rate).unwrap();
        let sig2 = analyzer.compute_signature(&signal2, sample_rate).unwrap();
        let sig3 = analyzer.compute_signature(&signal3, sample_rate).unwrap();

        // Similar signals should have high similarity
        assert!(sig1.similarity(&sig2) > 0.9);
        // Different signals should have lower similarity
        assert!(sig1.similarity(&sig3) < sig1.similarity(&sig2));
    }

    #[test]
    fn test_bandpass_filter() {
        let sample_rate = 44100;
        // Signal with two frequencies
        let samples: Vec<f32> = (0..sample_rate as usize)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 200.0 * t).sin() +
                (2.0 * std::f32::consts::PI * 2000.0 * t).sin()
            })
            .collect();

        let analyzer = FrequencyAnalyzer::new(4096, 2048);

        // Filter to keep only 150-250 Hz
        let filtered = analyzer.bandpass_filter(&samples, sample_rate, 150.0, 250.0).unwrap();

        // Analyze filtered signal
        let dominant = analyzer.dominant_frequencies(&filtered, sample_rate, 1).unwrap();

        // Dominant should be close to 200 Hz
        assert!((dominant[0].frequency_hz - 200.0).abs() < 30.0);
    }
}
