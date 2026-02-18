//! Tauri IPC commands
//!
//! Lightweight commands that work with the web frontend.
//! The actual video playback is handled by hls.js in the frontend.

use kino_core::{KinoColors, Chapter, TextTrack};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tauri::State;

/// Shared application state
pub struct AppState {
    pub current_url: Arc<RwLock<Option<String>>>,
    pub chapters: Arc<RwLock<Vec<Chapter>>>,
    pub text_tracks: Arc<RwLock<Vec<TextTrack>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_url: Arc::new(RwLock::new(None)),
            chapters: Arc::new(RwLock::new(Vec::new())),
            text_tracks: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Chapter info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInfo {
    pub id: String,
    pub title: String,
    pub start_time: f64,
    pub end_time: f64,
    pub thumbnail: Option<String>,
}

/// Text track info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextTrackInfo {
    pub id: String,
    pub kind: String,
    pub language: String,
    pub label: String,
    pub active: bool,
}

/// Theme colors for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeColors {
    pub primary: String,
    pub primary_dark: String,
    pub primary_deep: String,
    pub background: String,
    pub background_light: String,
    pub surface: String,
    pub text: String,
    pub text_soft: String,
}

// ============================================================================
// Tauri Commands - Frontend communicates directly with these
// ============================================================================

/// Store video URL (playback handled by frontend)
#[tauri::command]
pub async fn load_video(state: State<'_, AppState>, url: String) -> Result<(), String> {
    tracing::info!(url = %url, "Loading video");
    let mut current = state.current_url.write().await;
    *current = Some(url);
    Ok(())
}

/// Play - just logs, frontend handles
#[tauri::command]
pub async fn play(_state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Play requested");
    Ok(())
}

/// Pause - just logs, frontend handles
#[tauri::command]
pub async fn pause(_state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Pause requested");
    Ok(())
}

/// Stop - just logs, frontend handles
#[tauri::command]
pub async fn stop(_state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Stop requested");
    Ok(())
}

/// Seek - just logs, frontend handles
#[tauri::command]
pub async fn seek(_state: State<'_, AppState>, position: f64) -> Result<(), String> {
    tracing::info!(position, "Seeking");
    Ok(())
}

/// Set volume - frontend handles
#[tauri::command]
pub async fn set_volume(_state: State<'_, AppState>, _volume: f64) -> Result<(), String> {
    Ok(())
}

/// Set muted - frontend handles
#[tauri::command]
pub async fn set_muted(_state: State<'_, AppState>, _muted: bool) -> Result<(), String> {
    Ok(())
}

/// Set playback rate - frontend handles
#[tauri::command]
pub async fn set_playback_rate(_state: State<'_, AppState>, _rate: f64) -> Result<(), String> {
    Ok(())
}

/// Get player state - frontend provides this
#[tauri::command]
pub async fn get_state(_state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "state": "idle",
        "position": 0.0,
        "duration": null
    }))
}

/// Get quality levels - frontend provides this
#[tauri::command]
pub async fn get_qualities(_state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

/// Set quality - frontend handles
#[tauri::command]
pub async fn set_quality(_state: State<'_, AppState>, _quality_id: String) -> Result<(), String> {
    Ok(())
}

/// Get chapters
#[tauri::command]
pub async fn get_chapters(state: State<'_, AppState>) -> Result<Vec<ChapterInfo>, String> {
    let chapters = state.chapters.read().await;
    Ok(chapters.iter().map(|c| ChapterInfo {
        id: c.id.clone(),
        title: c.title.clone(),
        start_time: c.start_time,
        end_time: c.end_time,
        thumbnail: c.thumbnail.as_ref().map(|u| u.to_string()),
    }).collect())
}

/// Get text tracks
#[tauri::command]
pub async fn get_text_tracks(state: State<'_, AppState>) -> Result<Vec<TextTrackInfo>, String> {
    let tracks = state.text_tracks.read().await;
    Ok(tracks.iter().map(|t| TextTrackInfo {
        id: t.id.clone(),
        kind: format!("{:?}", t.kind),
        language: t.language.clone(),
        label: t.label.clone(),
        active: t.is_default,
    }).collect())
}

/// Set text track
#[tauri::command]
pub async fn set_text_track(_state: State<'_, AppState>, _track_id: Option<String>) -> Result<(), String> {
    Ok(())
}

/// Get Kino theme colors
#[tauri::command]
pub fn get_theme() -> ThemeColors {
    let colors = KinoColors::default();
    ThemeColors {
        primary: colors.primary.to_string(),
        primary_dark: colors.primary_dark.to_string(),
        primary_deep: colors.primary_deep.to_string(),
        background: colors.background.to_string(),
        background_light: colors.background_light.to_string(),
        surface: colors.surface.to_string(),
        text: colors.text.to_string(),
        text_soft: colors.text_soft.to_string(),
    }
}

/// Get player version
#[tauri::command]
pub fn get_version() -> String {
    kino_core::VERSION.to_string()
}
