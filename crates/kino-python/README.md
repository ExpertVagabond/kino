# PSM Frequency - Python Bindings

Python bindings for Kino's frequency analysis library.

## Installation

### From Source (Development)

```bash
# Install maturin
pip install maturin

# Build and install
cd crates/kino-python
maturin develop --release
```

### From Wheel

```bash
pip install psm_frequency-*.whl
```

## Quick Start

```python
from kino_frequency import FrequencyAnalyzer, Fingerprinter, ContentTagger

# Basic frequency analysis
analyzer = FrequencyAnalyzer(sample_rate=44100)
result = analyzer.analyze("music.wav")

print(f"Dominant frequency: {result.dominant_frequencies[0].frequency:.1f} Hz")
print(f"Spectral centroid: {result.spectral_centroid:.1f} Hz")

# Audio fingerprinting
fingerprinter = Fingerprinter()
fp = fingerprinter.fingerprint("music.wav")
print(f"Content hash: {fp.hash}")

# Content auto-tagging
tagger = ContentTagger()
tags = tagger.predict("music.wav")
for tag in tags:
    print(f"{tag.category}/{tag.name}: {tag.confidence:.1%}")
```

## API Reference

### FrequencyAnalyzer

Performs FFT-based frequency analysis on audio files.

```python
class FrequencyAnalyzer:
    def __init__(
        self,
        sample_rate: int = 44100,
        fft_size: int = 4096,
        hop_size: int = 2048
    ): ...

    def analyze(self, file_path: str) -> AnalysisResult: ...
    def dominant_frequencies(self, file_path: str, top_k: int = 10) -> list[DominantFrequency]: ...
    def compute_signature(self, file_path: str) -> FrequencySignature: ...
```

#### AnalysisResult

```python
@dataclass
class AnalysisResult:
    dominant_frequencies: list[DominantFrequency]
    band_energies: BandEnergies
    spectral_centroid: float
    spectral_rolloff: float
    spectral_flatness: float
    zero_crossing_rate: float
```

#### BandEnergies

```python
@dataclass
class BandEnergies:
    sub_bass: float    # 20-60 Hz
    bass: float        # 60-250 Hz
    low_mid: float     # 250-500 Hz
    mid: float         # 500-2000 Hz
    high_mid: float    # 2000-4000 Hz
    presence: float    # 4000-6000 Hz
    brilliance: float  # 6000-20000 Hz
```

### Fingerprinter

Generates audio fingerprints for content verification.

```python
class Fingerprinter:
    def __init__(self): ...
    def fingerprint(self, file_path: str) -> Fingerprint: ...
    def verify(self, file_path: str, expected_hash: str) -> bool: ...
```

#### Fingerprint

```python
@dataclass
class Fingerprint:
    hash: str           # SHA-256 hash of spectral peaks
    duration_secs: float
    peak_count: int
    sample_rate: int
```

### ContentTagger

Automatically tags content based on audio characteristics.

```python
class ContentTagger:
    def __init__(self): ...
    def predict(self, file_path: str, max_tags: int = 5) -> list[ContentTag]: ...
```

#### ContentTag

```python
@dataclass
class ContentTag:
    name: str           # e.g., "electronic", "speech"
    category: str       # e.g., "genre", "mood", "content_type"
    confidence: float   # 0.0 to 1.0
```

## Examples

### Batch Processing

```python
from pathlib import Path
from kino_frequency import FrequencyAnalyzer, Fingerprinter

analyzer = FrequencyAnalyzer()
fingerprinter = Fingerprinter()

# Process all WAV files in directory
for wav_file in Path("music/").glob("*.wav"):
    result = analyzer.analyze(str(wav_file))
    fp = fingerprinter.fingerprint(str(wav_file))

    print(f"{wav_file.name}:")
    print(f"  Hash: {fp.hash[:16]}...")
    print(f"  Centroid: {result.spectral_centroid:.0f} Hz")
```

### NumPy Integration

```python
import numpy as np
from kino_frequency import FrequencyAnalyzer

analyzer = FrequencyAnalyzer(sample_rate=44100)

# Get raw signature data as numpy arrays
signature = analyzer.compute_signature("audio.wav")
mel_bands = np.array(signature.mel_bands)
mfcc = np.array(signature.mfcc)

# Compute similarity with cosine distance
def cosine_similarity(a, b):
    return np.dot(a, b) / (np.linalg.norm(a) * np.linalg.norm(b))

sig1 = analyzer.compute_signature("track1.wav")
sig2 = analyzer.compute_signature("track2.wav")
similarity = cosine_similarity(sig1.mel_bands, sig2.mel_bands)
print(f"Similarity: {similarity:.1%}")
```

### Real-time Visualization

See `examples/realtime_visualization.py` for a complete real-time audio visualization example using matplotlib.

```bash
pip install sounddevice matplotlib
python examples/realtime_visualization.py
```

## Supported Formats

The library supports all formats that FFmpeg can decode:

- **Audio**: WAV, MP3, FLAC, OGG, M4A, AAC
- **Video**: MP4, MKV, WebM, AVI, MOV (audio track extracted)

Note: FFmpeg must be installed and available in PATH.

## Performance

Typical performance on Apple M1:

| Operation | Duration |
|-----------|----------|
| Analyze 10s audio | ~15ms |
| Fingerprint 10s audio | ~12ms |
| Compute signature | ~8ms |

## License

MIT OR Apache-2.0
