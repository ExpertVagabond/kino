# kino-mcp

MCP server that gives AI agents full access to video stream analysis, audio fingerprinting, quality monitoring, and encoding presets — powered by the `kino-cli` Rust binary.

## Tools (8)

| Tool | Description |
|------|-------------|
| `analyze_stream` | Parse HLS/DASH manifest — renditions, codecs, duration, live status |
| `validate_stream` | Check segment accessibility and bitrate conformance |
| `quality_check` | Full QC report — DRM, captions, bitrate ladder |
| `monitor_stream` | Live stream health check — latency, segment freshness |
| `fingerprint_audio` | Generate audio fingerprint for content identification |
| `autotag_content` | Auto-detect genre, mood, BPM from audio |
| `compare_streams` | Diff two streams for quality mismatches |
| `encode_video` | Generate encoding presets (mobile, desktop, 4k, low-bandwidth) |

## Install

```bash
npm install -g kino-mcp
```

### Requires `kino-cli`

```bash
# From source
cargo install --path crates/kino-cli

# Or from crates.io (when published)
cargo install kino-cli
```

If `kino-cli` is not in your PATH, set the `KINO_CLI_PATH` environment variable:

```bash
export KINO_CLI_PATH=/path/to/kino-cli
```

## Configure in Claude Desktop

Add to your MCP settings (`~/.claude/settings.json` or Claude Desktop config):

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

Or if installed locally:

```json
{
  "mcpServers": {
    "kino": {
      "command": "node",
      "args": ["/path/to/kino/mcp-server/index.js"]
    }
  }
}
```

## Usage

Once configured, any MCP-compatible AI agent can:

- Analyze HLS/DASH streams to understand available renditions and codecs
- Validate segment accessibility before going live
- Run automated QC checks on video assets
- Monitor live stream health in real time
- Fingerprint audio for content identification and deduplication
- Auto-tag media files with genre, mood, and tempo
- Compare streams to detect quality regressions
- Generate encoding presets for target platforms

## License

MIT
