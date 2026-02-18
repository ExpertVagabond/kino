/**
 * Kino Frequency Analysis Demo
 *
 * This example demonstrates how to use the frequency analysis
 * hooks and visualizer components in a React application.
 *
 * Features:
 * - Microphone input analysis
 * - Video/audio element analysis
 * - Multiple visualization modes
 * - Real-time spectral features display
 */

import React, { useState, useRef, useCallback } from 'react';
import {
  useFrequencyAnalysis,
  useMicrophoneAnalysis,
  useMediaElementAnalysis,
  FrequencyVisualizer,
  BandMeter,
  DominantFrequencyDisplay,
} from '@purplesquirrel/player-react';
import type { VisualizationMode } from '@purplesquirrel/player-react';

/**
 * Main demo component
 */
export function FrequencyAnalysisDemo() {
  const [activeTab, setActiveTab] = useState<'microphone' | 'file' | 'url'>('microphone');

  return (
    <div style={styles.container}>
      <header style={styles.header}>
        <h1 style={styles.title}>PSM Frequency Analysis</h1>
        <p style={styles.subtitle}>Real-time audio frequency analysis and visualization</p>
      </header>

      <nav style={styles.tabs}>
        <TabButton
          active={activeTab === 'microphone'}
          onClick={() => setActiveTab('microphone')}
        >
          Microphone
        </TabButton>
        <TabButton
          active={activeTab === 'file'}
          onClick={() => setActiveTab('file')}
        >
          Audio File
        </TabButton>
        <TabButton
          active={activeTab === 'url'}
          onClick={() => setActiveTab('url')}
        >
          Stream URL
        </TabButton>
      </nav>

      <main style={styles.main}>
        {activeTab === 'microphone' && <MicrophoneDemo />}
        {activeTab === 'file' && <FileDemo />}
        {activeTab === 'url' && <StreamDemo />}
      </main>
    </div>
  );
}

/**
 * Microphone input demo
 */
function MicrophoneDemo() {
  const {
    frequencyData,
    isActive,
    start,
    stop,
    error,
  } = useMicrophoneAnalysis({ fftSize: 2048 });

  const [visualMode, setVisualMode] = useState<VisualizationMode>('bars');

  return (
    <div style={styles.demoContainer}>
      <div style={styles.controls}>
        <button
          style={isActive ? styles.buttonActive : styles.button}
          onClick={isActive ? stop : start}
        >
          {isActive ? 'Stop' : 'Start'} Microphone
        </button>

        <select
          style={styles.select}
          value={visualMode}
          onChange={(e) => setVisualMode(e.target.value as VisualizationMode)}
        >
          <option value="bars">Bars</option>
          <option value="circular">Circular</option>
          <option value="waveform">Waveform</option>
          <option value="bands">Band Meter</option>
        </select>
      </div>

      {error && (
        <div style={styles.error}>
          Error: {error.message}. Make sure you've granted microphone permission.
        </div>
      )}

      <div style={styles.visualizerContainer}>
        <FrequencyVisualizer
          data={frequencyData}
          mode={visualMode}
          width={600}
          height={300}
          showLabels={true}
        />
      </div>

      <div style={styles.metersRow}>
        <div style={styles.meterCard}>
          <h3 style={styles.meterTitle}>Band Levels</h3>
          <BandMeter data={frequencyData} width={200} height={60} />
        </div>

        <DominantFrequencyDisplay data={frequencyData} />

        <div style={styles.meterCard}>
          <h3 style={styles.meterTitle}>Spectral Features</h3>
          {frequencyData ? (
            <div style={styles.featureList}>
              <div>Centroid: {Math.round(frequencyData.spectralCentroid)} Hz</div>
              <div>Dominant: {Math.round(frequencyData.dominantFrequency)} Hz</div>
            </div>
          ) : (
            <div style={styles.noData}>No data</div>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * Audio file demo
 */
function FileDemo() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [visualMode, setVisualMode] = useState<VisualizationMode>('bars');

  const {
    frequencyData,
    isConnected,
    connect,
    disconnect,
  } = useMediaElementAnalysis(audioRef.current, { fftSize: 2048 });

  const handleFileChange = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      const url = URL.createObjectURL(file);
      setAudioUrl(url);
    }
  }, []);

  const handlePlay = useCallback(() => {
    if (!isConnected && audioRef.current) {
      connect();
    }
  }, [isConnected, connect]);

  return (
    <div style={styles.demoContainer}>
      <div style={styles.controls}>
        <input
          type="file"
          accept="audio/*"
          onChange={handleFileChange}
          style={styles.fileInput}
        />

        <select
          style={styles.select}
          value={visualMode}
          onChange={(e) => setVisualMode(e.target.value as VisualizationMode)}
        >
          <option value="bars">Bars</option>
          <option value="circular">Circular</option>
          <option value="waveform">Waveform</option>
          <option value="bands">Band Meter</option>
        </select>
      </div>

      {audioUrl && (
        <audio
          ref={audioRef}
          src={audioUrl}
          controls
          onPlay={handlePlay}
          style={styles.audioPlayer}
        />
      )}

      <div style={styles.visualizerContainer}>
        <FrequencyVisualizer
          data={frequencyData}
          mode={visualMode}
          width={600}
          height={300}
          showLabels={true}
        />
      </div>

      <div style={styles.metersRow}>
        <div style={styles.meterCard}>
          <h3 style={styles.meterTitle}>Band Levels</h3>
          <BandMeter data={frequencyData} width={200} height={60} />
        </div>

        <DominantFrequencyDisplay data={frequencyData} />
      </div>
    </div>
  );
}

/**
 * Stream URL demo
 */
function StreamDemo() {
  const audioRef = useRef<HTMLAudioElement>(null);
  const [url, setUrl] = useState('');
  const [isPlaying, setIsPlaying] = useState(false);
  const [visualMode, setVisualMode] = useState<VisualizationMode>('bars');

  const {
    frequencyData,
    isConnected,
    connect,
  } = useMediaElementAnalysis(audioRef.current, { fftSize: 2048 });

  const handlePlay = useCallback(() => {
    if (audioRef.current && url) {
      audioRef.current.src = url;
      audioRef.current.play();
      setIsPlaying(true);

      if (!isConnected) {
        connect();
      }
    }
  }, [url, isConnected, connect]);

  const handleStop = useCallback(() => {
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.currentTime = 0;
      setIsPlaying(false);
    }
  }, []);

  return (
    <div style={styles.demoContainer}>
      <div style={styles.controls}>
        <input
          type="text"
          placeholder="Enter audio stream URL..."
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          style={styles.urlInput}
        />

        <button
          style={isPlaying ? styles.buttonActive : styles.button}
          onClick={isPlaying ? handleStop : handlePlay}
          disabled={!url}
        >
          {isPlaying ? 'Stop' : 'Play'}
        </button>

        <select
          style={styles.select}
          value={visualMode}
          onChange={(e) => setVisualMode(e.target.value as VisualizationMode)}
        >
          <option value="bars">Bars</option>
          <option value="circular">Circular</option>
          <option value="waveform">Waveform</option>
          <option value="bands">Band Meter</option>
        </select>
      </div>

      <audio ref={audioRef} style={{ display: 'none' }} crossOrigin="anonymous" />

      <div style={styles.visualizerContainer}>
        <FrequencyVisualizer
          data={frequencyData}
          mode={visualMode}
          width={600}
          height={300}
          showLabels={true}
        />
      </div>

      <div style={styles.metersRow}>
        <div style={styles.meterCard}>
          <h3 style={styles.meterTitle}>Band Levels</h3>
          <BandMeter data={frequencyData} width={200} height={60} />
        </div>

        <DominantFrequencyDisplay data={frequencyData} />
      </div>

      <div style={styles.hint}>
        Try: https://streams.example.com/audio.mp3
      </div>
    </div>
  );
}

/**
 * Tab button component
 */
function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      style={active ? styles.tabActive : styles.tab}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

/**
 * Styles
 */
const styles: Record<string, React.CSSProperties> = {
  container: {
    minHeight: '100vh',
    backgroundColor: '#0F0F0F',
    color: '#FFFFFF',
    fontFamily: 'system-ui, -apple-system, sans-serif',
    padding: '24px',
  },
  header: {
    textAlign: 'center',
    marginBottom: '32px',
  },
  title: {
    fontSize: '32px',
    fontWeight: 'bold',
    color: '#9333EA',
    margin: '0 0 8px 0',
  },
  subtitle: {
    fontSize: '16px',
    color: '#A0A0A0',
    margin: 0,
  },
  tabs: {
    display: 'flex',
    justifyContent: 'center',
    gap: '8px',
    marginBottom: '24px',
  },
  tab: {
    padding: '12px 24px',
    backgroundColor: '#1A1A1A',
    border: 'none',
    borderRadius: '8px',
    color: '#A0A0A0',
    fontSize: '14px',
    cursor: 'pointer',
    transition: 'all 0.2s',
  },
  tabActive: {
    padding: '12px 24px',
    backgroundColor: '#9333EA',
    border: 'none',
    borderRadius: '8px',
    color: '#FFFFFF',
    fontSize: '14px',
    cursor: 'pointer',
  },
  main: {
    maxWidth: '800px',
    margin: '0 auto',
  },
  demoContainer: {
    backgroundColor: '#1A1A1A',
    borderRadius: '12px',
    padding: '24px',
  },
  controls: {
    display: 'flex',
    gap: '12px',
    marginBottom: '24px',
    flexWrap: 'wrap',
    alignItems: 'center',
  },
  button: {
    padding: '12px 24px',
    backgroundColor: '#9333EA',
    border: 'none',
    borderRadius: '8px',
    color: '#FFFFFF',
    fontSize: '14px',
    fontWeight: 'bold',
    cursor: 'pointer',
    transition: 'all 0.2s',
  },
  buttonActive: {
    padding: '12px 24px',
    backgroundColor: '#DC2626',
    border: 'none',
    borderRadius: '8px',
    color: '#FFFFFF',
    fontSize: '14px',
    fontWeight: 'bold',
    cursor: 'pointer',
  },
  select: {
    padding: '12px 16px',
    backgroundColor: '#2A2A2A',
    border: '1px solid #3A3A3A',
    borderRadius: '8px',
    color: '#FFFFFF',
    fontSize: '14px',
    cursor: 'pointer',
  },
  fileInput: {
    padding: '8px',
    backgroundColor: '#2A2A2A',
    border: '1px solid #3A3A3A',
    borderRadius: '8px',
    color: '#FFFFFF',
    fontSize: '14px',
  },
  urlInput: {
    flex: 1,
    padding: '12px 16px',
    backgroundColor: '#2A2A2A',
    border: '1px solid #3A3A3A',
    borderRadius: '8px',
    color: '#FFFFFF',
    fontSize: '14px',
    minWidth: '200px',
  },
  audioPlayer: {
    width: '100%',
    marginBottom: '24px',
  },
  visualizerContainer: {
    display: 'flex',
    justifyContent: 'center',
    marginBottom: '24px',
  },
  metersRow: {
    display: 'flex',
    gap: '16px',
    justifyContent: 'center',
    flexWrap: 'wrap',
  },
  meterCard: {
    backgroundColor: '#2A2A2A',
    borderRadius: '8px',
    padding: '16px',
    textAlign: 'center',
  },
  meterTitle: {
    fontSize: '12px',
    color: '#A0A0A0',
    margin: '0 0 12px 0',
    textTransform: 'uppercase',
    letterSpacing: '0.5px',
  },
  featureList: {
    fontSize: '14px',
    color: '#FFFFFF',
    lineHeight: 1.6,
  },
  noData: {
    color: '#666666',
    fontSize: '14px',
  },
  error: {
    backgroundColor: '#7F1D1D',
    color: '#FCA5A5',
    padding: '12px 16px',
    borderRadius: '8px',
    marginBottom: '16px',
    fontSize: '14px',
  },
  hint: {
    textAlign: 'center',
    color: '#666666',
    fontSize: '12px',
    marginTop: '16px',
  },
};

export default FrequencyAnalysisDemo;
