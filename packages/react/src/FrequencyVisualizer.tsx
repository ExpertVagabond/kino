import React, { useRef, useEffect, useCallback, useMemo } from 'react';
import type { RealtimeFrequencyData, VisualizationMode, FrequencyVisualizerProps } from './frequency-types';

/**
 * PSM brand colors
 */
const PSM_COLORS = {
  primary: '#9333EA',
  primaryDark: '#7C3AED',
  primaryDeep: '#6B21A8',
  background: '#0F0F0F',
  surface: '#1A1A1A',
  text: '#FFFFFF',
  textSoft: '#A0A0A0',
};

/**
 * Band labels for display
 */
const BAND_LABELS = ['Sub', 'Bass', 'Low', 'Mid', 'High', 'Air'];

/**
 * Create gradient for bars
 */
function createBarGradient(ctx: CanvasRenderingContext2D, height: number): CanvasGradient {
  const gradient = ctx.createLinearGradient(0, height, 0, 0);
  gradient.addColorStop(0, PSM_COLORS.primaryDeep);
  gradient.addColorStop(0.5, PSM_COLORS.primary);
  gradient.addColorStop(1, '#C084FC');
  return gradient;
}

/**
 * Draw bar visualization
 */
function drawBars(
  ctx: CanvasRenderingContext2D,
  data: RealtimeFrequencyData,
  width: number,
  height: number,
  barCount: number,
  barColor: string | CanvasGradient,
  showLabels: boolean
) {
  const spectrum = data.spectrum;
  const barWidth = width / barCount;
  const gap = 2;
  const effectiveBarWidth = barWidth - gap;
  const samplesPerBar = Math.floor(spectrum.length / barCount);

  ctx.fillStyle = barColor;

  for (let i = 0; i < barCount; i++) {
    // Average samples for this bar
    let sum = 0;
    for (let j = 0; j < samplesPerBar; j++) {
      const idx = i * samplesPerBar + j;
      if (idx < spectrum.length) {
        sum += spectrum[idx];
      }
    }
    const avg = sum / samplesPerBar;

    // Scale to height (log scale for better visualization)
    const magnitude = Math.log10(avg * 1000 + 1) / 3;
    const barHeight = Math.max(2, magnitude * height * 0.9);

    const x = i * barWidth + gap / 2;
    const y = height - barHeight;

    // Draw bar with rounded corners
    ctx.beginPath();
    const radius = Math.min(effectiveBarWidth / 2, 4);
    ctx.moveTo(x + radius, y);
    ctx.lineTo(x + effectiveBarWidth - radius, y);
    ctx.quadraticCurveTo(x + effectiveBarWidth, y, x + effectiveBarWidth, y + radius);
    ctx.lineTo(x + effectiveBarWidth, height);
    ctx.lineTo(x, height);
    ctx.lineTo(x, y + radius);
    ctx.quadraticCurveTo(x, y, x + radius, y);
    ctx.closePath();
    ctx.fill();
  }

  // Draw labels if enabled
  if (showLabels) {
    ctx.fillStyle = PSM_COLORS.textSoft;
    ctx.font = '10px system-ui, sans-serif';
    ctx.textAlign = 'center';

    // Frequency labels
    const freqLabels = ['100', '500', '1k', '5k', '10k', '20k'];
    freqLabels.forEach((label, i) => {
      const x = (i / (freqLabels.length - 1)) * width;
      ctx.fillText(label, x, height - 5);
    });
  }
}

/**
 * Draw circular visualization
 */
function drawCircular(
  ctx: CanvasRenderingContext2D,
  data: RealtimeFrequencyData,
  width: number,
  height: number,
  barColor: string | CanvasGradient
) {
  const centerX = width / 2;
  const centerY = height / 2;
  const radius = Math.min(width, height) * 0.35;
  const spectrum = data.spectrum;
  const bars = 64;
  const angleStep = (Math.PI * 2) / bars;
  const samplesPerBar = Math.floor(spectrum.length / bars);

  ctx.strokeStyle = barColor;
  ctx.lineWidth = 3;
  ctx.lineCap = 'round';

  for (let i = 0; i < bars; i++) {
    // Average samples
    let sum = 0;
    for (let j = 0; j < samplesPerBar; j++) {
      const idx = i * samplesPerBar + j;
      if (idx < spectrum.length) {
        sum += spectrum[idx];
      }
    }
    const avg = sum / samplesPerBar;
    const magnitude = Math.log10(avg * 1000 + 1) / 3;
    const barLength = radius * 0.5 * magnitude;

    const angle = i * angleStep - Math.PI / 2;
    const x1 = centerX + Math.cos(angle) * radius;
    const y1 = centerY + Math.sin(angle) * radius;
    const x2 = centerX + Math.cos(angle) * (radius + barLength);
    const y2 = centerY + Math.sin(angle) * (radius + barLength);

    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
  }

  // Draw center info
  ctx.fillStyle = PSM_COLORS.text;
  ctx.font = 'bold 16px system-ui, sans-serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(`${Math.round(data.dominantFrequency)} Hz`, centerX, centerY);
}

/**
 * Draw waveform visualization
 */
function drawWaveform(
  ctx: CanvasRenderingContext2D,
  data: RealtimeFrequencyData,
  width: number,
  height: number,
  barColor: string | CanvasGradient
) {
  const spectrum = data.spectrum;
  const midY = height / 2;

  ctx.strokeStyle = barColor;
  ctx.lineWidth = 2;
  ctx.beginPath();

  for (let i = 0; i < spectrum.length; i++) {
    const x = (i / spectrum.length) * width;
    const magnitude = Math.log10(spectrum[i] * 1000 + 1) / 3;
    const y = midY - magnitude * height * 0.4;

    if (i === 0) {
      ctx.moveTo(x, y);
    } else {
      ctx.lineTo(x, y);
    }
  }

  ctx.stroke();

  // Mirror
  ctx.beginPath();
  for (let i = 0; i < spectrum.length; i++) {
    const x = (i / spectrum.length) * width;
    const magnitude = Math.log10(spectrum[i] * 1000 + 1) / 3;
    const y = midY + magnitude * height * 0.4;

    if (i === 0) {
      ctx.moveTo(x, y);
    } else {
      ctx.lineTo(x, y);
    }
  }
  ctx.stroke();
}

/**
 * Draw band energy visualization
 */
function drawBands(
  ctx: CanvasRenderingContext2D,
  data: RealtimeFrequencyData,
  width: number,
  height: number,
  barColor: string | CanvasGradient,
  showLabels: boolean
) {
  const bands = data.bandEnergies;
  const barWidth = width / 6;
  const gap = 8;
  const effectiveBarWidth = barWidth - gap;
  const labelHeight = showLabels ? 25 : 0;
  const availableHeight = height - labelHeight;

  ctx.fillStyle = barColor;

  for (let i = 0; i < 6; i++) {
    const energy = bands[i];
    const barHeight = Math.max(4, energy * availableHeight * 0.9);

    const x = i * barWidth + gap / 2;
    const y = availableHeight - barHeight;

    // Draw bar with rounded corners
    ctx.beginPath();
    const radius = Math.min(effectiveBarWidth / 2, 8);
    ctx.moveTo(x + radius, y);
    ctx.lineTo(x + effectiveBarWidth - radius, y);
    ctx.quadraticCurveTo(x + effectiveBarWidth, y, x + effectiveBarWidth, y + radius);
    ctx.lineTo(x + effectiveBarWidth, availableHeight);
    ctx.lineTo(x, availableHeight);
    ctx.lineTo(x, y + radius);
    ctx.quadraticCurveTo(x, y, x + radius, y);
    ctx.closePath();
    ctx.fill();

    // Draw label
    if (showLabels) {
      ctx.fillStyle = PSM_COLORS.textSoft;
      ctx.font = '11px system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(BAND_LABELS[i], x + effectiveBarWidth / 2, height - 8);
      ctx.fillStyle = barColor;
    }
  }
}

/**
 * FrequencyVisualizer Component
 *
 * Renders real-time frequency analysis data using canvas.
 */
export function FrequencyVisualizer({
  data,
  mode = 'bars',
  width = 400,
  height = 200,
  barColor,
  backgroundColor = PSM_COLORS.background,
  barCount = 64,
  showLabels = true,
  className,
  style,
}: FrequencyVisualizerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number | null>(null);

  // Memoize effective bar color
  const effectiveBarColor = useMemo(() => {
    if (barColor) return barColor;
    // Will be set in render
    return PSM_COLORS.primary;
  }, [barColor]);

  // Render function
  const render = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear canvas
    ctx.fillStyle = backgroundColor;
    ctx.fillRect(0, 0, width, height);

    if (!data) {
      // Draw placeholder
      ctx.fillStyle = PSM_COLORS.textSoft;
      ctx.font = '14px system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText('No audio data', width / 2, height / 2);
      return;
    }

    // Create gradient if no custom color
    const color = barColor || createBarGradient(ctx, height);

    // Draw based on mode
    switch (mode) {
      case 'bars':
        drawBars(ctx, data, width, height, barCount, color, showLabels);
        break;
      case 'circular':
        drawCircular(ctx, data, width, height, color);
        break;
      case 'waveform':
        drawWaveform(ctx, data, width, height, color);
        break;
      case 'bands':
        drawBands(ctx, data, width, height, color, showLabels);
        break;
      default:
        drawBars(ctx, data, width, height, barCount, color, showLabels);
    }

    // Draw info overlay
    if (showLabels && data && mode !== 'circular') {
      ctx.fillStyle = PSM_COLORS.text;
      ctx.font = '12px system-ui, sans-serif';
      ctx.textAlign = 'left';
      ctx.fillText(`${Math.round(data.dominantFrequency)} Hz`, 10, 20);

      ctx.textAlign = 'right';
      ctx.fillText(`Centroid: ${Math.round(data.spectralCentroid)} Hz`, width - 10, 20);
    }
  }, [data, mode, width, height, barColor, backgroundColor, barCount, showLabels]);

  // Render on data change
  useEffect(() => {
    render();
  }, [render]);

  // Handle resize
  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas) {
      canvas.width = width;
      canvas.height = height;
      render();
    }
  }, [width, height, render]);

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      className={className}
      style={{
        display: 'block',
        borderRadius: '8px',
        ...style,
      }}
    />
  );
}

/**
 * Compact band meter component
 */
export function BandMeter({
  data,
  width = 200,
  height = 40,
  className,
  style,
}: {
  data: RealtimeFrequencyData | null;
  width?: number;
  height?: number;
  className?: string;
  style?: React.CSSProperties;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    ctx.fillStyle = PSM_COLORS.surface;
    ctx.fillRect(0, 0, width, height);

    if (!data) return;

    const bands = data.bandEnergies;
    const barWidth = width / 6;
    const gap = 2;

    const gradient = createBarGradient(ctx, height);
    ctx.fillStyle = gradient;

    for (let i = 0; i < 6; i++) {
      const energy = bands[i];
      const barHeight = Math.max(2, energy * height * 0.9);
      const x = i * barWidth + gap / 2;
      const y = height - barHeight;

      ctx.fillRect(x, y, barWidth - gap, barHeight);
    }
  }, [data, width, height]);

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      className={className}
      style={{
        display: 'block',
        borderRadius: '4px',
        ...style,
      }}
    />
  );
}

/**
 * Dominant frequency display
 */
export function DominantFrequencyDisplay({
  data,
  className,
  style,
}: {
  data: RealtimeFrequencyData | null;
  className?: string;
  style?: React.CSSProperties;
}) {
  const frequency = data?.dominantFrequency ?? 0;
  const note = frequencyToNote(frequency);

  return (
    <div
      className={className}
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        padding: '16px',
        backgroundColor: PSM_COLORS.surface,
        borderRadius: '8px',
        color: PSM_COLORS.text,
        fontFamily: 'system-ui, sans-serif',
        ...style,
      }}
    >
      <div style={{ fontSize: '32px', fontWeight: 'bold', color: PSM_COLORS.primary }}>
        {Math.round(frequency)} <span style={{ fontSize: '16px' }}>Hz</span>
      </div>
      {note && (
        <div style={{ fontSize: '14px', color: PSM_COLORS.textSoft, marginTop: '4px' }}>
          {note}
        </div>
      )}
    </div>
  );
}

/**
 * Convert frequency to musical note
 */
function frequencyToNote(frequency: number): string | null {
  if (frequency < 20 || frequency > 20000) return null;

  const notes = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
  const a4 = 440;
  const c0 = a4 * Math.pow(2, -4.75);

  const halfSteps = Math.round(12 * Math.log2(frequency / c0));
  const octave = Math.floor(halfSteps / 12);
  const noteIndex = halfSteps % 12;

  if (noteIndex < 0 || noteIndex >= notes.length) return null;

  return `${notes[noteIndex]}${octave}`;
}

export default FrequencyVisualizer;
