//! Playback controls for desktop player

/// Keyboard/remote control handling
pub enum ControlAction {
    PlayPause,
    Stop,
    SeekForward(f64),
    SeekBackward(f64),
    VolumeUp,
    VolumeDown,
    Mute,
    Fullscreen,
    QualityUp,
    QualityDown,
}
