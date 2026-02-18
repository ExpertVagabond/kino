//! Integration tests for Kino Core

use kino_core::{
    PlayerConfig, PlayerState, Resolution,
    KinoColors, KinoTheme, CssVariables,
    DrmConfig, DrmManager, DrmSystem,
    WebVttParser, SrtParser,
    AbrAlgorithmType,
};

// =============================================================================
// Branding Tests
// =============================================================================

#[test]
fn test_branding_colors() {
    let colors = KinoColors::default();
    assert_eq!(colors.primary, "#9b30ff");
    assert_eq!(colors.background, "#0c0a12");
    assert_eq!(colors.text, "#f6f2ff");
}

#[test]
fn test_branding_rgba() {
    let colors = KinoColors::default();
    assert_eq!(colors.primary_rgba(0.5), "rgba(155, 48, 255, 0.5)");
    assert_eq!(colors.background_rgba(0.9), "rgba(12, 10, 18, 0.9)");
}

#[test]
fn test_css_variables() {
    let css = CssVariables::generate();
    assert!(css.contains("--kino-primary: #9b30ff"));
    assert!(css.contains("--plyr-color-main: #9b30ff"));
    assert!(css.contains(":root {"));
}

#[test]
fn test_theme_json() {
    let theme = KinoTheme::default();
    let json = theme.to_json();
    assert!(json.contains("#9b30ff"));
    assert!(json.contains("border_radius"));
}

// =============================================================================
// Types Tests
// =============================================================================

#[test]
fn test_resolution_quality_name() {
    assert_eq!(Resolution::new(854, 480).quality_name(), "480p");
    assert_eq!(Resolution::new(1280, 720).quality_name(), "720p");
    assert_eq!(Resolution::new(1920, 1080).quality_name(), "1080p");
    assert_eq!(Resolution::new(3840, 2160).quality_name(), "4K");
}

#[test]
fn test_player_state_transitions() {
    // Valid transitions
    assert!(PlayerState::Idle.can_transition_to(PlayerState::Loading));
    assert!(PlayerState::Loading.can_transition_to(PlayerState::Buffering));
    assert!(PlayerState::Buffering.can_transition_to(PlayerState::Playing));
    assert!(PlayerState::Playing.can_transition_to(PlayerState::Paused));
    assert!(PlayerState::Paused.can_transition_to(PlayerState::Playing));

    // Invalid transitions
    assert!(!PlayerState::Idle.can_transition_to(PlayerState::Playing));
    assert!(!PlayerState::Playing.can_transition_to(PlayerState::Idle));
}

#[test]
fn test_player_config_defaults() {
    let config = PlayerConfig::default();
    assert_eq!(config.min_buffer_time, 10.0);
    assert_eq!(config.max_buffer_time, 30.0);
    assert_eq!(config.abr_algorithm, AbrAlgorithmType::Bola);
    assert!(config.prefetch_enabled);
}

// =============================================================================
// DRM Tests
// =============================================================================

#[test]
fn test_drm_system_ids() {
    assert_eq!(
        DrmSystem::Widevine.system_id(),
        "edef8ba9-79d6-4ace-a3c8-27dcd51d21ed"
    );
    assert_eq!(
        DrmSystem::FairPlay.system_id(),
        "94ce86fb-07ff-4f43-adb8-93d2fa968ca2"
    );
}

#[test]
fn test_drm_config_widevine() {
    let url = url::Url::parse("https://license.example.com/widevine").unwrap();
    let config = DrmConfig::widevine(url);

    assert!(config.is_configured());
    assert!(config.supported_systems().contains(&DrmSystem::Widevine));
    assert!(!config.supported_systems().contains(&DrmSystem::FairPlay));
}

#[test]
fn test_drm_config_clearkey() {
    let mut keys = std::collections::HashMap::new();
    keys.insert("key-id-1".to_string(), "key-value-1".to_string());
    keys.insert("key-id-2".to_string(), "key-value-2".to_string());

    let config = DrmConfig::clearkey(keys);

    assert!(config.is_configured());
    assert!(config.supported_systems().contains(&DrmSystem::ClearKey));
}

#[test]
fn test_drm_manager_clearkey_license() {
    let mut keys = std::collections::HashMap::new();
    keys.insert("abc123".to_string(), "key456".to_string());

    let config = DrmConfig::clearkey(keys);
    let manager = DrmManager::new(config);

    let license = manager.get_clearkey_license().unwrap();
    assert_eq!(license.system, DrmSystem::ClearKey);
    assert!(!license.license.is_empty());
}

// =============================================================================
// Caption Parser Tests
// =============================================================================

#[test]
fn test_webvtt_basic_parsing() {
    let vtt = r#"WEBVTT

00:00:00.000 --> 00:00:04.000
First subtitle

00:00:05.000 --> 00:00:10.000
Second subtitle
"#;

    let cues = WebVttParser::parse(vtt).unwrap();
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[0].text, "First subtitle");
    assert_eq!(cues[0].start_time, 0.0);
    assert_eq!(cues[0].end_time, 4.0);
    assert_eq!(cues[1].text, "Second subtitle");
    assert_eq!(cues[1].start_time, 5.0);
}

#[test]
fn test_webvtt_multiline_cue() {
    let vtt = r#"WEBVTT

00:00:00.000 --> 00:00:04.000
Line one
Line two
Line three
"#;

    let cues = WebVttParser::parse(vtt).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].text, "Line one\nLine two\nLine three");
}

#[test]
fn test_webvtt_with_note() {
    let vtt = r#"WEBVTT

NOTE This is a comment

00:00:00.000 --> 00:00:04.000
Actual subtitle
"#;

    let cues = WebVttParser::parse(vtt).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].text, "Actual subtitle");
}

#[test]
fn test_webvtt_cue_settings() {
    let vtt = r#"WEBVTT

00:00:00.000 --> 00:00:04.000 align:center position:50% size:80%
Centered text
"#;

    let cues = WebVttParser::parse(vtt).unwrap();
    assert_eq!(cues.len(), 1);

    let settings = cues[0].settings.as_ref().unwrap();
    assert_eq!(settings.position, Some(50.0));
    assert_eq!(settings.size, Some(80.0));
}

#[test]
fn test_srt_basic_parsing() {
    let srt = r#"1
00:00:00,000 --> 00:00:04,000
First subtitle

2
00:00:05,000 --> 00:00:10,000
Second subtitle
"#;

    let cues = SrtParser::parse(srt).unwrap();
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[0].text, "First subtitle");
    assert_eq!(cues[1].text, "Second subtitle");
}

#[test]
fn test_timestamp_hours() {
    let vtt = r#"WEBVTT

01:30:00.000 --> 01:30:05.000
One and a half hours in
"#;

    let cues = WebVttParser::parse(vtt).unwrap();
    assert_eq!(cues[0].start_time, 5400.0); // 1.5 hours in seconds
}

#[test]
fn test_strip_vtt_tags() {
    assert_eq!(
        WebVttParser::strip_tags("<v Speaker>Hello, <b>world</b>!</v>"),
        "Hello, world!"
    );
    assert_eq!(
        WebVttParser::strip_tags("<c.yellow>Yellow text</c>"),
        "Yellow text"
    );
}

#[test]
fn test_srt_to_vtt_conversion() {
    let srt = "1\n00:00:00,000 --> 00:00:04,000\nHello!";
    let vtt = kino_core::captions::srt_to_vtt(srt);

    assert!(vtt.starts_with("WEBVTT"));
    assert!(vtt.contains("00:00:00.000 --> 00:00:04.000"));
}

#[test]
fn test_cue_active_at_time() {
    let vtt = r#"WEBVTT

00:00:00.000 --> 00:00:05.000
First

00:00:05.000 --> 00:00:10.000
Second
"#;

    let cues = WebVttParser::parse(vtt).unwrap();

    // At 2.5 seconds, first cue should be active
    let active = kino_core::captions::cues_at_time(&cues, 2.5);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].text, "First");

    // At 7.5 seconds, second cue should be active
    let active = kino_core::captions::cues_at_time(&cues, 7.5);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].text, "Second");

    // At 12 seconds, no cue should be active
    let active = kino_core::captions::cues_at_time(&cues, 12.0);
    assert!(active.is_empty());
}

// =============================================================================
// Chapter Tests
// =============================================================================

#[test]
fn test_chapter_contains_time() {
    let chapter = kino_core::Chapter::new("ch1", "Introduction", 0.0, 60.0);

    assert!(chapter.contains_time(0.0));
    assert!(chapter.contains_time(30.0));
    assert!(chapter.contains_time(59.99));
    assert!(!chapter.contains_time(60.0));
    assert!(!chapter.contains_time(-1.0));
}

#[test]
fn test_chapter_duration() {
    let chapter = kino_core::Chapter::new("ch1", "Introduction", 10.0, 70.0);
    assert_eq!(chapter.duration(), 60.0);
}

// =============================================================================
// Media Tracks Tests
// =============================================================================

#[test]
fn test_media_tracks_chapter_at() {
    use kino_core::{MediaTracks, Chapter};

    let mut tracks = MediaTracks::new();
    tracks.add_chapter(Chapter::new("ch1", "Intro", 0.0, 60.0));
    tracks.add_chapter(Chapter::new("ch2", "Main", 60.0, 180.0));
    tracks.add_chapter(Chapter::new("ch3", "Outro", 180.0, 240.0));

    let ch = tracks.chapter_at(30.0);
    assert!(ch.is_some());
    assert_eq!(ch.unwrap().title, "Intro");

    let ch = tracks.chapter_at(120.0);
    assert!(ch.is_some());
    assert_eq!(ch.unwrap().title, "Main");

    let ch = tracks.chapter_at(300.0);
    assert!(ch.is_none());
}
