import { useRef, useCallback, useState, useEffect } from 'react';
import type { KinoPlayerRef, PlayerState, QoeMetrics } from './types';

interface UseKinoPlayerOptions {
  autoPlay?: boolean;
}

interface UseKinoPlayerReturn {
  ref: React.RefObject<KinoPlayerRef>;
  state: PlayerState | null;
  qoe: QoeMetrics | null;
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
  skipForward: (seconds?: number) => void;
  skipBackward: (seconds?: number) => void;
  toggleMute: () => void;
  togglePlay: () => void;
}

export function useKinoPlayer(options: UseKinoPlayerOptions = {}): UseKinoPlayerReturn {
  const ref = useRef<KinoPlayerRef>(null);
  const [state, setState] = useState<PlayerState | null>(null);
  const [qoe, setQoe] = useState<QoeMetrics | null>(null);

  // Poll for state updates
  useEffect(() => {
    const interval = setInterval(() => {
      if (ref.current) {
        setState(ref.current.getState());
        setQoe(ref.current.getQoeMetrics());
      }
    }, 250);

    return () => clearInterval(interval);
  }, []);

  const play = useCallback(async () => {
    await ref.current?.play();
  }, []);

  const pause = useCallback(() => {
    ref.current?.pause();
  }, []);

  const seek = useCallback((time: number) => {
    ref.current?.seek(time);
  }, []);

  const setVolume = useCallback((volume: number) => {
    ref.current?.setVolume(volume);
  }, []);

  const setMuted = useCallback((muted: boolean) => {
    ref.current?.setMuted(muted);
  }, []);

  const setPlaybackRate = useCallback((rate: number) => {
    ref.current?.setPlaybackRate(rate);
  }, []);

  const setQuality = useCallback((index: number) => {
    ref.current?.setQuality(index);
  }, []);

  const setAutoQuality = useCallback((auto: boolean) => {
    ref.current?.setAutoQuality(auto);
  }, []);

  const setSubtitleTrack = useCallback((index: number) => {
    ref.current?.setSubtitleTrack(index);
  }, []);

  const toggleFullscreen = useCallback(() => {
    ref.current?.toggleFullscreen();
  }, []);

  const togglePiP = useCallback(async () => {
    await ref.current?.togglePiP();
  }, []);

  const skipForward = useCallback((seconds = 10) => {
    if (ref.current && state) {
      ref.current.seek(Math.min(state.duration, state.currentTime + seconds));
    }
  }, [state]);

  const skipBackward = useCallback((seconds = 10) => {
    if (ref.current && state) {
      ref.current.seek(Math.max(0, state.currentTime - seconds));
    }
  }, [state]);

  const toggleMute = useCallback(() => {
    if (ref.current && state) {
      ref.current.setMuted(!state.muted);
    }
  }, [state]);

  const togglePlay = useCallback(() => {
    if (state?.isPlaying) {
      pause();
    } else {
      play();
    }
  }, [state, play, pause]);

  return {
    ref,
    state,
    qoe,
    play,
    pause,
    seek,
    setVolume,
    setMuted,
    setPlaybackRate,
    setQuality,
    setAutoQuality,
    setSubtitleTrack,
    toggleFullscreen,
    togglePiP,
    skipForward,
    skipBackward,
    toggleMute,
    togglePlay,
  };
}
