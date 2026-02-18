//! Audio fingerprinting for content verification.
//!
//! This module implements a spectral peak constellation algorithm similar to
//! Shazam's approach, generating cryptographic fingerprints for audio content
//! that can be stored on-chain for verification.
//!
//! # Algorithm Overview
//!
//! 1. Compute spectrogram of audio
//! 2. Find spectral peaks in each time frame
//! 3. Create constellation map of peaks
//! 4. Generate hash pairs from peak combinations
//! 5. Produce final SHA-256 fingerprint hash
//!
//! # On-Chain Verification
//!
//! The fingerprint hash can be stored on Solana for decentralized content
//! verification, ensuring creator ownership without centralized control.

use std::collections::HashMap;
use anyhow::Result;
use ring::digest::{Context, SHA256};
use tracing::{debug, info};

use crate::fft::FrequencyAnalyzer;
use crate::types::*;

/// Fingerprinting configuration.
#[derive(Debug, Clone)]
pub struct FingerprintConfig {
    /// FFT window size
    pub fft_size: usize,
    /// Hop size between frames
    pub hop_size: usize,
    /// Number of frequency bands for peak detection
    pub num_bands: usize,
    /// Fan-out factor for hash generation
    pub fan_out: usize,
    /// Target zone time span (in frames)
    pub target_zone_frames: usize,
    /// Minimum peak prominence threshold
    pub peak_threshold: f32,
}

impl Default for FingerprintConfig {
    fn default() -> Self {
        Self {
            fft_size: 4096,
            hop_size: 2048,
            num_bands: 6,
            fan_out: 5,
            target_zone_frames: 50,
            peak_threshold: 0.1,
        }
    }
}

/// Audio fingerprinter using spectral peak constellation.
pub struct Fingerprinter {
    config: FingerprintConfig,
    analyzer: FrequencyAnalyzer,
}

impl Fingerprinter {
    /// Create a new fingerprinter with default configuration.
    pub fn new() -> Self {
        Self::with_config(FingerprintConfig::default())
    }

    /// Create a fingerprinter with custom configuration.
    pub fn with_config(config: FingerprintConfig) -> Self {
        let analyzer = FrequencyAnalyzer::new(config.fft_size, config.hop_size);
        Self { config, analyzer }
    }

    /// Generate a fingerprint from audio data.
    pub fn fingerprint(&self, audio: &AudioData) -> Result<AudioFingerprint> {
        info!("Generating fingerprint for {} samples", audio.samples.len());

        // Compute spectrogram
        let spectrogram = self.analyzer.compute_spectrogram(&audio.samples)?;
        debug!("Computed spectrogram with {} frames", spectrogram.len());

        // Find spectral peaks
        let peaks = self.find_peaks(&spectrogram)?;
        debug!("Found {} spectral peaks", peaks.len());

        // Generate constellation points
        let points = self.create_constellation(&peaks);
        debug!("Created {} constellation points", points.len());

        // Generate hash pairs
        let hash_pairs = self.generate_hash_pairs(&points);
        debug!("Generated {} hash pairs", hash_pairs.len());

        // Compute final fingerprint hash
        let hash = self.compute_hash(&hash_pairs);

        let duration_secs = audio.samples.len() as f64 / audio.sample_rate as f64;

        Ok(AudioFingerprint {
            hash,
            version: 1,
            points,
            duration_secs,
        })
    }

    /// Find spectral peaks in each frame using band-wise maximum detection.
    fn find_peaks(&self, spectrogram: &[Vec<f32>]) -> Result<Vec<SpectralPeak>> {
        let spectrum_size = spectrogram.first()
            .map(|f| f.len())
            .ok_or_else(|| anyhow::anyhow!("Empty spectrogram"))?;

        // Define frequency bands (log-spaced)
        let band_edges: Vec<usize> = (0..=self.config.num_bands)
            .map(|i| {
                let t = i as f32 / self.config.num_bands as f32;
                (spectrum_size as f32 * t.powf(2.0)) as usize
            })
            .collect();

        let mut peaks = Vec::new();

        for (time_idx, frame) in spectrogram.iter().enumerate() {
            // Find max in each frequency band
            for band_idx in 0..self.config.num_bands {
                let start = band_edges[band_idx];
                let end = band_edges[band_idx + 1].min(frame.len());

                if start >= end {
                    continue;
                }

                // Find maximum in this band
                let (local_max_idx, &max_val) = frame[start..end]
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or((0, &0.0));

                // Only keep peaks above threshold
                if max_val > self.config.peak_threshold {
                    peaks.push(SpectralPeak {
                        time_frame: time_idx as u32,
                        freq_bin: (start + local_max_idx) as u32,
                        magnitude: max_val,
                    });
                }
            }
        }

        Ok(peaks)
    }

    /// Create constellation points from spectral peaks.
    fn create_constellation(&self, peaks: &[SpectralPeak]) -> Vec<FingerprintPoint> {
        peaks.iter()
            .map(|peak| FingerprintPoint {
                time_offset: peak.time_frame,
                freq_bin: peak.freq_bin,
                amplitude: (peak.magnitude * 255.0).min(255.0) as u8,
            })
            .collect()
    }

    /// Generate hash pairs by pairing anchor points with target points.
    fn generate_hash_pairs(&self, points: &[FingerprintPoint]) -> Vec<HashPair> {
        let mut pairs = Vec::new();

        for (i, anchor) in points.iter().enumerate() {
            // Find target points in the target zone
            let mut targets_found = 0;

            for target in points.iter().skip(i + 1) {
                let time_delta = target.time_offset.saturating_sub(anchor.time_offset);

                // Only consider targets within the target zone
                if time_delta > 0 && time_delta <= self.config.target_zone_frames as u32 {
                    pairs.push(HashPair {
                        anchor_freq: anchor.freq_bin,
                        target_freq: target.freq_bin,
                        time_delta,
                        anchor_time: anchor.time_offset,
                    });

                    targets_found += 1;
                    if targets_found >= self.config.fan_out {
                        break;
                    }
                }
            }
        }

        pairs
    }

    /// Compute final SHA-256 hash from hash pairs.
    fn compute_hash(&self, pairs: &[HashPair]) -> String {
        let mut context = Context::new(&SHA256);

        // Add version
        context.update(&1u32.to_le_bytes());

        // Add all hash pairs
        for pair in pairs {
            context.update(&pair.anchor_freq.to_le_bytes());
            context.update(&pair.target_freq.to_le_bytes());
            context.update(&pair.time_delta.to_le_bytes());
        }

        let digest = context.finish();
        hex::encode(digest.as_ref())
    }

    /// Match two fingerprints and return similarity score.
    pub fn match_fingerprints(&self, fp1: &AudioFingerprint, fp2: &AudioFingerprint) -> MatchResult {
        // Build hash map from first fingerprint
        let pairs1 = self.generate_hash_pairs(&fp1.points);
        let pairs2 = self.generate_hash_pairs(&fp2.points);

        // Create lookup table for fp1
        let mut fp1_hashes: HashMap<(u32, u32, u32), Vec<u32>> = HashMap::new();
        for pair in &pairs1 {
            let key = (pair.anchor_freq, pair.target_freq, pair.time_delta);
            fp1_hashes.entry(key).or_default().push(pair.anchor_time);
        }

        // Count matches
        let mut _match_count = 0;
        let mut time_offsets: HashMap<i64, u32> = HashMap::new();

        for pair in &pairs2 {
            let key = (pair.anchor_freq, pair.target_freq, pair.time_delta);
            if let Some(fp1_times) = fp1_hashes.get(&key) {
                _match_count += 1;
                for &t1 in fp1_times {
                    let offset = pair.anchor_time as i64 - t1 as i64;
                    *time_offsets.entry(offset).or_default() += 1;
                }
            }
        }

        // Find best time offset alignment
        let best_offset = time_offsets.iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&offset, _)| offset)
            .unwrap_or(0);

        let aligned_matches = time_offsets.get(&best_offset).copied().unwrap_or(0);

        // Calculate similarity score
        let total_pairs = pairs1.len().max(pairs2.len()) as f32;
        let similarity = if total_pairs > 0.0 {
            aligned_matches as f32 / total_pairs
        } else {
            0.0
        };

        MatchResult {
            is_match: similarity > 0.1,
            similarity,
            time_offset_frames: best_offset as i32,
            matching_pairs: aligned_matches,
            total_pairs_checked: pairs2.len() as u32,
        }
    }

    /// Verify content against a known fingerprint hash.
    pub fn verify(&self, audio: &AudioData, expected_hash: &str) -> Result<VerificationResult> {
        let fingerprint = self.fingerprint(audio)?;

        let matches = fingerprint.hash == expected_hash;

        Ok(VerificationResult {
            verified: matches,
            computed_hash: fingerprint.hash,
            expected_hash: expected_hash.to_string(),
        })
    }
}

impl Default for Fingerprinter {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal spectral peak representation.
#[derive(Debug, Clone)]
struct SpectralPeak {
    time_frame: u32,
    freq_bin: u32,
    magnitude: f32,
}

/// Hash pair for fingerprint matching.
#[derive(Debug, Clone)]
struct HashPair {
    anchor_freq: u32,
    target_freq: u32,
    time_delta: u32,
    anchor_time: u32,
}

/// Result of fingerprint matching.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Whether the fingerprints are considered a match
    pub is_match: bool,
    /// Similarity score (0-1)
    pub similarity: f32,
    /// Time offset in frames between the two audio clips
    pub time_offset_frames: i32,
    /// Number of matching hash pairs
    pub matching_pairs: u32,
    /// Total hash pairs checked
    pub total_pairs_checked: u32,
}

/// Result of content verification.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether the content matches the expected fingerprint
    pub verified: bool,
    /// The computed fingerprint hash
    pub computed_hash: String,
    /// The expected fingerprint hash
    pub expected_hash: String,
}

/// Fingerprint database for content matching.
pub struct FingerprintDatabase {
    /// Map from hash pair key to (content_id, anchor_time)
    index: HashMap<(u32, u32, u32), Vec<(String, u32)>>,
}

impl FingerprintDatabase {
    /// Create a new empty database.
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    /// Add a fingerprint to the database.
    pub fn add(&mut self, content_id: &str, fingerprint: &AudioFingerprint) {
        let fingerprinter = Fingerprinter::new();
        let pairs = fingerprinter.generate_hash_pairs(&fingerprint.points);

        for pair in pairs {
            let key = (pair.anchor_freq, pair.target_freq, pair.time_delta);
            self.index.entry(key)
                .or_default()
                .push((content_id.to_string(), pair.anchor_time));
        }
    }

    /// Query the database for matching content.
    pub fn query(&self, fingerprint: &AudioFingerprint, threshold: f32) -> Vec<DatabaseMatch> {
        let fingerprinter = Fingerprinter::new();
        let pairs = fingerprinter.generate_hash_pairs(&fingerprint.points);

        // Count matches per content
        let mut content_matches: HashMap<String, HashMap<i64, u32>> = HashMap::new();

        for pair in &pairs {
            let key = (pair.anchor_freq, pair.target_freq, pair.time_delta);
            if let Some(entries) = self.index.get(&key) {
                for (content_id, db_time) in entries {
                    let offset = pair.anchor_time as i64 - *db_time as i64;
                    *content_matches
                        .entry(content_id.clone())
                        .or_default()
                        .entry(offset)
                        .or_default() += 1;
                }
            }
        }

        // Find best matches
        let mut results: Vec<DatabaseMatch> = content_matches.iter()
            .filter_map(|(content_id, offsets)| {
                let best_count = offsets.values().max().copied().unwrap_or(0);
                let similarity = best_count as f32 / pairs.len() as f32;

                if similarity >= threshold {
                    Some(DatabaseMatch {
                        content_id: content_id.clone(),
                        similarity,
                        matching_pairs: best_count,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

impl Default for FingerprintDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Match result from database query.
#[derive(Debug, Clone)]
pub struct DatabaseMatch {
    /// Content ID of the matched item
    pub content_id: String,
    /// Similarity score
    pub similarity: f32,
    /// Number of matching hash pairs
    pub matching_pairs: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_test_audio(freq: f32, duration_secs: f32) -> AudioData {
        let sample_rate = 44100;
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq * t).sin()
            })
            .collect();

        AudioData::new(samples, sample_rate)
    }

    #[test]
    fn test_fingerprint_generation() {
        let audio = generate_test_audio(440.0, 5.0);
        let fingerprinter = Fingerprinter::new();
        let fp = fingerprinter.fingerprint(&audio).unwrap();

        assert!(!fp.hash.is_empty());
        assert!(fp.points.len() > 0);
        assert_eq!(fp.version, 1);
    }

    #[test]
    fn test_fingerprint_consistency() {
        let audio = generate_test_audio(440.0, 5.0);
        let fingerprinter = Fingerprinter::new();

        let fp1 = fingerprinter.fingerprint(&audio).unwrap();
        let fp2 = fingerprinter.fingerprint(&audio).unwrap();

        // Same audio should produce same fingerprint
        assert_eq!(fp1.hash, fp2.hash);
    }

    #[test]
    fn test_fingerprint_matching() {
        let audio1 = generate_test_audio(440.0, 5.0);
        let audio2 = generate_test_audio(440.0, 5.0);
        let audio3 = generate_test_audio(880.0, 5.0);

        let fingerprinter = Fingerprinter::new();

        let fp1 = fingerprinter.fingerprint(&audio1).unwrap();
        let fp2 = fingerprinter.fingerprint(&audio2).unwrap();
        let fp3 = fingerprinter.fingerprint(&audio3).unwrap();

        // Same audio should match
        let match_same = fingerprinter.match_fingerprints(&fp1, &fp2);
        assert!(match_same.is_match);

        // Different audio should not match as well
        let match_diff = fingerprinter.match_fingerprints(&fp1, &fp3);
        assert!(match_same.similarity > match_diff.similarity);
    }

    #[test]
    fn test_verification() {
        let audio = generate_test_audio(440.0, 5.0);
        let fingerprinter = Fingerprinter::new();

        let fp = fingerprinter.fingerprint(&audio).unwrap();
        let result = fingerprinter.verify(&audio, &fp.hash).unwrap();

        assert!(result.verified);
        assert_eq!(result.computed_hash, result.expected_hash);
    }

    #[test]
    fn test_database_query() {
        let audio1 = generate_test_audio(440.0, 5.0);
        let audio2 = generate_test_audio(880.0, 5.0);
        let query_audio = generate_test_audio(440.0, 5.0);

        let fingerprinter = Fingerprinter::new();

        let fp1 = fingerprinter.fingerprint(&audio1).unwrap();
        let fp2 = fingerprinter.fingerprint(&audio2).unwrap();
        let query_fp = fingerprinter.fingerprint(&query_audio).unwrap();

        let mut db = FingerprintDatabase::new();
        db.add("content_1", &fp1);
        db.add("content_2", &fp2);

        let results = db.query(&query_fp, 0.1);

        assert!(!results.is_empty());
        assert_eq!(results[0].content_id, "content_1");
    }
}

// Add hex encoding helper
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
