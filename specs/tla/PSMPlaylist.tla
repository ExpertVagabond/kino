---------------------------- MODULE PSMPlaylist ----------------------------
(***************************************************************************
 * PSM Playlist Management Specification
 *
 * This module specifies playlist/queue management including track ordering,
 * shuffle mode, repeat modes, and track navigation.
 *
 * Author: Purple Squirrel Media
 * Version: 1.0
 ***************************************************************************)

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    MaxTracks,              \* Maximum tracks in playlist
    TrackIds                \* Set of all possible track IDs

VARIABLES
    tracks,                 \* Sequence of track IDs in playlist
    currentIndex,           \* Current track index (-1 = none)
    shuffleMode,            \* Whether shuffle is enabled
    repeatMode,             \* "none", "one", "all"
    shuffleOrder,           \* Shuffled indices when shuffle enabled
    shufflePosition,        \* Current position in shuffle order
    history,                \* Playback history for back navigation
    isPlaying,              \* Whether playlist is actively playing
    autoAdvance             \* Whether to auto-advance to next track

(***************************************************************************)
(* Type Definitions                                                         *)
(***************************************************************************)

RepeatModes == {"none", "one", "all"}

vars == <<tracks, currentIndex, shuffleMode, repeatMode, shuffleOrder,
          shufflePosition, history, isPlaying, autoAdvance>>

(***************************************************************************)
(* Type Invariant                                                           *)
(***************************************************************************)

TypeInvariant ==
    /\ tracks \in Seq(TrackIds)
    /\ Len(tracks) <= MaxTracks
    /\ currentIndex \in -1..MaxTracks-1
    /\ shuffleMode \in BOOLEAN
    /\ repeatMode \in RepeatModes
    /\ shuffleOrder \in Seq(0..MaxTracks-1)
    /\ shufflePosition \in -1..MaxTracks-1
    /\ history \in Seq(0..MaxTracks-1)
    /\ isPlaying \in BOOLEAN
    /\ autoAdvance \in BOOLEAN

(***************************************************************************)
(* Helper Functions                                                         *)
(***************************************************************************)

\* Check if playlist is empty
IsEmpty == Len(tracks) = 0

\* Check if at first track
IsAtStart ==
    IF shuffleMode
    THEN shufflePosition = 0
    ELSE currentIndex = 0

\* Check if at last track
IsAtEnd ==
    IF shuffleMode
    THEN shufflePosition = Len(shuffleOrder) - 1
    ELSE currentIndex = Len(tracks) - 1

\* Get next track index based on mode
NextTrackIndex ==
    IF IsEmpty THEN -1
    ELSE IF shuffleMode THEN
        IF shufflePosition < Len(shuffleOrder) - 1
        THEN shuffleOrder[shufflePosition + 2]  \* +2 because 1-indexed
        ELSE IF repeatMode = "all"
             THEN shuffleOrder[1]  \* Wrap to beginning
             ELSE -1  \* End of playlist
    ELSE
        IF currentIndex < Len(tracks) - 1
        THEN currentIndex + 1
        ELSE IF repeatMode = "all"
             THEN 0  \* Wrap to beginning
             ELSE -1  \* End of playlist

\* Get previous track index
PrevTrackIndex ==
    IF IsEmpty THEN -1
    ELSE IF Len(history) > 0 THEN history[Len(history)]
    ELSE IF shuffleMode THEN
        IF shufflePosition > 0
        THEN shuffleOrder[shufflePosition]  \* Previous in shuffle
        ELSE IF repeatMode = "all"
             THEN shuffleOrder[Len(shuffleOrder)]  \* Wrap to end
             ELSE currentIndex  \* Stay on current
    ELSE
        IF currentIndex > 0
        THEN currentIndex - 1
        ELSE IF repeatMode = "all"
             THEN Len(tracks) - 1  \* Wrap to end
             ELSE 0  \* Stay at start

\* Check if track ID exists in playlist
TrackExists(trackId) == trackId \in {tracks[i] : i \in 1..Len(tracks)}

\* Find index of track by ID
FindTrackIndex(trackId) ==
    IF ~TrackExists(trackId) THEN -1
    ELSE CHOOSE i \in 1..Len(tracks) : tracks[i] = trackId

(***************************************************************************)
(* Initial State                                                            *)
(***************************************************************************)

Init ==
    /\ tracks = <<>>
    /\ currentIndex = -1
    /\ shuffleMode = FALSE
    /\ repeatMode = "none"
    /\ shuffleOrder = <<>>
    /\ shufflePosition = -1
    /\ history = <<>>
    /\ isPlaying = FALSE
    /\ autoAdvance = TRUE

(***************************************************************************)
(* State Transitions                                                        *)
(***************************************************************************)

\* Add a track to the end of the playlist
AddTrack(trackId) ==
    /\ trackId \in TrackIds
    /\ Len(tracks) < MaxTracks
    /\ tracks' = Append(tracks, trackId)
    /\ IF shuffleMode
       THEN shuffleOrder' = Append(shuffleOrder, Len(tracks))
       ELSE UNCHANGED shuffleOrder
    /\ UNCHANGED <<currentIndex, shuffleMode, repeatMode, shufflePosition,
                   history, isPlaying, autoAdvance>>

\* Add track at specific position
InsertTrack(trackId, position) ==
    /\ trackId \in TrackIds
    /\ position \in 1..Len(tracks)+1
    /\ Len(tracks) < MaxTracks
    /\ LET before == SubSeq(tracks, 1, position-1)
           after == SubSeq(tracks, position, Len(tracks))
       IN tracks' = before \o <<trackId>> \o after
    /\ IF currentIndex >= position - 1
       THEN currentIndex' = currentIndex + 1
       ELSE UNCHANGED currentIndex
    /\ UNCHANGED <<shuffleMode, repeatMode, shuffleOrder, shufflePosition,
                   history, isPlaying, autoAdvance>>

\* Remove a track by index
RemoveTrack(idx) ==
    /\ idx \in 1..Len(tracks)
    /\ Len(tracks) > 0
    /\ LET before == SubSeq(tracks, 1, idx-1)
           after == SubSeq(tracks, idx+1, Len(tracks))
       IN tracks' = before \o after
    /\ IF currentIndex = idx - 1  \* 0-indexed comparison
       THEN currentIndex' = IF idx - 1 < Len(tracks') THEN idx - 1 ELSE Len(tracks') - 1
       ELSE IF currentIndex > idx - 1
            THEN currentIndex' = currentIndex - 1
            ELSE UNCHANGED currentIndex
    /\ UNCHANGED <<shuffleMode, repeatMode, shuffleOrder, shufflePosition,
                   history, isPlaying, autoAdvance>>

\* Clear entire playlist
ClearPlaylist ==
    /\ tracks' = <<>>
    /\ currentIndex' = -1
    /\ shuffleOrder' = <<>>
    /\ shufflePosition' = -1
    /\ history' = <<>>
    /\ isPlaying' = FALSE
    /\ UNCHANGED <<shuffleMode, repeatMode, autoAdvance>>

\* Play a specific track by index
PlayTrack(idx) ==
    /\ idx \in 0..Len(tracks)-1
    /\ ~IsEmpty
    /\ history' = IF currentIndex >= 0
                  THEN Append(history, currentIndex)
                  ELSE history
    /\ currentIndex' = idx
    /\ isPlaying' = TRUE
    /\ IF shuffleMode
       THEN LET pos == CHOOSE p \in 1..Len(shuffleOrder) : shuffleOrder[p] = idx
            IN shufflePosition' = pos - 1  \* Convert to 0-indexed
       ELSE UNCHANGED shufflePosition
    /\ UNCHANGED <<tracks, shuffleMode, repeatMode, shuffleOrder, autoAdvance>>

\* Skip to next track
NextTrack ==
    /\ ~IsEmpty
    /\ currentIndex >= 0
    /\ LET nextIdx == NextTrackIndex
       IN IF nextIdx >= 0
          THEN /\ history' = Append(history, currentIndex)
               /\ currentIndex' = nextIdx
               /\ IF shuffleMode
                  THEN shufflePosition' = (shufflePosition + 1) % Len(shuffleOrder)
                  ELSE UNCHANGED shufflePosition
               /\ UNCHANGED isPlaying
          ELSE /\ isPlaying' = FALSE
               /\ UNCHANGED <<currentIndex, shufflePosition, history>>
    /\ UNCHANGED <<tracks, shuffleMode, repeatMode, shuffleOrder, autoAdvance>>

\* Go to previous track
PrevTrack ==
    /\ ~IsEmpty
    /\ currentIndex >= 0
    /\ LET prevIdx == PrevTrackIndex
       IN /\ currentIndex' = prevIdx
          /\ history' = IF Len(history) > 0
                        THEN SubSeq(history, 1, Len(history)-1)
                        ELSE history
          /\ IF shuffleMode /\ shufflePosition > 0
             THEN shufflePosition' = shufflePosition - 1
             ELSE IF shuffleMode /\ repeatMode = "all"
                  THEN shufflePosition' = Len(shuffleOrder) - 1
                  ELSE UNCHANGED shufflePosition
    /\ UNCHANGED <<tracks, shuffleMode, repeatMode, shuffleOrder, isPlaying, autoAdvance>>

\* Track finished playing - handle auto-advance
TrackEnded ==
    /\ isPlaying
    /\ currentIndex >= 0
    /\ IF repeatMode = "one"
       THEN UNCHANGED <<currentIndex, shufflePosition, history, isPlaying>>
       ELSE IF autoAdvance
            THEN NextTrack
            ELSE /\ isPlaying' = FALSE
                 /\ UNCHANGED <<currentIndex, shufflePosition, history>>
    /\ UNCHANGED <<tracks, shuffleMode, repeatMode, shuffleOrder, autoAdvance>>

\* Toggle shuffle mode
ToggleShuffle ==
    /\ shuffleMode' = ~shuffleMode
    /\ IF ~shuffleMode  \* Enabling shuffle
       THEN /\ LET indices == 0..Len(tracks)-1
                   \* Simple shuffle: reverse order as placeholder
                   shuffled == [i \in 1..Len(tracks) |-> Len(tracks) - i]
               IN shuffleOrder' = [i \in 1..Len(tracks) |-> shuffled[i]]
            /\ shufflePosition' = IF currentIndex >= 0
                                  THEN CHOOSE p \in 0..Len(tracks)-1 :
                                       shuffleOrder'[p+1] = currentIndex
                                  ELSE -1
       ELSE /\ shuffleOrder' = <<>>
            /\ shufflePosition' = -1
    /\ UNCHANGED <<tracks, currentIndex, repeatMode, history, isPlaying, autoAdvance>>

\* Cycle repeat mode: none -> one -> all -> none
CycleRepeatMode ==
    /\ repeatMode' = CASE repeatMode = "none" -> "one"
                       [] repeatMode = "one" -> "all"
                       [] repeatMode = "all" -> "none"
    /\ UNCHANGED <<tracks, currentIndex, shuffleMode, shuffleOrder,
                   shufflePosition, history, isPlaying, autoAdvance>>

\* Set specific repeat mode
SetRepeatMode(mode) ==
    /\ mode \in RepeatModes
    /\ repeatMode' = mode
    /\ UNCHANGED <<tracks, currentIndex, shuffleMode, shuffleOrder,
                   shufflePosition, history, isPlaying, autoAdvance>>

\* Move track from one position to another
MoveTrack(fromIdx, toIdx) ==
    /\ fromIdx \in 1..Len(tracks)
    /\ toIdx \in 1..Len(tracks)
    /\ fromIdx # toIdx
    /\ LET track == tracks[fromIdx]
           withoutTrack == SubSeq(tracks, 1, fromIdx-1) \o
                          SubSeq(tracks, fromIdx+1, Len(tracks))
           newIdx == IF toIdx > fromIdx THEN toIdx - 1 ELSE toIdx
       IN tracks' = SubSeq(withoutTrack, 1, newIdx-1) \o
                    <<track>> \o
                    SubSeq(withoutTrack, newIdx, Len(withoutTrack))
    /\ IF currentIndex = fromIdx - 1
       THEN currentIndex' = toIdx - 1
       ELSE IF currentIndex > fromIdx - 1 /\ currentIndex < toIdx
            THEN currentIndex' = currentIndex - 1
            ELSE IF currentIndex < fromIdx - 1 /\ currentIndex >= toIdx - 1
                 THEN currentIndex' = currentIndex + 1
                 ELSE UNCHANGED currentIndex
    /\ UNCHANGED <<shuffleMode, repeatMode, shuffleOrder, shufflePosition,
                   history, isPlaying, autoAdvance>>

\* Pause playback
PausePlayback ==
    /\ isPlaying
    /\ isPlaying' = FALSE
    /\ UNCHANGED <<tracks, currentIndex, shuffleMode, repeatMode, shuffleOrder,
                   shufflePosition, history, autoAdvance>>

\* Resume playback
ResumePlayback ==
    /\ ~isPlaying
    /\ currentIndex >= 0
    /\ isPlaying' = TRUE
    /\ UNCHANGED <<tracks, currentIndex, shuffleMode, repeatMode, shuffleOrder,
                   shufflePosition, history, autoAdvance>>

\* Toggle auto-advance
ToggleAutoAdvance ==
    /\ autoAdvance' = ~autoAdvance
    /\ UNCHANGED <<tracks, currentIndex, shuffleMode, repeatMode, shuffleOrder,
                   shufflePosition, history, isPlaying>>

(***************************************************************************)
(* Next State Relation                                                      *)
(***************************************************************************)

Next ==
    \/ \E t \in TrackIds : AddTrack(t)
    \/ \E t \in TrackIds, p \in 1..MaxTracks : InsertTrack(t, p)
    \/ \E i \in 1..MaxTracks : RemoveTrack(i)
    \/ ClearPlaylist
    \/ \E i \in 0..MaxTracks-1 : PlayTrack(i)
    \/ NextTrack
    \/ PrevTrack
    \/ TrackEnded
    \/ ToggleShuffle
    \/ CycleRepeatMode
    \/ \E m \in RepeatModes : SetRepeatMode(m)
    \/ \E f \in 1..MaxTracks, t \in 1..MaxTracks : MoveTrack(f, t)
    \/ PausePlayback
    \/ ResumePlayback
    \/ ToggleAutoAdvance

(***************************************************************************)
(* Fairness                                                                 *)
(***************************************************************************)

Fairness ==
    /\ WF_vars(NextTrack)
    /\ WF_vars(TrackEnded)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                        *)
(***************************************************************************)

\* Current index is valid
CurrentIndexValid ==
    \/ currentIndex = -1  \* No track selected
    \/ (currentIndex >= 0 /\ currentIndex < Len(tracks))

\* Playlist size bounded
PlaylistSizeBounded ==
    Len(tracks) <= MaxTracks

\* Shuffle order matches playlist length when enabled
ShuffleOrderConsistent ==
    shuffleMode => Len(shuffleOrder) = Len(tracks)

\* Shuffle position valid when shuffle enabled
ShufflePositionValid ==
    shuffleMode =>
        (shufflePosition = -1 \/ (shufflePosition >= 0 /\ shufflePosition < Len(shuffleOrder)))

\* All shuffle order entries are valid indices
ShuffleOrderValid ==
    shuffleMode =>
        \A i \in 1..Len(shuffleOrder) :
            shuffleOrder[i] >= 0 /\ shuffleOrder[i] < Len(tracks)

\* History contains valid indices
HistoryValid ==
    \A i \in 1..Len(history) :
        history[i] >= 0 /\ history[i] < MaxTracks

\* Playing implies valid current track
PlayingImpliesValidTrack ==
    isPlaying => (currentIndex >= 0 /\ currentIndex < Len(tracks))

Safety ==
    /\ TypeInvariant
    /\ CurrentIndexValid
    /\ PlaylistSizeBounded
    /\ ShuffleOrderConsistent
    /\ ShufflePositionValid
    /\ HistoryValid
    /\ PlayingImpliesValidTrack

(***************************************************************************)
(* Liveness Properties                                                      *)
(***************************************************************************)

\* Playing eventually leads to track end or pause
PlayingResolves ==
    isPlaying ~> (~isPlaying \/ currentIndex' # currentIndex)

\* Track end advances playlist (unless last track with no repeat)
TrackEndAdvances ==
    (isPlaying /\ ~IsAtEnd) ~> (currentIndex' # currentIndex \/ ~isPlaying')

=============================================================================
