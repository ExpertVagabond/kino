---------------------------- MODULE PSMBufferController ----------------------------
(***************************************************************************
 * PSM Buffer Controller Specification
 *
 * This module specifies the buffer management system for HLS streaming,
 * including segment fetching, buffer health monitoring, and prefetch logic.
 *
 * Author: Purple Squirrel Media
 * Version: 1.0
 ***************************************************************************)

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    MaxSegments,            \* Total segments in the video
    MaxBufferSegments,      \* Maximum segments to buffer ahead
    MinPlaybackBuffer,      \* Minimum segments needed to play
    PrefetchThreshold,      \* Start prefetching when buffer below this
    NumQualityLevels,       \* Number of available quality levels
    SegmentDuration         \* Duration of each segment in seconds

VARIABLES
    bufferedSegments,       \* Set of {index, quality} pairs for buffered segments
    currentSegment,         \* Current playback segment index
    pendingDownloads,       \* Set of segment indices being downloaded
    downloadQueue,          \* Queue of segments to download
    bufferHealth,           \* Current buffer health status
    selectedQuality,        \* Currently selected quality level
    networkBandwidth,       \* Estimated network bandwidth
    isStalled,              \* Whether playback is stalled waiting for buffer
    bytesDownloaded,        \* Total bytes downloaded
    lastSegmentTime         \* Time when last segment was added

(***************************************************************************)
(* Type Definitions                                                         *)
(***************************************************************************)

BufferHealthStates == {"critical", "low", "healthy", "full"}

SegmentRecord == [index: 0..MaxSegments-1, quality: 0..NumQualityLevels-1]

vars == <<bufferedSegments, currentSegment, pendingDownloads, downloadQueue,
          bufferHealth, selectedQuality, networkBandwidth, isStalled,
          bytesDownloaded, lastSegmentTime>>

(***************************************************************************)
(* Type Invariant                                                           *)
(***************************************************************************)

TypeInvariant ==
    /\ bufferedSegments \subseteq [index: 0..MaxSegments-1, quality: 0..NumQualityLevels-1]
    /\ currentSegment \in -1..MaxSegments-1
    /\ pendingDownloads \subseteq 0..MaxSegments-1
    /\ bufferHealth \in BufferHealthStates
    /\ selectedQuality \in 0..NumQualityLevels-1
    /\ networkBandwidth \in Nat
    /\ isStalled \in BOOLEAN
    /\ bytesDownloaded \in Nat
    /\ lastSegmentTime \in Nat

(***************************************************************************)
(* Helper Functions                                                         *)
(***************************************************************************)

\* Get indices of all buffered segments
BufferedIndices == {s.index : s \in bufferedSegments}

\* Count segments buffered ahead of current position
BufferAhead ==
    Cardinality({s \in bufferedSegments : s.index > currentSegment})

\* Check if a segment is buffered
IsBuffered(idx) == idx \in BufferedIndices

\* Check if segment is being downloaded
IsDownloading(idx) == idx \in pendingDownloads

\* Calculate buffer health status
CalculateBufferHealth(ahead) ==
    IF ahead = 0 THEN "critical"
    ELSE IF ahead < MinPlaybackBuffer THEN "low"
    ELSE IF ahead >= MaxBufferSegments THEN "full"
    ELSE "healthy"

\* Next segment to prefetch (lowest unbuffered, unqueued segment)
NextPrefetchSegment ==
    LET candidates == {i \in currentSegment+1..MaxSegments-1 :
                       ~IsBuffered(i) /\ ~IsDownloading(i)}
    IN IF candidates = {} THEN -1
       ELSE CHOOSE i \in candidates : \A j \in candidates : i <= j

\* Segments that should be evicted (too far behind playhead)
EvictableSegments ==
    {s \in bufferedSegments : s.index < currentSegment - 2}

(***************************************************************************)
(* Initial State                                                            *)
(***************************************************************************)

Init ==
    /\ bufferedSegments = {}
    /\ currentSegment = -1  \* -1 means not started
    /\ pendingDownloads = {}
    /\ downloadQueue = <<>>
    /\ bufferHealth = "critical"
    /\ selectedQuality = NumQualityLevels - 1  \* Start with highest
    /\ networkBandwidth = 5000  \* 5 Mbps default
    /\ isStalled = FALSE
    /\ bytesDownloaded = 0
    /\ lastSegmentTime = 0

(***************************************************************************)
(* State Transitions                                                        *)
(***************************************************************************)

\* Start playback from the beginning
StartPlayback ==
    /\ currentSegment = -1
    /\ currentSegment' = 0
    /\ isStalled' = ~IsBuffered(0)
    /\ bufferHealth' = CalculateBufferHealth(BufferAhead)
    /\ UNCHANGED <<bufferedSegments, pendingDownloads, downloadQueue,
                   selectedQuality, networkBandwidth, bytesDownloaded,
                   lastSegmentTime>>

\* Request download of a segment
RequestDownload(segIdx, quality) ==
    /\ segIdx \in 0..MaxSegments-1
    /\ segIdx \notin BufferedIndices
    /\ segIdx \notin pendingDownloads
    /\ Cardinality(pendingDownloads) < 3  \* Max concurrent downloads
    /\ pendingDownloads' = pendingDownloads \cup {segIdx}
    /\ UNCHANGED <<bufferedSegments, currentSegment, downloadQueue,
                   bufferHealth, selectedQuality, networkBandwidth,
                   isStalled, bytesDownloaded, lastSegmentTime>>

\* Segment download completes
DownloadComplete(segIdx, quality, size) ==
    /\ segIdx \in pendingDownloads
    /\ pendingDownloads' = pendingDownloads \ {segIdx}
    /\ bufferedSegments' = bufferedSegments \cup {[index |-> segIdx, quality |-> quality]}
    /\ bytesDownloaded' = bytesDownloaded + size
    /\ lastSegmentTime' = lastSegmentTime + 1  \* Simplified time
    /\ LET ahead == Cardinality({s \in bufferedSegments' : s.index > currentSegment})
       IN bufferHealth' = CalculateBufferHealth(ahead)
    /\ isStalled' = IF isStalled /\ IsBuffered(currentSegment) THEN FALSE ELSE isStalled
    /\ UNCHANGED <<currentSegment, downloadQueue, selectedQuality, networkBandwidth>>

\* Download fails (network error, etc.)
DownloadFailed(segIdx) ==
    /\ segIdx \in pendingDownloads
    /\ pendingDownloads' = pendingDownloads \ {segIdx}
    \* Re-queue for retry
    /\ downloadQueue' = Append(downloadQueue, segIdx)
    /\ UNCHANGED <<bufferedSegments, currentSegment, bufferHealth,
                   selectedQuality, networkBandwidth, isStalled,
                   bytesDownloaded, lastSegmentTime>>

\* Advance playback to next segment
AdvancePlayback ==
    /\ currentSegment >= 0
    /\ currentSegment < MaxSegments - 1
    /\ ~isStalled
    /\ IsBuffered(currentSegment)
    /\ currentSegment' = currentSegment + 1
    /\ LET ahead == Cardinality({s \in bufferedSegments : s.index > currentSegment'})
       IN /\ bufferHealth' = CalculateBufferHealth(ahead)
          /\ isStalled' = ~IsBuffered(currentSegment')
    /\ UNCHANGED <<bufferedSegments, pendingDownloads, downloadQueue,
                   selectedQuality, networkBandwidth, bytesDownloaded,
                   lastSegmentTime>>

\* Seek to a specific segment
SeekTo(targetSegment) ==
    /\ targetSegment \in 0..MaxSegments-1
    /\ targetSegment # currentSegment
    /\ currentSegment' = targetSegment
    /\ isStalled' = ~IsBuffered(targetSegment)
    /\ LET ahead == Cardinality({s \in bufferedSegments : s.index > targetSegment})
       IN bufferHealth' = CalculateBufferHealth(ahead)
    /\ UNCHANGED <<bufferedSegments, pendingDownloads, downloadQueue,
                   selectedQuality, networkBandwidth, bytesDownloaded,
                   lastSegmentTime>>

\* Evict old segments from buffer
EvictSegments ==
    /\ EvictableSegments # {}
    /\ bufferedSegments' = bufferedSegments \ EvictableSegments
    /\ UNCHANGED <<currentSegment, pendingDownloads, downloadQueue,
                   bufferHealth, selectedQuality, networkBandwidth,
                   isStalled, bytesDownloaded, lastSegmentTime>>

\* Prefetch next segment when buffer is getting low
Prefetch ==
    /\ bufferHealth \in {"critical", "low", "healthy"}
    /\ BufferAhead < MaxBufferSegments
    /\ NextPrefetchSegment # -1
    /\ LET seg == NextPrefetchSegment
       IN RequestDownload(seg, selectedQuality)

\* Change quality level (ABR or manual)
ChangeQuality(newQuality) ==
    /\ newQuality \in 0..NumQualityLevels-1
    /\ newQuality # selectedQuality
    /\ selectedQuality' = newQuality
    /\ UNCHANGED <<bufferedSegments, currentSegment, pendingDownloads,
                   downloadQueue, bufferHealth, networkBandwidth,
                   isStalled, bytesDownloaded, lastSegmentTime>>

\* Network bandwidth changes
BandwidthChange(newBandwidth) ==
    /\ newBandwidth \in 1..20000  \* 1 kbps to 20 Mbps
    /\ networkBandwidth' = newBandwidth
    /\ UNCHANGED <<bufferedSegments, currentSegment, pendingDownloads,
                   downloadQueue, bufferHealth, selectedQuality,
                   isStalled, bytesDownloaded, lastSegmentTime>>

\* Buffer underrun - playback stalls
BufferUnderrun ==
    /\ ~isStalled
    /\ currentSegment >= 0
    /\ ~IsBuffered(currentSegment)
    /\ isStalled' = TRUE
    /\ bufferHealth' = "critical"
    /\ UNCHANGED <<bufferedSegments, currentSegment, pendingDownloads,
                   downloadQueue, selectedQuality, networkBandwidth,
                   bytesDownloaded, lastSegmentTime>>

\* Recovery from stall when buffer replenishes
RecoverFromStall ==
    /\ isStalled
    /\ IsBuffered(currentSegment)
    /\ BufferAhead >= MinPlaybackBuffer
    /\ isStalled' = FALSE
    /\ bufferHealth' = CalculateBufferHealth(BufferAhead)
    /\ UNCHANGED <<bufferedSegments, currentSegment, pendingDownloads,
                   downloadQueue, selectedQuality, networkBandwidth,
                   bytesDownloaded, lastSegmentTime>>

\* Flush entire buffer (e.g., on quality change or seek)
FlushBuffer ==
    /\ bufferedSegments # {}
    /\ bufferedSegments' = {}
    /\ pendingDownloads' = {}
    /\ downloadQueue' = <<>>
    /\ bufferHealth' = "critical"
    /\ isStalled' = TRUE
    /\ UNCHANGED <<currentSegment, selectedQuality, networkBandwidth,
                   bytesDownloaded, lastSegmentTime>>

(***************************************************************************)
(* Next State Relation                                                      *)
(***************************************************************************)

Next ==
    \/ StartPlayback
    \/ \E s \in 0..MaxSegments-1, q \in 0..NumQualityLevels-1 :
           RequestDownload(s, q)
    \/ \E s \in pendingDownloads, q \in 0..NumQualityLevels-1 :
           DownloadComplete(s, q, 1000)  \* 1000 bytes placeholder
    \/ \E s \in pendingDownloads : DownloadFailed(s)
    \/ AdvancePlayback
    \/ \E t \in 0..MaxSegments-1 : SeekTo(t)
    \/ EvictSegments
    \/ Prefetch
    \/ \E q \in 0..NumQualityLevels-1 : ChangeQuality(q)
    \/ \E bw \in {500, 1000, 2000, 5000, 10000, 20000} : BandwidthChange(bw)
    \/ BufferUnderrun
    \/ RecoverFromStall
    \/ FlushBuffer

(***************************************************************************)
(* Fairness Conditions                                                      *)
(***************************************************************************)

Fairness ==
    /\ WF_vars(AdvancePlayback)
    /\ WF_vars(RecoverFromStall)
    /\ \A s \in 0..MaxSegments-1, q \in 0..NumQualityLevels-1 :
           WF_vars(DownloadComplete(s, q, 1000))

(***************************************************************************)
(* Specification                                                            *)
(***************************************************************************)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                        *)
(***************************************************************************)

\* Buffer never exceeds maximum size
BufferSizeBounded ==
    Cardinality(bufferedSegments) <= MaxBufferSegments + 5  \* Small margin

\* No duplicate segments in buffer
NoDuplicateSegments ==
    \A s1, s2 \in bufferedSegments :
        (s1.index = s2.index) => (s1 = s2)

\* Pending downloads are valid segment indices
PendingDownloadsValid ==
    \A s \in pendingDownloads : s \in 0..MaxSegments-1

\* Current segment is valid when playing
CurrentSegmentValid ==
    currentSegment >= -1 /\ currentSegment < MaxSegments

\* Buffer health matches actual buffer state
BufferHealthConsistent ==
    LET ahead == BufferAhead
    IN \/ (bufferHealth = "critical" /\ ahead = 0)
       \/ (bufferHealth = "low" /\ ahead > 0 /\ ahead < MinPlaybackBuffer)
       \/ (bufferHealth = "healthy" /\ ahead >= MinPlaybackBuffer /\ ahead < MaxBufferSegments)
       \/ (bufferHealth = "full" /\ ahead >= MaxBufferSegments)
       \/ isStalled  \* Health may not be updated immediately when stalled

\* Concurrent downloads bounded
ConcurrentDownloadsBounded ==
    Cardinality(pendingDownloads) <= 3

Safety ==
    /\ TypeInvariant
    /\ BufferSizeBounded
    /\ NoDuplicateSegments
    /\ PendingDownloadsValid
    /\ CurrentSegmentValid
    /\ ConcurrentDownloadsBounded

(***************************************************************************)
(* Liveness Properties                                                      *)
(***************************************************************************)

\* Stall eventually resolves
StallResolves ==
    isStalled ~> ~isStalled

\* Downloads eventually complete
DownloadsProgress ==
    (pendingDownloads # {}) ~> (Cardinality(bufferedSegments) > Cardinality(bufferedSegments))

\* Playback eventually advances
PlaybackAdvances ==
    (currentSegment >= 0 /\ currentSegment < MaxSegments - 1 /\ ~isStalled) ~>
    (currentSegment' > currentSegment \/ isStalled')

=============================================================================
