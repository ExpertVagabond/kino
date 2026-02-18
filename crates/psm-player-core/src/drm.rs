//! DRM - Digital Rights Management module
//!
//! Provides support for:
//! - Widevine (Chrome, Android, Chromecast)
//! - FairPlay (Safari, iOS, tvOS)
//! - PlayReady (Edge, Windows)
//! - ClearKey (Open standard for testing)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                    DRM Manager                       │
//! ├─────────────────────────────────────────────────────┤
//! │                                                     │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────┐         │
//! │  │ Widevine │  │ FairPlay │  │ PlayReady│         │
//! │  │  Client  │  │  Client  │  │  Client  │         │
//! │  └────┬─────┘  └────┬─────┘  └────┬─────┘         │
//! │       │             │             │                │
//! │       └─────────────┼─────────────┘                │
//! │                     │                              │
//! │              ┌──────┴──────┐                       │
//! │              │ License     │                       │
//! │              │ Server      │                       │
//! │              │ Protocol    │                       │
//! │              └─────────────┘                       │
//! └─────────────────────────────────────────────────────┘
//! ```

use crate::error::{Error, Result};
use crate::types::DrmSystem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// PSSH (Protection System Specific Header) box data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsshBox {
    /// DRM system ID (UUID)
    pub system_id: String,
    /// Key IDs contained in this PSSH
    pub key_ids: Vec<String>,
    /// Raw PSSH data (base64 encoded)
    pub data: String,
}

impl PsshBox {
    /// Create a new PSSH box
    pub fn new(system_id: &str, data: &[u8]) -> Self {
        Self {
            system_id: system_id.to_string(),
            key_ids: Vec::new(),
            data: base64_encode(data),
        }
    }

    /// Parse system ID to DRM type
    pub fn drm_system(&self) -> Option<DrmSystem> {
        match self.system_id.to_lowercase().as_str() {
            "edef8ba9-79d6-4ace-a3c8-27dcd51d21ed" => Some(DrmSystem::Widevine),
            "94ce86fb-07ff-4f43-adb8-93d2fa968ca2" => Some(DrmSystem::FairPlay),
            "9a04f079-9840-4286-ab92-e65be0885f95" => Some(DrmSystem::PlayReady),
            "1077efec-c0b2-4d02-ace3-3c1e52e2fb4b" => Some(DrmSystem::ClearKey),
            _ => None,
        }
    }

    /// Get raw data as bytes
    pub fn data_bytes(&self) -> Result<Vec<u8>> {
        base64_decode(&self.data)
    }
}

/// DRM configuration for a content item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrmConfig {
    /// License server URL for Widevine
    pub widevine_license_url: Option<Url>,
    /// License server URL for PlayReady
    pub playready_license_url: Option<Url>,
    /// Certificate URL for FairPlay
    pub fairplay_certificate_url: Option<Url>,
    /// License server URL for FairPlay
    pub fairplay_license_url: Option<Url>,
    /// Custom headers for license requests
    pub license_headers: HashMap<String, String>,
    /// Content ID for FairPlay
    pub fairplay_content_id: Option<String>,
    /// ClearKey keys (key_id -> key mapping)
    pub clearkey_keys: HashMap<String, String>,
    /// Whether to persist licenses
    pub persist_license: bool,
    /// License duration in seconds (0 = forever)
    pub license_duration: u64,
}

impl Default for DrmConfig {
    fn default() -> Self {
        Self {
            widevine_license_url: None,
            playready_license_url: None,
            fairplay_certificate_url: None,
            fairplay_license_url: None,
            license_headers: HashMap::new(),
            fairplay_content_id: None,
            clearkey_keys: HashMap::new(),
            persist_license: false,
            license_duration: 0,
        }
    }
}

impl DrmConfig {
    /// Create a Widevine-only configuration
    pub fn widevine(license_url: Url) -> Self {
        Self {
            widevine_license_url: Some(license_url),
            ..Default::default()
        }
    }

    /// Create a FairPlay configuration
    pub fn fairplay(license_url: Url, certificate_url: Url) -> Self {
        Self {
            fairplay_license_url: Some(license_url),
            fairplay_certificate_url: Some(certificate_url),
            ..Default::default()
        }
    }

    /// Create a ClearKey configuration
    pub fn clearkey(keys: HashMap<String, String>) -> Self {
        Self {
            clearkey_keys: keys,
            ..Default::default()
        }
    }

    /// Add a custom header for license requests
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.license_headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Check if any DRM is configured
    pub fn is_configured(&self) -> bool {
        self.widevine_license_url.is_some()
            || self.playready_license_url.is_some()
            || self.fairplay_license_url.is_some()
            || !self.clearkey_keys.is_empty()
    }

    /// Get supported DRM systems
    pub fn supported_systems(&self) -> Vec<DrmSystem> {
        let mut systems = Vec::new();
        if self.widevine_license_url.is_some() {
            systems.push(DrmSystem::Widevine);
        }
        if self.playready_license_url.is_some() {
            systems.push(DrmSystem::PlayReady);
        }
        if self.fairplay_license_url.is_some() {
            systems.push(DrmSystem::FairPlay);
        }
        if !self.clearkey_keys.is_empty() {
            systems.push(DrmSystem::ClearKey);
        }
        systems
    }
}

/// License request/response for a DRM system
#[derive(Debug, Clone)]
pub struct LicenseRequest {
    /// DRM system type
    pub system: DrmSystem,
    /// Request body (challenge)
    pub challenge: Vec<u8>,
    /// License server URL
    pub license_url: Url,
    /// Request headers
    pub headers: HashMap<String, String>,
}

/// License response from server
#[derive(Debug, Clone)]
pub struct LicenseResponse {
    /// DRM system type
    pub system: DrmSystem,
    /// License data
    pub license: Vec<u8>,
    /// Expiration time (Unix timestamp, 0 = no expiration)
    pub expiration: u64,
}

/// DRM session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrmSessionState {
    /// Session not started
    Idle,
    /// Waiting for license server certificate
    AwaitingCertificate,
    /// Certificate received, generating challenge
    GeneratingChallenge,
    /// License request sent
    AwaitingLicense,
    /// License received and loaded
    Ready,
    /// Session expired
    Expired,
    /// Error occurred
    Error,
}

/// DRM session information
#[derive(Debug, Clone)]
pub struct DrmSession {
    /// Session ID
    pub id: String,
    /// DRM system
    pub system: DrmSystem,
    /// Current state
    pub state: DrmSessionState,
    /// Key IDs in this session
    pub key_ids: Vec<String>,
    /// Expiration time (Unix timestamp)
    pub expiration: u64,
    /// Error message if state is Error
    pub error: Option<String>,
}

impl DrmSession {
    /// Create a new DRM session
    pub fn new(system: DrmSystem) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            system,
            state: DrmSessionState::Idle,
            key_ids: Vec::new(),
            expiration: 0,
            error: None,
        }
    }

    /// Check if session is ready for decryption
    pub fn is_ready(&self) -> bool {
        self.state == DrmSessionState::Ready
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        if self.expiration == 0 {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now >= self.expiration
    }
}

/// DRM Manager - Handles license acquisition and session management
pub struct DrmManager {
    config: DrmConfig,
    sessions: HashMap<String, DrmSession>,
    pssh_boxes: Vec<PsshBox>,
}

impl DrmManager {
    /// Create a new DRM manager
    pub fn new(config: DrmConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
            pssh_boxes: Vec::new(),
        }
    }

    /// Set PSSH boxes from manifest or init segment
    pub fn set_pssh_boxes(&mut self, boxes: Vec<PsshBox>) {
        self.pssh_boxes = boxes;
    }

    /// Get PSSH box for a specific DRM system
    pub fn get_pssh(&self, system: DrmSystem) -> Option<&PsshBox> {
        let target_id = system.system_id().to_lowercase();
        self.pssh_boxes.iter().find(|p| p.system_id.to_lowercase() == target_id)
    }

    /// Create a license request for Widevine
    pub fn create_widevine_request(&self, challenge: Vec<u8>) -> Result<LicenseRequest> {
        let license_url = self.config.widevine_license_url.clone()
            .ok_or_else(|| Error::drm("Widevine license URL not configured"))?;

        Ok(LicenseRequest {
            system: DrmSystem::Widevine,
            challenge,
            license_url,
            headers: self.config.license_headers.clone(),
        })
    }

    /// Create a license request for FairPlay
    pub fn create_fairplay_request(&self, spc: Vec<u8>) -> Result<LicenseRequest> {
        let license_url = self.config.fairplay_license_url.clone()
            .ok_or_else(|| Error::drm("FairPlay license URL not configured"))?;

        Ok(LicenseRequest {
            system: DrmSystem::FairPlay,
            challenge: spc,
            license_url,
            headers: self.config.license_headers.clone(),
        })
    }

    /// Get ClearKey license (no server needed)
    pub fn get_clearkey_license(&self) -> Result<LicenseResponse> {
        if self.config.clearkey_keys.is_empty() {
            return Err(Error::drm("No ClearKey keys configured"));
        }

        // Build ClearKey license JSON
        let keys: Vec<serde_json::Value> = self.config.clearkey_keys.iter()
            .map(|(kid, key)| {
                serde_json::json!({
                    "kty": "oct",
                    "kid": kid,
                    "k": key,
                })
            })
            .collect();

        let license_json = serde_json::json!({
            "keys": keys,
            "type": "temporary",
        });

        Ok(LicenseResponse {
            system: DrmSystem::ClearKey,
            license: license_json.to_string().into_bytes(),
            expiration: 0,
        })
    }

    /// Create or get a session for a DRM system
    pub fn create_session(&mut self, system: DrmSystem) -> &DrmSession {
        let session = DrmSession::new(system);
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        self.sessions.get(&id).unwrap()
    }

    /// Update session with license response
    pub fn process_license(&mut self, session_id: &str, response: LicenseResponse) -> Result<()> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| Error::drm("Session not found"))?;

        session.state = DrmSessionState::Ready;
        session.expiration = response.expiration;

        Ok(())
    }

    /// Get all active sessions
    pub fn sessions(&self) -> impl Iterator<Item = &DrmSession> {
        self.sessions.values()
    }

    /// Get session by ID
    pub fn get_session(&self, id: &str) -> Option<&DrmSession> {
        self.sessions.get(id)
    }

    /// Close a session
    pub fn close_session(&mut self, id: &str) {
        self.sessions.remove(id);
    }

    /// Close all sessions
    pub fn close_all_sessions(&mut self) {
        self.sessions.clear();
    }

    /// Check if DRM is required for playback
    pub fn is_drm_required(&self) -> bool {
        !self.pssh_boxes.is_empty()
    }

    /// Get best available DRM system
    pub fn select_drm_system(&self) -> Option<DrmSystem> {
        let supported = self.config.supported_systems();

        // Check what PSSH boxes we have
        for system in &[DrmSystem::Widevine, DrmSystem::FairPlay, DrmSystem::PlayReady, DrmSystem::ClearKey] {
            if supported.contains(system) && self.get_pssh(*system).is_some() {
                return Some(*system);
            }
        }

        // Fall back to ClearKey if configured (doesn't need PSSH)
        if !self.config.clearkey_keys.is_empty() {
            return Some(DrmSystem::ClearKey);
        }

        None
    }
}

// Base64 encoding/decoding helpers (avoiding external dependency for core lib)
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b = match chunk.len() {
            3 => [chunk[0], chunk[1], chunk[2], 0],
            2 => [chunk[0], chunk[1], 0, 0],
            1 => [chunk[0], 0, 0, 0],
            _ => continue,
        };

        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);

        result.push(ALPHABET[((n >> 18) & 0x3F) as usize] as char);
        result.push(ALPHABET[((n >> 12) & 0x3F) as usize] as char);
        result.push(if chunk.len() > 1 { ALPHABET[((n >> 6) & 0x3F) as usize] as char } else { '=' });
        result.push(if chunk.len() > 2 { ALPHABET[(n & 0x3F) as usize] as char } else { '=' });
    }
    result
}

fn base64_decode(data: &str) -> Result<Vec<u8>> {
    const DECODE_TABLE: &[i8; 128] = &[
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1, -1, 63,
        52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1,
        -1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14,
        15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1, -1,
        -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
        41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];

    let input: Vec<u8> = data.bytes()
        .filter(|b| *b != b'=' && *b != b'\n' && *b != b'\r')
        .collect();

    let mut result = Vec::with_capacity(input.len() * 3 / 4);

    for chunk in input.chunks(4) {
        let mut n: u32 = 0;
        let chunk_len = chunk.len();

        for (i, &b) in chunk.iter().enumerate() {
            if b as usize >= 128 {
                return Err(Error::drm("Invalid base64 character"));
            }
            let val = DECODE_TABLE[b as usize];
            if val < 0 {
                return Err(Error::drm("Invalid base64 character"));
            }
            n |= (val as u32) << (18 - i * 6);
        }

        // Output bytes based on how many input characters we had
        result.push((n >> 16) as u8);
        if chunk_len > 2 {
            result.push((n >> 8) as u8);
        }
        if chunk_len > 3 {
            result.push(n as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drm_config() {
        let config = DrmConfig::default();
        assert!(!config.is_configured());

        let config = DrmConfig::widevine(Url::parse("https://license.example.com").unwrap());
        assert!(config.is_configured());
        assert!(config.supported_systems().contains(&DrmSystem::Widevine));
    }

    #[test]
    fn test_pssh_box() {
        let pssh = PsshBox::new(DrmSystem::Widevine.system_id(), b"test data");
        assert_eq!(pssh.drm_system(), Some(DrmSystem::Widevine));
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"Hello, DRM!";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(original.to_vec(), decoded);
    }

    #[test]
    fn test_clearkey_license() {
        let mut keys = HashMap::new();
        keys.insert("abc123".to_string(), "key456".to_string());

        let config = DrmConfig::clearkey(keys);
        let manager = DrmManager::new(config);

        let license = manager.get_clearkey_license().unwrap();
        assert_eq!(license.system, DrmSystem::ClearKey);
    }
}
