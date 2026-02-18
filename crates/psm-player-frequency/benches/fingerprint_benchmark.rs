//! Benchmark tests for frequency analysis operations
//!
//! Run with: cargo bench -p psm-player-frequency

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

// Helper to generate test audio
fn generate_sine_wave(freq: f32, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * freq * t).sin()
        })
        .collect()
}

fn generate_complex_audio(sample_rate: u32, duration_secs: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            // Mix of frequencies simulating music
            0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin() +
            0.3 * (2.0 * std::f32::consts::PI * 880.0 * t).sin() +
            0.2 * (2.0 * std::f32::consts::PI * 220.0 * t).sin()
        })
        .collect()
}

// ============================================================================
// FFT Benchmarks
// ============================================================================

fn bench_fft_sizes(c: &mut Criterion) {
    use rustfft::{FftPlanner, num_complex::Complex};

    let mut group = c.benchmark_group("FFT Size");

    for size in [512, 1024, 2048, 4096, 8192].iter() {
        let samples = generate_sine_wave(440.0, 44100, 1.0);

        group.bench_with_input(BenchmarkId::new("FFT", size), size, |b, &size| {
            let mut planner = FftPlanner::new();
            let fft = planner.plan_fft_forward(size);

            b.iter(|| {
                let mut buffer: Vec<Complex<f32>> = samples.iter()
                    .take(size)
                    .map(|&s| Complex::new(s, 0.0))
                    .collect();
                fft.process(black_box(&mut buffer));
                black_box(buffer)
            });
        });
    }

    group.finish();
}

// ============================================================================
// Fingerprint Benchmarks
// ============================================================================

fn bench_fingerprint_duration(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fingerprint Duration");

    for duration in [1.0, 5.0, 10.0, 30.0].iter() {
        let samples = generate_complex_audio(44100, *duration);

        group.bench_with_input(
            BenchmarkId::new("Fingerprint", format!("{}s", duration)),
            &samples,
            |b, samples| {
                b.iter(|| {
                    // Simple fingerprint simulation
                    let fft_size = 4096;
                    let hop_size = 2048;
                    let num_frames = (samples.len() - fft_size) / hop_size + 1;

                    let mut peaks = Vec::with_capacity(num_frames * 6);

                    for frame_idx in 0..num_frames {
                        let start = frame_idx * hop_size;
                        let frame = &samples[start..start + fft_size];

                        // Simplified peak detection
                        let energy: f32 = frame.iter().map(|&s| s * s).sum();
                        peaks.push(energy);
                    }

                    black_box(peaks)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Spectral Feature Benchmarks
// ============================================================================

fn bench_spectral_features(c: &mut Criterion) {
    let samples = generate_complex_audio(44100, 5.0);

    c.bench_function("Spectral Centroid", |b| {
        let spectrum: Vec<f32> = (0..2048)
            .map(|i| (i as f32 / 2048.0).sin().abs())
            .collect();
        let frequencies: Vec<f32> = (0..2048)
            .map(|i| i as f32 * 44100.0 / 4096.0)
            .collect();

        b.iter(|| {
            let weighted_sum: f32 = spectrum.iter()
                .zip(frequencies.iter())
                .map(|(&m, &f)| m * f)
                .sum();
            let total: f32 = spectrum.iter().sum();
            let centroid = if total > 0.0 { weighted_sum / total } else { 0.0 };
            black_box(centroid)
        });
    });

    c.bench_function("Spectral Flatness", |b| {
        let spectrum: Vec<f32> = (0..2048)
            .map(|i| (i as f32 / 2048.0).sin().abs() + 0.01)
            .collect();

        b.iter(|| {
            let n = spectrum.len() as f32;
            let log_sum: f32 = spectrum.iter()
                .map(|&x| x.ln())
                .sum();
            let geometric_mean = (log_sum / n).exp();
            let arithmetic_mean: f32 = spectrum.iter().sum::<f32>() / n;
            let flatness = geometric_mean / arithmetic_mean;
            black_box(flatness)
        });
    });

    c.bench_function("Band Energies", |b| {
        let spectrum: Vec<f32> = (0..2048)
            .map(|i| (i as f32 / 2048.0).sin().abs())
            .collect();
        let frequencies: Vec<f32> = (0..2048)
            .map(|i| i as f32 * 44100.0 / 4096.0)
            .collect();

        let bands = [
            (20.0, 60.0), (60.0, 250.0), (250.0, 500.0),
            (500.0, 2000.0), (2000.0, 4000.0), (4000.0, 20000.0),
        ];

        b.iter(|| {
            let mut energies = [0.0f32; 6];

            for (i, (low, high)) in bands.iter().enumerate() {
                for (j, &freq) in frequencies.iter().enumerate() {
                    if freq >= *low && freq < *high {
                        energies[i] += spectrum[j];
                    }
                }
            }

            let total: f32 = energies.iter().sum();
            if total > 0.0 {
                for e in &mut energies {
                    *e /= total;
                }
            }

            black_box(energies)
        });
    });
}

// ============================================================================
// Signature Similarity Benchmarks
// ============================================================================

fn bench_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("Similarity");

    for feature_size in [64, 128, 256].iter() {
        let sig1: Vec<f32> = (0..*feature_size)
            .map(|i| (i as f32 / *feature_size as f32).sin())
            .collect();
        let sig2: Vec<f32> = (0..*feature_size)
            .map(|i| (i as f32 / *feature_size as f32).cos())
            .collect();

        group.bench_with_input(
            BenchmarkId::new("Cosine", feature_size),
            &(sig1.clone(), sig2.clone()),
            |b, (s1, s2)| {
                b.iter(|| {
                    let dot: f32 = s1.iter().zip(s2.iter()).map(|(a, b)| a * b).sum();
                    let norm1: f32 = s1.iter().map(|x| x * x).sum::<f32>().sqrt();
                    let norm2: f32 = s2.iter().map(|x| x * x).sum::<f32>().sqrt();
                    let similarity = dot / (norm1 * norm2);
                    black_box(similarity)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Throughput Benchmarks
// ============================================================================

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("Throughput");
    group.throughput(criterion::Throughput::Elements(44100)); // 1 second of audio

    let samples = generate_complex_audio(44100, 1.0);

    group.bench_function("Full Analysis Pipeline", |b| {
        b.iter(|| {
            let fft_size = 4096;
            let hop_size = 2048;

            // Simulate full analysis
            let num_frames = (samples.len() - fft_size) / hop_size + 1;

            let mut all_spectra = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let start = frame_idx * hop_size;
                let frame = &samples[start..start + fft_size];

                // Simple spectrum (placeholder for FFT)
                let spectrum: Vec<f32> = frame.iter()
                    .take(fft_size / 2)
                    .map(|&s| s.abs())
                    .collect();

                all_spectra.push(spectrum);
            }

            // Average spectrum
            let mut avg_spectrum = vec![0.0f32; fft_size / 2];
            for spectrum in &all_spectra {
                for (i, &mag) in spectrum.iter().enumerate() {
                    avg_spectrum[i] += mag;
                }
            }
            for mag in &mut avg_spectrum {
                *mag /= num_frames as f32;
            }

            black_box(avg_spectrum)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fft_sizes,
    bench_fingerprint_duration,
    bench_spectral_features,
    bench_similarity,
    bench_throughput,
);

criterion_main!(benches);
