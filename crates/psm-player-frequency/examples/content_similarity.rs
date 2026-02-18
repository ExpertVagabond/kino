//! Content Similarity Example
//!
//! Demonstrates how to find similar content using frequency signatures.
//!
//! # Usage
//! ```bash
//! cargo run --example content_similarity --features recommend -- audio1.wav audio2.wav
//! ```

use anyhow::Result;
use psm_player_frequency::{AudioData, FrequencyAnalyzer};
use std::env;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <audio_file1> <audio_file2>", args[0]);
        eprintln!("\nCompares two audio files for similarity.");
        std::process::exit(1);
    }

    let file1 = &args[1];
    let file2 = &args[2];

    println!("Comparing audio files:");
    println!("  File 1: {}", file1);
    println!("  File 2: {}", file2);
    println!("{}", "-".repeat(50));

    // Load both files
    let audio1 = load_wav(file1)?;
    let audio2 = load_wav(file2)?;

    // Create analyzer
    let analyzer = FrequencyAnalyzer::new(4096, 2048);

    // Compute signatures
    println!("\nComputing frequency signatures...");
    let sig1 = analyzer.compute_signature(&audio1.samples, audio1.sample_rate)?;
    let sig2 = analyzer.compute_signature(&audio2.samples, audio2.sample_rate)?;

    // Compare feature vectors
    let feature_similarity = cosine_similarity(&sig1.features, &sig2.features);
    println!("  Feature similarity:   {:.2}%", feature_similarity * 100.0);

    // Compare band energies
    let bands1 = sig1.band_energies.to_vec();
    let bands2 = sig2.band_energies.to_vec();
    let band_similarity = cosine_similarity(&bands1, &bands2);
    println!("  Band similarity:      {:.2}%", band_similarity * 100.0);

    // Compare centroid and flatness
    let centroid_diff = (sig1.centroid - sig2.centroid).abs() / sig1.centroid.max(sig2.centroid).max(1.0);
    let flatness_diff = (sig1.flatness - sig2.flatness).abs() / sig1.flatness.max(sig2.flatness).max(0.001);

    println!("  Centroid difference:  {:.1}%", centroid_diff * 100.0);
    println!("  Flatness difference:  {:.1}%", flatness_diff * 100.0);

    // Use built-in similarity
    let overall = sig1.similarity(&sig2);
    println!("\n  Overall similarity:   {:.2}%", overall * 100.0);

    // Interpretation
    println!("\nInterpretation:");
    if overall > 0.9 {
        println!("  -> Very similar content (possible duplicate or remix)");
    } else if overall > 0.7 {
        println!("  -> Similar content (same genre/style)");
    } else if overall > 0.5 {
        println!("  -> Moderately similar content");
    } else if overall > 0.3 {
        println!("  -> Slightly similar content");
    } else {
        println!("  -> Different content");
    }

    // Spectral comparison
    println!("\nSpectral Comparison:");
    let analysis1 = analyzer.analyze(&audio1.samples, audio1.sample_rate)?;
    let analysis2 = analyzer.analyze(&audio2.samples, audio2.sample_rate)?;

    println!(
        "  Spectral centroid: {:.1} Hz vs {:.1} Hz (diff: {:.1}%)",
        analysis1.spectral_centroid,
        analysis2.spectral_centroid,
        ((analysis1.spectral_centroid - analysis2.spectral_centroid).abs()
            / analysis1.spectral_centroid.max(analysis2.spectral_centroid).max(1.0))
            * 100.0
    );

    println!(
        "  Spectral flatness: {:.4} vs {:.4} (diff: {:.1}%)",
        analysis1.spectral_flatness,
        analysis2.spectral_flatness,
        ((analysis1.spectral_flatness - analysis2.spectral_flatness).abs()
            / analysis1.spectral_flatness.max(analysis2.spectral_flatness).max(0.001))
            * 100.0
    );

    // Band energy comparison
    println!("\nBand Energy Comparison:");
    let bands1 = &analysis1.band_energies;
    let bands2 = &analysis2.band_energies;

    println!("                     File 1    File 2    Diff");
    println!(
        "  Sub-bass:         {:>7.4}   {:>7.4}   {:>+.4}",
        bands1.sub_bass,
        bands2.sub_bass,
        bands1.sub_bass - bands2.sub_bass
    );
    println!(
        "  Bass:             {:>7.4}   {:>7.4}   {:>+.4}",
        bands1.bass,
        bands2.bass,
        bands1.bass - bands2.bass
    );
    println!(
        "  Low-mid:          {:>7.4}   {:>7.4}   {:>+.4}",
        bands1.low_mid,
        bands2.low_mid,
        bands1.low_mid - bands2.low_mid
    );
    println!(
        "  Mid:              {:>7.4}   {:>7.4}   {:>+.4}",
        bands1.mid,
        bands2.mid,
        bands1.mid - bands2.mid
    );
    println!(
        "  High-mid:         {:>7.4}   {:>7.4}   {:>+.4}",
        bands1.high_mid,
        bands2.high_mid,
        bands1.high_mid - bands2.high_mid
    );
    println!(
        "  High:             {:>7.4}   {:>7.4}   {:>+.4}",
        bands1.high,
        bands2.high,
        bands1.high - bands2.high
    );

    Ok(())
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

fn load_wav(path: &str) -> Result<AudioData> {
    let reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / 32768.0)
        .collect();

    let duration_secs = samples.len() as f64 / spec.sample_rate as f64;

    Ok(AudioData {
        samples,
        sample_rate: spec.sample_rate,
        channels: spec.channels as u32,
        duration_secs,
    })
}
