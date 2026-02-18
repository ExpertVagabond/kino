# PSM Player - Purple Squirrel Media Video Player

A comprehensive Rust video player ecosystem for the Purple Squirrel Media platform.

## Crates

| Crate | Description |
|-------|-------------|
| `psm-player-core` | Core library with HLS/DASH parsing, buffer management, ABR algorithms |
| `psm-player-desktop` | Native desktop player using GStreamer |
| `psm-player-wasm` | WebAssembly library for browser integration |
| `psm-player-tauri` | Cross-platform desktop app with Tauri |
| `psm-player-cli` | CLI tool for stream analysis, QC, and monitoring |

## Features

### Core Library (`psm-player-core`)

- **Manifest Parsing**: HLS (m3u8) and DASH (MPD) support
- **Buffer Management**: Prefetching, memory limits, seek optimization
- **ABR Algorithms**: Throughput-based, BOLA, and Hybrid selection
- **Analytics**: QoE metrics, event emission, heartbeats
- **DRM Ready**: Widevine, FairPlay, PlayReady key management

### Desktop Player (`psm-player-desktop`)

- Hardware-accelerated decoding via GStreamer
- Full DRM support with Widevine CDM
- Low-latency playback for live streams

### WASM Library (`psm-player-wasm`)

- Runs in browser via WebAssembly
- Integrates with Media Source Extensions (MSE)
- Shares ABR logic with native players

### Tauri App (`psm-player-tauri`)

- Cross-platform (Windows, macOS, Linux)
- Native performance with web UI
- Electron-like DX with Rust backend

### CLI Tool (`psm-player-cli`)

```bash
# Analyze a manifest
psm-cli analyze https://example.com/master.m3u8

# Validate stream accessibility
psm-cli validate https://example.com/master.m3u8 --segments 20

# Run QC checks
psm-cli qc https://example.com/master.m3u8 --output report.json

# Monitor live stream
psm-cli monitor https://example.com/live.m3u8 --interval 5
```

## Building

```bash
# Build all crates
cargo build

# Build only the core library
cargo build -p psm-player-core

# Build the CLI tool
cargo build -p psm-player-cli --release

# Build WASM (requires wasm-pack)
cd crates/psm-player-wasm
wasm-pack build --target web
```

## Integration with purplesquirrel-core

The player integrates with the formally verified `purplesquirrel-core` library for:

- **Subscription Management**: Token-gated content access
- **Payment Processing**: Solana Pay integration
- **Analytics**: Verified event emission

```rust
use psm_player_core::{PlayerSession, PlayerConfig};
use purplesquirrel_core::types::SubscriptionState;

// Check subscription before allowing HD playback
async fn can_play_hd(user_subscription: &Subscription) -> bool {
    matches!(
        user_subscription.state,
        SubscriptionState::Active | SubscriptionState::Trialing
    )
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Application Layer                          │
├──────────────┬──────────────┬──────────────┬───────────────────┤
│  Desktop     │    WASM      │    Tauri     │       CLI         │
│  (GStreamer) │  (Browser)   │  (Desktop)   │   (Headless)      │
├──────────────┴──────────────┴──────────────┴───────────────────┤
│                      psm-player-core                            │
├─────────────────────────────────────────────────────────────────┤
│  Manifest  │  Buffer   │   ABR    │  Session  │  Analytics     │
│  Parser    │  Manager  │  Engine  │  Control  │  Emitter       │
├─────────────────────────────────────────────────────────────────┤
│                    purplesquirrel-core                          │
│         (Formally Verified Business Logic)                      │
└─────────────────────────────────────────────────────────────────┘
```

## License

MIT OR Apache-2.0
