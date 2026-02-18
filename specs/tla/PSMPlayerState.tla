---------------------------- MODULE PSMPlayerState ----------------------------
(***************************************************************************)
(* PSM Player State Machine                                                 *)
(*                                                                          *)
(* This specification models the core state machine of the Purple Squirrel  *)
(* Media video player, including playback states, buffering, and seeking.   *)
(*                                                                          *)
(* Author: Purple Squirrel Media                                            *)
(* Version: 0.1.0                                                           *)
(***************************************************************************)

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    MaxPosition,        \* Maximum video position (duration)
    MaxBuffer,          \* Maximum buffer size in seconds
    MinBufferToPlay,    \* Minimum buffer required to start/resume playback
    SeekPositions       \* Set of valid seek target positions

VARIABLES
    state,              \* Current player state
    position,           \* Current playback position
    bufferLevel,        \* Current buffer level (seconds ahead of position)
    bufferEnd,          \* Position up to which we have buffered
    volume,             \* Current volume (0-100)
    muted,              \* Whether audio is muted
    playbackRate,       \* Playback speed multiplier (1 = normal)
    error,              \* Current error state (NONE or error type)
    isFullscreen,       \* Fullscreen state
    isPiP               \* Picture-in-Picture state

(***************************************************************************)
(* Type Definitions                                                         *)
(***************************************************************************)

PlayerStates == {"Idle", "Loading", "Ready", "Playing", "Paused", "Buffering", "Seeking", "Ended", "Error"}
ErrorTypes == {"NONE", "NETWORK", "DECODE", "ABORTED", "NOT_SUPPORTED"}

vars == <<state, position, bufferLevel, bufferEnd, volume, muted, playbackRate, error, isFullscreen, isPiP>>

(***************************************************************************)
(* Type Invariant                                                           *)
(***************************************************************************)

TypeInvariant ==
    /\ state \in PlayerStates
    /\ position \in 0..MaxPosition
    /\ bufferLevel \in 0..MaxBuffer
    /\ bufferEnd \in 0..MaxPosition
    /\ volume \in 0..100
    /\ muted \in BOOLEAN
    /\ playbackRate \in {25, 50, 75, 100, 125, 150, 175, 200}  \* Percentages
    /\ error \in ErrorTypes
    /\ isFullscreen \in BOOLEAN
    /\ isPiP \in BOOLEAN

(***************************************************************************)
(* Initial State                                                            *)
(***************************************************************************)

Init ==
    /\ state = "Idle"
    /\ position = 0
    /\ bufferLevel = 0
    /\ bufferEnd = 0
    /\ volume = 100
    /\ muted = FALSE
    /\ playbackRate = 100
    /\ error = "NONE"
    /\ isFullscreen = FALSE
    /\ isPiP = FALSE

(***************************************************************************)
(* Helper Predicates                                                        *)
(***************************************************************************)

CanPlay ==
    /\ state \in {"Ready", "Paused", "Ended"}
    /\ bufferLevel >= MinBufferToPlay
    /\ error = "NONE"

CanPause == state = "Playing"

CanSeek == state \in {"Ready", "Playing", "Paused", "Buffering"}

HasEnoughBuffer == bufferLevel >= MinBufferToPlay

IsAtEnd == position >= MaxPosition

(***************************************************************************)
(* State Transitions                                                        *)
(***************************************************************************)

\* Load a video source
Load ==
    /\ state = "Idle"
    /\ state' = "Loading"
    /\ position' = 0
    /\ bufferLevel' = 0
    /\ bufferEnd' = 0
    /\ error' = "NONE"
    /\ UNCHANGED <<volume, muted, playbackRate, isFullscreen, isPiP>>

\* Video manifest loaded and ready
Ready ==
    /\ state = "Loading"
    /\ state' = "Ready"
    /\ UNCHANGED <<position, bufferLevel, bufferEnd, volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Start or resume playback
Play ==
    /\ CanPlay
    /\ state' = "Playing"
    /\ UNCHANGED <<position, bufferLevel, bufferEnd, volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Pause playback
Pause ==
    /\ CanPause
    /\ state' = "Paused"
    /\ UNCHANGED <<position, bufferLevel, bufferEnd, volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Playback progresses (time advances)
Tick ==
    /\ state = "Playing"
    /\ ~IsAtEnd
    /\ HasEnoughBuffer
    /\ position' = position + 1
    /\ bufferLevel' = bufferLevel - 1
    /\ UNCHANGED <<state, bufferEnd, volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Playback reaches end
End ==
    /\ state = "Playing"
    /\ IsAtEnd
    /\ state' = "Ended"
    /\ UNCHANGED <<position, bufferLevel, bufferEnd, volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Buffer depleted during playback - enter buffering state
EnterBuffering ==
    /\ state = "Playing"
    /\ bufferLevel < MinBufferToPlay
    /\ ~IsAtEnd
    /\ state' = "Buffering"
    /\ UNCHANGED <<position, bufferLevel, bufferEnd, volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Exit buffering when enough data is available
ExitBuffering ==
    /\ state = "Buffering"
    /\ bufferLevel >= MinBufferToPlay
    /\ state' = "Playing"
    /\ UNCHANGED <<position, bufferLevel, bufferEnd, volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Receive more buffer data
ReceiveBuffer ==
    /\ state \in {"Playing", "Buffering", "Paused", "Ready"}
    /\ bufferEnd < MaxPosition
    /\ bufferLevel < MaxBuffer
    /\ bufferLevel' = bufferLevel + 1
    /\ bufferEnd' = bufferEnd + 1
    /\ UNCHANGED <<state, position, volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Seek to a new position
Seek(target) ==
    /\ CanSeek
    /\ target \in SeekPositions
    /\ target # position
    /\ state' = "Seeking"
    /\ position' = target
    \* Buffer is invalidated on seek (simplified model)
    /\ bufferLevel' = 0
    /\ bufferEnd' = target
    /\ UNCHANGED <<volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Seek completes
SeekComplete ==
    /\ state = "Seeking"
    /\ state' = "Paused"  \* Return to paused, user must explicitly play
    /\ UNCHANGED <<position, bufferLevel, bufferEnd, volume, muted, playbackRate, error, isFullscreen, isPiP>>

\* Set volume
SetVolume(v) ==
    /\ v \in 0..100
    /\ volume' = v
    /\ UNCHANGED <<state, position, bufferLevel, bufferEnd, muted, playbackRate, error, isFullscreen, isPiP>>

\* Toggle mute
ToggleMute ==
    /\ muted' = ~muted
    /\ UNCHANGED <<state, position, bufferLevel, bufferEnd, volume, playbackRate, error, isFullscreen, isPiP>>

\* Set playback rate
SetPlaybackRate(rate) ==
    /\ rate \in {25, 50, 75, 100, 125, 150, 175, 200}
    /\ playbackRate' = rate
    /\ UNCHANGED <<state, position, bufferLevel, bufferEnd, volume, muted, error, isFullscreen, isPiP>>

\* Toggle fullscreen
ToggleFullscreen ==
    /\ state \in {"Ready", "Playing", "Paused", "Buffering"}
    \* Cannot be in PiP and fullscreen simultaneously
    /\ isFullscreen' = ~isFullscreen
    /\ isPiP' = IF ~isFullscreen THEN FALSE ELSE isPiP
    /\ UNCHANGED <<state, position, bufferLevel, bufferEnd, volume, muted, playbackRate, error>>

\* Toggle Picture-in-Picture
TogglePiP ==
    /\ state \in {"Playing", "Paused"}
    \* Cannot be in PiP and fullscreen simultaneously
    /\ isPiP' = ~isPiP
    /\ isFullscreen' = IF ~isPiP THEN FALSE ELSE isFullscreen
    /\ UNCHANGED <<state, position, bufferLevel, bufferEnd, volume, muted, playbackRate, error>>

\* Network error occurs
NetworkError ==
    /\ state \in {"Loading", "Playing", "Buffering", "Seeking"}
    /\ state' = "Error"
    /\ error' = "NETWORK"
    /\ UNCHANGED <<position, bufferLevel, bufferEnd, volume, muted, playbackRate, isFullscreen, isPiP>>

\* Decode error occurs
DecodeError ==
    /\ state \in {"Playing", "Buffering"}
    /\ state' = "Error"
    /\ error' = "DECODE"
    /\ UNCHANGED <<position, bufferLevel, bufferEnd, volume, muted, playbackRate, isFullscreen, isPiP>>

\* Reset player after error
Reset ==
    /\ state = "Error"
    /\ state' = "Idle"
    /\ position' = 0
    /\ bufferLevel' = 0
    /\ bufferEnd' = 0
    /\ error' = "NONE"
    /\ isFullscreen' = FALSE
    /\ isPiP' = FALSE
    /\ UNCHANGED <<volume, muted, playbackRate>>

\* Stop playback and return to idle
Stop ==
    /\ state \in {"Ready", "Playing", "Paused", "Buffering", "Ended"}
    /\ state' = "Idle"
    /\ position' = 0
    /\ bufferLevel' = 0
    /\ bufferEnd' = 0
    /\ isFullscreen' = FALSE
    /\ isPiP' = FALSE
    /\ UNCHANGED <<volume, muted, playbackRate, error>>

(***************************************************************************)
(* Next State Relation                                                      *)
(***************************************************************************)

Next ==
    \/ Load
    \/ Ready
    \/ Play
    \/ Pause
    \/ Tick
    \/ End
    \/ EnterBuffering
    \/ ExitBuffering
    \/ ReceiveBuffer
    \/ \E t \in SeekPositions : Seek(t)
    \/ SeekComplete
    \/ \E v \in 0..100 : SetVolume(v)
    \/ ToggleMute
    \/ \E r \in {25, 50, 75, 100, 125, 150, 175, 200} : SetPlaybackRate(r)
    \/ ToggleFullscreen
    \/ TogglePiP
    \/ NetworkError
    \/ DecodeError
    \/ Reset
    \/ Stop

(***************************************************************************)
(* Fairness Conditions                                                      *)
(***************************************************************************)

\* Eventually buffer data arrives if we're waiting
Fairness ==
    /\ WF_vars(ReceiveBuffer)
    /\ WF_vars(Tick)
    /\ WF_vars(ExitBuffering)
    /\ WF_vars(SeekComplete)

(***************************************************************************)
(* Specification                                                            *)
(***************************************************************************)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                        *)
(***************************************************************************)

\* Buffer level never exceeds maximum
BufferBounded == bufferLevel <= MaxBuffer

\* Position never exceeds duration
PositionBounded == position <= MaxPosition

\* Cannot be in fullscreen and PiP simultaneously
MutuallyExclusiveDisplayModes == ~(isFullscreen /\ isPiP)

\* Playing implies sufficient buffer (unless at end)
PlayingImpliesBuffer ==
    (state = "Playing" /\ ~IsAtEnd) => bufferLevel >= MinBufferToPlay

\* Error state implies error type is set
ErrorStateConsistent == (state = "Error") <=> (error # "NONE")

\* All safety properties combined
Safety ==
    /\ TypeInvariant
    /\ BufferBounded
    /\ PositionBounded
    /\ MutuallyExclusiveDisplayModes
    /\ ErrorStateConsistent

(***************************************************************************)
(* Liveness Properties                                                      *)
(***************************************************************************)

\* If we start playing, we eventually reach the end or get paused/stopped
EventuallyEndsOrStops ==
    (state = "Playing") ~> (state \in {"Ended", "Paused", "Idle", "Error"})

\* Buffering eventually resolves (either plays or errors)
BufferingResolves ==
    (state = "Buffering") ~> (state \in {"Playing", "Error", "Idle"})

\* Seeking eventually completes
SeekingCompletes ==
    (state = "Seeking") ~> (state # "Seeking")

\* Loading eventually completes
LoadingCompletes ==
    (state = "Loading") ~> (state \in {"Ready", "Error"})

=============================================================================
