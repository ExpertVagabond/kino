#!/usr/bin/env python3
"""
Jupyter Notebook Integration Demo

This script can be run in a Jupyter notebook to demonstrate
interactive frequency analysis with visualizations.

Usage in Jupyter:
    %run jupyter_notebook_demo.py

Or copy cells individually into notebook.
"""

# Cell 1: Imports and Setup
# =========================

import numpy as np
import matplotlib.pyplot as plt
from IPython.display import display, HTML, Audio
from ipywidgets import interact, FloatSlider, IntSlider, Dropdown

# Attempt to import psm_frequency
try:
    from psm_frequency import FrequencyAnalyzer, Fingerprinter, ContentTagger
    PSM_AVAILABLE = True
except ImportError:
    print("psm_frequency not installed. Install with: pip install psm-frequency")
    PSM_AVAILABLE = False

# Cell 2: Helper Functions
# ========================

def generate_test_signal(
    frequencies: list[float],
    amplitudes: list[float],
    duration: float = 2.0,
    sample_rate: int = 44100,
) -> np.ndarray:
    """Generate a test signal with multiple frequency components."""
    t = np.linspace(0, duration, int(sample_rate * duration), endpoint=False)
    signal = np.zeros_like(t)

    for freq, amp in zip(frequencies, amplitudes):
        signal += amp * np.sin(2 * np.pi * freq * t)

    # Normalize
    signal = signal / np.max(np.abs(signal)) * 0.8
    return signal.astype(np.float32)


def plot_spectrum(samples: np.ndarray, sample_rate: int = 44100, title: str = "Frequency Spectrum"):
    """Plot the frequency spectrum of a signal."""
    n = len(samples)
    fft = np.fft.rfft(samples * np.hanning(n))
    freqs = np.fft.rfftfreq(n, 1/sample_rate)
    magnitude = np.abs(fft) / n

    plt.figure(figsize=(12, 4))
    plt.subplot(1, 2, 1)
    plt.plot(freqs, magnitude)
    plt.xlabel('Frequency (Hz)')
    plt.ylabel('Magnitude')
    plt.title(title)
    plt.xlim(0, 5000)
    plt.grid(True, alpha=0.3)

    plt.subplot(1, 2, 2)
    plt.semilogx(freqs[1:], 20 * np.log10(magnitude[1:] + 1e-10))
    plt.xlabel('Frequency (Hz)')
    plt.ylabel('Magnitude (dB)')
    plt.title(f'{title} (Log Scale)')
    plt.xlim(20, 20000)
    plt.grid(True, alpha=0.3)

    plt.tight_layout()
    plt.show()


def plot_spectrogram(samples: np.ndarray, sample_rate: int = 44100, title: str = "Spectrogram"):
    """Plot a spectrogram of the signal."""
    plt.figure(figsize=(12, 4))
    plt.specgram(samples, Fs=sample_rate, cmap='magma', NFFT=2048, noverlap=1024)
    plt.colorbar(label='dB')
    plt.xlabel('Time (s)')
    plt.ylabel('Frequency (Hz)')
    plt.title(title)
    plt.ylim(0, 8000)
    plt.show()


# Cell 3: Interactive Signal Generator
# ====================================

def interactive_signal_demo():
    """Interactive signal generator with real-time visualization."""

    @interact(
        freq1=FloatSlider(min=100, max=2000, step=10, value=440, description='Freq 1 (Hz)'),
        freq2=FloatSlider(min=100, max=2000, step=10, value=880, description='Freq 2 (Hz)'),
        freq3=FloatSlider(min=100, max=2000, step=10, value=1320, description='Freq 3 (Hz)'),
        amp1=FloatSlider(min=0, max=1, step=0.1, value=1.0, description='Amp 1'),
        amp2=FloatSlider(min=0, max=1, step=0.1, value=0.5, description='Amp 2'),
        amp3=FloatSlider(min=0, max=1, step=0.1, value=0.25, description='Amp 3'),
    )
    def update(freq1, freq2, freq3, amp1, amp2, amp3):
        signal = generate_test_signal(
            frequencies=[freq1, freq2, freq3],
            amplitudes=[amp1, amp2, amp3],
            duration=1.0
        )

        # Plot waveform and spectrum
        fig, axes = plt.subplots(1, 2, figsize=(12, 3))

        # Waveform (first 500 samples)
        axes[0].plot(signal[:500])
        axes[0].set_title('Waveform')
        axes[0].set_xlabel('Sample')
        axes[0].set_ylabel('Amplitude')
        axes[0].grid(True, alpha=0.3)

        # Spectrum
        n = len(signal)
        fft = np.fft.rfft(signal * np.hanning(n))
        freqs = np.fft.rfftfreq(n, 1/44100)
        magnitude = np.abs(fft) / n

        axes[1].plot(freqs, magnitude)
        axes[1].set_title('Frequency Spectrum')
        axes[1].set_xlabel('Frequency (Hz)')
        axes[1].set_ylabel('Magnitude')
        axes[1].set_xlim(0, 3000)
        axes[1].grid(True, alpha=0.3)

        plt.tight_layout()
        plt.show()

        # Return audio player
        return Audio(signal, rate=44100)


# Cell 4: PSM Frequency Analysis Demo
# ===================================

def psm_analysis_demo():
    """Demonstrate PSM frequency analysis features."""

    if not PSM_AVAILABLE:
        print("psm_frequency module not available")
        return

    # Generate test signal
    signal = generate_test_signal(
        frequencies=[261.63, 329.63, 392.00],  # C4, E4, G4 (C major chord)
        amplitudes=[1.0, 0.8, 0.6],
        duration=2.0
    )

    print("Generated C major chord (C4, E4, G4)")
    print("=" * 50)

    # Save as temporary WAV for analysis
    import tempfile
    import wave

    with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as f:
        temp_path = f.name

    # Write WAV file
    with wave.open(temp_path, 'w') as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(44100)
        wav.writeframes((signal * 32767).astype(np.int16).tobytes())

    # Analyze with PSM
    analyzer = FrequencyAnalyzer(sample_rate=44100)
    result = analyzer.analyze(temp_path)

    print("\nDominant Frequencies:")
    for i, freq in enumerate(result.dominant_frequencies[:5], 1):
        note = frequency_to_note(freq.frequency)
        print(f"  {i}. {freq.frequency:>8.1f} Hz ({note:>4}) - magnitude: {freq.magnitude:.4f}")

    print("\nBand Energies:")
    bands = result.band_energies
    print(f"  Sub-bass:   {bands.sub_bass:.4f}")
    print(f"  Bass:       {bands.bass:.4f}")
    print(f"  Low-mid:    {bands.low_mid:.4f}")
    print(f"  Mid:        {bands.mid:.4f}")
    print(f"  High-mid:   {bands.high_mid:.4f}")
    print(f"  Presence:   {bands.presence:.4f}")
    print(f"  Brilliance: {bands.brilliance:.4f}")

    print(f"\nSpectral Features:")
    print(f"  Centroid:  {result.spectral_centroid:.1f} Hz")
    print(f"  Rolloff:   {result.spectral_rolloff:.1f} Hz")
    print(f"  Flatness:  {result.spectral_flatness:.4f}")
    print(f"  ZCR:       {result.zero_crossing_rate:.4f}")

    # Visualize
    plot_spectrum(signal, title="C Major Chord Spectrum")

    # Cleanup
    import os
    os.unlink(temp_path)

    return signal


def frequency_to_note(freq: float) -> str:
    """Convert frequency to musical note."""
    if freq <= 0:
        return "--"

    notes = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']
    a4 = 440.0

    semitones = 12 * np.log2(freq / a4)
    midi = int(round(semitones + 69))

    if midi < 0 or midi > 127:
        return f"{freq:.0f}Hz"

    note_idx = midi % 12
    octave = (midi // 12) - 1

    return f"{notes[note_idx]}{octave}"


# Cell 5: Fingerprint Comparison Demo
# ===================================

def fingerprint_demo():
    """Demonstrate audio fingerprinting and matching."""

    if not PSM_AVAILABLE:
        print("psm_frequency module not available")
        return

    import tempfile
    import wave

    # Generate original signal
    original = generate_test_signal(
        frequencies=[440, 554.37, 659.25],  # A4, C#5, E5 (A major chord)
        amplitudes=[1.0, 0.7, 0.5],
        duration=3.0
    )

    # Generate modified versions
    noisy = original + np.random.normal(0, 0.05, len(original)).astype(np.float32)
    pitched = generate_test_signal(
        frequencies=[466.16, 587.33, 698.46],  # A#4, D5, F5 (shifted up)
        amplitudes=[1.0, 0.7, 0.5],
        duration=3.0
    )
    different = generate_test_signal(
        frequencies=[261.63, 293.66, 329.63],  # C4, D4, E4 (different)
        amplitudes=[1.0, 0.8, 0.6],
        duration=3.0
    )

    signals = {
        'original': original,
        'noisy': noisy,
        'pitch_shifted': pitched,
        'different': different,
    }

    # Save as WAV files
    temp_files = {}
    for name, sig in signals.items():
        with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as f:
            temp_files[name] = f.name
            with wave.open(f.name, 'w') as wav:
                wav.setnchannels(1)
                wav.setsampwidth(2)
                wav.setframerate(44100)
                wav.writeframes((sig * 32767).astype(np.int16).tobytes())

    # Generate fingerprints
    fingerprinter = Fingerprinter()
    fingerprints = {}

    print("Audio Fingerprint Comparison")
    print("=" * 50)

    for name, path in temp_files.items():
        fp = fingerprinter.fingerprint(path)
        fingerprints[name] = fp
        print(f"\n{name}:")
        print(f"  Hash: {fp.hash[:32]}...")
        print(f"  Peaks: {fp.peak_count}")

    # Compare signatures
    analyzer = FrequencyAnalyzer(sample_rate=44100)
    signatures = {}

    for name, path in temp_files.items():
        signatures[name] = analyzer.compute_signature(path)

    print("\n\nSimilarity Scores (vs original):")
    print("-" * 40)

    orig_mel = np.array(signatures['original'].mel_bands)
    for name in ['noisy', 'pitch_shifted', 'different']:
        other_mel = np.array(signatures[name].mel_bands)
        similarity = np.dot(orig_mel, other_mel) / (np.linalg.norm(orig_mel) * np.linalg.norm(other_mel))
        print(f"  {name:15}: {similarity:.1%}")

    # Cleanup
    import os
    for path in temp_files.values():
        os.unlink(path)


# Cell 6: Run Demos
# =================

if __name__ == '__main__':
    print("PSM Frequency Jupyter Demo")
    print("=" * 50)
    print("\nAvailable demos:")
    print("  1. interactive_signal_demo() - Interactive signal generator")
    print("  2. psm_analysis_demo()       - PSM frequency analysis")
    print("  3. fingerprint_demo()        - Fingerprint comparison")
    print("\nRun any demo by calling the function.")

    # Run basic demo if PSM is available
    if PSM_AVAILABLE:
        print("\n" + "=" * 50)
        psm_analysis_demo()
