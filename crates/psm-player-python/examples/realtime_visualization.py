#!/usr/bin/env python3
"""
Real-time Audio Visualization Example

Captures audio from the microphone and displays a real-time frequency
spectrum visualization using matplotlib.

Requirements:
    pip install sounddevice matplotlib

Usage:
    python realtime_visualization.py
    python realtime_visualization.py --device 1  # Use specific audio device
"""

import sys
import argparse
import numpy as np

try:
    import sounddevice as sd
    import matplotlib.pyplot as plt
    from matplotlib.animation import FuncAnimation
except ImportError:
    print("This example requires additional dependencies:")
    print("  pip install sounddevice matplotlib")
    sys.exit(1)

from psm_frequency import FrequencyAnalyzer


class RealtimeVisualizer:
    """Real-time audio frequency visualizer."""

    def __init__(self, sample_rate: int = 44100, fft_size: int = 4096, device: int = None):
        self.sample_rate = sample_rate
        self.fft_size = fft_size
        self.device = device

        # Audio buffer
        self.buffer = np.zeros(fft_size)

        # Frequency analyzer
        self.analyzer = FrequencyAnalyzer(sample_rate=sample_rate, fft_size=fft_size)

        # Frequency axis for plotting
        self.freqs = np.fft.rfftfreq(fft_size, 1/sample_rate)

        # Magnitude history for spectrogram
        self.history_length = 100
        self.magnitude_history = np.zeros((self.history_length, len(self.freqs)))

        # Setup plot
        self.setup_plot()

    def setup_plot(self):
        """Initialize the matplotlib figure and axes."""
        plt.style.use('dark_background')
        self.fig, self.axes = plt.subplots(3, 1, figsize=(12, 8))
        self.fig.suptitle('PSM Frequency - Real-time Audio Analysis', fontsize=14)

        # Spectrum plot
        self.ax_spectrum = self.axes[0]
        self.ax_spectrum.set_xlim(20, 20000)
        self.ax_spectrum.set_ylim(0, 1)
        self.ax_spectrum.set_xscale('log')
        self.ax_spectrum.set_xlabel('Frequency (Hz)')
        self.ax_spectrum.set_ylabel('Magnitude')
        self.ax_spectrum.set_title('Frequency Spectrum')
        self.spectrum_line, = self.ax_spectrum.plot([], [], 'c-', lw=1)
        self.peak_scatter = self.ax_spectrum.scatter([], [], c='r', s=50, zorder=5)

        # Band energies bar plot
        self.ax_bands = self.axes[1]
        self.band_names = ['Sub-bass', 'Bass', 'Low-mid', 'Mid', 'High-mid', 'Presence', 'Brilliance']
        self.band_colors = ['#ff6b6b', '#feca57', '#48dbfb', '#1dd1a1', '#5f27cd', '#ff9ff3', '#54a0ff']
        x_pos = np.arange(len(self.band_names))
        self.band_bars = self.ax_bands.bar(x_pos, np.zeros(7), color=self.band_colors)
        self.ax_bands.set_xticks(x_pos)
        self.ax_bands.set_xticklabels(self.band_names, rotation=45, ha='right')
        self.ax_bands.set_ylim(0, 1)
        self.ax_bands.set_ylabel('Energy')
        self.ax_bands.set_title('Band Energies')

        # Spectrogram
        self.ax_spectrogram = self.axes[2]
        self.spectrogram_img = self.ax_spectrogram.imshow(
            self.magnitude_history.T[:500, :],  # Show up to ~10kHz
            aspect='auto',
            origin='lower',
            cmap='magma',
            vmin=0,
            vmax=0.5,
            extent=[0, self.history_length, 0, 10000]
        )
        self.ax_spectrogram.set_xlabel('Time')
        self.ax_spectrogram.set_ylabel('Frequency (Hz)')
        self.ax_spectrogram.set_title('Spectrogram')

        plt.tight_layout()

    def audio_callback(self, indata, frames, time, status):
        """Callback for audio input stream."""
        if status:
            print(f"Audio status: {status}")

        # Update buffer with new samples
        self.buffer = np.roll(self.buffer, -frames)
        self.buffer[-frames:] = indata[:, 0]

    def update_plot(self, frame):
        """Update the visualization."""
        # Compute FFT
        windowed = self.buffer * np.hanning(len(self.buffer))
        fft = np.fft.rfft(windowed)
        magnitude = np.abs(fft) / len(self.buffer)

        # Normalize for display
        magnitude_db = 20 * np.log10(magnitude + 1e-10)
        magnitude_norm = np.clip((magnitude_db + 80) / 80, 0, 1)

        # Update spectrum line
        self.spectrum_line.set_data(self.freqs, magnitude_norm)

        # Find peaks (dominant frequencies)
        peak_indices = []
        for i in range(1, len(magnitude_norm) - 1):
            if magnitude_norm[i] > magnitude_norm[i-1] and magnitude_norm[i] > magnitude_norm[i+1]:
                if magnitude_norm[i] > 0.3:  # Threshold
                    peak_indices.append(i)

        # Keep top 5 peaks
        peak_indices = sorted(peak_indices, key=lambda i: magnitude_norm[i], reverse=True)[:5]
        if peak_indices:
            peak_freqs = self.freqs[peak_indices]
            peak_mags = magnitude_norm[peak_indices]
            self.peak_scatter.set_offsets(np.column_stack([peak_freqs, peak_mags]))
        else:
            self.peak_scatter.set_offsets(np.empty((0, 2)))

        # Compute band energies
        band_limits = [(20, 60), (60, 250), (250, 500), (500, 2000), (2000, 4000), (4000, 6000), (6000, 20000)]
        band_energies = []
        for low, high in band_limits:
            mask = (self.freqs >= low) & (self.freqs < high)
            energy = np.mean(magnitude_norm[mask]) if np.any(mask) else 0
            band_energies.append(energy)

        # Update band bars
        for bar, energy in zip(self.band_bars, band_energies):
            bar.set_height(energy)

        # Update spectrogram history
        self.magnitude_history = np.roll(self.magnitude_history, -1, axis=0)
        self.magnitude_history[-1, :] = magnitude_norm
        self.spectrogram_img.set_array(self.magnitude_history.T[:500, :])

        return [self.spectrum_line, self.peak_scatter, *self.band_bars, self.spectrogram_img]

    def run(self):
        """Start the real-time visualization."""
        print("Starting real-time audio visualization...")
        print("Press Ctrl+C to stop")

        # List available devices
        print("\nAvailable audio devices:")
        print(sd.query_devices())
        print()

        # Start audio stream
        with sd.InputStream(
            device=self.device,
            channels=1,
            samplerate=self.sample_rate,
            blocksize=1024,
            callback=self.audio_callback
        ):
            # Start animation
            ani = FuncAnimation(
                self.fig,
                self.update_plot,
                interval=50,  # 20 FPS
                blit=True,
                cache_frame_data=False
            )
            plt.show()


def main():
    parser = argparse.ArgumentParser(description="Real-time audio frequency visualization")
    parser.add_argument("--device", "-d", type=int, help="Audio input device index")
    parser.add_argument("--sample-rate", "-r", type=int, default=44100, help="Sample rate")
    parser.add_argument("--fft-size", "-f", type=int, default=4096, help="FFT size")
    args = parser.parse_args()

    visualizer = RealtimeVisualizer(
        sample_rate=args.sample_rate,
        fft_size=args.fft_size,
        device=args.device
    )

    try:
        visualizer.run()
    except KeyboardInterrupt:
        print("\nStopped")


if __name__ == "__main__":
    main()
