//! Kino Branding - Official Purple Squirrel Media color palette
//!
//! This module provides the single source of truth for all Kino branding colors.
//! Use these constants across all player implementations (WASM, native, CLI).
//!
//! # Usage
//!
//! ```rust
//! use kino_core::branding::{KinoColors, KinoTheme};
//!
//! let theme = KinoTheme::default();
//! println!("Primary color: {}", theme.colors.primary);
//! ```

use serde::{Deserialize, Serialize};

/// Official Kino color palette
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinoColors {
    /// Primary purple - #9b30ff (RGB: 155, 48, 255)
    pub primary: &'static str,
    /// Darker primary for hover states - #7a1fe8
    pub primary_dark: &'static str,
    /// Deep purple for accents - #3b0b7d
    pub primary_deep: &'static str,
    /// Main background color - #0c0a12
    pub background: &'static str,
    /// Lighter background for cards/panels - #0f0b18
    pub background_light: &'static str,
    /// Surface color for elevated elements - #1a1625
    pub surface: &'static str,
    /// Main text color - #f6f2ff
    pub text: &'static str,
    /// Soft/muted text color - #d4cde9
    pub text_soft: &'static str,
    /// Success color - #22c55e
    pub success: &'static str,
    /// Warning color - #f59e0b
    pub warning: &'static str,
    /// Error color - #ef4444
    pub error: &'static str,
}

impl Default for KinoColors {
    fn default() -> Self {
        Self {
            primary: "#9b30ff",
            primary_dark: "#7a1fe8",
            primary_deep: "#3b0b7d",
            background: "#0c0a12",
            background_light: "#0f0b18",
            surface: "#1a1625",
            text: "#f6f2ff",
            text_soft: "#d4cde9",
            success: "#22c55e",
            warning: "#f59e0b",
            error: "#ef4444",
        }
    }
}

impl KinoColors {
    /// Get primary color as RGB tuple
    pub fn primary_rgb(&self) -> (u8, u8, u8) {
        (155, 48, 255)
    }

    /// Get primary color as RGBA with custom alpha
    pub fn primary_rgba(&self, alpha: f32) -> String {
        format!("rgba(155, 48, 255, {})", alpha)
    }

    /// Get background as RGBA with custom alpha
    pub fn background_rgba(&self, alpha: f32) -> String {
        format!("rgba(12, 10, 18, {})", alpha)
    }
}

/// CSS variable definitions for web players
pub struct CssVariables;

impl CssVariables {
    /// Generate CSS custom properties for the Kino theme
    pub fn generate() -> String {
        let colors = KinoColors::default();
        format!(
            r#":root {{
  /* Kino Primary Colors */
  --kino-primary: {};
  --kino-primary-dark: {};
  --kino-primary-deep: {};

  /* Kino Background Colors */
  --kino-background: {};
  --kino-background-light: {};
  --kino-surface: {};

  /* Kino Text Colors */
  --kino-text: {};
  --kino-text-soft: {};

  /* Kino Status Colors */
  --kino-success: {};
  --kino-warning: {};
  --kino-error: {};

  /* Kino Gradients */
  --kino-gradient-primary: linear-gradient(145deg, {}, {});
  --kino-gradient-controls: linear-gradient(transparent, rgba(12, 10, 18, 0.9));

  /* Kino Shadows */
  --kino-shadow-primary: 0 4px 20px rgba(155, 48, 255, 0.4);
  --kino-shadow-glow: 0 0 10px rgba(155, 48, 255, 0.5);

  /* Plyr compatibility */
  --plyr-color-main: {};
  --plyr-video-background: {};
  --plyr-menu-background: rgba(12, 10, 18, 0.95);
  --plyr-menu-color: {};
}}"#,
            colors.primary,
            colors.primary_dark,
            colors.primary_deep,
            colors.background,
            colors.background_light,
            colors.surface,
            colors.text,
            colors.text_soft,
            colors.success,
            colors.warning,
            colors.error,
            colors.primary_dark,
            colors.primary_deep,
            colors.primary,
            colors.background,
            colors.text,
        )
    }

    /// Generate player-specific CSS
    pub fn player_css() -> String {
        r#"
/* Kino Styles */
.kino {
  background: var(--kino-background);
  font-family: system-ui, -apple-system, sans-serif;
}

.kino__controls {
  background: var(--kino-gradient-controls) !important;
}

.kino__play-button {
  background: var(--kino-gradient-primary) !important;
  box-shadow: var(--kino-shadow-primary);
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.kino__play-button:hover {
  transform: scale(1.05);
  box-shadow: var(--kino-shadow-glow);
}

.kino__progress {
  background: var(--kino-background-light);
}

.kino__progress-bar {
  background: var(--kino-primary);
}

.kino__tooltip {
  background: rgba(12, 10, 18, 0.95);
  color: var(--kino-text);
  border: 1px solid rgba(155, 48, 255, 0.3);
}

.kino__menu {
  background: rgba(12, 10, 18, 0.95);
  border: 1px solid rgba(155, 48, 255, 0.3);
  color: var(--kino-text);
}

.kino__watermark {
  position: absolute;
  bottom: 45px;
  right: 10px;
  font-size: 10px;
  color: rgba(155, 48, 255, 0.3);
  pointer-events: none;
  z-index: 1;
}
"#.to_string()
    }
}

/// Complete Kino theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KinoTheme {
    /// Color palette
    pub colors: KinoColors,
    /// Border radius for UI elements
    pub border_radius: u8,
    /// Show Purple Squirrel watermark
    pub show_watermark: bool,
    /// Watermark text
    pub watermark_text: &'static str,
}

impl Default for KinoTheme {
    fn default() -> Self {
        Self {
            colors: KinoColors::default(),
            border_radius: 8,
            show_watermark: true,
            watermark_text: "Kino",
        }
    }
}

impl KinoTheme {
    /// Create theme with no watermark
    pub fn no_watermark() -> Self {
        Self {
            show_watermark: false,
            ..Default::default()
        }
    }

    /// Create theme with custom watermark
    pub fn with_watermark(text: &'static str) -> Self {
        Self {
            watermark_text: text,
            ..Default::default()
        }
    }

    /// Export theme as JSON for JS interop
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Generate a complete CSS stylesheet
    pub fn to_css(&self) -> String {
        format!("{}\n{}", CssVariables::generate(), CssVariables::player_css())
    }
}

/// JavaScript-compatible theme object for hls.js/React integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsTheme {
    pub primary_color: String,
    pub controls_background: String,
    pub progress_color: String,
    pub buffer_color: String,
    pub text_color: String,
    pub border_radius: u8,
}

impl Default for JsTheme {
    fn default() -> Self {
        let colors = KinoColors::default();
        Self {
            primary_color: colors.primary.to_string(),
            controls_background: colors.background_rgba(0.7),
            progress_color: colors.primary.to_string(),
            buffer_color: "rgba(255, 255, 255, 0.3)".to_string(),
            text_color: colors.text.to_string(),
            border_radius: 8,
        }
    }
}

impl JsTheme {
    /// Export as JSON for JavaScript consumption
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_colors() {
        let colors = KinoColors::default();
        assert_eq!(colors.primary, "#9b30ff");
        assert_eq!(colors.background, "#0c0a12");
    }

    #[test]
    fn test_rgba_generation() {
        let colors = KinoColors::default();
        assert_eq!(colors.primary_rgba(0.5), "rgba(155, 48, 255, 0.5)");
    }

    #[test]
    fn test_css_generation() {
        let css = CssVariables::generate();
        assert!(css.contains("--kino-primary: #9b30ff"));
        assert!(css.contains("--plyr-color-main: #9b30ff"));
    }

    #[test]
    fn test_theme_json() {
        let theme = KinoTheme::default();
        let json = theme.to_json();
        assert!(json.contains("#9b30ff"));
    }
}
