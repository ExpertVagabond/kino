# @purplesquirrel/player-react

React component wrapper for PSM Player with HLS streaming support.

## Installation

```bash
npm install @purplesquirrel/player-react
# or
yarn add @purplesquirrel/player-react
# or
pnpm add @purplesquirrel/player-react
```

## Quick Start

```tsx
import { PsmPlayer } from '@purplesquirrel/player-react';
import '@purplesquirrel/player-react/styles.css';

function App() {
  return (
    <PsmPlayer
      src="https://example.com/stream.m3u8"
      autoPlay
      controls
      onReady={() => console.log('Player ready')}
      onError={(err) => console.error('Playback error:', err)}
    />
  );
}
```

## Using the Hook

For more control over the player, use the `usePsmPlayer` hook:

```tsx
import { PsmPlayer, usePsmPlayer } from '@purplesquirrel/player-react';
import '@purplesquirrel/player-react/styles.css';

function App() {
  const {
    ref,
    state,
    qoe,
    play,
    pause,
    togglePlay,
    skipForward,
    skipBackward,
    setVolume,
    toggleMute,
    setQuality,
    toggleFullscreen,
  } = usePsmPlayer();

  return (
    <div>
      <PsmPlayer
        ref={ref}
        src="https://example.com/stream.m3u8"
        controls={false}
      />

      {/* Custom controls */}
      <div className="controls">
        <button onClick={() => skipBackward(10)}>-10s</button>
        <button onClick={togglePlay}>
          {state?.isPlaying ? 'Pause' : 'Play'}
        </button>
        <button onClick={() => skipForward(10)}>+10s</button>
        <button onClick={toggleMute}>
          {state?.muted ? 'Unmute' : 'Mute'}
        </button>
        <button onClick={toggleFullscreen}>Fullscreen</button>
      </div>

      {/* Stats */}
      <div className="stats">
        <p>Time: {state?.currentTime.toFixed(1)}s / {state?.duration.toFixed(1)}s</p>
        <p>Quality: {state?.qualities[state?.currentQuality]?.label || 'Auto'}</p>
        <p>QoE Score: {qoe?.score.toFixed(0)}</p>
      </div>
    </div>
  );
}
```

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `src` | `string` | required | HLS stream URL |
| `autoPlay` | `boolean` | `false` | Auto-play on load |
| `muted` | `boolean` | `false` | Start muted |
| `loop` | `boolean` | `false` | Loop playback |
| `controls` | `boolean` | `true` | Show native controls |
| `poster` | `string` | - | Poster image URL |
| `width` | `number \| string` | `'100%'` | Player width |
| `height` | `number \| string` | `'auto'` | Player height |
| `className` | `string` | - | Additional CSS class |
| `style` | `CSSProperties` | - | Inline styles |
| `startTime` | `number` | `0` | Initial playback position |
| `maxBitrate` | `number` | - | Maximum bitrate cap |
| `preferredQuality` | `number` | - | Preferred quality level index |
| `subtitlesEnabled` | `boolean` | `false` | Enable subtitles by default |
| `preferredSubtitleLang` | `string` | - | Preferred subtitle language |
| `keyboardShortcuts` | `boolean` | `true` | Enable keyboard shortcuts |
| `abrAlgorithm` | `'throughput' \| 'bola' \| 'hybrid'` | `'bola'` | ABR algorithm |

## Events

| Event | Callback | Description |
|-------|----------|-------------|
| `onReady` | `() => void` | Player initialized |
| `onPlay` | `() => void` | Playback started |
| `onPause` | `() => void` | Playback paused |
| `onEnded` | `() => void` | Playback ended |
| `onError` | `(error: Error) => void` | Playback error |
| `onTimeUpdate` | `(time: number) => void` | Current time changed |
| `onDurationChange` | `(duration: number) => void` | Duration available |
| `onBufferUpdate` | `(buffered: number) => void` | Buffer level changed |
| `onQualityChange` | `(quality: QualityLevel) => void` | Quality level switched |
| `onRebuffer` | `() => void` | Rebuffering started |
| `onSeek` | `(from: number, to: number) => void` | Seek performed |
| `onVolumeChange` | `(volume: number, muted: boolean) => void` | Volume changed |
| `onFullscreenChange` | `(isFullscreen: boolean) => void` | Fullscreen toggled |
| `onPiPChange` | `(isPiP: boolean) => void` | Picture-in-Picture toggled |
| `onSubtitleChange` | `(track: SubtitleTrack \| null) => void` | Subtitle track changed |

## Ref Methods

Access these methods via the `ref`:

```tsx
const playerRef = useRef<PsmPlayerRef>(null);

// Later...
playerRef.current?.play();
playerRef.current?.pause();
playerRef.current?.seek(30);
playerRef.current?.setVolume(0.5);
playerRef.current?.setMuted(true);
playerRef.current?.setPlaybackRate(1.5);
playerRef.current?.setQuality(2);
playerRef.current?.setAutoQuality(true);
playerRef.current?.setSubtitleTrack(0);
playerRef.current?.toggleFullscreen();
playerRef.current?.togglePiP();

const state = playerRef.current?.getState();
const qoe = playerRef.current?.getQoeMetrics();
const video = playerRef.current?.getVideoElement();
```

## Keyboard Shortcuts

When `keyboardShortcuts` is enabled (default):

| Key | Action |
|-----|--------|
| Space | Play/Pause |
| Left Arrow | Rewind 10s |
| Right Arrow | Forward 10s |
| Up Arrow | Volume up |
| Down Arrow | Volume down |
| M | Toggle mute |
| F | Toggle fullscreen |
| P | Toggle Picture-in-Picture |

## TypeScript

Full TypeScript support with exported types:

```tsx
import type {
  PsmPlayerProps,
  PsmPlayerRef,
  PlayerState,
  QualityLevel,
  SubtitleTrack,
  QoeMetrics,
} from '@purplesquirrel/player-react';
```

## License

MIT OR Apache-2.0
