import React, {
  forwardRef,
  useImperativeHandle,
  useRef,
  useEffect,
  useState,
  useCallback,
} from 'react';
import Hls from 'hls.js';
import type {
  KinoPlayerProps,
  KinoPlayerRef,
  PlayerState,
  QualityLevel,
  SubtitleTrack,
  QoeMetrics,
} from './types';

export const KinoPlayer = forwardRef<KinoPlayerRef, KinoPlayerProps>(
  (
    {
      src,
      autoPlay = false,
      muted = false,
      loop = false,
      controls = true,
      poster,
      width,
      height,
      className,
      style,
      startTime = 0,
      maxBitrate,
      preferredQuality,
      subtitlesEnabled = false,
      preferredSubtitleLang,
      keyboardShortcuts = true,
      abrAlgorithm = 'bola',
      onReady,
      onPlay,
      onPause,
      onEnded,
      onError,
      onTimeUpdate,
      onDurationChange,
      onBufferUpdate,
      onQualityChange,
      onRebuffer,
      onSeek,
      onVolumeChange,
      onFullscreenChange,
      onPiPChange,
      onSubtitleChange,
    },
    ref
  ) => {
    const videoRef = useRef<HTMLVideoElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const hlsRef = useRef<Hls | null>(null);
    const [isReady, setIsReady] = useState(false);
    const [state, setState] = useState<PlayerState>({
      isPlaying: false,
      isPaused: true,
      isBuffering: false,
      isSeeking: false,
      isEnded: false,
      currentTime: 0,
      duration: 0,
      buffered: 0,
      volume: 1,
      muted: false,
      playbackRate: 1,
      currentQuality: -1,
      autoQuality: true,
      qualities: [],
      subtitleTracks: [],
      currentSubtitle: -1,
      isFullscreen: false,
      isPiP: false,
    });

    // Analytics state
    const analyticsRef = useRef({
      startTime: 0,
      rebufferCount: 0,
      rebufferDuration: 0,
      qualitySwitches: 0,
      totalBitrate: 0,
      bitrateCount: 0,
      watchTime: 0,
      lastPosition: 0,
    });

    // Initialize HLS
    useEffect(() => {
      const video = videoRef.current;
      if (!video || !src) return;

      const initHls = () => {
        if (hlsRef.current) {
          hlsRef.current.destroy();
        }

        if (Hls.isSupported()) {
          const hls = new Hls({
            startLevel: preferredQuality ?? -1,
            maxMaxBufferLength: 30,
            capLevelToPlayerSize: true,
          });

          hls.on(Hls.Events.MANIFEST_PARSED, (_, data) => {
            const qualities: QualityLevel[] = data.levels.map((level, index) => ({
              index,
              height: level.height,
              width: level.width,
              bitrate: level.bitrate,
              label: `${level.height}p`,
            }));

            setState((prev) => ({ ...prev, qualities }));
            analyticsRef.current.startTime = performance.now();
            setIsReady(true);
            onReady?.();

            if (autoPlay) {
              video.play().catch(() => {});
            }
          });

          hls.on(Hls.Events.LEVEL_SWITCHED, (_, data) => {
            const level = hls.levels[data.level];
            if (level) {
              const quality: QualityLevel = {
                index: data.level,
                height: level.height,
                width: level.width,
                bitrate: level.bitrate,
                label: `${level.height}p`,
              };
              setState((prev) => ({ ...prev, currentQuality: data.level }));
              analyticsRef.current.qualitySwitches++;
              analyticsRef.current.totalBitrate += level.bitrate;
              analyticsRef.current.bitrateCount++;
              onQualityChange?.(quality);
            }
          });

          hls.on(Hls.Events.SUBTITLE_TRACKS_UPDATED, (_, data) => {
            const tracks: SubtitleTrack[] = data.subtitleTracks.map((track, index) => ({
              index,
              id: track.id,
              name: track.name || `Track ${index + 1}`,
              lang: track.lang || '',
              default: track.default || false,
            }));
            setState((prev) => ({ ...prev, subtitleTracks: tracks }));

            // Auto-enable preferred language
            if (subtitlesEnabled && preferredSubtitleLang) {
              const preferred = tracks.find((t) => t.lang === preferredSubtitleLang);
              if (preferred) {
                hls.subtitleTrack = preferred.index;
              }
            }
          });

          hls.on(Hls.Events.SUBTITLE_TRACK_SWITCH, (_, data) => {
            const track = state.subtitleTracks.find((t) => t.id === data.id);
            setState((prev) => ({ ...prev, currentSubtitle: data.id }));
            onSubtitleChange?.(track || null);
          });

          hls.on(Hls.Events.ERROR, (_, data) => {
            if (data.fatal) {
              onError?.(new Error(data.details));
            }
          });

          hls.loadSource(src);
          hls.attachMedia(video);
          hlsRef.current = hls;

          if (maxBitrate) {
            hls.autoLevelCapping = hls.levels.findIndex((l) => l.bitrate > maxBitrate) - 1;
          }
        } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
          // Native HLS support (Safari)
          video.src = src;
          video.addEventListener('loadedmetadata', () => {
            setIsReady(true);
            onReady?.();
            if (autoPlay) {
              video.play().catch(() => {});
            }
          });
        }
      };

      initHls();

      return () => {
        if (hlsRef.current) {
          hlsRef.current.destroy();
          hlsRef.current = null;
        }
      };
    }, [src]);

    // Video event listeners
    useEffect(() => {
      const video = videoRef.current;
      if (!video) return;

      const handlers = {
        play: () => {
          setState((prev) => ({ ...prev, isPlaying: true, isPaused: false }));
          onPlay?.();
        },
        pause: () => {
          setState((prev) => ({ ...prev, isPlaying: false, isPaused: true }));
          onPause?.();
        },
        ended: () => {
          setState((prev) => ({ ...prev, isEnded: true, isPlaying: false }));
          onEnded?.();
        },
        waiting: () => {
          setState((prev) => ({ ...prev, isBuffering: true }));
          analyticsRef.current.rebufferCount++;
          onRebuffer?.();
        },
        playing: () => {
          setState((prev) => ({ ...prev, isBuffering: false }));
        },
        seeking: () => {
          setState((prev) => ({ ...prev, isSeeking: true }));
        },
        seeked: () => {
          setState((prev) => ({ ...prev, isSeeking: false }));
        },
        timeupdate: () => {
          const time = video.currentTime;
          setState((prev) => ({ ...prev, currentTime: time }));
          analyticsRef.current.watchTime += time - analyticsRef.current.lastPosition;
          analyticsRef.current.lastPosition = time;
          onTimeUpdate?.(time);
        },
        durationchange: () => {
          setState((prev) => ({ ...prev, duration: video.duration }));
          onDurationChange?.(video.duration);
        },
        volumechange: () => {
          setState((prev) => ({
            ...prev,
            volume: video.volume,
            muted: video.muted,
          }));
          onVolumeChange?.(video.volume, video.muted);
        },
        progress: () => {
          if (video.buffered.length > 0) {
            const buffered = video.buffered.end(video.buffered.length - 1);
            setState((prev) => ({ ...prev, buffered }));
            onBufferUpdate?.(buffered);
          }
        },
        enterpictureinpicture: () => {
          setState((prev) => ({ ...prev, isPiP: true }));
          onPiPChange?.(true);
        },
        leavepictureinpicture: () => {
          setState((prev) => ({ ...prev, isPiP: false }));
          onPiPChange?.(false);
        },
      };

      Object.entries(handlers).forEach(([event, handler]) => {
        video.addEventListener(event, handler);
      });

      // Set initial state
      video.muted = muted;
      if (startTime > 0) {
        video.currentTime = startTime;
      }

      return () => {
        Object.entries(handlers).forEach(([event, handler]) => {
          video.removeEventListener(event, handler);
        });
      };
    }, []);

    // Fullscreen event listeners
    useEffect(() => {
      const handleFullscreen = () => {
        const isFullscreen = !!document.fullscreenElement;
        setState((prev) => ({ ...prev, isFullscreen }));
        onFullscreenChange?.(isFullscreen);
      };

      document.addEventListener('fullscreenchange', handleFullscreen);
      return () => document.removeEventListener('fullscreenchange', handleFullscreen);
    }, [onFullscreenChange]);

    // Keyboard shortcuts
    useEffect(() => {
      if (!keyboardShortcuts) return;

      const handleKeydown = (e: KeyboardEvent) => {
        if (e.target instanceof HTMLInputElement) return;

        const video = videoRef.current;
        if (!video) return;

        switch (e.code) {
          case 'Space':
            e.preventDefault();
            video.paused ? video.play() : video.pause();
            break;
          case 'ArrowLeft':
            video.currentTime = Math.max(0, video.currentTime - 10);
            break;
          case 'ArrowRight':
            video.currentTime = Math.min(video.duration, video.currentTime + 10);
            break;
          case 'ArrowUp':
            e.preventDefault();
            video.volume = Math.min(1, video.volume + 0.1);
            break;
          case 'ArrowDown':
            e.preventDefault();
            video.volume = Math.max(0, video.volume - 0.1);
            break;
          case 'KeyM':
            video.muted = !video.muted;
            break;
          case 'KeyF':
            toggleFullscreen();
            break;
          case 'KeyP':
            togglePiP();
            break;
        }
      };

      document.addEventListener('keydown', handleKeydown);
      return () => document.removeEventListener('keydown', handleKeydown);
    }, [keyboardShortcuts]);

    const toggleFullscreen = useCallback(() => {
      const container = containerRef.current;
      if (!container) return;

      if (document.fullscreenElement) {
        document.exitFullscreen();
      } else {
        container.requestFullscreen();
      }
    }, []);

    const togglePiP = useCallback(async () => {
      const video = videoRef.current;
      if (!video) return;

      try {
        if (document.pictureInPictureElement) {
          await document.exitPictureInPicture();
        } else if (document.pictureInPictureEnabled) {
          await video.requestPictureInPicture();
        }
      } catch (e) {
        console.error('PiP error:', e);
      }
    }, []);

    // Expose methods via ref
    useImperativeHandle(
      ref,
      () => ({
        play: async () => {
          await videoRef.current?.play();
        },
        pause: () => {
          videoRef.current?.pause();
        },
        seek: (time: number) => {
          if (videoRef.current) {
            const from = videoRef.current.currentTime;
            videoRef.current.currentTime = time;
            onSeek?.(from, time);
          }
        },
        setVolume: (volume: number) => {
          if (videoRef.current) {
            videoRef.current.volume = Math.max(0, Math.min(1, volume));
          }
        },
        setMuted: (muted: boolean) => {
          if (videoRef.current) {
            videoRef.current.muted = muted;
          }
        },
        setPlaybackRate: (rate: number) => {
          if (videoRef.current) {
            videoRef.current.playbackRate = rate;
            setState((prev) => ({ ...prev, playbackRate: rate }));
          }
        },
        setQuality: (index: number) => {
          if (hlsRef.current) {
            hlsRef.current.currentLevel = index;
            setState((prev) => ({ ...prev, autoQuality: index === -1 }));
          }
        },
        setAutoQuality: (auto: boolean) => {
          if (hlsRef.current) {
            hlsRef.current.currentLevel = auto ? -1 : hlsRef.current.currentLevel;
            setState((prev) => ({ ...prev, autoQuality: auto }));
          }
        },
        setSubtitleTrack: (index: number) => {
          if (hlsRef.current) {
            hlsRef.current.subtitleTrack = index;
          }
        },
        toggleFullscreen,
        togglePiP,
        getState: () => state,
        getQoeMetrics: (): QoeMetrics => {
          const analytics = analyticsRef.current;
          const startupTime = analytics.startTime
            ? performance.now() - analytics.startTime
            : 0;
          const avgBitrate =
            analytics.bitrateCount > 0
              ? analytics.totalBitrate / analytics.bitrateCount
              : 0;

          // Calculate QoE score (simplified formula)
          let score = 100;
          score -= analytics.rebufferCount * 10;
          score -= analytics.qualitySwitches * 2;
          if (startupTime > 3000) score -= 10;
          score = Math.max(0, Math.min(100, score));

          return {
            score,
            startupTime,
            rebufferCount: analytics.rebufferCount,
            rebufferDuration: analytics.rebufferDuration,
            qualitySwitches: analytics.qualitySwitches,
            avgBitrate,
            watchTime: analytics.watchTime,
          };
        },
        getVideoElement: () => videoRef.current,
      }),
      [state, toggleFullscreen, togglePiP, onSeek]
    );

    return (
      <div
        ref={containerRef}
        className={`kino ${className || ''}`}
        style={{
          width: width || '100%',
          height: height || 'auto',
          position: 'relative',
          backgroundColor: '#000',
          ...style,
        }}
      >
        <video
          ref={videoRef}
          style={{ width: '100%', height: '100%' }}
          controls={controls}
          poster={poster}
          loop={loop}
          playsInline
        />
      </div>
    );
  }
);

KinoPlayer.displayName = 'KinoPlayer';
