//! Frequency analysis CLI commands
//!
//! Provides CLI commands for audio frequency analysis:
//! - Fingerprint generation and verification
//! - Auto-tagging content
//! - Thumbnail selection
//! - Recommendation similarity

use std::path::PathBuf;
use anyhow::Result;
use psm_player_frequency::{
    AudioAnalyzer,
    fingerprint::Fingerprinter,
    tagging::ContentTagger,
    thumbnail::ThumbnailSelector,
    recommend::RecommendationEngine,
    types::*,
};

/// Analyze audio frequencies in a video file.
pub async fn analyze_frequency(
    input: &PathBuf,
    top_k: usize,
    output_json: bool,
) -> Result<()> {
    println!("Analyzing frequencies: {}", input.display());

    let analyzer = AudioAnalyzer::new(44100);
    let audio = analyzer.extract_audio(input).await?;

    println!("\nAudio Info:");
    println!("  Samples: {}", audio.samples.len());
    println!("  Sample Rate: {} Hz", audio.sample_rate);
    println!("  Duration: {:.2}s", audio.samples.len() as f64 / audio.sample_rate as f64);

    // Get dominant frequencies
    let dominant = analyzer.dominant_frequencies(&audio, top_k)?;

    println!("\nDominant Frequencies:");
    println!("  {:>4}  {:>12}  {:>10}", "Rank", "Frequency", "Magnitude");
    println!("  {:->4}  {:->12}  {:->10}", "", "", "");

    for freq in &dominant {
        println!(
            "  {:>4}  {:>10.1} Hz  {:>9.1}%",
            freq.rank,
            freq.frequency_hz,
            freq.magnitude * 100.0
        );
    }

    // Compute spectral analysis
    let analysis = analyzer.analyze(&audio)?;

    println!("\nSpectral Features:");
    println!("  Centroid: {:.1} Hz (brightness)", analysis.spectral_centroid);
    println!("  Rolloff: {:.1} Hz (95% energy)", analysis.spectral_rolloff);
    println!("  Flatness: {:.4} (0=tonal, 1=noise)", analysis.spectral_flatness);
    println!("  ZCR: {:.4} (zero crossing rate)", analysis.zero_crossing_rate);

    println!("\nBand Energies:");
    println!("  Sub-bass (20-60 Hz):    {:>5.1}%", analysis.band_energies.sub_bass * 100.0);
    println!("  Bass (60-250 Hz):       {:>5.1}%", analysis.band_energies.bass * 100.0);
    println!("  Low-mid (250-500 Hz):   {:>5.1}%", analysis.band_energies.low_mid * 100.0);
    println!("  Mid (500-2000 Hz):      {:>5.1}%", analysis.band_energies.mid * 100.0);
    println!("  High-mid (2000-4000 Hz):{:>5.1}%", analysis.band_energies.high_mid * 100.0);
    println!("  High (4000+ Hz):        {:>5.1}%", analysis.band_energies.high * 100.0);

    if output_json {
        let result = serde_json::json!({
            "dominant_frequencies": dominant,
            "spectral_features": {
                "centroid": analysis.spectral_centroid,
                "rolloff": analysis.spectral_rolloff,
                "flatness": analysis.spectral_flatness,
                "zcr": analysis.zero_crossing_rate,
            },
            "band_energies": analysis.band_energies,
        });
        println!("\nJSON Output:");
        println!("{}", serde_json::to_string_pretty(&result)?);
    }

    Ok(())
}

/// Generate audio fingerprint for content verification.
pub async fn fingerprint(
    input: &PathBuf,
    output: Option<PathBuf>,
    verify_hash: Option<String>,
) -> Result<()> {
    println!("Generating fingerprint: {}", input.display());

    let analyzer = AudioAnalyzer::new(44100);
    let audio = analyzer.extract_audio(input).await?;

    let fingerprinter = Fingerprinter::new();

    if let Some(expected_hash) = verify_hash {
        // Verification mode
        println!("\nVerifying against hash: {}", expected_hash);
        let result = fingerprinter.verify(&audio, &expected_hash)?;

        if result.verified {
            println!("\n✓ VERIFIED - Content matches fingerprint");
        } else {
            println!("\n✗ MISMATCH - Content does not match fingerprint");
            println!("  Expected: {}", result.expected_hash);
            println!("  Computed: {}", result.computed_hash);
            std::process::exit(1);
        }
    } else {
        // Generation mode
        let fp = fingerprinter.fingerprint(&audio)?;

        println!("\nFingerprint Generated:");
        println!("  Hash: {}", fp.hash);
        println!("  Version: {}", fp.version);
        println!("  Duration: {:.2}s", fp.duration_secs);
        println!("  Constellation Points: {}", fp.points.len());

        // Save if output specified
        if let Some(path) = output {
            let json = serde_json::to_string_pretty(&fp)?;
            std::fs::write(&path, &json)?;
            println!("\nSaved to: {}", path.display());
        }

        println!("\nTo verify later, run:");
        println!("  psm-cli fingerprint {} --verify {}", input.display(), fp.hash);
    }

    Ok(())
}

/// Auto-tag content based on audio analysis.
pub async fn autotag(
    input: &PathBuf,
    max_tags: usize,
    min_confidence: f32,
) -> Result<()> {
    println!("Auto-tagging: {}", input.display());

    let analyzer = AudioAnalyzer::new(44100);
    let audio = analyzer.extract_audio(input).await?;

    let tagger = ContentTagger::new();
    let tags = tagger.predict(&audio)?;

    println!("\nSuggested Tags:");
    println!("  {:>20}  {:>10}", "Tag", "Confidence");
    println!("  {:->20}  {:->10}", "", "");

    let filtered: Vec<_> = tags.iter()
        .filter(|t| t.confidence >= min_confidence)
        .take(max_tags)
        .collect();

    if filtered.is_empty() {
        println!("  No tags above confidence threshold ({:.0}%)", min_confidence * 100.0);
    } else {
        for tag in filtered {
            println!("  {:>20}  {:>9.0}%", tag.label, tag.confidence * 100.0);
        }
    }

    Ok(())
}

/// Select optimal thumbnail timestamp.
pub async fn thumbnail(
    input: &PathBuf,
    output: Option<PathBuf>,
    num_candidates: usize,
) -> Result<()> {
    println!("Finding optimal thumbnail: {}", input.display());

    let analyzer = AudioAnalyzer::new(44100);
    let audio = analyzer.extract_audio(input).await?;

    let selector = ThumbnailSelector::new();

    if num_candidates > 1 {
        // Show multiple candidates
        let candidates = selector.find_candidates(input, &audio, num_candidates)?;

        println!("\nThumbnail Candidates:");
        println!("  {:>4}  {:>10}  {:>10}  {:>10}  {:>10}",
            "Rank", "Timestamp", "Sharpness", "Contrast", "Score");
        println!("  {:->4}  {:->10}  {:->10}  {:->10}  {:->10}", "", "", "", "", "");

        for (i, c) in candidates.iter().enumerate() {
            println!(
                "  {:>4}  {:>9.2}s  {:>9.1}%  {:>9.1}%  {:>9.3}",
                i + 1,
                c.timestamp,
                c.sharpness * 100.0,
                c.contrast * 100.0,
                c.total_score
            );
        }

        // Extract first candidate if output specified
        if let Some(path) = output {
            if let Some(best) = candidates.first() {
                selector.extract_thumbnail(input, best.timestamp, &path)?;
                println!("\nExtracted thumbnail at {:.2}s to: {}", best.timestamp, path.display());
            }
        }
    } else {
        // Just get best timestamp
        let timestamp = selector.find_best_timestamp(input, &audio)?;
        println!("\nBest timestamp: {:.2}s", timestamp);

        if let Some(path) = output {
            selector.extract_thumbnail(input, timestamp, &path)?;
            println!("Extracted to: {}", path.display());
        } else {
            println!("\nTo extract thumbnail, run:");
            println!("  psm-cli thumbnail {} --output thumbnail.jpg", input.display());
        }
    }

    Ok(())
}

/// Find similar content using frequency signatures.
pub async fn similar(
    input: &PathBuf,
    library_dir: &PathBuf,
    limit: usize,
) -> Result<()> {
    println!("Finding similar content to: {}", input.display());
    println!("Scanning library: {}", library_dir.display());

    let analyzer = AudioAnalyzer::new(44100);
    let mut engine = RecommendationEngine::new();

    // Index library
    let entries = std::fs::read_dir(library_dir)?;
    let video_extensions = ["mp4", "mkv", "avi", "mov", "webm"];

    println!("\nIndexing library...");
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if video_extensions.contains(&ext.to_str().unwrap_or("")) {
                match analyzer.extract_audio(&path).await {
                    Ok(audio) => {
                        let id = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        if engine.add_content(&id, &audio, None).is_ok() {
                            println!("  Indexed: {}", id);
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
    }

    println!("\nIndexed {} items", engine.len());

    // Analyze input
    let input_audio = analyzer.extract_audio(input).await?;
    let recommendations = engine.get_recommendations_for_audio(&input_audio, limit)?;

    if recommendations.is_empty() {
        println!("\nNo similar content found.");
    } else {
        println!("\nSimilar Content:");
        println!("  {:>4}  {:>30}  {:>10}  {}", "Rank", "File", "Similarity", "Features");
        println!("  {:->4}  {:->30}  {:->10}  {:->20}", "", "", "", "");

        for (i, rec) in recommendations.iter().enumerate() {
            println!(
                "  {:>4}  {:>30}  {:>9.1}%  {}",
                i + 1,
                &rec.content_id[..rec.content_id.len().min(30)],
                rec.similarity * 100.0,
                rec.matching_features.join(", ")
            );
        }
    }

    Ok(())
}

/// Process a video through the complete frequency pipeline.
pub async fn process(
    input: &PathBuf,
    output_dir: &PathBuf,
    skip_fingerprint: bool,
    skip_tags: bool,
    skip_thumbnail: bool,
) -> Result<()> {
    println!("Processing video: {}", input.display());
    println!("Output directory: {}", output_dir.display());

    std::fs::create_dir_all(output_dir)?;

    let analyzer = AudioAnalyzer::new(44100);
    let audio = analyzer.extract_audio(input).await?;

    let mut result = ProcessingResult {
        content_id: uuid::Uuid::new_v4().to_string(),
        fingerprint: None,
        tags: Vec::new(),
        thumbnail_timestamp: None,
        signature: None,
        dominant_frequencies: analyzer.dominant_frequencies(&audio, 10)?,
    };

    // Fingerprint
    if !skip_fingerprint {
        println!("\n[1/3] Generating fingerprint...");
        let fingerprinter = Fingerprinter::new();
        let fp = fingerprinter.fingerprint(&audio)?;
        println!("  Hash: {}", fp.hash);
        result.fingerprint = Some(fp);
    }

    // Tags
    if !skip_tags {
        println!("\n[2/3] Auto-tagging...");
        let tagger = ContentTagger::new();
        let tags = tagger.predict(&audio)?;
        for tag in &tags {
            println!("  {}: {:.0}%", tag.label, tag.confidence * 100.0);
        }
        result.tags = tags;
    }

    // Thumbnail
    if !skip_thumbnail {
        println!("\n[3/3] Selecting thumbnail...");
        let selector = ThumbnailSelector::new();
        let timestamp = selector.find_best_timestamp(input, &audio)?;
        println!("  Best timestamp: {:.2}s", timestamp);

        let thumb_path = output_dir.join("thumbnail.jpg");
        selector.extract_thumbnail(input, timestamp, &thumb_path)?;
        println!("  Saved: {}", thumb_path.display());

        result.thumbnail_timestamp = Some(timestamp);
    }

    // Save complete result
    let result_path = output_dir.join("analysis.json");
    let json = serde_json::to_string_pretty(&result)?;
    std::fs::write(&result_path, &json)?;

    println!("\n✓ Processing complete!");
    println!("  Results saved to: {}", result_path.display());

    Ok(())
}
