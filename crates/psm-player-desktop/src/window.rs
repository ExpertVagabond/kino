//! Window management for desktop player

/// Video window configuration
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "PSM Player".to_string(),
            width: 1280,
            height: 720,
            fullscreen: false,
        }
    }
}
