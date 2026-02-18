---------------------------- MODULE PSMAbrAlgorithm ----------------------------
(***************************************************************************)
(* PSM Adaptive Bitrate (ABR) Algorithm Specification                       *)
(*                                                                          *)
(* This specification models the BOLA (Buffer Occupancy based Lyapunov      *)
(* Algorithm) and throughput-based ABR algorithms used in PSM Player.       *)
(*                                                                          *)
(* BOLA optimizes for QoE by balancing:                                     *)
(* - Video quality (higher bitrate = better)                                *)
(* - Rebuffering risk (lower buffer = higher risk)                          *)
(*                                                                          *)
(* Author: Purple Squirrel Media                                            *)
(* Version: 0.1.0                                                           *)
(***************************************************************************)

EXTENDS Integers, Sequences, FiniteSets, Reals, TLC

CONSTANTS
    QualityLevels,      \* Set of quality level indices {0, 1, 2, ...}
    Bitrates,           \* Function: QualityLevels -> bitrate in kbps
    SegmentDuration,    \* Duration of each segment in seconds
    MaxBufferSize,      \* Maximum buffer in seconds
    MinBufferSize,      \* Minimum safe buffer threshold
    BandwidthSamples,   \* Number of samples for bandwidth estimation
    GammaP,             \* BOLA parameter for quality weight
    GammaB              \* BOLA parameter for buffer weight

VARIABLES
    currentQuality,     \* Currently selected quality level
    bufferLevel,        \* Current buffer occupancy in seconds
    bandwidthHistory,   \* Recent bandwidth measurements
    estimatedBandwidth, \* Current bandwidth estimate in kbps
    downloadingSegment, \* Currently downloading segment (or NONE)
    segmentQueue,       \* Queue of segments to download
    totalRebuffers,     \* Count of rebuffer events
    qualitySwitches,    \* Count of quality switches
    algorithm           \* Current ABR algorithm: "bola" | "throughput" | "hybrid"

vars == <<currentQuality, bufferLevel, bandwidthHistory, estimatedBandwidth,
          downloadingSegment, segmentQueue, totalRebuffers, qualitySwitches, algorithm>>

(***************************************************************************)
(* Type Definitions                                                         *)
(***************************************************************************)

Algorithms == {"bola", "throughput", "hybrid"}
Segment == [quality: QualityLevels, index: Nat]
NONE == "NONE"

TypeInvariant ==
    /\ currentQuality \in QualityLevels
    /\ bufferLevel \in 0..MaxBufferSize
    /\ bandwidthHistory \in Seq(Nat)
    /\ Len(bandwidthHistory) <= BandwidthSamples
    /\ estimatedBandwidth \in Nat
    /\ (downloadingSegment = NONE \/ downloadingSegment \in Segment)
    /\ segmentQueue \in Seq(Segment)
    /\ totalRebuffers \in Nat
    /\ qualitySwitches \in Nat
    /\ algorithm \in Algorithms

(***************************************************************************)
(* Initial State                                                            *)
(***************************************************************************)

Init ==
    /\ currentQuality = 0  \* Start at lowest quality
    /\ bufferLevel = 0
    /\ bandwidthHistory = <<>>
    /\ estimatedBandwidth = 0
    /\ downloadingSegment = NONE
    /\ segmentQueue = <<>>
    /\ totalRebuffers = 0
    /\ qualitySwitches = 0
    /\ algorithm = "bola"

(***************************************************************************)
(* Bandwidth Estimation                                                     *)
(***************************************************************************)

\* Calculate average bandwidth from history
AverageBandwidth(history) ==
    IF Len(history) = 0 THEN 0
    ELSE LET sum == FoldFunction(LAMBDA x, y: x + y, 0, history)
         IN sum \div Len(history)

\* Exponentially weighted moving average (simplified)
EWMA(current, new, alpha) ==
    (alpha * new + (100 - alpha) * current) \div 100

\* Record new bandwidth sample
RecordBandwidth(sample) ==
    /\ bandwidthHistory' =
        IF Len(bandwidthHistory) >= BandwidthSamples
        THEN Append(Tail(bandwidthHistory), sample)
        ELSE Append(bandwidthHistory, sample)
    /\ estimatedBandwidth' = EWMA(estimatedBandwidth, sample, 30)

(***************************************************************************)
(* BOLA Algorithm                                                           *)
(* Buffer Occupancy based Lyapunov Algorithm                                *)
(*                                                                          *)
(* BOLA selects quality to maximize:                                        *)
(*   V * utility(quality) + gamma * buffer_level                            *)
(*                                                                          *)
(* Where utility is typically log(bitrate)                                  *)
(***************************************************************************)

\* Utility function: higher quality = higher utility
Utility(q) == q  \* Simplified: linear utility

\* BOLA objective function
BOLAObjective(q, buffer) ==
    GammaP * Utility(q) + GammaB * buffer

\* Select quality using BOLA
BOLASelect(buffer, bandwidth) ==
    LET
        \* Filter to qualities we can download in time
        feasibleQualities == {q \in QualityLevels : Bitrates[q] <= bandwidth}
        \* If buffer is low, be conservative
        safeQualities == IF buffer < MinBufferSize
                         THEN {0}  \* Drop to lowest
                         ELSE feasibleQualities
    IN
        IF safeQualities = {} THEN 0
        ELSE CHOOSE q \in safeQualities :
            \A other \in safeQualities :
                BOLAObjective(q, buffer) >= BOLAObjective(other, buffer)

(***************************************************************************)
(* Throughput-based Algorithm                                               *)
(* Simple: select highest quality that fits within bandwidth                *)
(***************************************************************************)

ThroughputSelect(bandwidth) ==
    LET feasibleQualities == {q \in QualityLevels : Bitrates[q] <= bandwidth * 80 \div 100}
    IN IF feasibleQualities = {} THEN 0
       ELSE CHOOSE q \in feasibleQualities :
           \A other \in feasibleQualities : Bitrates[q] >= Bitrates[other]

(***************************************************************************)
(* Hybrid Algorithm                                                         *)
(* Use throughput when buffer is healthy, BOLA when buffer is concerning    *)
(***************************************************************************)

HybridSelect(buffer, bandwidth) ==
    IF buffer >= MaxBufferSize * 70 \div 100
    THEN ThroughputSelect(bandwidth)
    ELSE BOLASelect(buffer, bandwidth)

(***************************************************************************)
(* Quality Selection (main entry point)                                     *)
(***************************************************************************)

SelectQuality(buffer, bandwidth, algo) ==
    CASE algo = "bola" -> BOLASelect(buffer, bandwidth)
      [] algo = "throughput" -> ThroughputSelect(bandwidth)
      [] algo = "hybrid" -> HybridSelect(buffer, bandwidth)
      [] OTHER -> 0

(***************************************************************************)
(* Actions                                                                  *)
(***************************************************************************)

\* Request next segment with selected quality
RequestSegment(segIndex) ==
    /\ downloadingSegment = NONE
    /\ bufferLevel < MaxBufferSize
    /\ LET selectedQuality == SelectQuality(bufferLevel, estimatedBandwidth, algorithm)
       IN /\ downloadingSegment' = [quality |-> selectedQuality, index |-> segIndex]
          /\ IF selectedQuality # currentQuality
             THEN qualitySwitches' = qualitySwitches + 1
             ELSE qualitySwitches' = qualitySwitches
          /\ currentQuality' = selectedQuality
    /\ UNCHANGED <<bufferLevel, bandwidthHistory, estimatedBandwidth,
                   segmentQueue, totalRebuffers, algorithm>>

\* Segment download completes
SegmentComplete(measuredBandwidth) ==
    /\ downloadingSegment # NONE
    /\ LET newBuffer == bufferLevel + SegmentDuration
       IN bufferLevel' = IF newBuffer > MaxBufferSize
                         THEN MaxBufferSize
                         ELSE newBuffer
    /\ RecordBandwidth(measuredBandwidth)
    /\ downloadingSegment' = NONE
    /\ UNCHANGED <<currentQuality, segmentQueue, totalRebuffers,
                   qualitySwitches, algorithm>>

\* Playback consumes buffer
ConsumeBuffer ==
    /\ bufferLevel > 0
    /\ bufferLevel' = bufferLevel - 1
    /\ UNCHANGED <<currentQuality, bandwidthHistory, estimatedBandwidth,
                   downloadingSegment, segmentQueue, totalRebuffers,
                   qualitySwitches, algorithm>>

\* Rebuffer event (buffer depleted)
Rebuffer ==
    /\ bufferLevel = 0
    /\ downloadingSegment # NONE  \* Still waiting for segment
    /\ totalRebuffers' = totalRebuffers + 1
    /\ UNCHANGED <<currentQuality, bufferLevel, bandwidthHistory, estimatedBandwidth,
                   downloadingSegment, segmentQueue, qualitySwitches, algorithm>>

\* Switch ABR algorithm
SwitchAlgorithm(newAlgo) ==
    /\ newAlgo \in Algorithms
    /\ newAlgo # algorithm
    /\ algorithm' = newAlgo
    /\ UNCHANGED <<currentQuality, bufferLevel, bandwidthHistory, estimatedBandwidth,
                   downloadingSegment, segmentQueue, totalRebuffers, qualitySwitches>>

\* Bandwidth drops suddenly (network change)
BandwidthDrop(newBandwidth) ==
    /\ newBandwidth < estimatedBandwidth
    /\ estimatedBandwidth' = newBandwidth
    \* Immediately downgrade quality if current is too high
    /\ LET safeQuality == SelectQuality(bufferLevel, newBandwidth, algorithm)
       IN IF safeQuality < currentQuality
          THEN /\ currentQuality' = safeQuality
               /\ qualitySwitches' = qualitySwitches + 1
          ELSE /\ currentQuality' = currentQuality
               /\ qualitySwitches' = qualitySwitches
    /\ UNCHANGED <<bufferLevel, bandwidthHistory, downloadingSegment,
                   segmentQueue, totalRebuffers, algorithm>>

\* Reset ABR state
Reset ==
    /\ currentQuality' = 0
    /\ bufferLevel' = 0
    /\ bandwidthHistory' = <<>>
    /\ estimatedBandwidth' = 0
    /\ downloadingSegment' = NONE
    /\ segmentQueue' = <<>>
    /\ totalRebuffers' = 0
    /\ qualitySwitches' = 0
    /\ UNCHANGED <<algorithm>>

(***************************************************************************)
(* Next State Relation                                                      *)
(***************************************************************************)

Next ==
    \/ \E i \in Nat : RequestSegment(i)
    \/ \E bw \in 100..10000 : SegmentComplete(bw)
    \/ ConsumeBuffer
    \/ Rebuffer
    \/ \E a \in Algorithms : SwitchAlgorithm(a)
    \/ \E bw \in 100..10000 : BandwidthDrop(bw)
    \/ Reset

(***************************************************************************)
(* Fairness                                                                 *)
(***************************************************************************)

Fairness ==
    /\ WF_vars(ConsumeBuffer)
    /\ \E bw \in 100..10000 : WF_vars(SegmentComplete(bw))

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                        *)
(***************************************************************************)

\* Buffer never exceeds maximum
BufferBounded == bufferLevel <= MaxBufferSize

\* Quality level is always valid
QualityValid == currentQuality \in QualityLevels

\* Selected quality respects bandwidth (eventually)
QualityRespectsBandwidth ==
    (estimatedBandwidth > 0) => (Bitrates[currentQuality] <= estimatedBandwidth * 120 \div 100)

\* No infinite quality oscillation (anti-flapping)
\* This would require temporal logic with bounded history

Safety ==
    /\ TypeInvariant
    /\ BufferBounded
    /\ QualityValid

(***************************************************************************)
(* Liveness Properties                                                      *)
(***************************************************************************)

\* If bandwidth is stable and high, quality eventually improves
QualityImproves ==
    LET highBandwidth == estimatedBandwidth >= Bitrates[Cardinality(QualityLevels) - 1]
    IN (highBandwidth /\ bufferLevel >= MinBufferSize) ~>
       (currentQuality = Cardinality(QualityLevels) - 1)

\* Rebuffering is temporary (buffer eventually refills)
RebufferingTemporary ==
    (bufferLevel = 0) ~> (bufferLevel > MinBufferSize)

\* Downloads make progress
DownloadProgress ==
    (downloadingSegment # NONE) ~> (downloadingSegment = NONE)

(***************************************************************************)
(* QoE (Quality of Experience) Metrics                                      *)
(* These are not formal properties but useful for understanding behavior    *)
(***************************************************************************)

\* QoE Score (simplified formula)
\* Higher is better, penalizes rebuffers and quality switches
QoEScore ==
    LET baseScore == 100
        rebufferPenalty == totalRebuffers * 15
        switchPenalty == qualitySwitches * 3
        qualityBonus == currentQuality * 5
    IN baseScore - rebufferPenalty - switchPenalty + qualityBonus

\* Is current QoE acceptable?
AcceptableQoE == QoEScore >= 60

=============================================================================
