//! Streaming Analysis Example
//!
//! Demonstrates real-time audio analysis using the streaming module.
//!
//! # Usage
//! ```bash
//! cargo run --example streaming_analysis -- path/to/audio.wav
//! ```

use anyhow::Result;
use kino_frequency::streaming::{AnalysisEvent, StreamAnalyzer, StreamConfig};
use kino_frequency::AudioData;
use std::env;
use std::sync::{Arc, Mutex};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <audio_file>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    println!("Streaming analysis: {}", file_path);
    println!("{}", "-".repeat(50));

    // Load audio
    let audio = load_wav(file_path)?;
    println!(
        "Processing {:.2} seconds of audio...\n",
        audio.duration_secs
    );

    // Configure streaming analyzer
    let config = StreamConfig {
        fft_size: 2048,
        hop_size: 512,
        sample_rate: audio.sample_rate,
        history_length: 100,
        beat_threshold: 1.5,
        silence_threshold: 0.01,
        frequency_change_threshold: 100.0,
    };

    // Create analyzer with config
    let mut analyzer = StreamAnalyzer::with_config(config);

    // Track events
    let events: Arc<Mutex<Vec<AnalysisEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();

    // Register event callback
    analyzer.on_event(move |event| {
        // Skip FrameAnalyzed events to avoid too much output
        if !matches!(event, AnalysisEvent::FrameAnalyzed { .. }) {
            events_clone.lock().unwrap().push(event);
        }
    });

    // Process audio in chunks (simulating real-time streaming)
    let chunk_size = 1024;
    let mut current_time = 0.0;

    for chunk in audio.samples.chunks(chunk_size) {
        let _frames = analyzer.process(chunk);

        // Print progress every second
        let new_time = current_time + (chunk.len() as f64 / audio.sample_rate as f64);
        if new_time.floor() > current_time.floor() {
            print!("\rProcessed: {:.0}s", new_time);
        }
        current_time = new_time;
    }
    println!("\n");

    // Get statistics
    let stats = analyzer.get_statistics();
    println!("Stream Statistics:");
    println!("  Window duration:   {:.2}s", stats.window_duration);
    println!("  Frame count:       {}", stats.frame_count);
    println!("  Avg RMS energy:    {:.4}", stats.avg_rms_energy);
    println!("  RMS variance:      {:.6}", stats.rms_variance);
    println!("  Avg dominant freq: {:.1} Hz", stats.avg_dominant_frequency);
    println!("  Freq variance:     {:.1}", stats.frequency_variance);
    println!("  Avg centroid:      {:.1} Hz", stats.avg_spectral_centroid);

    // Print band energies
    let bands = &stats.avg_band_energies;
    println!("\nAverage Band Energies:");
    println!("  Sub-bass:  {:.4}", bands.sub_bass);
    println!("  Bass:      {:.4}", bands.bass);
    println!("  Low-mid:   {:.4}", bands.low_mid);
    println!("  Mid:       {:.4}", bands.mid);
    println!("  High-mid:  {:.4}", bands.high_mid);
    println!("  High:      {:.4}", bands.high);

    // Print detected events
    let events = events.lock().unwrap();
    if !events.is_empty() {
        println!("\nDetected Events:");
        for event in events.iter().take(20) {
            match event {
                AnalysisEvent::BeatDetected { timestamp, strength } => {
                    println!(
                        "  [{:>6.2}s] Beat detected - strength: {:.2}",
                        timestamp, strength
                    );
                }
                AnalysisEvent::SilenceStart { timestamp } => {
                    println!("  [{:>6.2}s] Silence started", timestamp);
                }
                AnalysisEvent::SilenceEnd { timestamp, duration } => {
                    println!(
                        "  [{:>6.2}s] Silence ended (duration: {:.2}s)",
                        timestamp, duration
                    );
                }
                AnalysisEvent::DominantChange { old, new, timestamp } => {
                    println!(
                        "  [{:>6.2}s] Frequency change: {:.1} Hz -> {:.1} Hz",
                        timestamp, old, new
                    );
                }
                AnalysisEvent::SpectralShift { timestamp, magnitude } => {
                    println!(
                        "  [{:>6.2}s] Spectral shift - magnitude: {:.2}",
                        timestamp, magnitude
                    );
                }
                AnalysisEvent::FrameAnalyzed { .. } => {
                    // Skip frame events for brevity
                }
            }
        }

        if events.len() > 20 {
            println!("  ... and {} more events", events.len() - 20);
        }
    } else {
        println!("\nNo significant events detected.");
    }

    Ok(())
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
