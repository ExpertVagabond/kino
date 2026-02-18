---------------------------- MODULE PSMCaptions ----------------------------
(***************************************************************************
 * PSM Captions/Subtitles Specification
 *
 * This module specifies the caption and subtitle management system,
 * including track selection, rendering, and synchronization.
 *
 * Author: Purple Squirrel Media
 * Version: 1.0
 ***************************************************************************)

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    MaxTracks,              \* Maximum caption tracks
    MaxCues,                \* Maximum cues per track
    Languages,              \* Set of supported language codes
    MaxPosition             \* Maximum video position

VARIABLES
    tracks,                 \* Available caption tracks
    activeTrack,            \* Currently active track index (-1 = off)
    cues,                   \* Cues for active track
    visibleCues,            \* Currently visible cues
    currentPosition,        \* Current playback position
    captionsEnabled,        \* Master captions toggle
    fontSize,               \* Caption font size setting
    fontColor,              \* Caption font color
    backgroundColor,        \* Caption background color
    position,               \* Caption position (top/bottom)
    isLoading,              \* Whether captions are loading
    error                   \* Error state

(***************************************************************************)
(* Type Definitions                                                         *)
(***************************************************************************)

\* Caption track record
TrackRecord == [
    id: Nat,
    language: Languages,
    label: STRING,
    kind: {"subtitles", "captions", "descriptions"},
    isDefault: BOOLEAN
]

\* Cue record (single caption entry)
CueRecord == [
    id: Nat,
    startTime: 0..MaxPosition,
    endTime: 0..MaxPosition,
    text: STRING
]

FontSizes == {"small", "medium", "large", "xlarge"}
Positions == {"top", "bottom"}
ErrorTypes == {"none", "load_failed", "parse_error", "unsupported_format"}

vars == <<tracks, activeTrack, cues, visibleCues, currentPosition,
          captionsEnabled, fontSize, fontColor, backgroundColor,
          position, isLoading, error>>

(***************************************************************************)
(* Type Invariant                                                           *)
(***************************************************************************)

TypeInvariant ==
    /\ tracks \in Seq([id: Nat, language: Languages, kind: {"subtitles", "captions", "descriptions"}, isDefault: BOOLEAN])
    /\ Len(tracks) <= MaxTracks
    /\ activeTrack \in -1..MaxTracks-1
    /\ cues \in Seq([id: Nat, startTime: 0..MaxPosition, endTime: 0..MaxPosition])
    /\ visibleCues \subseteq Nat
    /\ currentPosition \in 0..MaxPosition
    /\ captionsEnabled \in BOOLEAN
    /\ fontSize \in FontSizes
    /\ position \in Positions
    /\ isLoading \in BOOLEAN
    /\ error \in ErrorTypes

(***************************************************************************)
(* Helper Functions                                                         *)
(***************************************************************************)

\* Check if a cue should be visible at given time
IsCueVisible(cue, time) ==
    /\ cue.startTime <= time
    /\ cue.endTime > time

\* Get all cues visible at current position
CuesAtPosition(time) ==
    {cue.id : cue \in {cues[i] : i \in 1..Len(cues)} : IsCueVisible(cue, time)}

\* Find track by language
FindTrackByLanguage(lang) ==
    IF \E i \in 1..Len(tracks) : tracks[i].language = lang
    THEN CHOOSE i \in 1..Len(tracks) : tracks[i].language = lang
    ELSE -1

\* Get default track
GetDefaultTrack ==
    IF \E i \in 1..Len(tracks) : tracks[i].isDefault
    THEN CHOOSE i \in 1..Len(tracks) : tracks[i].isDefault
    ELSE -1

\* Check if track index is valid
IsValidTrackIndex(idx) ==
    idx >= 0 /\ idx < Len(tracks)

(***************************************************************************)
(* Initial State                                                            *)
(***************************************************************************)

Init ==
    /\ tracks = <<>>
    /\ activeTrack = -1
    /\ cues = <<>>
    /\ visibleCues = {}
    /\ currentPosition = 0
    /\ captionsEnabled = FALSE
    /\ fontSize = "medium"
    /\ fontColor = "white"
    /\ backgroundColor = "black"
    /\ position = "bottom"
    /\ isLoading = FALSE
    /\ error = "none"

(***************************************************************************)
(* State Transitions                                                        *)
(***************************************************************************)

\* Add a caption track
AddTrack(trackId, language, kind, isDefault) ==
    /\ Len(tracks) < MaxTracks
    /\ ~(\E i \in 1..Len(tracks) : tracks[i].id = trackId)
    /\ tracks' = Append(tracks, [
        id |-> trackId,
        language |-> language,
        kind |-> kind,
        isDefault |-> isDefault
    ])
    /\ IF isDefault /\ activeTrack = -1 /\ captionsEnabled
       THEN activeTrack' = Len(tracks)
       ELSE UNCHANGED activeTrack
    /\ UNCHANGED <<cues, visibleCues, currentPosition, captionsEnabled,
                   fontSize, fontColor, backgroundColor, position,
                   isLoading, error>>

\* Remove a caption track
RemoveTrack(idx) ==
    /\ idx \in 1..Len(tracks)
    /\ LET before == SubSeq(tracks, 1, idx-1)
           after == SubSeq(tracks, idx+1, Len(tracks))
       IN tracks' = before \o after
    /\ IF activeTrack = idx - 1
       THEN /\ activeTrack' = -1
            /\ cues' = <<>>
            /\ visibleCues' = {}
       ELSE IF activeTrack > idx - 1
            THEN activeTrack' = activeTrack - 1
            ELSE UNCHANGED <<activeTrack, cues, visibleCues>>
    /\ UNCHANGED <<currentPosition, captionsEnabled, fontSize, fontColor,
                   backgroundColor, position, isLoading, error>>

\* Select a caption track
SelectTrack(idx) ==
    /\ IsValidTrackIndex(idx)
    /\ idx # activeTrack
    /\ activeTrack' = idx
    /\ isLoading' = TRUE
    /\ cues' = <<>>  \* Clear cues, will be loaded
    /\ visibleCues' = {}
    /\ error' = "none"
    /\ UNCHANGED <<tracks, currentPosition, captionsEnabled, fontSize,
                   fontColor, backgroundColor, position>>

\* Turn off captions (deselect track)
DisableCaptions ==
    /\ activeTrack # -1
    /\ activeTrack' = -1
    /\ cues' = <<>>
    /\ visibleCues' = {}
    /\ captionsEnabled' = FALSE
    /\ UNCHANGED <<tracks, currentPosition, fontSize, fontColor,
                   backgroundColor, position, isLoading, error>>

\* Toggle captions on/off
ToggleCaptions ==
    /\ captionsEnabled' = ~captionsEnabled
    /\ IF ~captionsEnabled /\ activeTrack = -1 /\ Len(tracks) > 0
       THEN LET defaultIdx == GetDefaultTrack
            IN IF defaultIdx >= 0
               THEN activeTrack' = defaultIdx - 1
               ELSE activeTrack' = 0  \* First track
       ELSE IF captionsEnabled
            THEN visibleCues' = {}
            ELSE UNCHANGED visibleCues
    /\ UNCHANGED <<tracks, cues, currentPosition, fontSize, fontColor,
                   backgroundColor, position, isLoading, error>>

\* Cues loaded successfully for track
CuesLoaded(newCues) ==
    /\ isLoading
    /\ cues' = newCues
    /\ isLoading' = FALSE
    /\ visibleCues' = CuesAtPosition(currentPosition)
    /\ UNCHANGED <<tracks, activeTrack, currentPosition, captionsEnabled,
                   fontSize, fontColor, backgroundColor, position, error>>

\* Cue loading failed
CueLoadFailed(errType) ==
    /\ isLoading
    /\ errType \in ErrorTypes \ {"none"}
    /\ error' = errType
    /\ isLoading' = FALSE
    /\ cues' = <<>>
    /\ visibleCues' = {}
    /\ UNCHANGED <<tracks, activeTrack, currentPosition, captionsEnabled,
                   fontSize, fontColor, backgroundColor, position>>

\* Time update - check for cue changes
TimeUpdate(newPosition) ==
    /\ newPosition \in 0..MaxPosition
    /\ currentPosition' = newPosition
    /\ IF captionsEnabled /\ activeTrack >= 0
       THEN visibleCues' = CuesAtPosition(newPosition)
       ELSE UNCHANGED visibleCues
    /\ UNCHANGED <<tracks, activeTrack, cues, captionsEnabled, fontSize,
                   fontColor, backgroundColor, position, isLoading, error>>

\* Set font size
SetFontSize(size) ==
    /\ size \in FontSizes
    /\ fontSize' = size
    /\ UNCHANGED <<tracks, activeTrack, cues, visibleCues, currentPosition,
                   captionsEnabled, fontColor, backgroundColor, position,
                   isLoading, error>>

\* Set caption position
SetPosition(pos) ==
    /\ pos \in Positions
    /\ position' = pos
    /\ UNCHANGED <<tracks, activeTrack, cues, visibleCues, currentPosition,
                   captionsEnabled, fontSize, fontColor, backgroundColor,
                   isLoading, error>>

\* Seek - update visible cues
Seek(targetPosition) ==
    /\ targetPosition \in 0..MaxPosition
    /\ targetPosition # currentPosition
    /\ currentPosition' = targetPosition
    /\ visibleCues' = IF captionsEnabled /\ activeTrack >= 0
                      THEN CuesAtPosition(targetPosition)
                      ELSE {}
    /\ UNCHANGED <<tracks, activeTrack, cues, captionsEnabled, fontSize,
                   fontColor, backgroundColor, position, isLoading, error>>

\* Clear error state
ClearError ==
    /\ error # "none"
    /\ error' = "none"
    /\ UNCHANGED <<tracks, activeTrack, cues, visibleCues, currentPosition,
                   captionsEnabled, fontSize, fontColor, backgroundColor,
                   position, isLoading>>

\* Reset captions (e.g., on video change)
Reset ==
    /\ tracks' = <<>>
    /\ activeTrack' = -1
    /\ cues' = <<>>
    /\ visibleCues' = {}
    /\ currentPosition' = 0
    /\ isLoading' = FALSE
    /\ error' = "none"
    /\ UNCHANGED <<captionsEnabled, fontSize, fontColor, backgroundColor, position>>

(***************************************************************************)
(* Next State Relation                                                      *)
(***************************************************************************)

Next ==
    \/ \E id \in Nat, lang \in Languages, k \in {"subtitles", "captions", "descriptions"}, d \in BOOLEAN :
           AddTrack(id, lang, k, d)
    \/ \E i \in 1..MaxTracks : RemoveTrack(i)
    \/ \E i \in 0..MaxTracks-1 : SelectTrack(i)
    \/ DisableCaptions
    \/ ToggleCaptions
    \/ \E c \in Seq([id: Nat, startTime: 0..MaxPosition, endTime: 0..MaxPosition]) :
           CuesLoaded(c)
    \/ \E e \in ErrorTypes \ {"none"} : CueLoadFailed(e)
    \/ \E p \in 0..MaxPosition : TimeUpdate(p)
    \/ \E s \in FontSizes : SetFontSize(s)
    \/ \E p \in Positions : SetPosition(p)
    \/ \E p \in 0..MaxPosition : Seek(p)
    \/ ClearError
    \/ Reset

(***************************************************************************)
(* Fairness                                                                 *)
(***************************************************************************)

Fairness ==
    /\ WF_vars(TimeUpdate)
    /\ WF_vars(CuesLoaded)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                        *)
(***************************************************************************)

\* Active track is valid
ActiveTrackValid ==
    activeTrack = -1 \/ IsValidTrackIndex(activeTrack)

\* Visible cues exist in cue list
VisibleCuesExist ==
    \A cueId \in visibleCues :
        \E i \in 1..Len(cues) : cues[i].id = cueId

\* Cue times are valid
CueTimesValid ==
    \A i \in 1..Len(cues) :
        cues[i].startTime < cues[i].endTime

\* No visible cues when captions disabled
NoCuesWhenDisabled ==
    (~captionsEnabled \/ activeTrack = -1) => visibleCues = {}

\* Loading state consistent
LoadingStateConsistent ==
    isLoading => (activeTrack >= 0 /\ Len(cues) = 0)

\* Error implies not loading
ErrorImpliesNotLoading ==
    error # "none" => ~isLoading

\* Track count bounded
TrackCountBounded ==
    Len(tracks) <= MaxTracks

Safety ==
    /\ TypeInvariant
    /\ ActiveTrackValid
    /\ NoCuesWhenDisabled
    /\ LoadingStateConsistent
    /\ ErrorImpliesNotLoading
    /\ TrackCountBounded

(***************************************************************************)
(* Liveness Properties                                                      *)
(***************************************************************************)

\* Loading eventually completes
LoadingCompletes ==
    isLoading ~> ~isLoading

\* Time eventually advances
TimeAdvances ==
    (currentPosition < MaxPosition) ~> (currentPosition' > currentPosition)

=============================================================================
