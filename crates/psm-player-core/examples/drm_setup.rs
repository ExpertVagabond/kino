//! DRM setup example
//!
//! Demonstrates how to configure DRM for protected content playback.
//!
//! Run with: cargo run -p psm-player-core --example drm_setup

use psm_player_core::{DrmConfig, DrmManager, DrmSystem, PsshBox};
use std::collections::HashMap;
use url::Url;

fn main() {
    println!("PSM Player Core - DRM Setup Example");
    println!("====================================\n");

    // Example 1: Widevine DRM setup
    println!("1. Widevine DRM Configuration");
    println!("------------------------------");

    let widevine_url = Url::parse("https://license.example.com/widevine").unwrap();
    let widevine_config = DrmConfig::widevine(widevine_url.clone());

    println!("  License URL: {}", widevine_url);
    println!("  Configured: {}", widevine_config.is_configured());
    println!("  Supported systems: {:?}\n", widevine_config.supported_systems());

    // Example 2: FairPlay DRM setup
    println!("2. FairPlay DRM Configuration");
    println!("------------------------------");

    let fairplay_url = Url::parse("https://license.example.com/fairplay").unwrap();
    let cert_url = Url::parse("https://license.example.com/fairplay/cert").unwrap();
    let fairplay_config = DrmConfig::fairplay(fairplay_url.clone(), cert_url.clone());

    println!("  License URL: {}", fairplay_url);
    println!("  Certificate URL: {}", cert_url);
    println!("  Configured: {}", fairplay_config.is_configured());
    println!("  Supported systems: {:?}\n", fairplay_config.supported_systems());

    // Example 3: ClearKey DRM (for testing)
    println!("3. ClearKey DRM Configuration (for testing)");
    println!("--------------------------------------------");

    let mut keys = HashMap::new();
    keys.insert("key-id-001".to_string(), "secret-key-value-001".to_string());
    keys.insert("key-id-002".to_string(), "secret-key-value-002".to_string());

    let clearkey_config = DrmConfig::clearkey(keys);
    let manager = DrmManager::new(clearkey_config);

    println!("  Keys configured: 2");
    println!("  Getting license...\n");

    match manager.get_clearkey_license() {
        Ok(license) => {
            println!("  License obtained:");
            println!("    System: {:?}", license.system);
            // The license data is bytes, so show length instead
            println!("    License data: {} bytes", license.license.len());
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }

    // Example 4: DRM System IDs
    println!("\n4. DRM System IDs (for manifest parsing)");
    println!("-----------------------------------------");

    let systems = [
        DrmSystem::Widevine,
        DrmSystem::FairPlay,
        DrmSystem::PlayReady,
        DrmSystem::ClearKey,
    ];

    for system in systems {
        println!("  {:?}: {}", system, system.system_id());
    }

    // Example 5: PSSH Box creation
    println!("\n5. PSSH Box Creation");
    println!("--------------------");

    // Create a simple PSSH box
    let pssh = PsshBox::new(DrmSystem::Widevine.system_id(), b"sample-data");
    println!("  Created PSSH box");
    println!("  System ID: {}", pssh.system_id);
    println!("  DRM System: {:?}", pssh.drm_system());
    println!("  Data (base64): {}", pssh.data);

    // Example 6: Adding custom headers
    println!("\n6. Custom License Request Headers");
    println!("----------------------------------");

    let config_with_headers = DrmConfig::widevine(
        Url::parse("https://license.example.com/widevine").unwrap()
    )
    .with_header("X-Custom-Token", "abc123")
    .with_header("Authorization", "Bearer token-here");

    println!("  Custom headers added:");
    for (key, value) in &config_with_headers.license_headers {
        println!("    {}: {}", key, if key == "Authorization" { "[redacted]" } else { value });
    }

    println!("\nDRM setup examples complete!");
}
