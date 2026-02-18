//! Video Processing Example
//!
//! Complete video processing pipeline: extract audio, fingerprint,
//! auto-tag, select thumbnail, and compute similarity signature.
//!
//! # Usage
//! ```bash
//! cargo run --example video_processing --features full -- video.mp4
//! ```

use anyhow::Result;
use kino_frequency::{process_video, ProcessingConfig, ProcessingResult};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <video_file>", args[0]);
        eprintln!("\nProcesses a video through the complete frequency analysis pipeline.");
        std::process::exit(1);
    }

    let video_path = &args[1];
    println!("Processing video: {}", video_path);
    println!("{}", "=".repeat(60));

    // Configure processing
    let config = ProcessingConfig {
        sample_rate: 44100,
        enable_fingerprint: true,
        enable_tagging: true,
        enable_thumbnail: true,
        enable_signature: true,
    };

    // Process the video
    println!("\n1. Extracting and analyzing audio...");
    let result = process_video(video_path, config).await?;

    // Display results
    print_results(&result);

    // Export as JSON
    let json = serde_json::to_string_pretty(&result)?;
    println!("\n{}", "=".repeat(60));
    println!("JSON Output:");
    println!("{}", json);

    Ok(())
}

fn print_results(result: &ProcessingResult) {
    println!("\n{}", "-".repeat(60));
    println!("PROCESSING RESULTS");
    println!("{}", "-".repeat(60));

    println!("\nContent ID: {}", result.content_id);

    // Fingerprint
    if let Some(ref fp) = result.fingerprint {
        println!("\n2. Audio Fingerprint:");
        println!("   Hash:        {}...", &fp.hash[..32.min(fp.hash.len())]);
        println!("   Duration:    {:.2}s", fp.duration_secs);
        println!("   Version:     {}", fp.version);
        println!("   Points:      {}", fp.points.len());
    }

    // Tags
    if !result.tags.is_empty() {
        println!("\n3. Auto-Generated Tags:");
        for tag in &result.tags {
            let bar = "#".repeat((tag.confidence * 20.0) as usize);
            println!(
                "   {:<20} {} ({:.0}%)",
                tag.label,
                bar,
                tag.confidence * 100.0
            );
        }
    }

    // Thumbnail
    if let Some(timestamp) = result.thumbnail_timestamp {
        println!("\n4. Recommended Thumbnail:");
        println!("   Timestamp: {:.2}s", timestamp);
        println!(
            "   Time code: {:02}:{:02}.{:03}",
            (timestamp / 60.0) as u32,
            (timestamp % 60.0) as u32,
            ((timestamp * 1000.0) % 1000.0) as u32
        );
    }

    // Dominant frequencies
    if !result.dominant_frequencies.is_empty() {
        println!("\n5. Dominant Frequencies:");
        for (i, freq) in result.dominant_frequencies.iter().enumerate() {
            let note = frequency_to_note(freq.frequency_hz);
            println!(
                "   {}. {:>8.1} Hz ({:>4}) - magnitude: {:.4}",
                i + 1,
                freq.frequency_hz,
                note,
                freq.magnitude
            );
        }
    }

    // Signature
    if let Some(ref sig) = result.signature {
        println!("\n6. Frequency Signature:");
        println!("   Features:  {} dimensions", sig.features.len());
        println!("   Centroid:  {:.1} Hz", sig.centroid);
        println!("   Flatness:  {:.4}", sig.flatness);

        // Visualize band energies as ASCII bars
        println!("\n   Band Energy Distribution:");
        let bands = &sig.band_energies;
        let max_energy = [
            bands.sub_bass, bands.bass, bands.low_mid,
            bands.mid, bands.high_mid, bands.high
        ].iter().cloned().fold(0.0f32, f32::max);

        if max_energy > 0.0 {
            let bar = |energy: f32| "#".repeat(((energy / max_energy) * 30.0) as usize);
            println!("   Sub-bass:  {}", bar(bands.sub_bass));
            println!("   Bass:      {}", bar(bands.bass));
            println!("   Low-mid:   {}", bar(bands.low_mid));
            println!("   Mid:       {}", bar(bands.mid));
            println!("   High-mid:  {}", bar(bands.high_mid));
            println!("   High:      {}", bar(bands.high));
        }
    }
}

/// Convert frequency to musical note
fn frequency_to_note(freq: f32) -> String {
    if freq <= 0.0 {
        return "-".to_string();
    }

    let notes = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let a4 = 440.0;

    // Calculate semitones from A4
    let semitones = 12.0 * (freq / a4).log2();
    let midi = (semitones + 69.0).round() as i32;

    if midi < 0 || midi > 127 {
        return format!("{:.0}Hz", freq);
    }

    let note_idx = (midi % 12) as usize;
    let octave = (midi / 12) - 1;

    format!("{}{}", notes[note_idx], octave)
}
