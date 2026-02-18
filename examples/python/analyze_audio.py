#!/usr/bin/env python3
"""
PSM Player Frequency Analysis Example

This script demonstrates how to use the psm-player-python bindings
to analyze audio files and extract frequency features.

Requirements:
    pip install psm-player numpy matplotlib

Usage:
    python analyze_audio.py input.wav
    python analyze_audio.py input.wav --plot
    python analyze_audio.py input.wav --fingerprint
"""

import argparse
import sys
import wave
import struct
from pathlib import Path

try:
    import psm_player
    HAS_PSM = True
except ImportError:
    HAS_PSM = False
    print("Warning: psm_player not installed. Using fallback implementation.")

try:
    import numpy as np
    HAS_NUMPY = True
except ImportError:
    HAS_NUMPY = False

try:
    import matplotlib.pyplot as plt
    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False


def load_wav(filepath: str) -> tuple[list[float], int]:
    """Load a WAV file and return samples as floats in [-1, 1] range."""
    with wave.open(filepath, 'rb') as wav:
        n_channels = wav.getnchannels()
        sample_width = wav.getsampwidth()
        sample_rate = wav.getframerate()
        n_frames = wav.getnframes()

        raw_data = wav.readframes(n_frames)

        # Convert to samples
        if sample_width == 1:
            fmt = f'{n_frames * n_channels}B'
            samples = struct.unpack(fmt, raw_data)
            samples = [(s - 128) / 128.0 for s in samples]
        elif sample_width == 2:
            fmt = f'{n_frames * n_channels}h'
            samples = struct.unpack(fmt, raw_data)
            samples = [s / 32768.0 for s in samples]
        else:
            raise ValueError(f"Unsupported sample width: {sample_width}")

        # Convert stereo to mono by averaging channels
        if n_channels == 2:
            samples = [(samples[i] + samples[i + 1]) / 2 for i in range(0, len(samples), 2)]

        return samples, sample_rate


def analyze_with_psm(samples: list[float], sample_rate: int) -> dict:
    """Analyze audio using PSM Player library."""
    if not HAS_PSM:
        raise RuntimeError("psm_player is not installed")

    analyzer = psm_player.FrequencyAnalyzer(
        fft_size=4096,
        hop_size=2048,
        sample_rate=sample_rate
    )

    result = analyzer.analyze(samples)

    return {
        'dominant_frequencies': result.dominant_frequencies,
        'spectral_centroid': result.spectral_centroid,
        'spectral_rolloff': result.spectral_rolloff,
        'spectral_flatness': result.spectral_flatness,
        'band_energies': result.band_energies,
    }


def analyze_fallback(samples: list[float], sample_rate: int, fft_size: int = 4096) -> dict:
    """Fallback analysis using pure Python/NumPy."""
    import math

    n = min(fft_size, len(samples))

    # Apply Hann window
    windowed = []
    for i in range(n):
        window = 0.5 * (1 - math.cos(2 * math.pi * i / (n - 1)))
        windowed.append(samples[i] * window)

    # Simple DFT
    spectrum = []
    for k in range(n // 2):
        real = 0.0
        imag = 0.0
        for i, sample in enumerate(windowed):
            angle = 2 * math.pi * k * i / n
            real += sample * math.cos(angle)
            imag -= sample * math.sin(angle)
        magnitude = math.sqrt(real * real + imag * imag) * 2 / n
        spectrum.append(magnitude)

    freq_resolution = sample_rate / fft_size

    # Compute centroid
    weighted_sum = sum(m * (i * freq_resolution) for i, m in enumerate(spectrum))
    total = sum(spectrum)
    centroid = weighted_sum / total if total > 0 else 0

    # Compute rolloff (95%)
    target = sum(spectrum) * 0.95
    cumulative = 0
    rolloff = 0
    for i, m in enumerate(spectrum):
        cumulative += m
        if cumulative >= target:
            rolloff = i * freq_resolution
            break

    # Compute flatness
    n_bins = len(spectrum)
    log_sum = sum(math.log(m + 1e-10) for m in spectrum)
    geometric_mean = math.exp(log_sum / n_bins)
    arithmetic_mean = sum(spectrum) / n_bins
    flatness = geometric_mean / arithmetic_mean if arithmetic_mean > 0 else 0

    # Find dominant frequencies
    indexed = [(i, m) for i, m in enumerate(spectrum)]
    indexed.sort(key=lambda x: x[1], reverse=True)
    max_mag = indexed[0][1] if indexed else 1

    dominant = []
    for rank, (idx, mag) in enumerate(indexed[:10]):
        dominant.append({
            'frequency_hz': idx * freq_resolution,
            'magnitude': mag / max_mag,
            'rank': rank + 1,
        })

    # Compute band energies
    bands = [
        ('sub_bass', 20, 60),
        ('bass', 60, 250),
        ('low_mid', 250, 500),
        ('mid', 500, 2000),
        ('high_mid', 2000, 4000),
        ('high', 4000, 20000),
    ]

    band_energies = {}
    for name, low, high in bands:
        energy = 0
        for i, m in enumerate(spectrum):
            freq = i * freq_resolution
            if low <= freq < high:
                energy += m
        band_energies[name] = energy

    # Normalize
    total_band = sum(band_energies.values())
    if total_band > 0:
        band_energies = {k: v / total_band for k, v in band_energies.items()}

    return {
        'dominant_frequencies': dominant,
        'spectral_centroid': centroid,
        'spectral_rolloff': rolloff,
        'spectral_flatness': flatness,
        'band_energies': band_energies,
        'spectrum': spectrum,
    }


def generate_fingerprint(samples: list[float], sample_rate: int) -> str:
    """Generate a simple audio fingerprint."""
    import hashlib

    fft_size = 4096
    hop_size = 2048
    hash_data = []

    num_frames = (len(samples) - fft_size) // hop_size + 1

    for frame_idx in range(min(num_frames, 100)):
        start = frame_idx * hop_size
        frame = samples[start:start + fft_size]

        if len(frame) < fft_size:
            break

        # Calculate frame energy
        energy = sum(s * s for s in frame)
        hash_data.append(int(energy * 255) % 256)

    # Create hash
    if hash_data:
        data_bytes = bytes(hash_data)
        return hashlib.sha256(data_bytes).hexdigest()[:32]

    return ""


def plot_analysis(samples: list[float], result: dict, sample_rate: int):
    """Plot the analysis results."""
    if not HAS_MATPLOTLIB:
        print("matplotlib not installed. Cannot plot.")
        return

    fig, axes = plt.subplots(2, 2, figsize=(12, 8))
    fig.suptitle('PSM Player Frequency Analysis', fontsize=14, fontweight='bold')

    # Waveform
    ax = axes[0, 0]
    time = [i / sample_rate for i in range(len(samples[:sample_rate]))]
    ax.plot(time, samples[:sample_rate], color='#9333EA', linewidth=0.5)
    ax.set_xlabel('Time (s)')
    ax.set_ylabel('Amplitude')
    ax.set_title('Waveform (first second)')
    ax.grid(True, alpha=0.3)

    # Spectrum
    ax = axes[0, 1]
    if 'spectrum' in result:
        freqs = [i * sample_rate / (len(result['spectrum']) * 2) for i in range(len(result['spectrum']))]
        ax.plot(freqs[:1000], result['spectrum'][:1000], color='#7C3AED', linewidth=0.8)
    ax.axvline(x=result['spectral_centroid'], color='red', linestyle='--', label=f"Centroid: {result['spectral_centroid']:.0f} Hz")
    ax.axvline(x=result['spectral_rolloff'], color='orange', linestyle='--', label=f"Rolloff: {result['spectral_rolloff']:.0f} Hz")
    ax.set_xlabel('Frequency (Hz)')
    ax.set_ylabel('Magnitude')
    ax.set_title('Spectrum')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Dominant frequencies
    ax = axes[1, 0]
    dominant = result['dominant_frequencies'][:8]
    freqs = [d['frequency_hz'] for d in dominant]
    mags = [d['magnitude'] for d in dominant]
    colors = plt.cm.Purples([(i + 3) / 10 for i in range(len(freqs))])
    ax.barh(range(len(freqs)), freqs, color=colors)
    ax.set_yticks(range(len(freqs)))
    ax.set_yticklabels([f"#{d['rank']}: {d['frequency_hz']:.0f} Hz" for d in dominant[:8]])
    ax.set_xlabel('Frequency (Hz)')
    ax.set_title('Dominant Frequencies')
    ax.invert_yaxis()

    # Band energies
    ax = axes[1, 1]
    bands = result['band_energies']
    names = list(bands.keys())
    values = list(bands.values())
    colors = ['#6B21A8', '#7C3AED', '#9333EA', '#A855F7', '#C084FC', '#E9D5FF']
    ax.bar(names, values, color=colors)
    ax.set_ylabel('Normalized Energy')
    ax.set_title('Band Energy Distribution')
    ax.tick_params(axis='x', rotation=45)

    plt.tight_layout()
    plt.show()


def main():
    parser = argparse.ArgumentParser(
        description='Analyze audio files using PSM Player frequency analysis'
    )
    parser.add_argument('input', help='Input audio file (WAV format)')
    parser.add_argument('--plot', action='store_true', help='Show visualization plots')
    parser.add_argument('--fingerprint', action='store_true', help='Generate audio fingerprint')
    parser.add_argument('--json', action='store_true', help='Output as JSON')

    args = parser.parse_args()

    # Check file exists
    input_path = Path(args.input)
    if not input_path.exists():
        print(f"Error: File not found: {args.input}")
        sys.exit(1)

    if not input_path.suffix.lower() == '.wav':
        print("Warning: Only WAV files are fully supported")

    # Load audio
    print(f"Loading: {args.input}")
    samples, sample_rate = load_wav(args.input)
    duration = len(samples) / sample_rate
    print(f"Duration: {duration:.2f}s, Sample rate: {sample_rate} Hz, Samples: {len(samples)}")

    # Analyze
    print("\nAnalyzing...")
    if HAS_PSM:
        result = analyze_with_psm(samples, sample_rate)
    else:
        result = analyze_fallback(samples, sample_rate)

    # Print results
    if args.json:
        import json
        print(json.dumps(result, indent=2, default=str))
    else:
        print("\n=== Analysis Results ===")
        print(f"Spectral Centroid: {result['spectral_centroid']:.2f} Hz")
        print(f"Spectral Rolloff:  {result['spectral_rolloff']:.2f} Hz")
        print(f"Spectral Flatness: {result['spectral_flatness']:.4f}")

        print("\nDominant Frequencies:")
        for freq in result['dominant_frequencies'][:5]:
            print(f"  #{freq['rank']}: {freq['frequency_hz']:.2f} Hz (mag: {freq['magnitude']:.2f})")

        print("\nBand Energies:")
        for band, energy in result['band_energies'].items():
            bar = '=' * int(energy * 40)
            print(f"  {band:8s}: {bar} {energy:.1%}")

    # Fingerprint
    if args.fingerprint:
        print("\nGenerating fingerprint...")
        fingerprint = generate_fingerprint(samples, sample_rate)
        print(f"Fingerprint: {fingerprint}")

    # Plot
    if args.plot:
        plot_analysis(samples, result, sample_rate)


if __name__ == '__main__':
    main()
