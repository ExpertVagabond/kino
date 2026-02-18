---------------------------- MODULE PSMDrm ----------------------------
(***************************************************************************
 * PSM DRM (Digital Rights Management) Specification
 *
 * This module specifies the DRM license acquisition and key management
 * system for encrypted content playback (Widevine, FairPlay, PlayReady).
 *
 * Author: Purple Squirrel Media
 * Version: 1.0
 ***************************************************************************)

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    MaxKeyIds,              \* Maximum concurrent key IDs
    MaxRetries,             \* Maximum license request retries
    LicenseExpiry,          \* License expiry duration
    KeySystems              \* Available key systems

ASSUME KeySystems \subseteq {"widevine", "fairplay", "playready", "clearkey"}

VARIABLES
    drmState,               \* Overall DRM state
    selectedKeySystem,      \* Currently selected key system
    licenseServerUrl,       \* License server URL
    pendingKeyIds,          \* Key IDs awaiting licenses
    activeLicenses,         \* Active licenses with keys
    sessionId,              \* Current DRM session ID
    challenge,              \* License request challenge
    licenseResponse,        \* License server response
    retryCount,             \* Current retry count for license request
    lastError,              \* Last error that occurred
    securityLevel,          \* DRM security level
    persistedLicenses,      \* Offline/persisted licenses
    robustnessLevel         \* Content robustness requirement

(***************************************************************************)
(* Type Definitions                                                         *)
(***************************************************************************)

DrmStates == {
    "idle",                 \* No DRM content loaded
    "detecting",            \* Detecting key system requirements
    "initializing",         \* Initializing CDM (Content Decryption Module)
    "session_creating",     \* Creating media key session
    "license_requesting",   \* Requesting license from server
    "license_updating",     \* Updating session with license
    "active",               \* DRM active, content playable
    "key_expired",          \* Keys have expired
    "error"                 \* Error state
}

SecurityLevels == {"L1", "L2", "L3", "SW"}  \* Hardware to software
RobustnessLevels == {"HW_SECURE_ALL", "HW_SECURE_DECODE", "SW_SECURE_CRYPTO", "SW_SECURE_DECODE"}

ErrorTypes == {
    "none",
    "key_system_not_supported",
    "cdm_init_failed",
    "session_failed",
    "license_denied",
    "license_expired",
    "network_error",
    "invalid_response",
    "output_restricted",
    "internal_error"
}

\* License record
LicenseRecord == [
    keyId: Nat,
    key: Nat,  \* Simplified: actual key would be bytes
    expiry: Nat,
    type: {"temporary", "persistent"},
    usageCount: Nat
]

vars == <<drmState, selectedKeySystem, licenseServerUrl, pendingKeyIds,
          activeLicenses, sessionId, challenge, licenseResponse, retryCount,
          lastError, securityLevel, persistedLicenses, robustnessLevel>>

(***************************************************************************)
(* Type Invariant                                                           *)
(***************************************************************************)

TypeInvariant ==
    /\ drmState \in DrmStates
    /\ selectedKeySystem \in KeySystems \cup {"none"}
    /\ pendingKeyIds \subseteq Nat
    /\ Cardinality(pendingKeyIds) <= MaxKeyIds
    /\ activeLicenses \in SUBSET [keyId: Nat, expiry: Nat, type: {"temporary", "persistent"}]
    /\ sessionId \in Nat \cup {-1}
    /\ challenge \in Nat \cup {-1}
    /\ licenseResponse \in Nat \cup {-1}
    /\ retryCount \in 0..MaxRetries
    /\ lastError \in ErrorTypes
    /\ securityLevel \in SecurityLevels \cup {"unknown"}
    /\ persistedLicenses \in SUBSET [keyId: Nat, expiry: Nat]
    /\ robustnessLevel \in RobustnessLevels \cup {"none"}

(***************************************************************************)
(* Helper Functions                                                         *)
(***************************************************************************)

\* Check if key system is supported
IsKeySystemSupported(ks) == ks \in KeySystems

\* Check if a license is expired
IsLicenseExpired(license, currentTime) ==
    license.expiry <= currentTime

\* Check if all pending keys have licenses
AllKeysLicensed ==
    \A keyId \in pendingKeyIds :
        \E license \in activeLicenses : license.keyId = keyId

\* Get license for key ID
GetLicense(keyId) ==
    IF \E l \in activeLicenses : l.keyId = keyId
    THEN CHOOSE l \in activeLicenses : l.keyId = keyId
    ELSE [keyId |-> -1, expiry |-> 0, type |-> "temporary"]

\* Check for persisted license
HasPersistedLicense(keyId) ==
    \E l \in persistedLicenses : l.keyId = keyId

(***************************************************************************)
(* Initial State                                                            *)
(***************************************************************************)

Init ==
    /\ drmState = "idle"
    /\ selectedKeySystem = "none"
    /\ licenseServerUrl = ""
    /\ pendingKeyIds = {}
    /\ activeLicenses = {}
    /\ sessionId = -1
    /\ challenge = -1
    /\ licenseResponse = -1
    /\ retryCount = 0
    /\ lastError = "none"
    /\ securityLevel = "unknown"
    /\ persistedLicenses = {}
    /\ robustnessLevel = "none"

(***************************************************************************)
(* State Transitions                                                        *)
(***************************************************************************)

\* Encrypted content detected - start DRM detection
DetectDrm(keyIds, robustness) ==
    /\ drmState = "idle"
    /\ keyIds # {}
    /\ Cardinality(keyIds) <= MaxKeyIds
    /\ drmState' = "detecting"
    /\ pendingKeyIds' = keyIds
    /\ robustnessLevel' = robustness
    /\ lastError' = "none"
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, activeLicenses,
                   sessionId, challenge, licenseResponse, retryCount,
                   securityLevel, persistedLicenses>>

\* Key system detected and selected
SelectKeySystem(ks, serverUrl) ==
    /\ drmState = "detecting"
    /\ IsKeySystemSupported(ks)
    /\ drmState' = "initializing"
    /\ selectedKeySystem' = ks
    /\ licenseServerUrl' = serverUrl
    /\ UNCHANGED <<pendingKeyIds, activeLicenses, sessionId, challenge,
                   licenseResponse, retryCount, lastError, securityLevel,
                   persistedLicenses, robustnessLevel>>

\* Key system not supported
KeySystemNotSupported ==
    /\ drmState = "detecting"
    /\ drmState' = "error"
    /\ lastError' = "key_system_not_supported"
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   retryCount, securityLevel, persistedLicenses, robustnessLevel>>

\* CDM initialization successful
CdmInitialized(secLevel) ==
    /\ drmState = "initializing"
    /\ secLevel \in SecurityLevels
    /\ drmState' = "session_creating"
    /\ securityLevel' = secLevel
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   retryCount, lastError, persistedLicenses, robustnessLevel>>

\* CDM initialization failed
CdmInitFailed ==
    /\ drmState = "initializing"
    /\ drmState' = "error"
    /\ lastError' = "cdm_init_failed"
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   retryCount, securityLevel, persistedLicenses, robustnessLevel>>

\* Media key session created
SessionCreated(sessId, challengeData) ==
    /\ drmState = "session_creating"
    /\ sessId >= 0
    /\ challengeData >= 0
    /\ drmState' = "license_requesting"
    /\ sessionId' = sessId
    /\ challenge' = challengeData
    /\ retryCount' = 0
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, licenseResponse, lastError, securityLevel,
                   persistedLicenses, robustnessLevel>>

\* Session creation failed
SessionFailed ==
    /\ drmState = "session_creating"
    /\ drmState' = "error"
    /\ lastError' = "session_failed"
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   retryCount, securityLevel, persistedLicenses, robustnessLevel>>

\* License received from server
LicenseReceived(response) ==
    /\ drmState = "license_requesting"
    /\ response >= 0
    /\ drmState' = "license_updating"
    /\ licenseResponse' = response
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, retryCount,
                   lastError, securityLevel, persistedLicenses, robustnessLevel>>

\* License request failed - retry
LicenseRequestFailed ==
    /\ drmState = "license_requesting"
    /\ retryCount < MaxRetries
    /\ retryCount' = retryCount + 1
    \* Stay in license_requesting to retry
    /\ UNCHANGED <<drmState, selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   lastError, securityLevel, persistedLicenses, robustnessLevel>>

\* License request failed - max retries exceeded
LicenseRequestMaxRetries ==
    /\ drmState = "license_requesting"
    /\ retryCount >= MaxRetries
    /\ drmState' = "error"
    /\ lastError' = "network_error"
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   retryCount, securityLevel, persistedLicenses, robustnessLevel>>

\* License denied by server
LicenseDenied ==
    /\ drmState = "license_requesting"
    /\ drmState' = "error"
    /\ lastError' = "license_denied"
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   retryCount, securityLevel, persistedLicenses, robustnessLevel>>

\* License update successful - keys available
LicenseUpdated(newLicenses) ==
    /\ drmState = "license_updating"
    /\ newLicenses # {}
    /\ drmState' = "active"
    /\ activeLicenses' = activeLicenses \cup newLicenses
    /\ lastError' = "none"
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   sessionId, challenge, licenseResponse, retryCount,
                   securityLevel, persistedLicenses, robustnessLevel>>

\* License update failed
LicenseUpdateFailed ==
    /\ drmState = "license_updating"
    /\ drmState' = "error"
    /\ lastError' = "invalid_response"
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   retryCount, securityLevel, persistedLicenses, robustnessLevel>>

\* Key expired during playback
KeyExpired(keyId) ==
    /\ drmState = "active"
    /\ \E l \in activeLicenses : l.keyId = keyId
    /\ drmState' = "key_expired"
    /\ lastError' = "license_expired"
    /\ activeLicenses' = {l \in activeLicenses : l.keyId # keyId}
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   sessionId, challenge, licenseResponse, retryCount,
                   securityLevel, persistedLicenses, robustnessLevel>>

\* Renew license for expired key
RenewLicense ==
    /\ drmState = "key_expired"
    /\ drmState' = "license_requesting"
    /\ retryCount' = 0
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   lastError, securityLevel, persistedLicenses, robustnessLevel>>

\* Persist license for offline playback
PersistLicense(license) ==
    /\ drmState = "active"
    /\ license.type = "persistent"
    /\ persistedLicenses' = persistedLicenses \cup {[keyId |-> license.keyId, expiry |-> license.expiry]}
    /\ UNCHANGED <<drmState, selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   retryCount, lastError, securityLevel, robustnessLevel>>

\* Load persisted license
LoadPersistedLicense(keyId) ==
    /\ drmState \in {"detecting", "idle"}
    /\ HasPersistedLicense(keyId)
    /\ LET persistedLicense == CHOOSE l \in persistedLicenses : l.keyId = keyId
       IN activeLicenses' = activeLicenses \cup {[keyId |-> keyId, expiry |-> persistedLicense.expiry, type |-> "persistent"]}
    /\ UNCHANGED <<drmState, selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   sessionId, challenge, licenseResponse, retryCount,
                   lastError, securityLevel, persistedLicenses, robustnessLevel>>

\* Output restricted (HDCP failure, etc.)
OutputRestricted ==
    /\ drmState = "active"
    /\ drmState' = "error"
    /\ lastError' = "output_restricted"
    /\ UNCHANGED <<selectedKeySystem, licenseServerUrl, pendingKeyIds,
                   activeLicenses, sessionId, challenge, licenseResponse,
                   retryCount, securityLevel, persistedLicenses, robustnessLevel>>

\* Close DRM session
CloseSession ==
    /\ drmState \in {"active", "key_expired", "error"}
    /\ drmState' = "idle"
    /\ selectedKeySystem' = "none"
    /\ activeLicenses' = {}
    /\ sessionId' = -1
    /\ challenge' = -1
    /\ licenseResponse' = -1
    /\ lastError' = "none"
    /\ UNCHANGED <<licenseServerUrl, pendingKeyIds, retryCount, securityLevel,
                   persistedLicenses, robustnessLevel>>

\* Reset after error
ResetFromError ==
    /\ drmState = "error"
    /\ drmState' = "idle"
    /\ selectedKeySystem' = "none"
    /\ pendingKeyIds' = {}
    /\ activeLicenses' = {}
    /\ sessionId' = -1
    /\ challenge' = -1
    /\ licenseResponse' = -1
    /\ retryCount' = 0
    /\ lastError' = "none"
    /\ securityLevel' = "unknown"
    /\ robustnessLevel' = "none"
    /\ UNCHANGED <<licenseServerUrl, persistedLicenses>>

(***************************************************************************)
(* Next State Relation                                                      *)
(***************************************************************************)

Next ==
    \/ \E keyIds \in SUBSET Nat, r \in RobustnessLevels : DetectDrm(keyIds, r)
    \/ \E ks \in KeySystems, url \in STRING : SelectKeySystem(ks, url)
    \/ KeySystemNotSupported
    \/ \E sl \in SecurityLevels : CdmInitialized(sl)
    \/ CdmInitFailed
    \/ \E sessId \in Nat, ch \in Nat : SessionCreated(sessId, ch)
    \/ SessionFailed
    \/ \E resp \in Nat : LicenseReceived(resp)
    \/ LicenseRequestFailed
    \/ LicenseRequestMaxRetries
    \/ LicenseDenied
    \/ \E licenses \in SUBSET [keyId: Nat, expiry: Nat, type: {"temporary", "persistent"}] :
           LicenseUpdated(licenses)
    \/ LicenseUpdateFailed
    \/ \E keyId \in Nat : KeyExpired(keyId)
    \/ RenewLicense
    \/ \E l \in [keyId: Nat, expiry: Nat, type: {"temporary", "persistent"}] : PersistLicense(l)
    \/ \E keyId \in Nat : LoadPersistedLicense(keyId)
    \/ OutputRestricted
    \/ CloseSession
    \/ ResetFromError

(***************************************************************************)
(* Fairness                                                                 *)
(***************************************************************************)

Fairness ==
    /\ WF_vars(LicenseReceived)
    /\ WF_vars(LicenseUpdated)
    /\ WF_vars(RenewLicense)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                        *)
(***************************************************************************)

\* Active state requires licenses
ActiveRequiresLicenses ==
    (drmState = "active") => (activeLicenses # {})

\* Session ID valid when session exists
SessionIdValid ==
    (drmState \in {"license_requesting", "license_updating", "active", "key_expired"}) =>
    (sessionId >= 0)

\* Challenge exists when requesting license
ChallengeExists ==
    (drmState = "license_requesting") => (challenge >= 0)

\* Key system selected when not idle
KeySystemSelected ==
    (drmState \notin {"idle", "detecting", "error"}) =>
    (selectedKeySystem # "none")

\* Retry count bounded
RetryCountBounded ==
    retryCount <= MaxRetries

\* Error state implies error type set
ErrorStateConsistent ==
    (drmState = "error") <=> (lastError # "none")

\* Security level known when active
SecurityLevelKnown ==
    (drmState = "active") => (securityLevel # "unknown")

Safety ==
    /\ TypeInvariant
    /\ ActiveRequiresLicenses
    /\ SessionIdValid
    /\ KeySystemSelected
    /\ RetryCountBounded
    /\ ErrorStateConsistent

(***************************************************************************)
(* Liveness Properties                                                      *)
(***************************************************************************)

\* DRM detection eventually completes
DetectionCompletes ==
    (drmState = "detecting") ~> (drmState \in {"initializing", "error"})

\* License request eventually completes
LicenseRequestCompletes ==
    (drmState = "license_requesting") ~>
    (drmState \in {"license_updating", "error"})

\* Expired keys eventually renewed or session closed
ExpiredKeysHandled ==
    (drmState = "key_expired") ~>
    (drmState \in {"license_requesting", "active", "idle", "error"})

=============================================================================
