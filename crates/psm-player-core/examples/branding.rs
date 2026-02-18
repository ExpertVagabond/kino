//! Branding and theming example
//!
//! Demonstrates how to use PSM branding colors and generate CSS.
//!
//! Run with: cargo run -p psm-player-core --example branding

use psm_player_core::{PsmColors, PsmTheme, CssVariables};

fn main() {
    println!("PSM Player Core - Branding Example");
    println!("===================================\n");

    // Get default PSM colors
    let colors = PsmColors::default();

    println!("PSM Brand Colors:");
    println!("-----------------");
    println!("  Primary:          {} (Purple Squirrel signature)", colors.primary);
    println!("  Primary Dark:     {} (hover states)", colors.primary_dark);
    println!("  Primary Deep:     {} (accents)", colors.primary_deep);
    println!("  Background:       {} (main bg)", colors.background);
    println!("  Background Light: {} (cards)", colors.background_light);
    println!("  Surface:          {} (elevated)", colors.surface);
    println!("  Text:             {} (primary text)", colors.text);
    println!("  Text Soft:        {} (secondary text)", colors.text_soft);
    println!("  Success:          {}", colors.success);
    println!("  Warning:          {}", colors.warning);
    println!("  Error:            {}", colors.error);
    println!();

    // Generate RGBA colors with custom alpha
    println!("RGBA Color Generation:");
    println!("----------------------");
    println!("  Primary at 50%:    {}", colors.primary_rgba(0.5));
    println!("  Primary at 20%:    {}", colors.primary_rgba(0.2));
    println!("  Background at 90%: {}", colors.background_rgba(0.9));
    println!("  Background at 75%: {}", colors.background_rgba(0.75));
    println!();

    // Generate CSS variables
    println!("CSS Variables (for web integration):");
    println!("-------------------------------------");
    let css = CssVariables::generate();
    // Show first few lines
    for line in css.lines().take(15) {
        println!("  {}", line);
    }
    println!("  ...");
    println!();

    // Get full theme
    let theme = PsmTheme::default();

    println!("Theme Configuration:");
    println!("--------------------");
    println!("  Border Radius: {}px", theme.border_radius);
    println!("  Watermark: {}", if theme.show_watermark { "enabled" } else { "disabled" });
    println!("  Watermark Text: {}", theme.watermark_text);
    println!();

    // Export theme as JSON (for JavaScript/TypeScript)
    println!("Theme as JSON (for JS integration):");
    println!("------------------------------------");
    let json = theme.to_json();
    // Pretty print with indentation
    for (i, line) in json.lines().enumerate() {
        if i < 10 {
            println!("  {}", line);
        }
    }
    if json.lines().count() > 10 {
        println!("  ...");
    }
    println!();

    // Show player-specific CSS
    println!("Player CSS Snippet:");
    println!("-------------------");
    let player_css = generate_player_css(&colors);
    for line in player_css.lines().take(20) {
        println!("  {}", line);
    }
    println!("  ...");
    println!();

    println!("Use these colors to maintain consistent Purple Squirrel branding!");
}

/// Generate sample player-specific CSS
fn generate_player_css(colors: &PsmColors) -> String {
    format!(
        r#".psm-player {{
  --control-bg: {};
  --control-bg-hover: {};
  --progress-track: rgba(255, 255, 255, 0.2);
  --progress-fill: {};
  --progress-buffer: rgba(155, 48, 255, 0.3);
}}

.psm-player-controls {{
  background: linear-gradient(
    to top,
    {} 0%,
    transparent 100%
  );
}}

.psm-play-button {{
  background: {};
  border-radius: 50%;
  transition: transform 0.2s, background 0.2s;
}}

.psm-play-button:hover {{
  background: {};
  transform: scale(1.1);
}}"#,
        colors.background_rgba(0.9),
        colors.surface,
        colors.primary,
        colors.background_rgba(0.95),
        colors.primary,
        colors.primary_dark,
    )
}
