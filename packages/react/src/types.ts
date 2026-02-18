import { RefObject } from 'react';

export interface QualityLevel {
  index: number;
  height: number;
  width: number;
  bitrate: number;
  label: string;
}

export interface SubtitleTrack {
  index: number;
  id: number;
  name: string;
  lang: string;
  default: boolean;
}

export interface QoeMetrics {
  score: number;
  startupTime: number;
  rebufferCount: number;
  rebufferDuration: number;
  qualitySwitches: number;
  avgBitrate: number;
  watchTime: number;
}

export interface PlayerState {
  isPlaying: boolean;
  isPaused: boolean;
  isBuffering: boolean;
  isSeeking: boolean;
  isEnded: boolean;
  currentTime: number;
  duration: number;
  buffered: number;
  volume: number;
  muted: boolean;
  playbackRate: number;
  currentQuality: number;
  autoQuality: boolean;
  qualities: QualityLevel[];
  subtitleTracks: SubtitleTrack[];
  currentSubtitle: number;
  isFullscreen: boolean;
  isPiP: boolean;
}

export interface PlayerEvents {
  onReady?: () => void;
  onPlay?: () => void;
  onPause?: () => void;
  onEnded?: () => void;
  onError?: (error: Error) => void;
  onTimeUpdate?: (time: number) => void;
  onDurationChange?: (duration: number) => void;
  onBufferUpdate?: (buffered: number) => void;
  onQualityChange?: (quality: QualityLevel) => void;
  onRebuffer?: () => void;
  onSeek?: (from: number, to: number) => void;
  onVolumeChange?: (volume: number, muted: boolean) => void;
  onFullscreenChange?: (isFullscreen: boolean) => void;
  onPiPChange?: (isPiP: boolean) => void;
  onSubtitleChange?: (track: SubtitleTrack | null) => void;
}

export interface KinoPlayerProps extends PlayerEvents {
  src: string;
  autoPlay?: boolean;
  muted?: boolean;
  loop?: boolean;
  controls?: boolean;
  poster?: string;
  width?: number | string;
  height?: number | string;
  className?: string;
  style?: React.CSSProperties;
  startTime?: number;
  maxBitrate?: number;
  preferredQuality?: number;
  subtitlesEnabled?: boolean;
  preferredSubtitleLang?: string;
  keyboardShortcuts?: boolean;
  abrAlgorithm?: 'throughput' | 'bola' | 'hybrid';
}

export interface KinoPlayerRef {
  play: () => Promise<void>;
  pause: () => void;
  seek: (time: number) => void;
  setVolume: (volume: number) => void;
  setMuted: (muted: boolean) => void;
  setPlaybackRate: (rate: number) => void;
  setQuality: (index: number) => void;
  setAutoQuality: (auto: boolean) => void;
  setSubtitleTrack: (index: number) => void;
  toggleFullscreen: () => void;
  togglePiP: () => Promise<void>;
  getState: () => PlayerState;
  getQoeMetrics: () => QoeMetrics;
  getVideoElement: () => HTMLVideoElement | null;
}
