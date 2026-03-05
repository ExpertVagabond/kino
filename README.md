<div align="center">

# Kino

**Production-grade Rust SDK for video streaming infrastructure -- HLS/DASH manifest parsing, adaptive bitrate control, audio fingerprinting, and formally verified state machines.**

[![CI](https://github.com/ExpertVagabond/kino/actions/workflows/ci.yml/badge.svg)](https://github.com/ExpertVagabond/kino/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-2021_edition-orange.svg)](https://www.rust-lang.org)
[![TLA+](https://img.shields.io/badge/formal_verification-TLA%2B-blueviolet.svg)](#formal-verification-tla)
[![WASM](https://img.shields.io/badge/wasm-supported-green.svg)](#wasm-player)

[Web Demo](https://kino-demo.pages.dev) &middot; [API Docs](https://docs.rs/kino-core) &middot; [MCP Server](#mcp-server)

</div>

---

Kino is a 7-crate Rust workspace that provides everything needed to build, analyze, and serve video streaming infrastructure. It parses HLS and DASH manifests, implements BOLA and throughput-based adaptive bitrate algorithms, performs audio fingerprinting via FFT spectral analysis, and ships as a CLI, WASM module, Tauri desktop app, and Python library. Critical playback state machines are formally verified with 8 TLA+ specifications checked in CI.

## Architecture

```
                                 +--------------+
                                 |   kino-cli   |  14 subcommands
                                 |   (binary)   |  stream analysis, QC,
                                 +------+-------+  encoding, fingerprinting
                                        |
             +--------------------------+----------------------------+
             |                          |                            |
      +------+-------+         +-------+--------+           +-------+-------+
      |  kino-core   |         | kino-frequency |           |  kino-wasm   |
      |              |         |                |           |  (browser)   |
      | HLS parser   |         | FFT engine     |           |              |
      | DASH parser  |         | Fingerprinting |           | BOLA ABR     |
      | BOLA ABR     |    +--->| Auto-tagging   |           | MSE buffers  |
      | Buffer mgmt  |    |   | Thumbnails     |           | Analytics    |
      | DRM / Widevine|    |   | Recommendations|           +--------------+
      | Captions     |    |   +----------------+
      | Analytics    |    |
      +------+-------+    |   +----------------+           +---------------+
             |            |   | kino-python    |           | kino-desktop  |
             |            +---| (PyO3 + NumPy) |           | (GStreamer)   |
             |                +----------------+           +---------------+
             |
      +------+-------+
      |  kino-tauri   |       +----------------+
      |  (desktop)    |       |   kino-mcp     |  8-tool MCP server
      |  macOS/Linux/ |       |   (Node.js)    |  for AI agent access
      |  Windows      |       +----------------+
      +--------------+

      +================================================+
      |          specs/tla/ -- 8 TLA+ Specifications    |
      |  PlayerState | ABR | Buffer | Concurrent | DRM  |
      |  Playlist | Captions | Full System Composition  |
      +================================================+
```

## Crates

| Crate | Description | Key Dependencies |
|-------|-------------|-----------------|
| **[kino-core](crates/kino-core)** | HLS/DASH manifest parsing, BOLA and throughput ABR, buffer management, DRM (Widevine/FairPlay/PlayReady/ClearKey), caption parsing (WebVTT/SRT), analytics events | `m3u8-rs`, `nom`, `reqwest`, `ring` |
| **[kino-frequency](crates/kino-frequency)** | FFT spectral analysis, SHA-256 audio fingerprinting, ML auto-tagging, optimal thumbnail selection, content similarity via frequency signatures | `rustfft`, `realfft`, `symphonia`, `ndarray` |
| **[kino-cli](crates/kino-cli)** | 14-subcommand CLI for stream analysis, QC validation, encoding, monitoring, and audio fingerprinting | `clap`, `indicatif`, `tabled` |
| **[kino-wasm](crates/kino-wasm)** | WebAssembly build for browser playback via Media Source Extensions (MSE), includes ABR controller and real-time frequency analysis | `wasm-bindgen`, `web-sys` |
| **[kino-desktop](crates/kino-desktop)** | Native desktop player using GStreamer pipeline with hardware acceleration | `gstreamer`, `winit` |
| **[kino-tauri](crates/kino-tauri)** | Cross-platform desktop app (macOS, Linux, Windows) using Tauri 2 | `tauri` |
| **[kino-python](crates/kino-python)** | Python bindings via PyO3 for frequency analysis, fingerprinting, and auto-tagging with NumPy interop | `pyo3`, `numpy` |

## Getting Started

### Prerequisites

- Rust 2021 edition (1.70+)
- FFmpeg (for audio extraction)
- wasm-pack (for WASM builds)
- maturin (for Python bindings)

### Install the CLI

```bash
# Build from source
cargo install --path crates/kino-cli

# Or build the workspace
cargo build --release --workspace
```

### Build everything

```bash
make all          # Rust + WASM + Python
make test         # Run all tests
make lint         # Clippy with -D warnings
make bench        # Frequency benchmarks
```

## Usage

### CLI -- Stream Analysis

```bash
# Analyze an HLS or DASH manifest
kino-cli analyze https://cdn.example.com/master.m3u8

# Validate segment accessibility
kino-cli validate https://cdn.example.com/master.m3u8 --segments 20

# Run full QC check with JSON report
kino-cli qc https://cdn.example.com/master.m3u8 --output report.json --strict

# Monitor a live stream
kino-cli monitor https://cdn.example.com/live.m3u8 --interval 5

# Encode video to HLS
kino-cli encode input.mp4 --output dist/ --preset web --format hls
```

### CLI -- Audio Fingerprinting

```bash
# Generate audio fingerprint
kino-cli fingerprint video.mp4 --output fingerprint.json

# Auto-tag content from audio (genre, mood, BPM)
kino-cli autotag video.mp4 --max-tags 5 --min-confidence 0.3

# Find similar content in a media library
kino-cli similar video.mp4 --library ./media/ --limit 10
```

All 14 subcommands: `analyze`, `validate`, `qc`, `extract`, `compare`, `monitor`, `encode`, `preset`, `frequency`, `fingerprint`, `autotag`, `thumbnail`, `similar`, `process`.

### Rust -- Core Library

```rust
use kino_core::manifest::{create_parser, ManifestType};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = Url::parse("https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8")?;
    let parser = create_parser(&url);
    let manifest = parser.parse(&url).await?;

    println!("Live: {}, Renditions: {}", manifest.is_live, manifest.renditions.len());

    for r in &manifest.renditions {
        println!("  {} -- {} bps, {:?}", r.id, r.bandwidth, r.resolution);
    }

    Ok(())
}
```

### Rust -- Frequency Analysis

```rust
use kino_frequency::{AudioAnalyzer, fingerprint::Fingerprinter, tagging::ContentTagger};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let analyzer = AudioAnalyzer::new(44100);
    let audio = analyzer.extract_audio("video.mp4").await?;

    // Generate cryptographic fingerprint
    let fingerprint = Fingerprinter::new().fingerprint(&audio)?;
    println!("Content hash: {}", fingerprint.hash);

    // Auto-tag with ML inference
    let tags = ContentTagger::new().predict(&audio)?;
    for tag in &tags {
        println!("{}: {:.1}%", tag.label, tag.confidence * 100.0);
    }

    Ok(())
}
```

### Python

```python
import numpy as np
from kino_frequency import FrequencyAnalyzer, Fingerprinter, ContentTagger

samples = np.array([...], dtype=np.float32)
sample_rate = 44100

# Frequency analysis
analyzer = FrequencyAnalyzer(sample_rate)
result = analyzer.analyze(samples)
print(f"Dominant: {result.dominant_frequencies[0].frequency_hz} Hz")
print(f"Spectral centroid: {result.spectral_centroid} Hz")

# Audio fingerprinting
fingerprinter = Fingerprinter()
fp = fingerprinter.fingerprint(samples, sample_rate)
print(f"Hash: {fp.hash}")

# Auto-tagging
tagger = ContentTagger()
for tag in tagger.predict(samples, sample_rate):
    print(f"{tag.label}: {tag.confidence:.0%}")
```

Build the Python wheel:

```bash
cd crates/kino-python && maturin develop --release
```

### WASM Player

```javascript
import init, { KinoAbrController, WasmConfig } from '@kino/wasm';

await init();

// Configure for VOD
const config = WasmConfig.vod();  // BOLA ABR, 60s buffer

// Or low-latency live
const config = WasmConfig.low_latency();  // throughput ABR, 6s buffer

const abr = new KinoAbrController();
```

Build the WASM package:

```bash
cd crates/kino-wasm && wasm-pack build --target web --release
```

### MCP Server

The `kino-mcp` package exposes 8 tools for AI agent integration:

| Tool | Description |
|------|-------------|
| `analyze_stream` | Parse HLS/DASH manifest -- renditions, codecs, duration |
| `validate_stream` | Check segment accessibility and bitrate conformance |
| `quality_check` | Full QC report -- DRM, captions, bitrate ladder |
| `monitor_stream` | Live stream health -- latency, segment freshness |
| `fingerprint_audio` | Audio fingerprint for content identification |
| `autotag_content` | Auto-detect genre, mood, BPM from audio |
| `compare_streams` | Diff two streams for quality mismatches |
| `encode_video` | Generate encoding presets for target platforms |

```json
{
  "mcpServers": {
    "kino": {
      "command": "kino-mcp",
      "env": { "KINO_CLI_PATH": "/path/to/kino-cli" }
    }
  }
}
```

### Docker

```bash
docker build -t kino .
docker run -p 8080:80 kino

# Or with docker compose (includes media server and encoder)
docker compose up
docker compose --profile media up     # with HLS media server
docker compose --profile encoder up   # with encoding worker
```

## Formal Verification (TLA+)

Kino includes **8 TLA+ specifications** that formally model and verify the correctness of all critical state machines. These specs are checked in CI via the TLC model checker.

| Specification | What It Verifies |
|---------------|-----------------|
| **PSMPlayerState** | Player lifecycle: idle, loading, playing, paused, buffering, seeking, ended, error |
| **PSMAbrAlgorithm** | BOLA and throughput-based ABR switching correctness, QoE bounds |
| **PSMBufferController** | Segment prefetch scheduling, buffer health monitoring, eviction policy |
| **PSMConcurrentStreaming** | Parallel segment download with worker pool limits, no duplicate downloads |
| **PSMPlaylist** | Queue management with shuffle (Fisher-Yates), repeat modes, history tracking |
| **PSMCaptions** | Caption track selection, cue visibility, loading state consistency |
| **PSMDrm** | DRM license lifecycle: detection, session creation, license acquisition, renewal |
| **PSMPlayer** | Full system composition -- end-to-end playback, quality adaptation, error recovery |

### Verified Safety Properties

These invariants hold across **all** reachable states:

- `BufferBounded` -- buffer never exceeds maximum capacity
- `WorkersBounded` -- concurrent downloads never exceed the worker pool limit
- `QualityValid` -- selected quality level is always within the available range
- `MutualExclusionDisplayModes` -- fullscreen and picture-in-picture are mutually exclusive
- `ActiveRequiresLicenses` -- DRM-protected playback always has valid licenses
- `NoDuplicateDownloads` -- no segment is downloaded twice

### Verified Liveness Properties

These temporal properties guarantee progress:

- `EventuallyPlaysOrErrors` -- loading always resolves to playback or error
- `BufferingResolves` -- buffering always exits
- `StallResolves` -- playback stalls always recover
- `DownloadsProgress` -- active downloads always increase buffer

### Running the Specs

```bash
# Check a single spec
tlc PSMPlayerState.tla -config PSMPlayerState.cfg

# Check all 8 specs
for spec in PSMPlayerState PSMAbrAlgorithm PSMBufferController \
            PSMConcurrentStreaming PSMPlaylist PSMCaptions PSMDrm PSMPlayer; do
    echo "=== $spec ===" && tlc "$spec.tla" -config "$spec.cfg"
done
```

The QoE model uses the scoring formula: `QoE = 100 - (rebuffers * 15) - (quality_switches * 3) + (quality_level * 10)`, with a floor of 60 enforced by the ABR spec.

## CI / CD

The GitHub Actions pipeline runs on every push to `main` and `develop`:

- **Rust** -- fmt, clippy, build, test across `{ubuntu, macos, windows} x {stable, beta}`
- **WASM** -- `wasm-pack build` producing browser-ready artifacts
- **Tauri** -- cross-platform desktop builds (macOS, Linux, Windows)
- **Python** -- maturin wheel build + install verification
- **Frequency** -- crate build, tests, benchmark dry-run
- **Docker** -- multi-stage image build with buildx caching
- **TLA+** -- TLC model checking for player state and ABR specs

## Project Structure

```
kino/
  crates/
    kino-core/          Core library -- manifest parsing, ABR, buffer, DRM
    kino-frequency/     Audio analysis -- FFT, fingerprint, tagging, thumbnails
    kino-cli/           14-subcommand CLI binary
    kino-wasm/          WebAssembly module for browser playback
    kino-desktop/       Native desktop player (GStreamer)
    kino-tauri/         Cross-platform desktop app (Tauri 2)
    kino-python/        Python bindings (PyO3 + NumPy)
  specs/tla/            8 TLA+ formal specifications + configs
  mcp-server/           MCP server for AI agent integration (Node.js)
  examples/             Rust, Python, JavaScript, React examples
  embed/                Embeddable player widget + generator
  web/                  Web player with analytics dashboard
  server/               Server-side data and configuration
  site/                 Project homepage and pitch deck
  docker/               Docker configs (nginx, entrypoint)
  .github/workflows/    CI (Rust/WASM/Tauri/Python/Docker/TLA+)
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Run formatting and lints:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   ```
4. Run the test suite:
   ```bash
   cargo test --workspace --all-features
   ```
5. If modifying player logic, verify TLA+ specs still pass
6. Open a pull request

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

Copyright 2026 [Purple Squirrel Media LLC](https://purplesquirrelmedia.io).
