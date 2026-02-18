------------------------ MODULE PSMConcurrentStreaming ------------------------
(***************************************************************************)
(* PSM Concurrent Streaming Specification                                   *)
(*                                                                          *)
(* This specification models the concurrent behavior of HLS streaming:      *)
(* - Manifest fetching and parsing                                          *)
(* - Segment downloading (potentially parallel)                             *)
(* - Buffer management with multiple workers                                *)
(* - Rendition switching and segment boundary handling                      *)
(*                                                                          *)
(* Key concerns:                                                             *)
(* - Race conditions between downloads and playback                         *)
(* - Buffer consistency during quality switches                             *)
(* - Proper handling of segment discontinuities                             *)
(*                                                                          *)
(* Author: Purple Squirrel Media                                            *)
(* Version: 0.1.0                                                           *)
(***************************************************************************)

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    MaxSegments,        \* Total number of segments in the video
    MaxWorkers,         \* Maximum parallel download workers
    QualityLevels,      \* Set of quality levels (0 = lowest)
    MaxBufferSegments,  \* Maximum segments that can be buffered
    NetworkLatencyRange \* Set of possible network latency values

VARIABLES
    \* Manifest state
    manifestLoaded,     \* Has the manifest been fetched?
    availableSegments,  \* Segments available per quality level

    \* Download workers
    workers,            \* Set of active download tasks
    pendingRequests,    \* Queue of segment requests
    completedDownloads, \* Set of downloaded segments

    \* Buffer state
    buffer,             \* Sequence of buffered segments (in playback order)
    bufferQuality,      \* Quality level of each buffered segment

    \* Playback state
    playbackPosition,   \* Current segment being played
    isPlaying,          \* Is playback active?

    \* Quality state
    currentQuality,     \* Currently selected quality level
    pendingQualitySwitch, \* Quality switch pending at segment boundary

    \* Network simulation
    networkAvailable,   \* Is network available?

    \* Error tracking
    failedDownloads,    \* Segments that failed to download
    retryQueue          \* Segments queued for retry

vars == <<manifestLoaded, availableSegments, workers, pendingRequests,
          completedDownloads, buffer, bufferQuality, playbackPosition,
          isPlaying, currentQuality, pendingQualitySwitch, networkAvailable,
          failedDownloads, retryQueue>>

(***************************************************************************)
(* Type Definitions                                                         *)
(***************************************************************************)

SegmentId == 0..MaxSegments-1
WorkerId == 1..MaxWorkers

Segment == [
    index: SegmentId,
    quality: QualityLevels
]

DownloadTask == [
    worker: WorkerId,
    segment: Segment,
    started: BOOLEAN,
    progress: 0..100
]

NONE == "NONE"

TypeInvariant ==
    /\ manifestLoaded \in BOOLEAN
    /\ availableSegments \in [QualityLevels -> SUBSET SegmentId]
    /\ workers \in SUBSET DownloadTask
    /\ Cardinality(workers) <= MaxWorkers
    /\ pendingRequests \in Seq(Segment)
    /\ completedDownloads \in SUBSET Segment
    /\ buffer \in Seq(Segment)
    /\ Len(buffer) <= MaxBufferSegments
    /\ playbackPosition \in SegmentId \cup {-1}
    /\ isPlaying \in BOOLEAN
    /\ currentQuality \in QualityLevels
    /\ pendingQualitySwitch \in QualityLevels \cup {NONE}
    /\ networkAvailable \in BOOLEAN
    /\ failedDownloads \in SUBSET Segment
    /\ retryQueue \in Seq(Segment)

(***************************************************************************)
(* Initial State                                                            *)
(***************************************************************************)

Init ==
    /\ manifestLoaded = FALSE
    /\ availableSegments = [q \in QualityLevels |-> {}]
    /\ workers = {}
    /\ pendingRequests = <<>>
    /\ completedDownloads = {}
    /\ buffer = <<>>
    /\ bufferQuality = <<>>
    /\ playbackPosition = -1
    /\ isPlaying = FALSE
    /\ currentQuality = 0
    /\ pendingQualitySwitch = NONE
    /\ networkAvailable = TRUE
    /\ failedDownloads = {}
    /\ retryQueue = <<>>

(***************************************************************************)
(* Manifest Operations                                                      *)
(***************************************************************************)

\* Fetch and parse the HLS manifest
FetchManifest ==
    /\ ~manifestLoaded
    /\ networkAvailable
    /\ manifestLoaded' = TRUE
    \* All segments become available at all quality levels
    /\ availableSegments' = [q \in QualityLevels |-> SegmentId]
    /\ UNCHANGED <<workers, pendingRequests, completedDownloads, buffer,
                   bufferQuality, playbackPosition, isPlaying, currentQuality,
                   pendingQualitySwitch, networkAvailable, failedDownloads, retryQueue>>

(***************************************************************************)
(* Download Operations                                                      *)
(***************************************************************************)

\* Get next segment to download based on buffer state
NextSegmentToFetch ==
    LET
        bufferedIndices == {s.index : s \in Range(buffer)}
        downloadingIndices == {t.segment.index : t \in workers}
        pendingIndices == {s.index : s \in Range(pendingRequests)}
        lastBuffered == IF buffer = <<>>
                        THEN playbackPosition
                        ELSE buffer[Len(buffer)].index
        nextIndex == IF lastBuffered = -1 THEN 0 ELSE lastBuffered + 1
    IN
        IF nextIndex \in SegmentId
           /\ nextIndex \notin bufferedIndices
           /\ nextIndex \notin downloadingIndices
           /\ nextIndex \notin pendingIndices
        THEN [index |-> nextIndex, quality |-> currentQuality]
        ELSE NONE

\* Request a segment download
RequestDownload ==
    /\ manifestLoaded
    /\ Len(pendingRequests) < MaxBufferSegments
    /\ Len(buffer) < MaxBufferSegments
    /\ LET seg == NextSegmentToFetch
       IN /\ seg # NONE
          /\ pendingRequests' = Append(pendingRequests, seg)
    /\ UNCHANGED <<manifestLoaded, availableSegments, workers, completedDownloads,
                   buffer, bufferQuality, playbackPosition, isPlaying, currentQuality,
                   pendingQualitySwitch, networkAvailable, failedDownloads, retryQueue>>

\* Start a download worker
StartWorker(workerId) ==
    /\ manifestLoaded
    /\ networkAvailable
    /\ Len(pendingRequests) > 0
    /\ Cardinality(workers) < MaxWorkers
    /\ ~\E w \in workers : w.worker = workerId
    /\ LET seg == Head(pendingRequests)
           task == [worker |-> workerId, segment |-> seg, started |-> TRUE, progress |-> 0]
       IN /\ workers' = workers \cup {task}
          /\ pendingRequests' = Tail(pendingRequests)
    /\ UNCHANGED <<manifestLoaded, availableSegments, completedDownloads, buffer,
                   bufferQuality, playbackPosition, isPlaying, currentQuality,
                   pendingQualitySwitch, networkAvailable, failedDownloads, retryQueue>>

\* Download makes progress
DownloadProgress(workerId) ==
    /\ \E w \in workers : w.worker = workerId /\ w.progress < 100
    /\ networkAvailable
    /\ LET w == CHOOSE w \in workers : w.worker = workerId
           newProgress == IF w.progress + 25 > 100 THEN 100 ELSE w.progress + 25
           newTask == [w EXCEPT !.progress = newProgress]
       IN workers' = (workers \ {w}) \cup {newTask}
    /\ UNCHANGED <<manifestLoaded, availableSegments, pendingRequests, completedDownloads,
                   buffer, bufferQuality, playbackPosition, isPlaying, currentQuality,
                   pendingQualitySwitch, networkAvailable, failedDownloads, retryQueue>>

\* Download completes successfully
DownloadComplete(workerId) ==
    /\ \E w \in workers : w.worker = workerId /\ w.progress = 100
    /\ LET w == CHOOSE w \in workers : w.worker = workerId /\ w.progress = 100
       IN /\ completedDownloads' = completedDownloads \cup {w.segment}
          /\ workers' = workers \ {w}
          \* Add to buffer in correct position
          /\ IF w.segment.index = (IF buffer = <<>>
                                   THEN (IF playbackPosition = -1 THEN 0 ELSE playbackPosition + 1)
                                   ELSE buffer[Len(buffer)].index + 1)
             THEN buffer' = Append(buffer, w.segment)
             ELSE buffer' = buffer  \* Out of order, will be handled later
    /\ UNCHANGED <<manifestLoaded, availableSegments, pendingRequests, bufferQuality,
                   playbackPosition, isPlaying, currentQuality, pendingQualitySwitch,
                   networkAvailable, failedDownloads, retryQueue>>

\* Download fails
DownloadFail(workerId) ==
    /\ \E w \in workers : w.worker = workerId
    /\ LET w == CHOOSE w \in workers : w.worker = workerId
       IN /\ failedDownloads' = failedDownloads \cup {w.segment}
          /\ retryQueue' = Append(retryQueue, w.segment)
          /\ workers' = workers \ {w}
    /\ UNCHANGED <<manifestLoaded, availableSegments, pendingRequests, completedDownloads,
                   buffer, bufferQuality, playbackPosition, isPlaying, currentQuality,
                   pendingQualitySwitch, networkAvailable>>

\* Retry a failed download
RetryDownload ==
    /\ Len(retryQueue) > 0
    /\ networkAvailable
    /\ LET seg == Head(retryQueue)
       IN /\ pendingRequests' = Append(pendingRequests, seg)
          /\ retryQueue' = Tail(retryQueue)
          /\ failedDownloads' = failedDownloads \ {seg}
    /\ UNCHANGED <<manifestLoaded, availableSegments, workers, completedDownloads,
                   buffer, bufferQuality, playbackPosition, isPlaying, currentQuality,
                   pendingQualitySwitch, networkAvailable>>

(***************************************************************************)
(* Playback Operations                                                      *)
(***************************************************************************)

\* Start playback
StartPlayback ==
    /\ manifestLoaded
    /\ ~isPlaying
    /\ Len(buffer) > 0  \* Need at least one buffered segment
    /\ isPlaying' = TRUE
    /\ playbackPosition' = IF playbackPosition = -1 THEN 0 ELSE playbackPosition
    /\ UNCHANGED <<manifestLoaded, availableSegments, workers, pendingRequests,
                   completedDownloads, buffer, bufferQuality, currentQuality,
                   pendingQualitySwitch, networkAvailable, failedDownloads, retryQueue>>

\* Pause playback
PausePlayback ==
    /\ isPlaying
    /\ isPlaying' = FALSE
    /\ UNCHANGED <<manifestLoaded, availableSegments, workers, pendingRequests,
                   completedDownloads, buffer, bufferQuality, playbackPosition,
                   currentQuality, pendingQualitySwitch, networkAvailable,
                   failedDownloads, retryQueue>>

\* Advance playback (consume a segment)
AdvancePlayback ==
    /\ isPlaying
    /\ Len(buffer) > 0
    /\ playbackPosition < MaxSegments - 1
    /\ buffer' = Tail(buffer)
    /\ playbackPosition' = playbackPosition + 1
    \* Apply pending quality switch at segment boundary
    /\ IF pendingQualitySwitch # NONE
       THEN /\ currentQuality' = pendingQualitySwitch
            /\ pendingQualitySwitch' = NONE
       ELSE UNCHANGED <<currentQuality, pendingQualitySwitch>>
    /\ UNCHANGED <<manifestLoaded, availableSegments, workers, pendingRequests,
                   completedDownloads, bufferQuality, isPlaying, networkAvailable,
                   failedDownloads, retryQueue>>

\* Playback ends (reached last segment)
PlaybackEnd ==
    /\ isPlaying
    /\ playbackPosition = MaxSegments - 1
    /\ Len(buffer) = 0
    /\ isPlaying' = FALSE
    /\ UNCHANGED <<manifestLoaded, availableSegments, workers, pendingRequests,
                   completedDownloads, buffer, bufferQuality, playbackPosition,
                   currentQuality, pendingQualitySwitch, networkAvailable,
                   failedDownloads, retryQueue>>

(***************************************************************************)
(* Quality Switching                                                        *)
(***************************************************************************)

\* Request quality switch (happens at next segment boundary)
RequestQualitySwitch(newQuality) ==
    /\ newQuality \in QualityLevels
    /\ newQuality # currentQuality
    /\ pendingQualitySwitch' = newQuality
    /\ UNCHANGED <<manifestLoaded, availableSegments, workers, pendingRequests,
                   completedDownloads, buffer, bufferQuality, playbackPosition,
                   isPlaying, currentQuality, networkAvailable, failedDownloads, retryQueue>>

\* Immediate quality switch (cancels pending downloads)
ImmediateQualitySwitch(newQuality) ==
    /\ newQuality \in QualityLevels
    /\ newQuality # currentQuality
    /\ currentQuality' = newQuality
    /\ pendingQualitySwitch' = NONE
    \* Cancel all pending requests for old quality
    /\ pendingRequests' = SelectSeq(pendingRequests, LAMBDA s: s.quality = newQuality)
    \* Remove workers downloading old quality
    /\ workers' = {w \in workers : w.segment.quality = newQuality}
    /\ UNCHANGED <<manifestLoaded, availableSegments, completedDownloads, buffer,
                   bufferQuality, playbackPosition, isPlaying, networkAvailable,
                   failedDownloads, retryQueue>>

(***************************************************************************)
(* Network Simulation                                                       *)
(***************************************************************************)

\* Network becomes unavailable
NetworkDown ==
    /\ networkAvailable
    /\ networkAvailable' = FALSE
    /\ UNCHANGED <<manifestLoaded, availableSegments, workers, pendingRequests,
                   completedDownloads, buffer, bufferQuality, playbackPosition,
                   isPlaying, currentQuality, pendingQualitySwitch, failedDownloads, retryQueue>>

\* Network becomes available again
NetworkUp ==
    /\ ~networkAvailable
    /\ networkAvailable' = TRUE
    /\ UNCHANGED <<manifestLoaded, availableSegments, workers, pendingRequests,
                   completedDownloads, buffer, bufferQuality, playbackPosition,
                   isPlaying, currentQuality, pendingQualitySwitch, failedDownloads, retryQueue>>

(***************************************************************************)
(* Seek Operation                                                           *)
(***************************************************************************)

\* Seek to a specific segment
Seek(targetSegment) ==
    /\ manifestLoaded
    /\ targetSegment \in SegmentId
    /\ targetSegment # playbackPosition
    \* Clear buffer and pending downloads
    /\ buffer' = <<>>
    /\ pendingRequests' = <<>>
    /\ workers' = {}
    /\ playbackPosition' = targetSegment - 1  \* Will be advanced to target
    /\ isPlaying' = FALSE  \* Pause during seek
    /\ UNCHANGED <<manifestLoaded, availableSegments, completedDownloads, bufferQuality,
                   currentQuality, pendingQualitySwitch, networkAvailable,
                   failedDownloads, retryQueue>>

(***************************************************************************)
(* Next State Relation                                                      *)
(***************************************************************************)

Next ==
    \/ FetchManifest
    \/ RequestDownload
    \/ \E w \in WorkerId : StartWorker(w)
    \/ \E w \in WorkerId : DownloadProgress(w)
    \/ \E w \in WorkerId : DownloadComplete(w)
    \/ \E w \in WorkerId : DownloadFail(w)
    \/ RetryDownload
    \/ StartPlayback
    \/ PausePlayback
    \/ AdvancePlayback
    \/ PlaybackEnd
    \/ \E q \in QualityLevels : RequestQualitySwitch(q)
    \/ \E q \in QualityLevels : ImmediateQualitySwitch(q)
    \/ NetworkDown
    \/ NetworkUp
    \/ \E s \in SegmentId : Seek(s)

(***************************************************************************)
(* Fairness Conditions                                                      *)
(***************************************************************************)

Fairness ==
    /\ WF_vars(FetchManifest)
    /\ WF_vars(RequestDownload)
    /\ \A w \in WorkerId : WF_vars(StartWorker(w))
    /\ \A w \in WorkerId : WF_vars(DownloadProgress(w))
    /\ \A w \in WorkerId : WF_vars(DownloadComplete(w))
    /\ WF_vars(AdvancePlayback)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                        *)
(***************************************************************************)

\* Never more workers than maximum allowed
WorkerLimit == Cardinality(workers) <= MaxWorkers

\* Buffer never exceeds maximum
BufferLimit == Len(buffer) <= MaxBufferSegments

\* Segments in buffer are contiguous
BufferContiguous ==
    \A i \in 1..Len(buffer)-1 :
        buffer[i+1].index = buffer[i].index + 1

\* No duplicate downloads
NoDuplicateDownloads ==
    LET downloadingSegments == {w.segment : w \in workers}
    IN Cardinality(downloadingSegments) = Cardinality(workers)

\* Playback position never exceeds available segments
PlaybackInBounds ==
    playbackPosition < MaxSegments

\* Cannot play without manifest
ManifestRequired ==
    isPlaying => manifestLoaded

Safety ==
    /\ TypeInvariant
    /\ WorkerLimit
    /\ BufferLimit
    /\ NoDuplicateDownloads
    /\ PlaybackInBounds
    /\ ManifestRequired

(***************************************************************************)
(* Liveness Properties                                                      *)
(***************************************************************************)

\* Eventually fetch manifest if requested
ManifestEventuallyLoads ==
    networkAvailable ~> manifestLoaded

\* Downloads eventually complete
DownloadsComplete ==
    (Cardinality(workers) > 0) ~> (Cardinality(workers) = 0 \/ ~networkAvailable)

\* Buffer eventually fills
BufferFills ==
    (manifestLoaded /\ networkAvailable /\ Len(buffer) < MaxBufferSegments) ~>
    (Len(buffer) = MaxBufferSegments \/ playbackPosition = MaxSegments - 1)

\* Playback eventually completes or pauses
PlaybackEventuallyEnds ==
    isPlaying ~> (~isPlaying)

\* Failed downloads eventually retry
FailuresRetry ==
    (Cardinality(failedDownloads) > 0 /\ networkAvailable) ~>
    (Cardinality(failedDownloads) = 0)

(***************************************************************************)
(* Deadlock Freedom                                                         *)
(***************************************************************************)

\* The system can always make progress (no deadlock)
NoDeadlock ==
    \/ ~manifestLoaded  \* Can fetch manifest
    \/ Len(pendingRequests) > 0  \* Can start workers
    \/ Cardinality(workers) > 0  \* Workers can progress
    \/ (isPlaying /\ Len(buffer) > 0)  \* Can advance playback
    \/ playbackPosition = MaxSegments - 1  \* Reached end

=============================================================================
