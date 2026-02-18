//! Output formatting for CLI

use serde::Serialize;

/// Output format options
#[allow(dead_code)]
pub enum OutputFormat {
    Text,
    Json,
    Table,
}

impl From<&str> for OutputFormat {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "table" => OutputFormat::Table,
            _ => OutputFormat::Text,
        }
    }
}

/// Format output based on selected format
#[allow(dead_code)]
pub fn format_output<T: Serialize>(data: &T, format: &str) -> String {
    match OutputFormat::from(format) {
        OutputFormat::Json => {
            serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string())
        }
        OutputFormat::Table | OutputFormat::Text => {
            // For text, we rely on Display implementations
            format!("{:?}", serde_json::to_value(data).unwrap_or_default())
        }
    }
}
