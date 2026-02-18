#!/usr/bin/env python3
"""
Basic Frequency Analysis Example

This example demonstrates how to use the psm_frequency Python bindings
to analyze audio files and extract frequency information.

Usage:
    python basic_analysis.py audio.wav
    python basic_analysis.py video.mp4
"""

import sys
import numpy as np
from kino_frequency import FrequencyAnalyzer, Fingerprinter, ContentTagger


def analyze_audio_file(file_path: str) -> None:
    """Perform basic frequency analysis on an audio/video file."""

    # Initialize the analyzer with 44.1kHz sample rate
    analyzer = FrequencyAnalyzer(sample_rate=44100)

    print(f"Analyzing: {file_path}")
    print("-" * 50)

    # Analyze the file
    result = analyzer.analyze(file_path)

    # Display dominant frequencies
    print("\nDominant Frequencies:")
    for i, freq in enumerate(result.dominant_frequencies[:5], 1):
        print(f"  {i}. {freq.frequency:.1f} Hz (magnitude: {freq.magnitude:.2f})")

    # Display band energies
    print("\nBand Energies:")
    bands = result.band_energies
    print(f"  Sub-bass (20-60 Hz):    {bands.sub_bass:.4f}")
    print(f"  Bass (60-250 Hz):       {bands.bass:.4f}")
    print(f"  Low-mid (250-500 Hz):   {bands.low_mid:.4f}")
    print(f"  Mid (500-2000 Hz):      {bands.mid:.4f}")
    print(f"  High-mid (2-4 kHz):     {bands.high_mid:.4f}")
    print(f"  Presence (4-6 kHz):     {bands.presence:.4f}")
    print(f"  Brilliance (6-20 kHz):  {bands.brilliance:.4f}")

    # Display spectral features
    print("\nSpectral Features:")
    print(f"  Centroid:  {result.spectral_centroid:.1f} Hz")
    print(f"  Rolloff:   {result.spectral_rolloff:.1f} Hz")
    print(f"  Flatness:  {result.spectral_flatness:.4f}")
    print(f"  ZCR:       {result.zero_crossing_rate:.4f}")


def generate_fingerprint(file_path: str) -> None:
    """Generate an audio fingerprint for content verification."""

    fingerprinter = Fingerprinter()

    print(f"\nGenerating fingerprint for: {file_path}")
    print("-" * 50)

    fingerprint = fingerprinter.fingerprint(file_path)

    print(f"  Hash:        {fingerprint.hash[:32]}...")
    print(f"  Duration:    {fingerprint.duration_secs:.2f} seconds")
    print(f"  Peak count:  {fingerprint.peak_count}")
    print(f"  Sample rate: {fingerprint.sample_rate} Hz")


def auto_tag_content(file_path: str) -> None:
    """Automatically generate content tags based on audio characteristics."""

    tagger = ContentTagger()

    print(f"\nAuto-tagging: {file_path}")
    print("-" * 50)

    tags = tagger.predict(file_path)

    print("  Predicted tags:")
    for tag in tags:
        confidence_bar = "█" * int(tag.confidence * 20)
        print(f"    [{tag.category}] {tag.name}: {confidence_bar} ({tag.confidence:.1%})")


def main():
    if len(sys.argv) < 2:
        print("Usage: python basic_analysis.py <audio_or_video_file>")
        print("\nSupported formats: WAV, MP3, FLAC, MP4, MKV, etc.")
        sys.exit(1)

    file_path = sys.argv[1]

    try:
        # Run all analyses
        analyze_audio_file(file_path)
        generate_fingerprint(file_path)
        auto_tag_content(file_path)

    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
