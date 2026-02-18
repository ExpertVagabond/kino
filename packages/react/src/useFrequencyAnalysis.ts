import { useState, useEffect, useCallback, useRef } from 'react';
import type {
  RealtimeFrequencyData,
  FrequencyAnalysisResult,
  BandEnergies,
  DominantFrequency,
  SpectralFeatures,
  UseFrequencyAnalysisOptions,
  UseFrequencyAnalysisResult,
} from './frequency-types';

/**
 * Default FFT size for analysis
 */
const DEFAULT_FFT_SIZE = 2048;

/**
 * Default sample rate
 */
const DEFAULT_SAMPLE_RATE = 44100;

/**
 * Frequency band definitions (Hz)
 */
const BANDS = [
  { name: 'subBass', low: 20, high: 60 },
  { name: 'bass', low: 60, high: 250 },
  { name: 'lowMid', low: 250, high: 500 },
  { name: 'mid', low: 500, high: 2000 },
  { name: 'highMid', low: 2000, high: 4000 },
  { name: 'high', low: 4000, high: 20000 },
] as const;

/**
 * Simple FFT implementation for client-side analysis
 */
function computeSpectrum(samples: Float32Array, fftSize: number): Float32Array {
  const n = Math.min(fftSize, samples.length);
  const spectrum = new Float32Array(n / 2);

  // Apply Hann window
  const windowed = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const window = 0.5 * (1 - Math.cos((2 * Math.PI * i) / (n - 1)));
    windowed[i] = samples[i] * window;
  }

  // DFT (simplified - in production use Web Audio API's AnalyserNode)
  for (let k = 0; k < n / 2; k++) {
    let real = 0;
    let imag = 0;

    for (let i = 0; i < n; i++) {
      const angle = (2 * Math.PI * k * i) / n;
      real += windowed[i] * Math.cos(angle);
      imag -= windowed[i] * Math.sin(angle);
    }

    spectrum[k] = Math.sqrt(real * real + imag * imag) * (2 / n);
  }

  return spectrum;
}

/**
 * Compute spectral centroid (center of mass of spectrum)
 */
function computeCentroid(spectrum: Float32Array, sampleRate: number, fftSize: number): number {
  const freqResolution = sampleRate / fftSize;
  let weightedSum = 0;
  let totalMagnitude = 0;

  for (let i = 0; i < spectrum.length; i++) {
    const freq = i * freqResolution;
    weightedSum += spectrum[i] * freq;
    totalMagnitude += spectrum[i];
  }

  return totalMagnitude > 0 ? weightedSum / totalMagnitude : 0;
}

/**
 * Compute spectral rolloff (frequency below which X% of energy is contained)
 */
function computeRolloff(spectrum: Float32Array, sampleRate: number, fftSize: number, threshold = 0.95): number {
  const freqResolution = sampleRate / fftSize;
  const totalEnergy = spectrum.reduce((sum, val) => sum + val, 0);
  const target = totalEnergy * threshold;
  let cumulative = 0;

  for (let i = 0; i < spectrum.length; i++) {
    cumulative += spectrum[i];
    if (cumulative >= target) {
      return i * freqResolution;
    }
  }

  return (spectrum.length - 1) * freqResolution;
}

/**
 * Compute spectral flatness (0 = tonal, 1 = noisy)
 */
function computeFlatness(spectrum: Float32Array): number {
  const n = spectrum.length;
  let logSum = 0;
  let arithmeticSum = 0;

  for (let i = 0; i < n; i++) {
    logSum += Math.log(spectrum[i] + 1e-10);
    arithmeticSum += spectrum[i];
  }

  const geometricMean = Math.exp(logSum / n);
  const arithmeticMean = arithmeticSum / n;

  return arithmeticMean > 0 ? geometricMean / arithmeticMean : 0;
}

/**
 * Compute zero-crossing rate
 */
function computeZCR(samples: Float32Array): number {
  let crossings = 0;
  for (let i = 1; i < samples.length; i++) {
    if ((samples[i] >= 0) !== (samples[i - 1] >= 0)) {
      crossings++;
    }
  }
  return crossings / samples.length;
}

/**
 * Compute band energies
 */
function computeBandEnergies(
  spectrum: Float32Array,
  sampleRate: number,
  fftSize: number
): BandEnergies {
  const freqResolution = sampleRate / fftSize;
  const energies: Record<string, number> = {};
  let totalEnergy = 0;

  for (const band of BANDS) {
    let bandEnergy = 0;
    for (let i = 0; i < spectrum.length; i++) {
      const freq = i * freqResolution;
      if (freq >= band.low && freq < band.high) {
        bandEnergy += spectrum[i];
      }
    }
    energies[band.name] = bandEnergy;
    totalEnergy += bandEnergy;
  }

  // Normalize
  if (totalEnergy > 0) {
    for (const band of BANDS) {
      energies[band.name] /= totalEnergy;
    }
  }

  return energies as unknown as BandEnergies;
}

/**
 * Find dominant frequencies
 */
function findDominantFrequencies(
  spectrum: Float32Array,
  sampleRate: number,
  fftSize: number,
  topK: number = 10
): DominantFrequency[] {
  const freqResolution = sampleRate / fftSize;
  const indexed: Array<{ index: number; magnitude: number }> = [];

  for (let i = 0; i < spectrum.length; i++) {
    indexed.push({ index: i, magnitude: spectrum[i] });
  }

  indexed.sort((a, b) => b.magnitude - a.magnitude);

  const maxMag = indexed[0]?.magnitude || 1;

  return indexed.slice(0, topK).map((item, rank) => ({
    frequencyHz: item.index * freqResolution,
    magnitude: item.magnitude / maxMag,
    rank: rank + 1,
  }));
}

/**
 * Hook for real-time frequency analysis using Web Audio API
 */
export function useFrequencyAnalysis(
  options: UseFrequencyAnalysisOptions = {}
): UseFrequencyAnalysisResult {
  const {
    fftSize = DEFAULT_FFT_SIZE,
    sampleRate = DEFAULT_SAMPLE_RATE,
    streaming = false,
    autoStart = false,
  } = options;

  const [frequencyData, setFrequencyData] = useState<RealtimeFrequencyData | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const analyserRef = useRef<AnalyserNode | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);

  /**
   * Analyze a buffer of samples (non-streaming)
   */
  const analyze = useCallback(
    (samples: Float32Array): FrequencyAnalysisResult => {
      const startTime = performance.now();

      const spectrum = computeSpectrum(samples, fftSize);
      const centroid = computeCentroid(spectrum, sampleRate, fftSize);
      const rolloff = computeRolloff(spectrum, sampleRate, fftSize);
      const flatness = computeFlatness(spectrum);
      const zcr = computeZCR(samples);
      const bandEnergies = computeBandEnergies(spectrum, sampleRate, fftSize);
      const dominantFrequencies = findDominantFrequencies(spectrum, sampleRate, fftSize);

      const processingTimeMs = performance.now() - startTime;

      return {
        dominantFrequencies,
        spectralFeatures: {
          centroid,
          rolloff,
          flatness,
          zcr,
          bandEnergies,
        },
        processingTimeMs,
      };
    },
    [fftSize, sampleRate]
  );

  /**
   * Process audio data from AnalyserNode
   */
  const processAudioData = useCallback(() => {
    if (!analyserRef.current) return;

    const analyser = analyserRef.current;
    const bufferLength = analyser.frequencyBinCount;
    const dataArray = new Float32Array(bufferLength);

    // Get frequency data
    analyser.getFloatFrequencyData(dataArray);

    // Convert from dB to linear magnitude
    const spectrum = new Float32Array(bufferLength);
    for (let i = 0; i < bufferLength; i++) {
      spectrum[i] = Math.pow(10, dataArray[i] / 20);
    }

    // Compute features
    const effectiveSampleRate = audioContextRef.current?.sampleRate || sampleRate;
    const effectiveFftSize = analyser.fftSize;

    const centroid = computeCentroid(spectrum, effectiveSampleRate, effectiveFftSize);
    const bandEnergies = computeBandEnergies(spectrum, effectiveSampleRate, effectiveFftSize);

    // Find dominant frequency
    let maxIdx = 0;
    let maxVal = 0;
    for (let i = 0; i < spectrum.length; i++) {
      if (spectrum[i] > maxVal) {
        maxVal = spectrum[i];
        maxIdx = i;
      }
    }
    const dominantFreq = (maxIdx * effectiveSampleRate) / effectiveFftSize;

    const data: RealtimeFrequencyData = {
      spectrum,
      bandEnergies: [
        bandEnergies.subBass,
        bandEnergies.bass,
        bandEnergies.lowMid,
        bandEnergies.mid,
        bandEnergies.highMid,
        bandEnergies.high,
      ],
      dominantFrequency: dominantFreq,
      spectralCentroid: centroid,
    };

    setFrequencyData(data);

    // Continue animation loop
    animationFrameRef.current = requestAnimationFrame(processAudioData);
  }, [sampleRate]);

  /**
   * Start streaming analysis from an audio source
   */
  const startStreaming = useCallback(
    (source: AudioNode, audioContext: AudioContext) => {
      try {
        // Stop any existing streaming
        if (animationFrameRef.current) {
          cancelAnimationFrame(animationFrameRef.current);
        }

        // Create analyser node
        const analyser = audioContext.createAnalyser();
        analyser.fftSize = fftSize;
        analyser.smoothingTimeConstant = 0.8;

        // Connect source to analyser
        source.connect(analyser);

        analyserRef.current = analyser;
        audioContextRef.current = audioContext;

        // Start processing loop
        setIsStreaming(true);
        animationFrameRef.current = requestAnimationFrame(processAudioData);
      } catch (err) {
        setError(err instanceof Error ? err : new Error('Failed to start streaming'));
      }
    },
    [fftSize, processAudioData]
  );

  /**
   * Stop streaming analysis
   */
  const stopStreaming = useCallback(() => {
    if (animationFrameRef.current) {
      cancelAnimationFrame(animationFrameRef.current);
      animationFrameRef.current = null;
    }

    if (analyserRef.current) {
      analyserRef.current.disconnect();
      analyserRef.current = null;
    }

    setIsStreaming(false);
    setFrequencyData(null);
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, []);

  return {
    frequencyData,
    analyze,
    startStreaming,
    stopStreaming,
    isStreaming,
    error,
  };
}

/**
 * Hook for microphone input frequency analysis
 */
export function useMicrophoneAnalysis(options: UseFrequencyAnalysisOptions = {}) {
  const analysis = useFrequencyAnalysis(options);
  const [isActive, setIsActive] = useState(false);
  const streamRef = useRef<MediaStream | null>(null);
  const sourceRef = useRef<MediaStreamAudioSourceNode | null>(null);
  const contextRef = useRef<AudioContext | null>(null);

  const start = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const audioContext = new AudioContext();
      const source = audioContext.createMediaStreamSource(stream);

      streamRef.current = stream;
      sourceRef.current = source;
      contextRef.current = audioContext;

      analysis.startStreaming(source, audioContext);
      setIsActive(true);
    } catch (err) {
      console.error('Failed to access microphone:', err);
    }
  }, [analysis]);

  const stop = useCallback(() => {
    analysis.stopStreaming();

    if (streamRef.current) {
      streamRef.current.getTracks().forEach((track) => track.stop());
      streamRef.current = null;
    }

    if (contextRef.current) {
      contextRef.current.close();
      contextRef.current = null;
    }

    setIsActive(false);
  }, [analysis]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((track) => track.stop());
      }
      if (contextRef.current) {
        contextRef.current.close();
      }
    };
  }, []);

  return {
    ...analysis,
    isActive,
    start,
    stop,
  };
}

/**
 * Hook for analyzing audio from HTMLMediaElement (video/audio)
 */
export function useMediaElementAnalysis(
  mediaElement: HTMLMediaElement | null,
  options: UseFrequencyAnalysisOptions = {}
) {
  const analysis = useFrequencyAnalysis(options);
  const sourceRef = useRef<MediaElementAudioSourceNode | null>(null);
  const contextRef = useRef<AudioContext | null>(null);
  const [isConnected, setIsConnected] = useState(false);

  const connect = useCallback(() => {
    if (!mediaElement || isConnected) return;

    try {
      const audioContext = new AudioContext();
      const source = audioContext.createMediaElementSource(mediaElement);

      // Connect to destination so audio still plays
      source.connect(audioContext.destination);

      sourceRef.current = source;
      contextRef.current = audioContext;

      analysis.startStreaming(source, audioContext);
      setIsConnected(true);
    } catch (err) {
      console.error('Failed to connect to media element:', err);
    }
  }, [mediaElement, isConnected, analysis]);

  const disconnect = useCallback(() => {
    analysis.stopStreaming();

    if (sourceRef.current) {
      sourceRef.current.disconnect();
      sourceRef.current = null;
    }

    if (contextRef.current) {
      contextRef.current.close();
      contextRef.current = null;
    }

    setIsConnected(false);
  }, [analysis]);

  // Auto-connect when media element is available
  useEffect(() => {
    if (mediaElement && options.autoStart) {
      connect();
    }

    return () => {
      if (isConnected) {
        disconnect();
      }
    };
  }, [mediaElement, options.autoStart]);

  return {
    ...analysis,
    isConnected,
    connect,
    disconnect,
  };
}

export default useFrequencyAnalysis;
