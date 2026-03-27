# kino-mcp

**Video stream analysis and quality checking MCP -- monitor, validate, and encode with AI agents.**

[![npm version](https://img.shields.io/npm/v/kino-mcp)](https://npmjs.com/package/kino-mcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE-MIT)
[![Tools: 8](https://img.shields.io/badge/tools-8-green)]()

---

8 tools for HLS/DASH stream analysis, real-time monitoring, audio fingerprinting, content auto-tagging, and encoding preset generation. Powered by the `kino-cli` Rust binary with formally verified state machines (8 TLA+ specs). Give any MCP-compatible AI agent full video infrastructure awareness.

## Install

```bash
npm install -g kino-mcp
```

Requires the `kino-cli` binary:

```bash
# From the parent workspace
cargo install --path ../crates/kino-cli

# Or set the path manually
export KINO_CLI_PATH=/path/to/kino-cli
```

## Configure

Add to `claude_desktop_config.json` or `~/.mcp.json`:

```json
{
  "mcpServers": {
    "kino": {
      "command": "kino-mcp",
      "env": {
        "KINO_CLI_PATH": "/path/to/kino-cli"
      }
    }
  }
}
```

## Tool Reference

| Tool | Description |
|---|---|
| `analyze_stream` | Parse HLS/DASH manifest -- renditions, codecs, duration, live status |
| `validate_stream` | Check segment accessibility and bitrate conformance |
| `quality_check` | Full QC report -- DRM status, captions, bitrate ladder validation |
| `monitor_stream` | Live stream health -- latency, segment freshness, error rates |
| `fingerprint_audio` | Generate SHA-256 audio fingerprint for content identification |
| `autotag_content` | Auto-detect genre, mood, BPM from audio via FFT spectral analysis |
| `compare_streams` | Diff two streams for quality mismatches and regression detection |
| `encode_video` | Generate encoding presets for target platforms (mobile, desktop, 4K, low-bandwidth) |

## Usage Examples

```
> Analyze this HLS stream and tell me about the available renditions
  -> analyze_stream { url: "https://cdn.example.com/master.m3u8" }

> Run a full quality check before we go live
  -> quality_check { url: "https://cdn.example.com/master.m3u8", strict: true }

> Monitor the live stream health every 5 seconds
  -> monitor_stream { url: "https://cdn.example.com/live.m3u8", interval: 5 }

> Fingerprint this video for content identification
  -> fingerprint_audio { file: "video.mp4" }

> Compare our staging and production streams
  -> compare_streams { url_a: "https://staging.example.com/master.m3u8", url_b: "https://cdn.example.com/master.m3u8" }
```

## Why This One?

| | kino-mcp | Bitmovin QA | Mux Data |
|---|---|---|---|
| **MCP integration** | Native 8-tool MCP server | No MCP | No MCP |
| **Stream analysis** | HLS + DASH, all codecs | Enterprise QA only | Analytics only |
| **Audio fingerprinting** | Built-in (FFT + SHA-256) | No | No |
| **Auto-tagging** | Genre, mood, BPM from audio | No | No |
| **Quality monitoring** | Real-time segment-level checks | Batch reports | Viewer metrics |
| **Encoding presets** | Generate per platform | Separate product | No |
| **Formally verified** | 8 TLA+ specs in CI | No | No |
| **Cost** | Free / MIT | Enterprise pricing | Usage-based |

The only MCP server for video stream infrastructure. Enterprise tools exist for QA and analytics, but none expose stream analysis, fingerprinting, and monitoring through the Model Context Protocol.

## Architecture

kino-mcp is a thin Node.js MCP wrapper around the `kino-cli` Rust binary, which is part of the [Kino](https://github.com/ExpertVagabond/kino) 7-crate workspace. The CLI handles all heavy lifting -- manifest parsing (kino-core), FFT analysis (kino-frequency), and encoding -- while the MCP layer provides schema validation and tool dispatch.

## License

[MIT](../LICENSE-MIT) -- [Purple Squirrel Media](https://purplesquirrelmedia.io)
