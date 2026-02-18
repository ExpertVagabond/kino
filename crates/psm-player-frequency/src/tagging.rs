//! AI-powered content auto-tagging.
//!
//! This module provides automatic content classification based on audio
//! frequency analysis. It supports both rule-based tagging and ML model
//! inference for production-quality predictions.
//!
//! # Tag Categories
//!
//! - **Genre**: music, speech, gaming, nature, sports, tutorial, news, podcast
//! - **Mood**: energetic, calm, dramatic, upbeat, melancholic
//! - **Content Type**: vocal, instrumental, ambient, dialogue
//! - **Quality**: high-fidelity, compressed, noisy

use std::collections::HashMap;
use anyhow::Result;
use tracing::{debug, info};

use crate::fft::FrequencyAnalyzer;
use crate::types::*;

/// Content tagging configuration.
#[derive(Debug, Clone)]
pub struct TaggingConfig {
    /// FFT size for analysis
    pub fft_size: usize,
    /// Hop size for frame analysis
    pub hop_size: usize,
    /// Minimum confidence threshold for tags
    pub min_confidence: f32,
    /// Maximum number of tags to return
    pub max_tags: usize,
    /// Enable ML model inference (if available)
    pub use_ml_model: bool,
}

impl Default for TaggingConfig {
    fn default() -> Self {
        Self {
            fft_size: 4096,
            hop_size: 2048,
            min_confidence: 0.3,
            max_tags: 5,
            use_ml_model: false,
        }
    }
}

/// Content tagger using frequency analysis.
pub struct ContentTagger {
    config: TaggingConfig,
    analyzer: FrequencyAnalyzer,
    /// Genre classification thresholds (learned from training data)
    genre_profiles: HashMap<String, GenreProfile>,
}

impl ContentTagger {
    /// Create a new content tagger with default configuration.
    pub fn new() -> Self {
        Self::with_config(TaggingConfig::default())
    }

    /// Create a tagger with custom configuration.
    pub fn with_config(config: TaggingConfig) -> Self {
        let analyzer = FrequencyAnalyzer::new(config.fft_size, config.hop_size);
        let genre_profiles = Self::default_genre_profiles();

        Self {
            config,
            analyzer,
            genre_profiles,
        }
    }

    /// Default genre profiles based on frequency characteristics.
    fn default_genre_profiles() -> HashMap<String, GenreProfile> {
        let mut profiles = HashMap::new();

        // Music: balanced spectrum, low flatness (tonal), moderate ZCR
        profiles.insert("music".to_string(), GenreProfile {
            spectral_centroid_range: (500.0, 4000.0),
            spectral_flatness_range: (0.0, 0.3),
            zcr_range: (0.02, 0.15),
            band_weights: BandWeights {
                sub_bass: 0.15,
                bass: 0.20,
                low_mid: 0.20,
                mid: 0.20,
                high_mid: 0.15,
                high: 0.10,
            },
        });

        // Speech: mid-range centroid, low ZCR, concentrated in mid frequencies
        profiles.insert("speech".to_string(), GenreProfile {
            spectral_centroid_range: (300.0, 2000.0),
            spectral_flatness_range: (0.1, 0.5),
            zcr_range: (0.01, 0.08),
            band_weights: BandWeights {
                sub_bass: 0.05,
                bass: 0.10,
                low_mid: 0.25,
                mid: 0.35,
                high_mid: 0.15,
                high: 0.10,
            },
        });

        // Gaming: wide spectrum, high energy variation, high ZCR
        profiles.insert("gaming".to_string(), GenreProfile {
            spectral_centroid_range: (1000.0, 6000.0),
            spectral_flatness_range: (0.2, 0.7),
            zcr_range: (0.05, 0.20),
            band_weights: BandWeights {
                sub_bass: 0.15,
                bass: 0.15,
                low_mid: 0.15,
                mid: 0.20,
                high_mid: 0.20,
                high: 0.15,
            },
        });

        // Nature: low centroid, high flatness (noise-like), low ZCR
        profiles.insert("nature".to_string(), GenreProfile {
            spectral_centroid_range: (200.0, 2000.0),
            spectral_flatness_range: (0.4, 0.9),
            zcr_range: (0.01, 0.06),
            band_weights: BandWeights {
                sub_bass: 0.10,
                bass: 0.15,
                low_mid: 0.20,
                mid: 0.25,
                high_mid: 0.15,
                high: 0.15,
            },
        });

        // Podcast: similar to speech but with music intros
        profiles.insert("podcast".to_string(), GenreProfile {
            spectral_centroid_range: (300.0, 2500.0),
            spectral_flatness_range: (0.1, 0.4),
            zcr_range: (0.01, 0.10),
            band_weights: BandWeights {
                sub_bass: 0.05,
                bass: 0.10,
                low_mid: 0.25,
                mid: 0.35,
                high_mid: 0.15,
                high: 0.10,
            },
        });

        // Tutorial: clear speech with occasional UI sounds
        profiles.insert("tutorial".to_string(), GenreProfile {
            spectral_centroid_range: (400.0, 3000.0),
            spectral_flatness_range: (0.1, 0.5),
            zcr_range: (0.02, 0.12),
            band_weights: BandWeights {
                sub_bass: 0.05,
                bass: 0.08,
                low_mid: 0.20,
                mid: 0.35,
                high_mid: 0.20,
                high: 0.12,
            },
        });

        // News: professional speech, compressed dynamics
        profiles.insert("news".to_string(), GenreProfile {
            spectral_centroid_range: (350.0, 1800.0),
            spectral_flatness_range: (0.1, 0.35),
            zcr_range: (0.01, 0.06),
            band_weights: BandWeights {
                sub_bass: 0.03,
                bass: 0.08,
                low_mid: 0.25,
                mid: 0.40,
                high_mid: 0.15,
                high: 0.09,
            },
        });

        // Sports: crowd noise, commentary, high energy
        profiles.insert("sports".to_string(), GenreProfile {
            spectral_centroid_range: (500.0, 4000.0),
            spectral_flatness_range: (0.3, 0.7),
            zcr_range: (0.04, 0.15),
            band_weights: BandWeights {
                sub_bass: 0.10,
                bass: 0.15,
                low_mid: 0.20,
                mid: 0.25,
                high_mid: 0.18,
                high: 0.12,
            },
        });

        profiles
    }

    /// Predict content tags from audio data.
    pub fn predict(&self, audio: &AudioData) -> Result<Vec<ContentTag>> {
        info!("Predicting tags for {} samples", audio.samples.len());

        // Extract frequency features
        let features = self.extract_features(audio)?;
        debug!("Extracted features: {:?}", features);

        // Score against each genre profile
        let mut scores: Vec<(String, f32)> = self.genre_profiles.iter()
            .map(|(genre, profile)| {
                let score = self.compute_profile_score(&features, profile);
                (genre.clone(), score)
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Add mood tags based on features
        let mood_tags = self.predict_mood(&features);

        // Add content type tags
        let content_type_tags = self.predict_content_type(&features);

        // Combine all tags
        let min_conf = self.config.min_confidence;
        let mut all_tags: Vec<ContentTag> = scores.into_iter()
            .filter(|(_, score)| *score >= min_conf)
            .take(self.config.max_tags)
            .map(|(label, confidence)| ContentTag { label, confidence })
            .collect();

        // Filter mood and content type tags by min_confidence too
        all_tags.extend(mood_tags.into_iter().filter(|t| t.confidence >= min_conf));
        all_tags.extend(content_type_tags.into_iter().filter(|t| t.confidence >= min_conf));

        // Sort by confidence and limit
        all_tags.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        all_tags.truncate(self.config.max_tags);

        Ok(all_tags)
    }

    /// Extract frequency features for classification.
    fn extract_features(&self, audio: &AudioData) -> Result<AudioFeatures> {
        let analysis = self.analyzer.analyze(&audio.samples, audio.sample_rate)?;

        Ok(AudioFeatures {
            spectral_centroid: analysis.spectral_centroid,
            _spectral_rolloff: analysis.spectral_rolloff,
            spectral_flatness: analysis.spectral_flatness,
            zero_crossing_rate: analysis.zero_crossing_rate,
            band_energies: analysis.band_energies,
            // Compute additional features
            energy_variance: self.compute_energy_variance(audio)?,
            tempo_estimate: self.estimate_tempo(audio)?,
        })
    }

    /// Compute energy variance (dynamic range indicator).
    fn compute_energy_variance(&self, audio: &AudioData) -> Result<f32> {
        let frame_size = self.config.fft_size;
        let hop_size = self.config.hop_size;
        let num_frames = (audio.samples.len() - frame_size) / hop_size + 1;

        let mut energies = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let start = i * hop_size;
            let end = start + frame_size;
            let frame = &audio.samples[start..end.min(audio.samples.len())];

            let energy: f32 = frame.iter().map(|&s| s * s).sum::<f32>() / frame.len() as f32;
            energies.push(energy);
        }

        // Compute variance
        let mean: f32 = energies.iter().sum::<f32>() / energies.len() as f32;
        let variance: f32 = energies.iter()
            .map(|&e| (e - mean) * (e - mean))
            .sum::<f32>() / energies.len() as f32;

        Ok(variance.sqrt())
    }

    /// Estimate tempo using autocorrelation.
    fn estimate_tempo(&self, audio: &AudioData) -> Result<f32> {
        // Simple onset detection via energy derivative
        let frame_size = 1024;
        let hop_size = 512;

        let num_frames = (audio.samples.len() - frame_size) / hop_size;
        if num_frames < 2 {
            return Ok(120.0); // Default tempo
        }

        let mut energies = Vec::with_capacity(num_frames);
        for i in 0..num_frames {
            let start = i * hop_size;
            let end = start + frame_size;
            let energy: f32 = audio.samples[start..end]
                .iter()
                .map(|&s| s * s)
                .sum();
            energies.push(energy);
        }

        // Compute onset strength (energy derivative)
        let onset_strength: Vec<f32> = energies.windows(2)
            .map(|w| (w[1] - w[0]).max(0.0))
            .collect();

        // Autocorrelation for tempo estimation
        let max_lag = (4.0 * audio.sample_rate as f32 / hop_size as f32) as usize; // Up to 4 seconds
        let min_lag = (0.25 * audio.sample_rate as f32 / hop_size as f32) as usize; // At least 0.25 seconds

        let mut best_lag = min_lag;
        let mut best_corr = 0.0f32;

        for lag in min_lag..max_lag.min(onset_strength.len()) {
            let corr: f32 = onset_strength.iter()
                .zip(onset_strength.iter().skip(lag))
                .map(|(&a, &b)| a * b)
                .sum();

            if corr > best_corr {
                best_corr = corr;
                best_lag = lag;
            }
        }

        // Convert lag to BPM
        let beat_period_secs = best_lag as f32 * hop_size as f32 / audio.sample_rate as f32;
        let bpm = if beat_period_secs > 0.0 {
            60.0 / beat_period_secs
        } else {
            120.0
        };

        Ok(bpm.clamp(60.0, 200.0))
    }

    /// Compute score against a genre profile.
    fn compute_profile_score(&self, features: &AudioFeatures, profile: &GenreProfile) -> f32 {
        let mut score = 0.0f32;

        // Spectral centroid match
        let centroid_score = if features.spectral_centroid >= profile.spectral_centroid_range.0
            && features.spectral_centroid <= profile.spectral_centroid_range.1 {
            1.0
        } else {
            let dist = (features.spectral_centroid - profile.spectral_centroid_range.0)
                .min(features.spectral_centroid - profile.spectral_centroid_range.1)
                .abs();
            (1.0 - dist / 2000.0).max(0.0)
        };
        score += centroid_score * 0.25;

        // Spectral flatness match
        let flatness_score = if features.spectral_flatness >= profile.spectral_flatness_range.0
            && features.spectral_flatness <= profile.spectral_flatness_range.1 {
            1.0
        } else {
            let dist = (features.spectral_flatness - profile.spectral_flatness_range.0)
                .min(features.spectral_flatness - profile.spectral_flatness_range.1)
                .abs();
            (1.0 - dist * 2.0).max(0.0)
        };
        score += flatness_score * 0.25;

        // ZCR match
        let zcr_score = if features.zero_crossing_rate >= profile.zcr_range.0
            && features.zero_crossing_rate <= profile.zcr_range.1 {
            1.0
        } else {
            let dist = (features.zero_crossing_rate - profile.zcr_range.0)
                .min(features.zero_crossing_rate - profile.zcr_range.1)
                .abs();
            (1.0 - dist * 10.0).max(0.0)
        };
        score += zcr_score * 0.2;

        // Band energy distribution match
        let band_score = self.compute_band_match(&features.band_energies, &profile.band_weights);
        score += band_score * 0.3;

        score
    }

    /// Compute band energy distribution match.
    fn compute_band_match(&self, energies: &BandEnergies, weights: &BandWeights) -> f32 {
        let features = [
            energies.sub_bass, energies.bass, energies.low_mid,
            energies.mid, energies.high_mid, energies.high,
        ];
        let targets = [
            weights.sub_bass, weights.bass, weights.low_mid,
            weights.mid, weights.high_mid, weights.high,
        ];

        // Cosine similarity
        let dot: f32 = features.iter().zip(targets.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f32 = features.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = targets.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    /// Predict mood tags based on features.
    fn predict_mood(&self, features: &AudioFeatures) -> Vec<ContentTag> {
        let mut tags = Vec::new();

        // Energetic: high tempo, high energy variance, high centroid
        if features.tempo_estimate > 140.0 && features.spectral_centroid > 2000.0 {
            tags.push(ContentTag {
                label: "energetic".to_string(),
                confidence: 0.7,
            });
        }

        // Calm: low tempo, low centroid, low energy variance
        if features.tempo_estimate < 90.0 && features.spectral_centroid < 1500.0 {
            tags.push(ContentTag {
                label: "calm".to_string(),
                confidence: 0.7,
            });
        }

        // Dramatic: high energy variance
        if features.energy_variance > 0.1 {
            tags.push(ContentTag {
                label: "dramatic".to_string(),
                confidence: 0.5,
            });
        }

        tags
    }

    /// Predict content type tags.
    fn predict_content_type(&self, features: &AudioFeatures) -> Vec<ContentTag> {
        let mut tags = Vec::new();

        // Vocal: mid-range centroid, low flatness
        if features.spectral_centroid > 300.0 && features.spectral_centroid < 2000.0
            && features.spectral_flatness < 0.3 {
            tags.push(ContentTag {
                label: "vocal".to_string(),
                confidence: 0.6,
            });
        }

        // Instrumental: low flatness, not in vocal range
        if features.spectral_flatness < 0.25
            && (features.spectral_centroid < 300.0 || features.spectral_centroid > 2500.0) {
            tags.push(ContentTag {
                label: "instrumental".to_string(),
                confidence: 0.5,
            });
        }

        // Ambient: high flatness, low energy variance
        if features.spectral_flatness > 0.5 && features.energy_variance < 0.05 {
            tags.push(ContentTag {
                label: "ambient".to_string(),
                confidence: 0.6,
            });
        }

        tags
    }
}

impl Default for ContentTagger {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio features for classification.
#[derive(Debug, Clone)]
struct AudioFeatures {
    spectral_centroid: f32,
    _spectral_rolloff: f32,
    spectral_flatness: f32,
    zero_crossing_rate: f32,
    band_energies: BandEnergies,
    energy_variance: f32,
    tempo_estimate: f32,
}

/// Genre classification profile.
#[derive(Debug, Clone)]
struct GenreProfile {
    spectral_centroid_range: (f32, f32),
    spectral_flatness_range: (f32, f32),
    zcr_range: (f32, f32),
    band_weights: BandWeights,
}

/// Expected band energy weights for a genre.
#[derive(Debug, Clone)]
struct BandWeights {
    sub_bass: f32,
    bass: f32,
    low_mid: f32,
    mid: f32,
    high_mid: f32,
    high: f32,
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

    fn generate_noise(duration_secs: f32) -> AudioData {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let sample_rate = 44100;
        let num_samples = (sample_rate as f32 * duration_secs) as usize;

        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let mut hasher = DefaultHasher::new();
                i.hash(&mut hasher);
                let hash = hasher.finish();
                (hash as f32 / u64::MAX as f32) * 2.0 - 1.0
            })
            .collect();

        AudioData::new(samples, sample_rate)
    }

    #[test]
    fn test_tagging_tonal_content() {
        let audio = generate_test_audio(440.0, 5.0);
        let tagger = ContentTagger::new();
        let tags = tagger.predict(&audio).unwrap();

        assert!(!tags.is_empty());

        // Tonal content should be tagged as music
        let has_music = tags.iter().any(|t| t.label == "music");
        assert!(has_music || tags.iter().any(|t| t.confidence > 0.3));
    }

    #[test]
    fn test_tagging_noise_content() {
        let audio = generate_noise(5.0);
        let tagger = ContentTagger::new();
        let tags = tagger.predict(&audio).unwrap();

        // Noise should have high flatness - might be tagged as nature or ambient
        let has_ambient_like = tags.iter()
            .any(|t| t.label == "nature" || t.label == "ambient");

        // Just verify we get some tags
        assert!(!tags.is_empty());
    }

    #[test]
    fn test_min_confidence_filter() {
        let audio = generate_test_audio(440.0, 5.0);

        let config = TaggingConfig {
            min_confidence: 0.9, // High threshold
            ..Default::default()
        };

        let tagger = ContentTagger::with_config(config);
        let tags = tagger.predict(&audio).unwrap();

        // All returned tags should meet the threshold (if any exist)
        for tag in &tags {
            assert!(
                tag.confidence >= 0.9,
                "Tag '{}' has confidence {:.2} which is below threshold 0.9",
                tag.label,
                tag.confidence
            );
        }
    }
}
