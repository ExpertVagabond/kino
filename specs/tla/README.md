# Kino TLA+ Specifications

Formal specifications for the Purple Squirrel Media video player using TLA+ (Temporal Logic of Actions).

## Overview

These specifications model and verify the correctness of the Kino's core behaviors:

| Specification | Description |
|---------------|-------------|
| `PSMPlayerState.tla` | Core player state machine (play, pause, buffer, seek, etc.) |
| `PSMAbrAlgorithm.tla` | BOLA and throughput-based ABR algorithm |
| `PSMBufferController.tla` | Segment buffer management and prefetch logic |
| `PSMConcurrentStreaming.tla` | Concurrent segment downloading and buffer management |
| `PSMPlaylist.tla` | Playlist/queue management with shuffle and repeat |
| `PSMCaptions.tla` | Caption/subtitle track selection and rendering |
| `PSMDrm.tla` | DRM license acquisition and key management |
| `PSMPlayer.tla` | Complete system composition |

## Quick Start

### Prerequisites

Install TLA+ Toolbox or the command-line tools:

```bash
# macOS with Homebrew
brew install tla-plus-toolbox

# Or download from https://lamport.azurewebsites.net/tla/toolbox.html
```

### Running Model Checker

```bash
# Check player state machine
tlc PSMPlayerState.tla -config PSMPlayerState.cfg

# Check ABR algorithm
tlc PSMAbrAlgorithm.tla -config PSMAbrAlgorithm.cfg

# Check buffer controller
tlc PSMBufferController.tla -config PSMBufferController.cfg

# Check concurrent streaming
tlc PSMConcurrentStreaming.tla -config PSMConcurrentStreaming.cfg

# Check playlist management
tlc PSMPlaylist.tla -config PSMPlaylist.cfg

# Check captions
tlc PSMCaptions.tla -config PSMCaptions.cfg

# Check DRM
tlc PSMDrm.tla -config PSMDrm.cfg

# Check complete system
tlc PSMPlayer.tla -config PSMPlayer.cfg
```

### Running All Specs

```bash
#!/bin/bash
# Run all TLA+ specifications
for spec in PSMPlayerState PSMAbrAlgorithm PSMBufferController \
            PSMConcurrentStreaming PSMPlaylist PSMCaptions PSMDrm PSMPlayer; do
    echo "=== Checking $spec ==="
    tlc "$spec.tla" -config "$spec.cfg"
done
```

## Specifications

### 1. Player State Machine (`PSMPlayerState.tla`)

Models the video player's state transitions:

```
States: Idle → Loading → Ready → Playing ↔ Paused
                              ↓         ↓
                          Buffering → Seeking
                              ↓
                            Ended
                              ↓
                            Error
```

**Key Properties:**
- `TypeInvariant` - All variables have valid types
- `BufferBounded` - Buffer never exceeds maximum
- `MutuallyExclusiveDisplayModes` - Can't be fullscreen AND PiP
- `PlayingImpliesBuffer` - Playing requires sufficient buffer
- `BufferingResolves` - Buffering eventually exits

### 2. ABR Algorithm (`PSMAbrAlgorithm.tla`)

Models the adaptive bitrate selection:

**BOLA Algorithm:**
- Buffer Occupancy based Lyapunov Algorithm
- Balances quality vs. rebuffer risk
- Conservative when buffer is low

**Throughput Algorithm:**
- Simple bandwidth-based selection
- Picks highest feasible quality

**Hybrid:**
- Uses throughput when buffer healthy
- Falls back to BOLA when buffer concerning

**Key Properties:**
- `QualityRespectsBandwidth` - Selected quality fits within bandwidth
- `RebufferingTemporary` - Rebuffering always resolves
- `AcceptableQoE` - QoE score stays acceptable

### 3. Concurrent Streaming (`PSMConcurrentStreaming.tla`)

Models parallel segment downloading:

**Components:**
- Manifest fetcher
- Download worker pool (max N workers)
- Segment buffer
- Quality switcher

**Key Properties:**
- `WorkerLimit` - Never exceed max workers
- `NoDuplicateDownloads` - No segment downloaded twice
- `BufferContiguous` - Buffer segments are contiguous
- `DownloadsComplete` - Downloads eventually finish

### 4. Buffer Controller (`PSMBufferController.tla`)

Models segment buffer management:

**Components:**
- Segment prefetch scheduling
- Buffer health monitoring (critical/low/healthy/full)
- Download queue management
- Eviction of old segments

**Key Properties:**
- `BufferSizeBounded` - Buffer never exceeds maximum
- `NoDuplicateSegments` - No segment buffered twice
- `ConcurrentDownloadsBounded` - Max 3 parallel downloads
- `StallResolves` - Playback stalls eventually recover

### 5. Playlist Management (`PSMPlaylist.tla`)

Models playlist/queue functionality:

**Features:**
- Add, remove, reorder tracks
- Shuffle mode with Fisher-Yates-style ordering
- Repeat modes: none, one, all
- History tracking for back navigation
- Auto-advance on track completion

**Key Properties:**
- `CurrentIndexValid` - Current track index always valid
- `ShuffleOrderConsistent` - Shuffle order matches playlist size
- `PlayingImpliesValidTrack` - Playing requires valid track
- `PlayingResolves` - Playback eventually ends or pauses

### 6. Captions/Subtitles (`PSMCaptions.tla`)

Models caption track management:

**Features:**
- Multiple caption tracks with language selection
- Cue visibility based on time position
- Font size and position settings
- Loading states and error handling

**Key Properties:**
- `ActiveTrackValid` - Selected track exists
- `NoCuesWhenDisabled` - No visible cues when captions off
- `LoadingStateConsistent` - Loading implies no cues yet
- `LoadingCompletes` - Caption loading eventually finishes

### 7. DRM License Management (`PSMDrm.tla`)

Models DRM (Digital Rights Management) workflow:

**Supported Key Systems:**
- Widevine, FairPlay, PlayReady, ClearKey

**State Machine:**
```
idle → detecting → initializing → session_creating →
license_requesting → license_updating → active
                                          ↓
                                    key_expired → (renew)
```

**Key Properties:**
- `ActiveRequiresLicenses` - Active playback has valid licenses
- `SessionIdValid` - Session exists when expected
- `RetryCountBounded` - License retries are limited
- `ErrorStateConsistent` - Error state has error type

### 8. Complete System (`PSMPlayer.tla`)

Composes all components into unified model:

**Verified Properties:**
- End-to-end playback correctness
- Quality adaptation under network changes
- Error recovery
- Resource bounds

## Safety Properties

Properties that must ALWAYS hold:

| Property | Description |
|----------|-------------|
| `TypeOK` | All variables have valid types |
| `BufferBounded` | Buffer ≤ MaxBuffer |
| `WorkersBounded` | Active downloads ≤ MaxWorkers |
| `QualityValid` | Quality ∈ {0..N-1} |
| `PositionValid` | Position ∈ {-1..Duration-1} |
| `MutualExclusionDisplayModes` | ¬(fullscreen ∧ PiP) |

## Liveness Properties

Properties that must EVENTUALLY hold:

| Property | Description |
|----------|-------------|
| `EventuallyPlaysOrErrors` | Loading → Playing ∨ Error |
| `BufferingResolves` | Buffering → Playing ∨ Error |
| `PlaybackCompletes` | Playing → Ended ∨ Paused ∨ Error |
| `DownloadsProgress` | Active downloads → Buffer grows |

## QoE Metrics

The specs include QoE (Quality of Experience) calculations:

```
QoE Score = 100 - (rebuffers × 15) - (switches × 3) + (quality × 10)
```

- **Acceptable QoE**: Score ≥ 60
- **Penalties**: Rebuffers (15 pts), Quality switches (3 pts)
- **Bonus**: Higher quality levels

## Configuration

Each `.cfg` file defines:
- **CONSTANTS**: System parameters
- **SPECIFICATION**: Main spec to check
- **INVARIANTS**: Safety properties
- **PROPERTIES**: Liveness properties

Adjust constants to explore different scenarios:

```
CONSTANTS
    VideoDuration = 10      \* Number of segments
    MaxBuffer = 30          \* Buffer size in seconds
    MinBufferToPlay = 8     \* Min buffer to start
    MaxBandwidth = 6000     \* Max bandwidth in kbps
    MaxDownloadWorkers = 3  \* Parallel downloads
```

## Extending the Specs

### Adding New States

1. Add to `PlayerStates` set
2. Add transition actions
3. Update `Next` relation
4. Add relevant invariants

### Adding New Properties

```tla
\* Safety (invariant)
MyProperty == condition

\* Liveness (temporal)
MyLiveness == P ~> Q  \* P leads to Q
```

## Integration with Code

The TLA+ specs serve as:
1. **Design documentation** - Formal description of behavior
2. **Test oracle** - Generate test cases from traces
3. **Bug prevention** - Find edge cases before coding

## References

- [TLA+ Home](https://lamport.azurewebsites.net/tla/tla.html)
- [Learn TLA+](https://learntla.com/)
- [BOLA Paper](https://arxiv.org/abs/1601.06748)
- [HLS Spec](https://datatracker.ietf.org/doc/html/rfc8216)

## License

MIT OR Apache-2.0
