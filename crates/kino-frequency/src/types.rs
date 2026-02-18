//! Core types for frequency analysis.

use serde::{Deserialize, Serialize};

/// Raw audio data extracted from a video file.
#[derive(Debug, Clone)]
pub struct AudioData {
    /// PCM samples normalized to [-1.0, 1.0]
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of audio channels
    pub channels: u32,
    /// Duration in seconds
    pub duration_secs: f64,
}

impl AudioData {
    /// Create new audio data from samples.
    pub fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        let duration_secs = samples.len() as f64 / sample_rate as f64;
        Self {
            samples,
            sample_rate,
            channels: 1,
            duration_secs,
        }
    }

    /// Get a slice of samples for a specific time range.
    pub fn slice(&self, start_secs: f64, end_secs: f64) -> &[f32] {
        let start_idx = (start_secs * self.sample_rate as f64) as usize;
        let end_idx = (end_secs * self.sample_rate as f64) as usize;
        &self.samples[start_idx.min(self.samples.len())..end_idx.min(self.samples.len())]
    }

    /// Get number of samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Check if audio data is empty.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// A dominant frequency detected in the audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DominantFrequency {
    /// Frequency in Hz
    pub frequency_hz: f32,
    /// Magnitude (normalized 0-1)
    pub magnitude: f32,
    /// Rank (1 = highest magnitude)
    pub rank: usize,
}

/// Complete frequency analysis results.
#[derive(Debug, Clone)]
pub struct FrequencyAnalysis {
    /// Full magnitude spectrum
    pub spectrum: Vec<f32>,
    /// Frequency bins (Hz)
    pub frequencies: Vec<f32>,
    /// Spectral centroid (brightness)
    pub spectral_centroid: f32,
    /// Spectral rolloff (95% energy point)
    pub spectral_rolloff: f32,
    /// Spectral flatness (tonality measure)
    pub spectral_flatness: f32,
    /// Band energies (sub-bass, bass, low-mid, mid, high-mid, high)
    pub band_energies: BandEnergies,
    /// Zero crossing rate
    pub zero_crossing_rate: f32,
}

/// Energy distribution across frequency bands.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BandEnergies {
    /// Sub-bass: 20-60 Hz
    pub sub_bass: f32,
    /// Bass: 60-250 Hz
    pub bass: f32,
    /// Low-mid: 250-500 Hz
    pub low_mid: f32,
    /// Mid: 500-2000 Hz
    pub mid: f32,
    /// High-mid: 2000-4000 Hz
    pub high_mid: f32,
    /// High: 4000-20000 Hz
    pub high: f32,
}

impl BandEnergies {
    /// Create band energies from a spectrum and frequency bins.
    pub fn from_spectrum(spectrum: &[f32], frequencies: &[f32]) -> Self {
        let bands = [
            (20.0, 60.0),     // sub_bass
            (60.0, 250.0),    // bass
            (250.0, 500.0),   // low_mid
            (500.0, 2000.0),  // mid
            (2000.0, 4000.0), // high_mid
            (4000.0, 20000.0), // high
        ];

        let mut energies = [0.0f32; 6];

        for (i, (low, high)) in bands.iter().enumerate() {
            for (j, &freq) in frequencies.iter().enumerate() {
                if freq >= *low && freq < *high {
                    energies[i] += spectrum[j];
                }
            }
        }

        // Normalize
        let total: f32 = energies.iter().sum();
        if total > 0.0 {
            for e in &mut energies {
                *e /= total;
            }
        }

        Self {
            sub_bass: energies[0],
            bass: energies[1],
            low_mid: energies[2],
            mid: energies[3],
            high_mid: energies[4],
            high: energies[5],
        }
    }

    /// Convert to a vector for ML features.
    pub fn to_vec(&self) -> Vec<f32> {
        vec![
            self.sub_bass,
            self.bass,
            self.low_mid,
            self.mid,
            self.high_mid,
            self.high,
        ]
    }
}

/// Compact frequency signature for similarity matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencySignature {
    /// 128-dimensional feature vector (mel-scale inspired)
    pub features: Vec<f32>,
    /// Band energies
    pub band_energies: BandEnergies,
    /// Spectral centroid
    pub centroid: f32,
    /// Spectral flatness
    pub flatness: f32,
}

impl FrequencySignature {
    /// Compute cosine similarity with another signature.
    pub fn similarity(&self, other: &FrequencySignature) -> f32 {
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

/// Audio fingerprint for content verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFingerprint {
    /// SHA-256 hash of fingerprint data
    pub hash: String,
    /// Version of fingerprinting algorithm
    pub version: u32,
    /// Fingerprint constellation points
    pub points: Vec<FingerprintPoint>,
    /// Duration of analyzed audio in seconds
    pub duration_secs: f64,
}

/// A single point in the fingerprint constellation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintPoint {
    /// Time offset in frames
    pub time_offset: u32,
    /// Frequency bin index
    pub freq_bin: u32,
    /// Amplitude (quantized)
    pub amplitude: u8,
}

/// Content tag with confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTag {
    /// Tag label
    pub label: String,
    /// Confidence score (0-1)
    pub confidence: f32,
}

/// Configuration for video processing pipeline.
#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    /// Target sample rate for analysis
    pub sample_rate: u32,
    /// Enable fingerprint generation
    pub enable_fingerprint: bool,
    /// Enable auto-tagging
    pub enable_tagging: bool,
    /// Enable thumbnail selection
    pub enable_thumbnail: bool,
    /// Enable signature generation
    pub enable_signature: bool,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            enable_fingerprint: true,
            enable_tagging: true,
            enable_thumbnail: true,
            enable_signature: true,
        }
    }
}

/// Result of complete video processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    /// Unique content identifier
    pub content_id: String,
    /// Audio fingerprint (if enabled)
    pub fingerprint: Option<AudioFingerprint>,
    /// Content tags (if enabled)
    pub tags: Vec<ContentTag>,
    /// Optimal thumbnail timestamp in seconds (if enabled)
    pub thumbnail_timestamp: Option<f64>,
    /// Frequency signature (if enabled)
    pub signature: Option<FrequencySignature>,
    /// Top dominant frequencies
    pub dominant_frequencies: Vec<DominantFrequency>,
}

/// Frame quality metrics for thumbnail selection.
#[derive(Debug, Clone)]
pub struct FrameQuality {
    /// Timestamp in seconds
    pub timestamp: f64,
    /// FFT-based sharpness score
    pub sharpness: f32,
    /// Contrast score
    pub contrast: f32,
    /// Number of detected faces
    pub face_count: u32,
    /// Overall quality score
    pub score: f32,
}

/// Recommendation with similarity score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Content ID of recommended item
    pub content_id: String,
    /// Similarity score (0-1)
    pub similarity: f32,
    /// Matching features that contributed to similarity
    pub matching_features: Vec<String>,
}
