------------------------------ MODULE PSMPlayer ------------------------------
(***************************************************************************)
(* PSM Player - Complete System Specification                               *)
(*                                                                          *)
(* This module composes the player state machine, ABR algorithm, and        *)
(* concurrent streaming components into a complete system model.            *)
(*                                                                          *)
(* The specification verifies end-to-end properties of the PSM Player:      *)
(* - Video playback from idle to completion                                 *)
(* - Quality adaptation under varying network conditions                    *)
(* - Concurrent segment fetching and buffer management                      *)
(* - Error handling and recovery                                            *)
(*                                                                          *)
(* Author: Purple Squirrel Media                                            *)
(* Version: 0.1.0                                                           *)
(***************************************************************************)

EXTENDS Integers, Sequences, FiniteSets, TLC

(***************************************************************************)
(* Constants                                                                *)
(***************************************************************************)

CONSTANTS
    \* Video properties
    VideoDuration,          \* Total duration in segments
    SegmentDuration,        \* Duration of each segment in seconds

    \* Quality levels
    NumQualityLevels,       \* Number of available quality levels
    QualityBitrates,        \* Bitrate for each quality level (kbps)

    \* Buffer configuration
    MaxBuffer,              \* Maximum buffer size (seconds)
    MinBufferToPlay,        \* Minimum buffer to start/resume playback
    TargetBuffer,           \* Target buffer level for ABR

    \* Network
    MaxBandwidth,           \* Maximum network bandwidth (kbps)
    MinBandwidth,           \* Minimum network bandwidth (kbps)

    \* Concurrency
    MaxDownloadWorkers      \* Maximum parallel download workers

(***************************************************************************)
(* Derived Constants                                                        *)
(***************************************************************************)

QualityLevels == 0..NumQualityLevels-1
SegmentIds == 0..VideoDuration-1
BandwidthRange == MinBandwidth..MaxBandwidth

(***************************************************************************)
(* Variables                                                                *)
(***************************************************************************)

VARIABLES
    \* === Player State ===
    playerState,            \* Current state: Idle, Loading, Playing, etc.
    playbackPosition,       \* Current position (segment index)
    isFullscreen,           \* Fullscreen mode
    isPiP,                  \* Picture-in-Picture mode
    volume,                 \* Volume level (0-100)
    muted,                  \* Mute state

    \* === Streaming State ===
    manifestLoaded,         \* Has manifest been fetched?
    buffer,                 \* Set of buffered segment indices
    bufferQualities,        \* Quality level of each buffered segment
    downloadQueue,          \* Queue of segments to download
    activeDownloads,        \* Currently downloading segments
    downloadedBytes,        \* Total bytes downloaded

    \* === ABR State ===
    currentQuality,         \* Currently selected quality level
    bandwidthEstimate,      \* Estimated bandwidth (kbps)
    bandwidthHistory,       \* Recent bandwidth samples
    abrMode,                \* ABR algorithm: "bola", "throughput", "hybrid"

    \* === Network State ===
    networkBandwidth,       \* Current actual bandwidth
    networkAvailable,       \* Is network up?

    \* === Metrics ===
    rebufferCount,          \* Number of rebuffer events
    qualitySwitchCount,     \* Number of quality switches
    startupTime,            \* Time to first frame
    totalWatchTime          \* Total playback time

(***************************************************************************)
(* Variable Groups                                                          *)
(***************************************************************************)

playerVars == <<playerState, playbackPosition, isFullscreen, isPiP, volume, muted>>
streamingVars == <<manifestLoaded, buffer, bufferQualities, downloadQueue, activeDownloads, downloadedBytes>>
abrVars == <<currentQuality, bandwidthEstimate, bandwidthHistory, abrMode>>
networkVars == <<networkBandwidth, networkAvailable>>
metricVars == <<rebufferCount, qualitySwitchCount, startupTime, totalWatchTime>>

vars == <<playerVars, streamingVars, abrVars, networkVars, metricVars>>

(***************************************************************************)
(* Type Invariant                                                           *)
(***************************************************************************)

PlayerStates == {"Idle", "Loading", "Ready", "Playing", "Paused", "Buffering", "Seeking", "Ended", "Error"}
AbrModes == {"bola", "throughput", "hybrid"}

TypeOK ==
    /\ playerState \in PlayerStates
    /\ playbackPosition \in -1..VideoDuration
    /\ isFullscreen \in BOOLEAN
    /\ isPiP \in BOOLEAN
    /\ volume \in 0..100
    /\ muted \in BOOLEAN
    /\ manifestLoaded \in BOOLEAN
    /\ buffer \subseteq SegmentIds
    /\ bufferQualities \in [SegmentIds -> QualityLevels \cup {-1}]
    /\ downloadQueue \in Seq(SegmentIds)
    /\ activeDownloads \subseteq SegmentIds
    /\ Cardinality(activeDownloads) <= MaxDownloadWorkers
    /\ downloadedBytes \in Nat
    /\ currentQuality \in QualityLevels
    /\ bandwidthEstimate \in Nat
    /\ bandwidthHistory \in Seq(Nat)
    /\ abrMode \in AbrModes
    /\ networkBandwidth \in BandwidthRange
    /\ networkAvailable \in BOOLEAN
    /\ rebufferCount \in Nat
    /\ qualitySwitchCount \in Nat
    /\ startupTime \in Nat
    /\ totalWatchTime \in Nat

(***************************************************************************)
(* Initial State                                                            *)
(***************************************************************************)

Init ==
    \* Player
    /\ playerState = "Idle"
    /\ playbackPosition = -1
    /\ isFullscreen = FALSE
    /\ isPiP = FALSE
    /\ volume = 100
    /\ muted = FALSE
    \* Streaming
    /\ manifestLoaded = FALSE
    /\ buffer = {}
    /\ bufferQualities = [s \in SegmentIds |-> -1]
    /\ downloadQueue = <<>>
    /\ activeDownloads = {}
    /\ downloadedBytes = 0
    \* ABR
    /\ currentQuality = 0
    /\ bandwidthEstimate = 0
    /\ bandwidthHistory = <<>>
    /\ abrMode = "bola"
    \* Network
    /\ networkBandwidth = MaxBandwidth
    /\ networkAvailable = TRUE
    \* Metrics
    /\ rebufferCount = 0
    /\ qualitySwitchCount = 0
    /\ startupTime = 0
    /\ totalWatchTime = 0

(***************************************************************************)
(* Helper Functions                                                         *)
(***************************************************************************)

\* Current buffer level in seconds
BufferLevel == Cardinality(buffer) * SegmentDuration

\* Check if we have enough buffer to play
CanStartPlaying == BufferLevel >= MinBufferToPlay

\* Next segment needed (after current position and buffer)
NextNeededSegment ==
    LET maxBuffered == IF buffer = {} THEN playbackPosition ELSE Max(buffer)
        next == maxBuffered + 1
    IN IF next < VideoDuration THEN next ELSE -1

\* BOLA quality selection
BOLASelectQuality(bufLevel, bwEstimate) ==
    LET
        feasible == {q \in QualityLevels : QualityBitrates[q] <= bwEstimate}
        conservative == IF bufLevel < MinBufferToPlay THEN {0}
                        ELSE IF bufLevel < TargetBuffer THEN {q \in feasible : q <= currentQuality}
                        ELSE feasible
    IN IF conservative = {} THEN 0 ELSE Max(conservative)

\* Throughput-based quality selection
ThroughputSelectQuality(bwEstimate) ==
    LET feasible == {q \in QualityLevels : QualityBitrates[q] <= bwEstimate * 80 \div 100}
    IN IF feasible = {} THEN 0 ELSE Max(feasible)

\* Combined quality selection
SelectQuality ==
    CASE abrMode = "bola" -> BOLASelectQuality(BufferLevel, bandwidthEstimate)
      [] abrMode = "throughput" -> ThroughputSelectQuality(bandwidthEstimate)
      [] abrMode = "hybrid" ->
            IF BufferLevel >= TargetBuffer
            THEN ThroughputSelectQuality(bandwidthEstimate)
            ELSE BOLASelectQuality(BufferLevel, bandwidthEstimate)
      [] OTHER -> 0

(***************************************************************************)
(* Player State Transitions                                                 *)
(***************************************************************************)

\* Load video source
Load ==
    /\ playerState = "Idle"
    /\ playerState' = "Loading"
    /\ UNCHANGED <<playbackPosition, isFullscreen, isPiP, volume, muted,
                   streamingVars, abrVars, networkVars, metricVars>>

\* Manifest loaded successfully
ManifestLoaded ==
    /\ playerState = "Loading"
    /\ networkAvailable
    /\ manifestLoaded' = TRUE
    /\ playerState' = "Ready"
    /\ playbackPosition' = 0
    \* Queue first few segments
    /\ downloadQueue' = <<0, 1, 2>>
    /\ UNCHANGED <<isFullscreen, isPiP, volume, muted, buffer, bufferQualities,
                   activeDownloads, downloadedBytes, abrVars, networkVars, metricVars>>

\* Start playback
Play ==
    /\ playerState \in {"Ready", "Paused", "Ended"}
    /\ CanStartPlaying
    /\ playerState' = "Playing"
    /\ UNCHANGED <<playbackPosition, isFullscreen, isPiP, volume, muted,
                   streamingVars, abrVars, networkVars, metricVars>>

\* Pause playback
Pause ==
    /\ playerState = "Playing"
    /\ playerState' = "Paused"
    /\ UNCHANGED <<playbackPosition, isFullscreen, isPiP, volume, muted,
                   streamingVars, abrVars, networkVars, metricVars>>

\* Playback tick (advance by one segment)
PlayTick ==
    /\ playerState = "Playing"
    /\ playbackPosition < VideoDuration - 1
    /\ (playbackPosition + 1) \in buffer
    /\ playbackPosition' = playbackPosition + 1
    /\ buffer' = buffer \ {playbackPosition}
    /\ totalWatchTime' = totalWatchTime + SegmentDuration
    /\ UNCHANGED <<playerState, isFullscreen, isPiP, volume, muted,
                   manifestLoaded, bufferQualities, downloadQueue, activeDownloads,
                   downloadedBytes, abrVars, networkVars, rebufferCount,
                   qualitySwitchCount, startupTime>>

\* Enter buffering state (buffer depleted)
EnterBuffering ==
    /\ playerState = "Playing"
    /\ ~CanStartPlaying
    /\ playbackPosition < VideoDuration - 1
    /\ playerState' = "Buffering"
    /\ rebufferCount' = rebufferCount + 1
    /\ UNCHANGED <<playbackPosition, isFullscreen, isPiP, volume, muted,
                   streamingVars, abrVars, networkVars, qualitySwitchCount,
                   startupTime, totalWatchTime>>

\* Exit buffering (enough buffer accumulated)
ExitBuffering ==
    /\ playerState = "Buffering"
    /\ CanStartPlaying
    /\ playerState' = "Playing"
    /\ UNCHANGED <<playbackPosition, isFullscreen, isPiP, volume, muted,
                   streamingVars, abrVars, networkVars, metricVars>>

\* Playback ends
PlaybackEnds ==
    /\ playerState = "Playing"
    /\ playbackPosition = VideoDuration - 1
    /\ playerState' = "Ended"
    /\ UNCHANGED <<playbackPosition, isFullscreen, isPiP, volume, muted,
                   streamingVars, abrVars, networkVars, metricVars>>

(***************************************************************************)
(* Download Operations                                                      *)
(***************************************************************************)

\* Start downloading a segment
StartDownload ==
    /\ manifestLoaded
    /\ networkAvailable
    /\ Len(downloadQueue) > 0
    /\ Cardinality(activeDownloads) < MaxDownloadWorkers
    /\ LET seg == Head(downloadQueue)
       IN /\ activeDownloads' = activeDownloads \cup {seg}
          /\ downloadQueue' = Tail(downloadQueue)
    /\ UNCHANGED <<playerVars, manifestLoaded, buffer, bufferQualities,
                   downloadedBytes, abrVars, networkVars, metricVars>>

\* Download completes
DownloadComplete(seg) ==
    /\ seg \in activeDownloads
    /\ LET
         selectedQuality == SelectQuality
         segmentSize == QualityBitrates[selectedQuality] * SegmentDuration \div 8  \* bytes
       IN
         /\ buffer' = buffer \cup {seg}
         /\ bufferQualities' = [bufferQualities EXCEPT ![seg] = selectedQuality]
         /\ activeDownloads' = activeDownloads \ {seg}
         /\ downloadedBytes' = downloadedBytes + segmentSize
         \* Update bandwidth estimate
         /\ bandwidthEstimate' = networkBandwidth
         /\ bandwidthHistory' =
              IF Len(bandwidthHistory) >= 5
              THEN Append(Tail(bandwidthHistory), networkBandwidth)
              ELSE Append(bandwidthHistory, networkBandwidth)
         \* Track quality switches
         /\ IF selectedQuality # currentQuality
            THEN /\ currentQuality' = selectedQuality
                 /\ qualitySwitchCount' = qualitySwitchCount + 1
            ELSE UNCHANGED <<currentQuality, qualitySwitchCount>>
    /\ UNCHANGED <<playerVars, manifestLoaded, downloadQueue, abrMode,
                   networkVars, rebufferCount, startupTime, totalWatchTime>>

\* Queue more segments for download
QueueMoreSegments ==
    /\ manifestLoaded
    /\ Cardinality(buffer) + Cardinality(activeDownloads) + Len(downloadQueue) < MaxBuffer \div SegmentDuration
    /\ LET next == NextNeededSegment
       IN /\ next >= 0
          /\ next \notin buffer
          /\ next \notin activeDownloads
          /\ next \notin Range(downloadQueue)
          /\ downloadQueue' = Append(downloadQueue, next)
    /\ UNCHANGED <<playerVars, manifestLoaded, buffer, bufferQualities,
                   activeDownloads, downloadedBytes, abrVars, networkVars, metricVars>>

(***************************************************************************)
(* Network Simulation                                                       *)
(***************************************************************************)

\* Network bandwidth changes
BandwidthChange(newBw) ==
    /\ newBw \in BandwidthRange
    /\ newBw # networkBandwidth
    /\ networkBandwidth' = newBw
    /\ UNCHANGED <<playerVars, streamingVars, abrVars, networkAvailable, metricVars>>

\* Network goes down
NetworkDown ==
    /\ networkAvailable
    /\ networkAvailable' = FALSE
    /\ UNCHANGED <<playerVars, streamingVars, abrVars, networkBandwidth, metricVars>>

\* Network comes back up
NetworkUp ==
    /\ ~networkAvailable
    /\ networkAvailable' = TRUE
    /\ UNCHANGED <<playerVars, streamingVars, abrVars, networkBandwidth, metricVars>>

(***************************************************************************)
(* ABR Mode Switching                                                       *)
(***************************************************************************)

SwitchAbrMode(mode) ==
    /\ mode \in AbrModes
    /\ mode # abrMode
    /\ abrMode' = mode
    /\ UNCHANGED <<playerVars, streamingVars, currentQuality, bandwidthEstimate,
                   bandwidthHistory, networkVars, metricVars>>

(***************************************************************************)
(* User Actions                                                             *)
(***************************************************************************)

ToggleFullscreen ==
    /\ playerState \in {"Ready", "Playing", "Paused", "Buffering"}
    /\ isFullscreen' = ~isFullscreen
    /\ isPiP' = IF isFullscreen THEN isPiP ELSE FALSE
    /\ UNCHANGED <<playerState, playbackPosition, volume, muted,
                   streamingVars, abrVars, networkVars, metricVars>>

TogglePictureInPicture ==
    /\ playerState \in {"Playing", "Paused"}
    /\ isPiP' = ~isPiP
    /\ isFullscreen' = IF isPiP THEN isFullscreen ELSE FALSE
    /\ UNCHANGED <<playerState, playbackPosition, volume, muted,
                   streamingVars, abrVars, networkVars, metricVars>>

SetVolume(v) ==
    /\ v \in 0..100
    /\ volume' = v
    /\ UNCHANGED <<playerState, playbackPosition, isFullscreen, isPiP, muted,
                   streamingVars, abrVars, networkVars, metricVars>>

ToggleMute ==
    /\ muted' = ~muted
    /\ UNCHANGED <<playerState, playbackPosition, isFullscreen, isPiP, volume,
                   streamingVars, abrVars, networkVars, metricVars>>

(***************************************************************************)
(* Error Handling                                                           *)
(***************************************************************************)

NetworkError ==
    /\ playerState \in {"Loading", "Playing", "Buffering"}
    /\ ~networkAvailable
    /\ playerState' = "Error"
    /\ UNCHANGED <<playbackPosition, isFullscreen, isPiP, volume, muted,
                   streamingVars, abrVars, networkVars, metricVars>>

Reset ==
    /\ playerState = "Error"
    /\ Init

(***************************************************************************)
(* Next State Relation                                                      *)
(***************************************************************************)

Next ==
    \* Player transitions
    \/ Load
    \/ ManifestLoaded
    \/ Play
    \/ Pause
    \/ PlayTick
    \/ EnterBuffering
    \/ ExitBuffering
    \/ PlaybackEnds
    \* Downloads
    \/ StartDownload
    \/ \E s \in SegmentIds : DownloadComplete(s)
    \/ QueueMoreSegments
    \* Network
    \/ \E bw \in BandwidthRange : BandwidthChange(bw)
    \/ NetworkDown
    \/ NetworkUp
    \* ABR
    \/ \E m \in AbrModes : SwitchAbrMode(m)
    \* User actions
    \/ ToggleFullscreen
    \/ TogglePictureInPicture
    \/ \E v \in 0..100 : SetVolume(v)
    \/ ToggleMute
    \* Errors
    \/ NetworkError
    \/ Reset

(***************************************************************************)
(* Fairness                                                                 *)
(***************************************************************************)

Fairness ==
    /\ WF_vars(ManifestLoaded)
    /\ WF_vars(StartDownload)
    /\ \A s \in SegmentIds : WF_vars(DownloadComplete(s))
    /\ WF_vars(PlayTick)
    /\ WF_vars(ExitBuffering)
    /\ WF_vars(QueueMoreSegments)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                        *)
(***************************************************************************)

\* Cannot be fullscreen and PiP simultaneously
MutualExclusionDisplayModes == ~(isFullscreen /\ isPiP)

\* Buffer never exceeds maximum
BufferBounded == BufferLevel <= MaxBuffer

\* Download workers never exceed limit
WorkersBounded == Cardinality(activeDownloads) <= MaxDownloadWorkers

\* Quality is always valid
QualityValid == currentQuality \in QualityLevels

\* Playback position is valid
PositionValid == playbackPosition >= -1 /\ playbackPosition < VideoDuration

\* Playing implies sufficient buffer
PlayingImpliesBuffer ==
    (playerState = "Playing" /\ playbackPosition < VideoDuration - 1)
    => (playbackPosition + 1) \in buffer

Safety ==
    /\ TypeOK
    /\ MutualExclusionDisplayModes
    /\ BufferBounded
    /\ WorkersBounded
    /\ QualityValid
    /\ PositionValid

(***************************************************************************)
(* Liveness Properties                                                      *)
(***************************************************************************)

\* Eventually start playing or error
EventuallyPlaysOrErrors ==
    (playerState = "Loading") ~> (playerState \in {"Playing", "Error"})

\* Buffering eventually resolves
BufferingResolves ==
    (playerState = "Buffering") ~> (playerState \in {"Playing", "Error", "Idle"})

\* Playback eventually completes
PlaybackCompletes ==
    (playerState = "Playing") ~> (playerState \in {"Ended", "Paused", "Error", "Idle"})

\* Downloads make progress
DownloadsProgress ==
    (Cardinality(activeDownloads) > 0 /\ networkAvailable) ~>
    (\E s \in SegmentIds : s \in buffer)

(***************************************************************************)
(* QoE Metrics (for analysis)                                               *)
(***************************************************************************)

\* Calculate QoE score
QoEScore ==
    LET
        rebufferPenalty == rebufferCount * 15
        switchPenalty == qualitySwitchCount * 3
        qualityBonus == currentQuality * 10
        baseScore == 100
    IN Max(0, baseScore - rebufferPenalty - switchPenalty + qualityBonus)

\* Is QoE acceptable?
AcceptableQoE == QoEScore >= 60

\* High quality playback
HighQualityPlayback ==
    (playerState = "Playing") => (currentQuality >= NumQualityLevels \div 2)

=============================================================================
