//! PSM Player Frequency Analysis Example
//!
//! This example demonstrates how to use the psm-player-frequency crate
//! to analyze audio files.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example frequency_analysis -- input.wav
//! cargo run --example frequency_analysis -- input.wav --fingerprint
//! cargo run --example frequency_analysis -- input.wav --json
//! ```

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

// Simplified types for the example
// In production, use the actual psm-player-frequency crate

/// Band energy distribution
#[derive(Debug, Clone)]
struct BandEnergies {
    sub_bass: f32,
    bass: f32,
    low_mid: f32,
    mid: f32,
    high_mid: f32,
    high: f32,
}

/// Dominant frequency
#[derive(Debug, Clone)]
struct DominantFrequency {
    frequency_hz: f32,
    magnitude: f32,
    rank: usize,
}

/// Analysis result
#[derive(Debug)]
struct AnalysisResult {
    spectral_centroid: f32,
    spectral_rolloff: f32,
    spectral_flatness: f32,
    zero_crossing_rate: f32,
    band_energies: BandEnergies,
    dominant_frequencies: Vec<DominantFrequency>,
}

/// Simple WAV loader (only supports 16-bit PCM)
fn load_wav(path: &Path) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Parse WAV header
    if &buffer[0..4] != b"RIFF" || &buffer[8..12] != b"WAVE" {
        return Err("Invalid WAV file".into());
    }

    // Find fmt chunk
    let mut pos = 12;
    let mut sample_rate = 44100u32;
    let mut bits_per_sample = 16u16;
    let mut num_channels = 2u16;

    while pos < buffer.len() - 8 {
        let chunk_id = &buffer[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            buffer[pos + 4],
            buffer[pos + 5],
            buffer[pos + 6],
            buffer[pos + 7],
        ]) as usize;

        if chunk_id == b"fmt " {
            num_channels = u16::from_le_bytes([buffer[pos + 10], buffer[pos + 11]]);
            sample_rate = u32::from_le_bytes([
                buffer[pos + 12],
                buffer[pos + 13],
                buffer[pos + 14],
                buffer[pos + 15],
            ]);
            bits_per_sample = u16::from_le_bytes([buffer[pos + 22], buffer[pos + 23]]);
        } else if chunk_id == b"data" {
            // Parse audio data
            let data_start = pos + 8;
            let data_end = data_start + chunk_size;

            if bits_per_sample != 16 {
                return Err("Only 16-bit WAV supported".into());
            }

            let mut samples = Vec::new();
            let mut i = data_start;
            while i + 2 <= data_end && i + 2 <= buffer.len() {
                let sample = i16::from_le_bytes([buffer[i], buffer[i + 1]]);
                samples.push(sample as f32 / 32768.0);
                i += 2;
            }

            // Convert stereo to mono
            if num_channels == 2 {
                let mono: Vec<f32> = samples
                    .chunks(2)
                    .map(|chunk| {
                        if chunk.len() == 2 {
                            (chunk[0] + chunk[1]) / 2.0
                        } else {
                            chunk[0]
                        }
                    })
                    .collect();
                return Ok((mono, sample_rate));
            }

            return Ok((samples, sample_rate));
        }

        pos += 8 + chunk_size;
        if chunk_size % 2 != 0 {
            pos += 1; // Padding byte
        }
    }

    Err("No audio data found".into())
}

/// Compute FFT spectrum
fn compute_spectrum(samples: &[f32], fft_size: usize) -> Vec<f32> {
    let n = fft_size.min(samples.len());
    let mut spectrum = vec![0.0f32; n / 2];

    for k in 0..n / 2 {
        let mut real = 0.0f32;
        let mut imag = 0.0f32;

        for (i, &sample) in samples.iter().take(n).enumerate() {
            // Hann window
            let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos());
            let windowed = sample * window;

            let angle = 2.0 * std::f32::consts::PI * k as f32 * i as f32 / n as f32;
            real += windowed * angle.cos();
            imag -= windowed * angle.sin();
        }

        spectrum[k] = (real * real + imag * imag).sqrt() * 2.0 / n as f32;
    }

    spectrum
}

/// Analyze audio samples
fn analyze(samples: &[f32], sample_rate: u32) -> AnalysisResult {
    let fft_size = 4096;
    let spectrum = compute_spectrum(samples, fft_size);
    let freq_resolution = sample_rate as f32 / fft_size as f32;

    // Spectral centroid
    let weighted: f32 = spectrum
        .iter()
        .enumerate()
        .map(|(i, &m)| m * i as f32 * freq_resolution)
        .sum();
    let total: f32 = spectrum.iter().sum();
    let centroid = if total > 0.0 { weighted / total } else { 0.0 };

    // Spectral rolloff (95%)
    let target = total * 0.95;
    let mut cumulative = 0.0f32;
    let mut rolloff = 0.0f32;
    for (i, &mag) in spectrum.iter().enumerate() {
        cumulative += mag;
        if cumulative >= target {
            rolloff = i as f32 * freq_resolution;
            break;
        }
    }

    // Spectral flatness
    let n = spectrum.len() as f32;
    let log_sum: f32 = spectrum.iter().map(|&x| (x + 1e-10).ln()).sum();
    let geometric_mean = (log_sum / n).exp();
    let arithmetic_mean = total / n;
    let flatness = if arithmetic_mean > 0.0 {
        geometric_mean / arithmetic_mean
    } else {
        0.0
    };

    // Zero crossing rate
    let crossings: usize = samples
        .windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    let zcr = crossings as f32 / samples.len() as f32;

    // Band energies
    let bands = [
        (20.0, 60.0),
        (60.0, 250.0),
        (250.0, 500.0),
        (500.0, 2000.0),
        (2000.0, 4000.0),
        (4000.0, 20000.0),
    ];

    let mut energies = [0.0f32; 6];
    for (i, (low, high)) in bands.iter().enumerate() {
        for (j, &mag) in spectrum.iter().enumerate() {
            let freq = j as f32 * freq_resolution;
            if freq >= *low && freq < *high {
                energies[i] += mag;
            }
        }
    }

    let band_total: f32 = energies.iter().sum();
    if band_total > 0.0 {
        for e in &mut energies {
            *e /= band_total;
        }
    }

    // Dominant frequencies
    let mut indexed: Vec<(usize, f32)> = spectrum.iter().enumerate().map(|(i, &m)| (i, m)).collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let max_mag = indexed.first().map(|(_, m)| *m).unwrap_or(1.0);
    let dominant: Vec<DominantFrequency> = indexed
        .iter()
        .take(10)
        .enumerate()
        .map(|(rank, (idx, mag))| DominantFrequency {
            frequency_hz: *idx as f32 * freq_resolution,
            magnitude: mag / max_mag,
            rank: rank + 1,
        })
        .collect();

    AnalysisResult {
        spectral_centroid: centroid,
        spectral_rolloff: rolloff,
        spectral_flatness: flatness,
        zero_crossing_rate: zcr,
        band_energies: BandEnergies {
            sub_bass: energies[0],
            bass: energies[1],
            low_mid: energies[2],
            mid: energies[3],
            high_mid: energies[4],
            high: energies[5],
        },
        dominant_frequencies: dominant,
    }
}

/// Generate audio fingerprint
fn fingerprint(samples: &[f32], sample_rate: u32) -> String {
    let fft_size = 4096;
    let hop_size = 2048;
    let mut hash_data = Vec::new();

    let num_frames = samples.len().saturating_sub(fft_size) / hop_size + 1;

    for frame_idx in 0..num_frames.min(100) {
        let start = frame_idx * hop_size;
        let end = (start + fft_size).min(samples.len());
        let frame = &samples[start..end];

        if frame.len() < fft_size {
            break;
        }

        // Calculate energy
        let energy: f32 = frame.iter().map(|s| s * s).sum();
        hash_data.push((energy * 255.0).min(255.0) as u8);
    }

    // Simple hash
    let hash: u64 = hash_data
        .iter()
        .enumerate()
        .fold(0u64, |acc, (i, &b)| {
            acc.wrapping_add((b as u64).wrapping_mul(31u64.wrapping_pow((i % 16) as u32)))
        });

    format!("{:016x}", hash)
}

fn print_bar(value: f32, width: usize) -> String {
    let filled = (value * width as f32) as usize;
    format!(
        "[{}{}]",
        "=".repeat(filled),
        " ".repeat(width.saturating_sub(filled))
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <input.wav> [--fingerprint] [--json]", args[0]);
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let show_fingerprint = args.iter().any(|a| a == "--fingerprint");
    let output_json = args.iter().any(|a| a == "--json");

    // Load audio
    println!("Loading: {}", input_path.display());
    let (samples, sample_rate) = match load_wav(input_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error loading file: {}", e);
            std::process::exit(1);
        }
    };

    let duration = samples.len() as f32 / sample_rate as f32;
    println!(
        "Duration: {:.2}s, Sample rate: {} Hz, Samples: {}",
        duration,
        sample_rate,
        samples.len()
    );

    // Analyze
    println!("\nAnalyzing...");
    let result = analyze(&samples, sample_rate);

    if output_json {
        println!(
            r#"{{
  "spectral_centroid": {:.2},
  "spectral_rolloff": {:.2},
  "spectral_flatness": {:.4},
  "zero_crossing_rate": {:.4},
  "band_energies": {{
    "sub_bass": {:.3},
    "bass": {:.3},
    "low_mid": {:.3},
    "mid": {:.3},
    "high_mid": {:.3},
    "high": {:.3}
  }},
  "dominant_frequencies": [
{}
  ]
}}"#,
            result.spectral_centroid,
            result.spectral_rolloff,
            result.spectral_flatness,
            result.zero_crossing_rate,
            result.band_energies.sub_bass,
            result.band_energies.bass,
            result.band_energies.low_mid,
            result.band_energies.mid,
            result.band_energies.high_mid,
            result.band_energies.high,
            result
                .dominant_frequencies
                .iter()
                .take(5)
                .map(|f| format!(
                    r#"    {{ "frequency_hz": {:.2}, "magnitude": {:.2}, "rank": {} }}"#,
                    f.frequency_hz, f.magnitude, f.rank
                ))
                .collect::<Vec<_>>()
                .join(",\n")
        );
    } else {
        println!("\n=== Analysis Results ===");
        println!("Spectral Centroid: {:.2} Hz", result.spectral_centroid);
        println!("Spectral Rolloff:  {:.2} Hz", result.spectral_rolloff);
        println!("Spectral Flatness: {:.4}", result.spectral_flatness);
        println!("Zero Crossing Rate: {:.4}", result.zero_crossing_rate);

        println!("\nDominant Frequencies:");
        for freq in result.dominant_frequencies.iter().take(5) {
            println!(
                "  #{}: {:.2} Hz (magnitude: {:.2})",
                freq.rank, freq.frequency_hz, freq.magnitude
            );
        }

        println!("\nBand Energies:");
        println!(
            "  Sub-bass (20-60 Hz):    {} {:.1}%",
            print_bar(result.band_energies.sub_bass, 20),
            result.band_energies.sub_bass * 100.0
        );
        println!(
            "  Bass (60-250 Hz):       {} {:.1}%",
            print_bar(result.band_energies.bass, 20),
            result.band_energies.bass * 100.0
        );
        println!(
            "  Low-mid (250-500 Hz):   {} {:.1}%",
            print_bar(result.band_energies.low_mid, 20),
            result.band_energies.low_mid * 100.0
        );
        println!(
            "  Mid (500-2000 Hz):      {} {:.1}%",
            print_bar(result.band_energies.mid, 20),
            result.band_energies.mid * 100.0
        );
        println!(
            "  High-mid (2-4 kHz):     {} {:.1}%",
            print_bar(result.band_energies.high_mid, 20),
            result.band_energies.high_mid * 100.0
        );
        println!(
            "  High (4-20 kHz):        {} {:.1}%",
            print_bar(result.band_energies.high, 20),
            result.band_energies.high * 100.0
        );
    }

    // Fingerprint
    if show_fingerprint {
        println!("\nGenerating fingerprint...");
        let fp = fingerprint(&samples, sample_rate);
        println!("Fingerprint: {}", fp);
    }
}
