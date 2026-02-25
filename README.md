<div align="center">

# Kino

**Rust-powered video SDK -- HLS/DASH streaming, audio fingerprinting, formal verification**

[![Crates.io](https://img.shields.io/crates/v/kino-core.svg)](https://crates.io/crates/kino-core)
[![docs.rs](https://docs.rs/kino-core/badge.svg)](https://docs.rs/kino-core)
[![CI](https://github.com/ExpertVagabond/kino/actions/workflows/ci.yml/badge.svg)](https://github.com/ExpertVagabond/kino/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)

[Homepage](https://kino-player.pages.dev) | [Web Demo](https://kino-demo.pages.dev) | [API Docs](https://docs.rs/kino-core)

</div>

---

## Overview

Kino is a 7-crate Rust workspace for video streaming infrastructure. It handles manifest parsing (HLS/DASH), adaptive bitrate selection, audio frequency analysis, and ships as a CLI, WASM module, desktop app, and Python library.

## Crates

| Crate | Description |
|-------|-------------|
| **kino-core** | HLS/DASH parsing, ABR algorithms, buffer management, DRM, analytics |
| **kino-frequency** | Audio fingerprinting, AI auto-tagging, thumbnail selection, recommendations |
| **kino-cli** | 14-subcommand CLI for stream analysis, QC, encoding, and frequency tools |
| **kino-wasm** | WebAssembly build for browser-based playback via MSE |
| **kino-desktop** | Native desktop player using GStreamer |
| **kino-tauri** | Cross-platform desktop app (macOS/Linux/Windows) |
| **kino-python** | Python bindings via PyO3 for frequency analysis |

## Quick Start

```bash
# Build the CLI
cargo build -p kino-cli --release

# Or install it
cargo install --path crates/kino-cli

# Analyze an HLS stream
kino-cli analyze https://example.com/master.m3u8

# Build the WASM module
cd crates/kino-wasm && wasm-pack build --target web
```

## CLI Usage

The `kino-cli` binary has 14 subcommands spanning stream analysis, encoding, and audio processing:

```bash
# Analyze a manifest (HLS or DASH)
kino-cli analyze https://cdn.example.com/master.m3u8

# Validate segment accessibility
kino-cli validate https://cdn.example.com/master.m3u8 --segments 20

# Run QC checks with JSON report
kino-cli qc https://cdn.example.com/master.m3u8 --output report.json --strict

# Monitor a live stream
kino-cli monitor https://cdn.example.com/live.m3u8 --interval 5

# Encode video to HLS
kino-cli encode input.mp4 --output dist/ --preset web --format hls

# Generate audio fingerprint
kino-cli fingerprint video.mp4 --output fingerprint.json

# Auto-tag content from audio
kino-cli autotag video.mp4 --max-tags 5 --min-confidence 0.3

# Find similar content
kino-cli similar video.mp4 --library ./media/ --limit 10
```

Full subcommand list: `analyze`, `validate`, `qc`, `extract`, `compare`, `monitor`, `encode`, `preset`, `frequency`, `fingerprint`, `autotag`, `thumbnail`, `similar`, `process`.

## Architecture

```
                        +-----------+
                        |  kino-cli |
                        +-----+-----+
                              |
          +-------------------+-------------------+
          |                   |                   |
    +-----+------+    +------+-------+    +------+-------+
    | kino-core  |    |kino-frequency|    | kino-wasm    |
    |            |    |              |    | (browser)    |
    | HLS/DASH   |    | FFT engine   |    +--------------+
    | ABR engine  |    | Fingerprint  |
    | Buffer mgmt |    | Auto-tagger  |    +--------------+
    | DRM/Captions|    | Thumbnails   |    | kino-desktop |
    +------+------+    +--------------+    | (GStreamer)  |
           |                               +--------------+
    +------+------+
    | kino-tauri  |    +--------------+
    | (desktop)   |    | kino-python  |
    +--------------+    | (PyO3)      |
                       +--------------+
```

## WASM

```javascript
import init, { WasmPlayer } from './pkg/psm_player_wasm.js';

await init();
const player = new WasmPlayer();
player.load("https://cdn.example.com/master.m3u8");
```

## Formal Verification

Kino includes TLA+ specifications for critical state machines:

- **PSMPlayerState** -- player lifecycle (idle/loading/playing/paused/error)
- **PSMAbrAlgorithm** -- ABR switching correctness
- **PSMBufferController** -- buffer management invariants
- **PSMConcurrentStreaming** -- multi-stream coordination

Specs are in `specs/tla/` and verified in CI.

## Building

```bash
cargo build                              # all default crates
cargo build -p kino-core                 # core library only
cargo build -p kino-cli --release        # release CLI binary
cargo test --all-features                # run all tests
cargo bench -p kino-frequency            # frequency benchmarks
```

## Examples

See `examples/` for complete examples in Rust, Python, JavaScript, and React. Crate-level examples are in each crate's `examples/` directory:

```bash
# Core playback example
cargo run -p kino-core --example basic_playback

# Frequency analysis
cargo run -p kino-frequency --example basic_analysis
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Run `cargo fmt --all` and `cargo clippy --all-targets --all-features`
4. Add tests for new functionality
5. Open a pull request

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

Copyright 2026 [Purple Squirrel Media LLC](https://purplesquirrelmedia.io).
