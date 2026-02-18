# Kino Frequency Analysis

High-performance audio frequency analysis library for Kino's decentralized video platform.

## Features

- **FFT-based Analysis** - Real-time and batch frequency analysis using rustfft
- **Audio Fingerprinting** - Shazam-style spectral peak constellation for content verification
- **AI Auto-Tagging** - Rule-based content classification using spectral features
- **Thumbnail Generation** - 2D FFT-based frame quality scoring
- **Recommendations** - Frequency signature matching for content similarity
- **Solana Integration** - On-chain fingerprint storage for verification
- **Streaming Analysis** - Real-time event-driven processing

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kino-frequency = { path = "../kino-frequency", features = ["full"] }
```

### Feature Flags

| Feature | Description |
|---------|-------------|
| `fingerprint` | Audio fingerprinting with SHA-256 hashing |
| `tagging` | AI-powered content auto-tagging |
| `thumbnail` | 2D FFT thumbnail selection |
| `recommend` | Content similarity recommendations |
| `solana` | On-chain fingerprint storage |
| `full` | All features enabled |

## Quick Start

### Basic Analysis

```rust
use kino_frequency::{AudioAnalyzer, FrequencyAnalyzer};

// Create analyzer
let analyzer = FrequencyAnalyzer::new(4096, 2048);

// Analyze audio samples
let analysis = analyzer.analyze(&samples, sample_rate)?;

println!("Spectral centroid: {} Hz", analysis.spectral_centroid);
println!("Band energies: {:?}", analysis.band_energies);
```

### Audio Fingerprinting

```rust
use kino_frequency::fingerprint::Fingerprinter;

let fingerprinter = Fingerprinter::new();
let fingerprint = fingerprinter.fingerprint(&audio)?;

println!("Content hash: {}", fingerprint.hash);
println!("Peak count: {}", fingerprint.peak_count);

// Verify content
let verification = fingerprinter.verify(&audio, &expected_hash)?;
if verification.is_match {
    println!("Content verified!");
}
```

### Content Auto-Tagging

```rust
use kino_frequency::tagging::ContentTagger;

let tagger = ContentTagger::new();
let tags = tagger.predict(&audio)?;

for tag in tags {
    println!("{}: {} (confidence: {:.1}%)",
             tag.category, tag.name, tag.confidence * 100.0);
}
```

### Streaming Analysis

```rust
use kino_frequency::streaming::{StreamAnalyzer, StreamConfig};

let config = StreamConfig {
    fft_size: 2048,
    hop_size: 512,
    sample_rate: 44100,
    beat_threshold: 1.5,
    silence_threshold: 0.01,
    frequency_change_threshold: 100.0,
};

let mut analyzer = StreamAnalyzer::new(config);

// Register event callbacks
analyzer.on_event(|timestamp, event| {
    match event {
        AnalysisEvent::BeatDetected { energy, tempo_estimate } => {
            println!("Beat at {:.2}s, tempo: {:.1} BPM", timestamp, tempo_estimate);
        }
        AnalysisEvent::SilenceStart => {
            println!("Silence detected at {:.2}s", timestamp);
        }
        _ => {}
    }
});

// Process audio chunks
for chunk in audio_stream {
    analyzer.process(&chunk);
}
```

### Content Recommendations

```rust
use kino_frequency::recommend::RecommendationEngine;

let mut engine = RecommendationEngine::new();

// Index content
engine.add_content("video_1", &audio1, None)?;
engine.add_content("video_2", &audio2, None)?;

// Find similar content
let similar = engine.get_similar("video_1", 10);
for rec in similar {
    println!("{}: {:.1}% similar", rec.content_id, rec.similarity * 100.0);
}
```

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Video Upload   │───▶│  Audio Extract   │───▶│  FFT Analysis   │
└─────────────────┘    └──────────────────┘    └────────┬────────┘
                                                        │
        ┌───────────────────────────────────────────────┼───────────────────────┐
        │                                               │                       │
        ▼                                               ▼                       ▼
┌───────────────┐                              ┌────────────────┐       ┌───────────────┐
│ Fingerprint   │                              │  Auto-Tagging  │       │ Recommendations│
│ (SHA-256)     │                              │  (ML Model)    │       │ (Similarity)   │
└───────┬───────┘                              └────────┬───────┘       └───────┬───────┘
        │                                               │                       │
        ▼                                               ▼                       ▼
┌───────────────┐                              ┌────────────────┐       ┌───────────────┐
│ Solana Chain  │                              │  Content Tags  │       │ Similar Items │
│ (Verification)│                              │  (Metadata)    │       │ (API Response)│
└───────────────┘                              └────────────────┘       └───────────────┘
```

## API Reference

### Core Types

#### `AudioData`
```rust
pub struct AudioData {
    pub samples: Vec<f32>,      // Normalized samples (-1.0 to 1.0)
    pub sample_rate: u32,       // Sample rate in Hz
    pub channels: u32,          // Number of channels
    pub duration_secs: f64,     // Duration in seconds
}
```

#### `FrequencyAnalysis`
```rust
pub struct FrequencyAnalysis {
    pub dominant_frequencies: Vec<DominantFrequency>,
    pub band_energies: BandEnergies,
    pub spectral_centroid: f32,
    pub spectral_rolloff: f32,
    pub spectral_flatness: f32,
    pub zero_crossing_rate: f32,
}
```

#### `BandEnergies`
```rust
pub struct BandEnergies {
    pub sub_bass: f32,    // 20-60 Hz
    pub bass: f32,        // 60-250 Hz
    pub low_mid: f32,     // 250-500 Hz
    pub mid: f32,         // 500-2000 Hz
    pub high_mid: f32,    // 2000-4000 Hz
    pub presence: f32,    // 4000-6000 Hz
    pub brilliance: f32,  // 6000-20000 Hz
}
```

#### `FrequencySignature`
```rust
pub struct FrequencySignature {
    pub mel_bands: Vec<f32>,       // 40-band mel spectrogram
    pub mfcc: Vec<f32>,            // 13 MFCCs
    pub chroma: Vec<f32>,          // 12-bin chroma features
    pub temporal_pattern: Vec<f32>, // Energy envelope
}
```

### FrequencyAnalyzer Methods

| Method | Description |
|--------|-------------|
| `new(fft_size, hop_size)` | Create analyzer with specified parameters |
| `analyze(samples, sample_rate)` | Perform full frequency analysis |
| `dominant_frequencies(samples, sample_rate, top_k)` | Extract top K frequencies |
| `compute_signature(samples, sample_rate)` | Generate frequency signature |
| `compute_spectrogram(samples)` | Compute full spectrogram |
| `bandpass_filter(samples, sample_rate, low, high)` | Apply bandpass filter |
| `project_to_dominant(samples, sample_rate, top_k)` | Reconstruct with only dominant frequencies |

### Streaming Events

| Event | Description |
|-------|-------------|
| `BeatDetected` | Beat detected with energy and tempo estimate |
| `SilenceStart` | Start of silence period |
| `SilenceEnd` | End of silence with duration |
| `DominantFrequencyChange` | Significant frequency shift |
| `FrameAnalyzed` | New analysis frame available |

## CLI Usage

The Kino CLI includes frequency analysis commands:

```bash
# Analyze frequencies
kino frequency input.wav --top-k 10 --json

# Generate fingerprint
kino fingerprint input.wav --output fingerprint.json

# Auto-tag content
kino autotag input.wav --max-tags 5

# Generate thumbnail
kino thumbnail video.mp4 --output thumb.jpg

# Find similar content
kino similar input.wav --library /path/to/media --limit 10

# Full processing pipeline
kino process video.mp4 --output results.json
```

## Python Bindings

Install with maturin:

```bash
cd crates/kino-python
maturin develop --release
```

Usage:

```python
from kino_frequency import FrequencyAnalyzer, Fingerprinter, ContentTagger

# Analyze audio
analyzer = FrequencyAnalyzer(sample_rate=44100)
result = analyzer.analyze("audio.wav")
print(f"Dominant frequency: {result.dominant_frequencies[0].frequency} Hz")

# Generate fingerprint
fingerprinter = Fingerprinter()
fp = fingerprinter.fingerprint("audio.wav")
print(f"Hash: {fp.hash}")

# Auto-tag
tagger = ContentTagger()
tags = tagger.predict("audio.wav")
for tag in tags:
    print(f"{tag.name}: {tag.confidence:.1%}")
```

## WebAssembly

The frequency analysis is available as a WASM module:

```javascript
import init, { KinoFrequencyAnalyzer, KinoFingerprinter } from 'kino-wasm';

await init();

const analyzer = new KinoFrequencyAnalyzer(44100, 4096);
const result = analyzer.analyze(audioSamples);

console.log('Dominant frequencies:', result.dominant_frequencies);
console.log('Spectral centroid:', result.spectral_centroid);
```

## Benchmarks

Run benchmarks:

```bash
cargo bench --package kino-frequency
```

Typical performance (Apple M1):

| Operation | Duration | Throughput |
|-----------|----------|------------|
| FFT 4096 | 45µs | 22,000 ops/s |
| Fingerprint (10s) | 12ms | 83 files/s |
| Signature | 8ms | 125 files/s |
| Similarity | 2µs | 500,000 comparisons/s |

## License

MIT OR Apache-2.0
