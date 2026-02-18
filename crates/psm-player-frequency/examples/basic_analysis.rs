//! Basic Frequency Analysis Example
//!
//! Demonstrates how to use the psm-player-frequency crate for audio analysis.
//!
//! # Usage
//! ```bash
//! cargo run --example basic_analysis -- path/to/audio.wav
//! ```

use anyhow::Result;
use psm_player_frequency::{AudioData, FrequencyAnalyzer};
use std::env;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get file path from arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <audio_file>", args[0]);
        eprintln!("\nSupported formats: WAV");
        std::process::exit(1);
    }

    let file_path = &args[1];
    println!("Analyzing: {}", file_path);
    println!("{}", "-".repeat(50));

    // Load audio file
    let audio = load_wav(file_path)?;
    println!("Loaded {} samples at {}Hz", audio.samples.len(), audio.sample_rate);

    // Create analyzer
    let analyzer = FrequencyAnalyzer::new(4096, 2048);

    // Get dominant frequencies
    println!("\nDominant Frequencies:");
    let dominant = analyzer.dominant_frequencies(&audio.samples, audio.sample_rate, 10)?;
    for (i, freq) in dominant.iter().enumerate() {
        println!(
            "  {}. {:>8.1} Hz  (magnitude: {:.4})",
            i + 1,
            freq.frequency_hz,
            freq.magnitude
        );
    }

    // Compute frequency signature
    println!("\nFrequency Signature:");
    let signature = analyzer.compute_signature(&audio.samples, audio.sample_rate)?;

    println!("  Feature vector: {} dimensions", signature.features.len());
    println!("  Centroid:       {:.1} Hz", signature.centroid);
    println!("  Flatness:       {:.4}", signature.flatness);

    // Full analysis
    println!("\nFull Analysis:");
    let analysis = analyzer.analyze(&audio.samples, audio.sample_rate)?;

    println!("  Spectral centroid:  {:.1} Hz", analysis.spectral_centroid);
    println!("  Spectral rolloff:   {:.1} Hz", analysis.spectral_rolloff);
    println!("  Spectral flatness:  {:.4}", analysis.spectral_flatness);
    println!("  Zero crossing rate: {:.4}", analysis.zero_crossing_rate);

    println!("\nBand Energies:");
    println!("  Sub-bass (20-60 Hz):    {:.4}", analysis.band_energies.sub_bass);
    println!("  Bass (60-250 Hz):       {:.4}", analysis.band_energies.bass);
    println!("  Low-mid (250-500 Hz):   {:.4}", analysis.band_energies.low_mid);
    println!("  Mid (500-2000 Hz):      {:.4}", analysis.band_energies.mid);
    println!("  High-mid (2-4 kHz):     {:.4}", analysis.band_energies.high_mid);
    println!("  High (4-20 kHz):        {:.4}", analysis.band_energies.high);

    Ok(())
}

/// Load a WAV file into AudioData
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
