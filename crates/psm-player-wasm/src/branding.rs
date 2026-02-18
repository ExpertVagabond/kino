//! PSM Branding - WASM-compatible color palette and theming

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

/// PSM color palette constants
pub struct Colors;

impl Colors {
    pub const PRIMARY: &'static str = "#9b30ff";
    pub const PRIMARY_DARK: &'static str = "#7a1fe8";
    pub const PRIMARY_DEEP: &'static str = "#3b0b7d";
    pub const BACKGROUND: &'static str = "#0c0a12";
    pub const BACKGROUND_LIGHT: &'static str = "#0f0b18";
    pub const SURFACE: &'static str = "#1a1625";
    pub const TEXT: &'static str = "#f6f2ff";
    pub const TEXT_SOFT: &'static str = "#d4cde9";
    pub const SUCCESS: &'static str = "#22c55e";
    pub const WARNING: &'static str = "#f59e0b";
    pub const ERROR: &'static str = "#ef4444";
}

/// PSM branding colors exposed to JavaScript
#[wasm_bindgen]
pub struct PsmBranding;

#[wasm_bindgen]
impl PsmBranding {
    #[wasm_bindgen(getter)]
    pub fn primary() -> String { Colors::PRIMARY.to_string() }

    #[wasm_bindgen(getter)]
    pub fn primary_dark() -> String { Colors::PRIMARY_DARK.to_string() }

    #[wasm_bindgen(getter)]
    pub fn primary_deep() -> String { Colors::PRIMARY_DEEP.to_string() }

    #[wasm_bindgen(getter)]
    pub fn background() -> String { Colors::BACKGROUND.to_string() }

    #[wasm_bindgen(getter)]
    pub fn background_light() -> String { Colors::BACKGROUND_LIGHT.to_string() }

    #[wasm_bindgen(getter)]
    pub fn surface() -> String { Colors::SURFACE.to_string() }

    #[wasm_bindgen(getter)]
    pub fn text() -> String { Colors::TEXT.to_string() }

    #[wasm_bindgen(getter)]
    pub fn text_soft() -> String { Colors::TEXT_SOFT.to_string() }

    /// Get primary color as RGBA with custom alpha
    #[wasm_bindgen]
    pub fn primary_rgba(alpha: f32) -> String {
        format!("rgba(155, 48, 255, {})", alpha)
    }

    /// Get background color as RGBA with custom alpha
    #[wasm_bindgen]
    pub fn background_rgba(alpha: f32) -> String {
        format!("rgba(12, 10, 18, {})", alpha)
    }

    /// Get complete CSS variables for the PSM theme
    #[wasm_bindgen]
    pub fn get_css_variables() -> String {
        format!(
            r#":root {{
  /* PSM Primary Colors */
  --psm-primary: {};
  --psm-primary-dark: {};
  --psm-primary-deep: {};

  /* PSM Background Colors */
  --psm-background: {};
  --psm-background-light: {};
  --psm-surface: {};

  /* PSM Text Colors */
  --psm-text: {};
  --psm-text-soft: {};

  /* PSM Status Colors */
  --psm-success: {};
  --psm-warning: {};
  --psm-error: {};

  /* PSM Gradients */
  --psm-gradient-primary: linear-gradient(145deg, {}, {});
  --psm-gradient-controls: linear-gradient(transparent, rgba(12, 10, 18, 0.9));

  /* PSM Shadows */
  --psm-shadow-primary: 0 4px 20px rgba(155, 48, 255, 0.4);
  --psm-shadow-glow: 0 0 10px rgba(155, 48, 255, 0.5);

  /* Plyr/hls.js compatibility */
  --plyr-color-main: {};
  --plyr-video-background: {};
  --plyr-menu-background: rgba(12, 10, 18, 0.95);
  --plyr-menu-color: {};
}}"#,
            Colors::PRIMARY,
            Colors::PRIMARY_DARK,
            Colors::PRIMARY_DEEP,
            Colors::BACKGROUND,
            Colors::BACKGROUND_LIGHT,
            Colors::SURFACE,
            Colors::TEXT,
            Colors::TEXT_SOFT,
            Colors::SUCCESS,
            Colors::WARNING,
            Colors::ERROR,
            Colors::PRIMARY_DARK,
            Colors::PRIMARY_DEEP,
            Colors::PRIMARY,
            Colors::BACKGROUND,
            Colors::TEXT,
        )
    }

    /// Get complete player CSS stylesheet
    #[wasm_bindgen]
    pub fn get_player_css() -> String {
        r#"
/* PSM Player Styles */
.psm-player {
  background: var(--psm-background);
  font-family: system-ui, -apple-system, sans-serif;
}

.psm-player__controls {
  background: var(--psm-gradient-controls) !important;
}

.psm-player__play-button {
  background: var(--psm-gradient-primary) !important;
  box-shadow: var(--psm-shadow-primary);
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

.psm-player__play-button:hover {
  transform: scale(1.05);
  box-shadow: var(--psm-shadow-glow);
}

.psm-player__progress {
  background: var(--psm-background-light);
}

.psm-player__progress-bar {
  background: var(--psm-primary);
}

.psm-player__tooltip {
  background: rgba(12, 10, 18, 0.95);
  color: var(--psm-text);
  border: 1px solid rgba(155, 48, 255, 0.3);
}
"#.to_string()
    }

    /// Get theme as JSON object
    #[wasm_bindgen]
    pub fn get_theme_json() -> String {
        serde_json::json!({
            "colors": {
                "primary": Colors::PRIMARY,
                "primary_dark": Colors::PRIMARY_DARK,
                "primary_deep": Colors::PRIMARY_DEEP,
                "background": Colors::BACKGROUND,
                "background_light": Colors::BACKGROUND_LIGHT,
                "surface": Colors::SURFACE,
                "text": Colors::TEXT,
                "text_soft": Colors::TEXT_SOFT,
                "success": Colors::SUCCESS,
                "warning": Colors::WARNING,
                "error": Colors::ERROR
            },
            "border_radius": 8,
            "show_watermark": true,
            "watermark_text": "Purple Squirrel"
        }).to_string()
    }
}
