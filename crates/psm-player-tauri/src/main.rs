//! PSM Player Tauri - Cross-platform Desktop Application
//!
//! Combines Rust backend with web frontend for a native experience.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use commands::AppState;
use tauri::Manager;

mod commands;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,psm_player=debug".to_string())
        )
        .init();

    tracing::info!(
        version = psm_player_core::VERSION,
        "Starting PSM Player"
    );

    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Playback control
            commands::load_video,
            commands::play,
            commands::pause,
            commands::stop,
            commands::seek,
            commands::set_volume,
            commands::set_muted,
            commands::set_playback_rate,
            // State queries
            commands::get_state,
            commands::get_qualities,
            commands::set_quality,
            // Chapters & tracks
            commands::get_chapters,
            commands::get_text_tracks,
            commands::set_text_track,
            // Theme & info
            commands::get_theme,
            commands::get_version,
        ])
        .setup(|app| {
            tracing::info!("PSM Player initialized");

            // Open devtools in debug mode
            #[cfg(debug_assertions)]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.open_devtools();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running PSM Player");
}
