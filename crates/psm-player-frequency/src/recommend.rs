//! Content recommendation engine using frequency similarity.
//!
//! This module provides content-based recommendations by comparing
//! frequency signatures of audio content. It supports:
//!
//! - **Similar content**: Find content with similar audio characteristics
//! - **User preferences**: Learn user taste from watch history
//! - **Hybrid scoring**: Combine multiple similarity metrics

use std::collections::HashMap;
use anyhow::Result;
use tracing::{debug, info};

use crate::fft::FrequencyAnalyzer;
use crate::types::*;

/// Configuration for the recommendation engine.
#[derive(Debug, Clone)]
pub struct RecommendConfig {
    /// Number of features in frequency signature
    pub signature_size: usize,
    /// Weight for frequency signature similarity
    pub signature_weight: f32,
    /// Weight for band energy similarity
    pub band_weight: f32,
    /// Weight for spectral features similarity
    pub spectral_weight: f32,
    /// Minimum similarity threshold for recommendations
    pub min_similarity: f32,
}

impl Default for RecommendConfig {
    fn default() -> Self {
        Self {
            signature_size: 128,
            signature_weight: 0.5,
            band_weight: 0.3,
            spectral_weight: 0.2,
            min_similarity: 0.3,
        }
    }
}

/// Content-based recommendation engine.
pub struct RecommendationEngine {
    config: RecommendConfig,
    /// Content signatures indexed by content ID
    content_index: HashMap<String, ContentEntry>,
    /// Analyzer for computing signatures
    analyzer: FrequencyAnalyzer,
}

impl RecommendationEngine {
    /// Create a new recommendation engine.
    pub fn new() -> Self {
        Self::with_config(RecommendConfig::default())
    }

    /// Create an engine with custom configuration.
    pub fn with_config(config: RecommendConfig) -> Self {
        Self {
            config,
            content_index: HashMap::new(),
            analyzer: FrequencyAnalyzer::new(4096, 2048),
        }
    }

    /// Add content to the recommendation index.
    pub fn add_content(
        &mut self,
        content_id: &str,
        audio: &AudioData,
        metadata: Option<ContentMetadata>,
    ) -> Result<()> {
        let signature = self.analyzer.compute_signature(&audio.samples, audio.sample_rate)?;

        info!("Indexed content: {} (signature size: {})", content_id, signature.features.len());

        self.content_index.insert(content_id.to_string(), ContentEntry {
            content_id: content_id.to_string(),
            signature,
            metadata,
        });

        Ok(())
    }

    /// Add content with a pre-computed signature.
    pub fn add_content_with_signature(
        &mut self,
        content_id: &str,
        signature: FrequencySignature,
        metadata: Option<ContentMetadata>,
    ) {
        self.content_index.insert(content_id.to_string(), ContentEntry {
            content_id: content_id.to_string(),
            signature,
            metadata,
        });
    }

    /// Remove content from the index.
    pub fn remove_content(&mut self, content_id: &str) -> bool {
        self.content_index.remove(content_id).is_some()
    }

    /// Get recommendations for a specific content item.
    pub fn get_similar(
        &self,
        content_id: &str,
        limit: usize,
    ) -> Vec<Recommendation> {
        let target = match self.content_index.get(content_id) {
            Some(entry) => &entry.signature,
            None => return Vec::new(),
        };

        self.find_similar_to_signature(target, Some(content_id), limit)
    }

    /// Get recommendations based on audio data.
    pub fn get_recommendations_for_audio(
        &self,
        audio: &AudioData,
        limit: usize,
    ) -> Result<Vec<Recommendation>> {
        let signature = self.analyzer.compute_signature(&audio.samples, audio.sample_rate)?;
        Ok(self.find_similar_to_signature(&signature, None, limit))
    }

    /// Get personalized recommendations based on user watch history.
    pub fn get_user_recommendations(
        &self,
        watch_history: &[String],
        limit: usize,
    ) -> Vec<Recommendation> {
        if watch_history.is_empty() {
            return Vec::new();
        }

        // Compute average signature from watch history
        let history_signatures: Vec<&FrequencySignature> = watch_history.iter()
            .filter_map(|id| self.content_index.get(id))
            .map(|entry| &entry.signature)
            .collect();

        if history_signatures.is_empty() {
            return Vec::new();
        }

        let avg_signature = self.average_signatures(&history_signatures);

        // Find similar content not in history
        let mut recommendations = self.find_similar_to_signature(&avg_signature, None, limit * 2);

        // Filter out already watched
        recommendations.retain(|r| !watch_history.contains(&r.content_id));
        recommendations.truncate(limit);

        recommendations
    }

    /// Get diverse recommendations (explore vs exploit).
    pub fn get_diverse_recommendations(
        &self,
        watch_history: &[String],
        explore_ratio: f32,
        limit: usize,
    ) -> Vec<Recommendation> {
        let exploit_count = ((1.0 - explore_ratio) * limit as f32) as usize;
        let explore_count = limit - exploit_count;

        // Exploit: similar to history
        let mut exploit_recs = self.get_user_recommendations(watch_history, exploit_count);

        // Explore: random diverse content
        let mut explore_recs = self.get_diverse_content(watch_history, explore_count);

        // Interleave results
        let mut results = Vec::with_capacity(limit);
        let mut exploit_iter = exploit_recs.drain(..);
        let mut explore_iter = explore_recs.drain(..);

        for i in 0..limit {
            if i % 3 == 2 {
                // Every 3rd item is exploratory
                if let Some(r) = explore_iter.next() {
                    results.push(r);
                } else if let Some(r) = exploit_iter.next() {
                    results.push(r);
                }
            } else {
                if let Some(r) = exploit_iter.next() {
                    results.push(r);
                } else if let Some(r) = explore_iter.next() {
                    results.push(r);
                }
            }
        }

        results
    }

    /// Find content similar to a signature.
    fn find_similar_to_signature(
        &self,
        target: &FrequencySignature,
        exclude_id: Option<&str>,
        limit: usize,
    ) -> Vec<Recommendation> {
        let mut similarities: Vec<(String, f32, Vec<String>)> = self.content_index.iter()
            .filter(|(id, _)| exclude_id.map_or(true, |ex| *id != ex))
            .map(|(id, entry)| {
                let (similarity, features) = self.compute_similarity(target, &entry.signature);
                (id.clone(), similarity, features)
            })
            .filter(|(_, sim, _)| *sim >= self.config.min_similarity)
            .collect();

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        similarities.into_iter()
            .take(limit)
            .map(|(content_id, similarity, matching_features)| Recommendation {
                content_id,
                similarity,
                matching_features,
            })
            .collect()
    }

    /// Compute similarity between two signatures.
    fn compute_similarity(
        &self,
        sig1: &FrequencySignature,
        sig2: &FrequencySignature,
    ) -> (f32, Vec<String>) {
        let mut matching_features = Vec::new();

        // Feature vector cosine similarity
        let feature_sim = sig1.similarity(sig2);
        if feature_sim > 0.7 {
            matching_features.push("frequency_pattern".to_string());
        }

        // Band energy similarity
        let band_sim = self.band_similarity(&sig1.band_energies, &sig2.band_energies);
        if band_sim > 0.8 {
            matching_features.push("energy_distribution".to_string());
        }

        // Spectral feature similarity
        let centroid_diff = (sig1.centroid - sig2.centroid).abs() / sig1.centroid.max(sig2.centroid).max(1.0);
        let flatness_diff = (sig1.flatness - sig2.flatness).abs();

        let spectral_sim = 1.0 - (centroid_diff * 0.5 + flatness_diff * 0.5);
        if spectral_sim > 0.8 {
            matching_features.push("tonal_quality".to_string());
        }

        // Weighted combination
        let total_similarity =
            feature_sim * self.config.signature_weight +
            band_sim * self.config.band_weight +
            spectral_sim * self.config.spectral_weight;

        (total_similarity, matching_features)
    }

    /// Compute band energy similarity.
    fn band_similarity(&self, band1: &BandEnergies, band2: &BandEnergies) -> f32 {
        let v1 = band1.to_vec();
        let v2 = band2.to_vec();

        // Cosine similarity
        let dot: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = v1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = v2.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm1 > 0.0 && norm2 > 0.0 {
            dot / (norm1 * norm2)
        } else {
            0.0
        }
    }

    /// Compute average of multiple signatures.
    fn average_signatures(&self, signatures: &[&FrequencySignature]) -> FrequencySignature {
        if signatures.is_empty() {
            return FrequencySignature {
                features: vec![0.0; self.config.signature_size],
                band_energies: BandEnergies {
                    sub_bass: 0.0,
                    bass: 0.0,
                    low_mid: 0.0,
                    mid: 0.0,
                    high_mid: 0.0,
                    high: 0.0,
                },
                centroid: 0.0,
                flatness: 0.0,
            };
        }

        let n = signatures.len() as f32;
        let feature_len = signatures[0].features.len();

        // Average features
        let mut avg_features = vec![0.0f32; feature_len];
        for sig in signatures {
            for (i, &f) in sig.features.iter().enumerate() {
                if i < feature_len {
                    avg_features[i] += f / n;
                }
            }
        }

        // Average band energies
        let avg_band = BandEnergies {
            sub_bass: signatures.iter().map(|s| s.band_energies.sub_bass).sum::<f32>() / n,
            bass: signatures.iter().map(|s| s.band_energies.bass).sum::<f32>() / n,
            low_mid: signatures.iter().map(|s| s.band_energies.low_mid).sum::<f32>() / n,
            mid: signatures.iter().map(|s| s.band_energies.mid).sum::<f32>() / n,
            high_mid: signatures.iter().map(|s| s.band_energies.high_mid).sum::<f32>() / n,
            high: signatures.iter().map(|s| s.band_energies.high).sum::<f32>() / n,
        };

        // Average spectral features
        let avg_centroid = signatures.iter().map(|s| s.centroid).sum::<f32>() / n;
        let avg_flatness = signatures.iter().map(|s| s.flatness).sum::<f32>() / n;

        FrequencySignature {
            features: avg_features,
            band_energies: avg_band,
            centroid: avg_centroid,
            flatness: avg_flatness,
        }
    }

    /// Get diverse content for exploration.
    fn get_diverse_content(&self, exclude: &[String], limit: usize) -> Vec<Recommendation> {
        // Simple diversity: pick content with different band energy profiles
        let mut clusters: HashMap<usize, Vec<&ContentEntry>> = HashMap::new();

        for entry in self.content_index.values() {
            if exclude.contains(&entry.content_id) {
                continue;
            }

            // Classify by dominant band
            let bands = entry.signature.band_energies.to_vec();
            let dominant_band = bands.iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);

            clusters.entry(dominant_band).or_default().push(entry);
        }

        // Pick from each cluster
        let mut results = Vec::new();
        let mut cluster_iter = clusters.values().cycle();
        let mut seen = std::collections::HashSet::new();

        while results.len() < limit {
            if let Some(cluster) = cluster_iter.next() {
                for entry in cluster {
                    if !seen.contains(&entry.content_id) {
                        seen.insert(entry.content_id.clone());
                        results.push(Recommendation {
                            content_id: entry.content_id.clone(),
                            similarity: 0.5, // Exploration score
                            matching_features: vec!["diverse".to_string()],
                        });
                        break;
                    }
                }
            }

            // Prevent infinite loop
            if seen.len() >= self.content_index.len() {
                break;
            }
        }

        results.truncate(limit);
        results
    }

    /// Get the number of indexed items.
    pub fn len(&self) -> usize {
        self.content_index.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.content_index.is_empty()
    }

    /// Export the index for persistence.
    pub fn export_index(&self) -> Vec<(String, FrequencySignature)> {
        self.content_index.iter()
            .map(|(id, entry)| (id.clone(), entry.signature.clone()))
            .collect()
    }

    /// Import signatures from persistence.
    pub fn import_index(&mut self, data: Vec<(String, FrequencySignature)>) {
        for (id, signature) in data {
            self.content_index.insert(id.clone(), ContentEntry {
                content_id: id,
                signature,
                metadata: None,
            });
        }
    }
}

impl Default for RecommendationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal content entry in the index.
#[derive(Debug, Clone)]
struct ContentEntry {
    content_id: String,
    signature: FrequencySignature,
    metadata: Option<ContentMetadata>,
}

/// Optional metadata for content items.
#[derive(Debug, Clone)]
pub struct ContentMetadata {
    /// Content title
    pub title: Option<String>,
    /// Creator ID
    pub creator_id: Option<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Duration in seconds
    pub duration_secs: Option<f64>,
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
    fn test_add_and_retrieve() {
        let mut engine = RecommendationEngine::new();

        let audio1 = generate_test_audio(440.0, 5.0);
        let audio2 = generate_test_audio(880.0, 5.0);

        engine.add_content("content_1", &audio1, None).unwrap();
        engine.add_content("content_2", &audio2, None).unwrap();

        assert_eq!(engine.len(), 2);
    }

    #[test]
    fn test_similar_content() {
        let mut engine = RecommendationEngine::new();

        // Similar frequencies
        let audio1 = generate_test_audio(440.0, 5.0);
        let audio2 = generate_test_audio(445.0, 5.0);  // Very close to 440
        let audio3 = generate_test_audio(1000.0, 5.0); // Different

        engine.add_content("similar_1", &audio1, None).unwrap();
        engine.add_content("similar_2", &audio2, None).unwrap();
        engine.add_content("different", &audio3, None).unwrap();

        let recommendations = engine.get_similar("similar_1", 2);

        // similar_2 should be ranked higher than different
        assert!(!recommendations.is_empty());
        if recommendations.len() >= 2 {
            let sim_to_close = recommendations.iter()
                .find(|r| r.content_id == "similar_2")
                .map(|r| r.similarity);
            let sim_to_diff = recommendations.iter()
                .find(|r| r.content_id == "different")
                .map(|r| r.similarity);

            if let (Some(s1), Some(s2)) = (sim_to_close, sim_to_diff) {
                assert!(s1 >= s2);
            }
        }
    }

    #[test]
    fn test_user_recommendations() {
        let mut engine = RecommendationEngine::new();

        // User watched low-frequency content
        let audio1 = generate_test_audio(200.0, 5.0);
        let audio2 = generate_test_audio(250.0, 5.0);

        // Unwatched content
        let audio3 = generate_test_audio(220.0, 5.0);  // Similar
        let audio4 = generate_test_audio(5000.0, 5.0); // Different

        engine.add_content("watched_1", &audio1, None).unwrap();
        engine.add_content("watched_2", &audio2, None).unwrap();
        engine.add_content("unwatched_similar", &audio3, None).unwrap();
        engine.add_content("unwatched_different", &audio4, None).unwrap();

        let history = vec!["watched_1".to_string(), "watched_2".to_string()];
        let recommendations = engine.get_user_recommendations(&history, 2);

        // Should not recommend already watched content
        for rec in &recommendations {
            assert!(!history.contains(&rec.content_id));
        }
    }

    #[test]
    fn test_export_import() {
        let mut engine1 = RecommendationEngine::new();

        let audio = generate_test_audio(440.0, 5.0);
        engine1.add_content("test_content", &audio, None).unwrap();

        // Export
        let exported = engine1.export_index();
        assert_eq!(exported.len(), 1);

        // Import into new engine
        let mut engine2 = RecommendationEngine::new();
        engine2.import_index(exported);

        assert_eq!(engine2.len(), 1);
    }
}
