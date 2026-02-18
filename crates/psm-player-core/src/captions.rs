//! Caption parsing - WebVTT and SRT parser
//!
//! Provides parsers for common caption/subtitle formats:
//! - WebVTT (Web Video Text Tracks)
//! - SRT (SubRip)
//!
//! # Example
//!
//! ```rust
//! use psm_player_core::captions::WebVttParser;
//!
//! let vtt = r#"WEBVTT
//!
//! 00:00:00.000 --> 00:00:04.000
//! Hello, world!
//!
//! 00:00:04.000 --> 00:00:08.000
//! This is a subtitle.
//! "#;
//!
//! let cues = WebVttParser::parse(vtt).unwrap();
//! assert_eq!(cues.len(), 2);
//! ```

use crate::error::{Error, Result};
use crate::types::{TextCue, CueSettings, CueAlignment};
use std::str::FromStr;

/// WebVTT parser
pub struct WebVttParser;

impl WebVttParser {
    /// Parse a WebVTT string into a list of cues
    pub fn parse(input: &str) -> Result<Vec<TextCue>> {
        let mut cues = Vec::new();
        let mut lines = input.lines().peekable();

        // Check for WEBVTT header
        let first_line = lines.next().unwrap_or("");
        if !first_line.starts_with("WEBVTT") {
            return Err(Error::ManifestParse("Invalid WebVTT: missing WEBVTT header".to_string()));
        }

        // Skip header metadata until first blank line
        while let Some(line) = lines.peek() {
            if line.is_empty() {
                lines.next();
                break;
            }
            lines.next();
        }

        let mut cue_id = 0;
        while lines.peek().is_some() {
            // Skip blank lines
            while lines.peek().map(|l| l.is_empty()).unwrap_or(false) {
                lines.next();
            }

            if lines.peek().is_none() {
                break;
            }

            // Check for NOTE (comment)
            if lines.peek().map(|l| l.starts_with("NOTE")).unwrap_or(false) {
                // Skip comment block
                while let Some(line) = lines.next() {
                    if line.is_empty() {
                        break;
                    }
                }
                continue;
            }

            // Check for STYLE block
            if lines.peek().map(|l| l.starts_with("STYLE")).unwrap_or(false) {
                // Skip style block
                while let Some(line) = lines.next() {
                    if line.is_empty() {
                        break;
                    }
                }
                continue;
            }

            // Check for REGION block
            if lines.peek().map(|l| l.starts_with("REGION")).unwrap_or(false) {
                // Skip region block
                while let Some(line) = lines.next() {
                    if line.is_empty() {
                        break;
                    }
                }
                continue;
            }

            // Parse cue
            let mut id = None;
            let first_line = lines.next().unwrap_or("");

            // Check if this is a cue identifier (doesn't contain -->)
            let timing_line = if !first_line.contains("-->") {
                id = Some(first_line.to_string());
                lines.next().unwrap_or("")
            } else {
                first_line
            };

            // Parse timing line
            if !timing_line.contains("-->") {
                continue; // Invalid cue, skip
            }

            let (start_time, end_time, settings) = Self::parse_timing_line(timing_line)?;

            // Collect cue text
            let mut text = String::new();
            while let Some(line) = lines.peek() {
                if line.is_empty() {
                    break;
                }
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(lines.next().unwrap());
            }

            cue_id += 1;
            cues.push(TextCue {
                id: id.unwrap_or_else(|| format!("cue-{}", cue_id)),
                start_time,
                end_time,
                text,
                settings,
            });
        }

        Ok(cues)
    }

    /// Parse a timing line: "00:00:00.000 --> 00:00:04.000 align:center"
    fn parse_timing_line(line: &str) -> Result<(f64, f64, Option<CueSettings>)> {
        let parts: Vec<&str> = line.split("-->").collect();
        if parts.len() != 2 {
            return Err(Error::ManifestParse("Invalid timing line".to_string()));
        }

        let start = Self::parse_timestamp(parts[0].trim())?;

        // End time might have settings after it
        let end_parts: Vec<&str> = parts[1].trim().split_whitespace().collect();
        let end = Self::parse_timestamp(end_parts[0])?;

        // Parse settings
        let settings = if end_parts.len() > 1 {
            Some(Self::parse_settings(&end_parts[1..]))
        } else {
            None
        };

        Ok((start, end, settings))
    }

    /// Parse a timestamp: "00:00:00.000" or "00:00.000"
    fn parse_timestamp(ts: &str) -> Result<f64> {
        let parts: Vec<&str> = ts.split(':').collect();

        match parts.len() {
            // mm:ss.mmm
            2 => {
                let minutes: f64 = parts[0].parse()
                    .map_err(|_| Error::ManifestParse(format!("Invalid minutes: {}", parts[0])))?;
                let seconds = Self::parse_seconds(parts[1])?;
                Ok(minutes * 60.0 + seconds)
            }
            // hh:mm:ss.mmm
            3 => {
                let hours: f64 = parts[0].parse()
                    .map_err(|_| Error::ManifestParse(format!("Invalid hours: {}", parts[0])))?;
                let minutes: f64 = parts[1].parse()
                    .map_err(|_| Error::ManifestParse(format!("Invalid minutes: {}", parts[1])))?;
                let seconds = Self::parse_seconds(parts[2])?;
                Ok(hours * 3600.0 + minutes * 60.0 + seconds)
            }
            _ => Err(Error::ManifestParse(format!("Invalid timestamp: {}", ts))),
        }
    }

    /// Parse seconds with milliseconds: "00.000"
    fn parse_seconds(s: &str) -> Result<f64> {
        // Handle both . and , as decimal separator
        let s = s.replace(',', ".");
        s.parse()
            .map_err(|_| Error::ManifestParse(format!("Invalid seconds: {}", s)))
    }

    /// Parse cue settings
    fn parse_settings(parts: &[&str]) -> CueSettings {
        let mut settings = CueSettings {
            vertical: None,
            line: None,
            position: None,
            size: None,
            align: None,
        };

        for part in parts {
            if let Some((key, value)) = part.split_once(':') {
                match key {
                    "vertical" => settings.vertical = Some(value.to_string()),
                    "line" => settings.line = value.parse().ok(),
                    "position" => {
                        settings.position = value.trim_end_matches('%').parse().ok();
                    }
                    "size" => {
                        settings.size = value.trim_end_matches('%').parse().ok();
                    }
                    "align" => {
                        settings.align = match value {
                            "start" => Some(CueAlignment::Start),
                            "center" | "middle" => Some(CueAlignment::Center),
                            "end" => Some(CueAlignment::End),
                            "left" => Some(CueAlignment::Left),
                            "right" => Some(CueAlignment::Right),
                            _ => None,
                        };
                    }
                    _ => {}
                }
            }
        }

        settings
    }

    /// Strip VTT markup tags from text
    pub fn strip_tags(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut in_tag = false;

        for ch in text.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(ch),
                _ => {}
            }
        }

        result
    }
}

/// SRT (SubRip) parser
pub struct SrtParser;

impl SrtParser {
    /// Parse an SRT string into a list of cues
    pub fn parse(input: &str) -> Result<Vec<TextCue>> {
        let mut cues = Vec::new();
        let mut lines = input.lines().peekable();

        while lines.peek().is_some() {
            // Skip blank lines
            while lines.peek().map(|l| l.trim().is_empty()).unwrap_or(false) {
                lines.next();
            }

            if lines.peek().is_none() {
                break;
            }

            // Cue number
            let cue_number = lines.next().unwrap_or("").trim();
            if cue_number.is_empty() {
                continue;
            }

            // Timing line
            let timing_line = match lines.next() {
                Some(line) => line,
                None => break,
            };

            if !timing_line.contains("-->") {
                continue;
            }

            let (start_time, end_time) = Self::parse_timing_line(timing_line)?;

            // Collect cue text
            let mut text = String::new();
            while let Some(line) = lines.peek() {
                if line.trim().is_empty() {
                    break;
                }
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(lines.next().unwrap());
            }

            cues.push(TextCue {
                id: format!("srt-{}", cue_number),
                start_time,
                end_time,
                text,
                settings: None,
            });
        }

        Ok(cues)
    }

    /// Parse timing line: "00:00:00,000 --> 00:00:04,000"
    fn parse_timing_line(line: &str) -> Result<(f64, f64)> {
        let parts: Vec<&str> = line.split("-->").collect();
        if parts.len() != 2 {
            return Err(Error::ManifestParse("Invalid SRT timing line".to_string()));
        }

        let start = Self::parse_timestamp(parts[0].trim())?;
        let end = Self::parse_timestamp(parts[1].trim())?;

        Ok((start, end))
    }

    /// Parse timestamp: "00:00:00,000"
    fn parse_timestamp(ts: &str) -> Result<f64> {
        let parts: Vec<&str> = ts.split(':').collect();
        if parts.len() != 3 {
            return Err(Error::ManifestParse(format!("Invalid SRT timestamp: {}", ts)));
        }

        let hours: f64 = parts[0].parse()
            .map_err(|_| Error::ManifestParse(format!("Invalid hours: {}", parts[0])))?;
        let minutes: f64 = parts[1].parse()
            .map_err(|_| Error::ManifestParse(format!("Invalid minutes: {}", parts[1])))?;

        // SRT uses comma as decimal separator
        let seconds: f64 = parts[2].replace(',', ".").parse()
            .map_err(|_| Error::ManifestParse(format!("Invalid seconds: {}", parts[2])))?;

        Ok(hours * 3600.0 + minutes * 60.0 + seconds)
    }

    /// Strip HTML tags from SRT text
    pub fn strip_tags(text: &str) -> String {
        WebVttParser::strip_tags(text)
    }
}

/// Convert SRT to WebVTT format
pub fn srt_to_vtt(srt: &str) -> String {
    let mut vtt = String::from("WEBVTT\n\n");

    for line in srt.lines() {
        if line.contains("-->") {
            // Convert SRT timing to VTT (replace comma with period)
            vtt.push_str(&line.replace(',', "."));
        } else {
            vtt.push_str(line);
        }
        vtt.push('\n');
    }

    vtt
}

/// Find cues active at a given time
pub fn cues_at_time(cues: &[TextCue], time: f64) -> Vec<&TextCue> {
    cues.iter().filter(|c| c.is_active_at(time)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_webvtt() {
        let vtt = r#"WEBVTT

00:00:00.000 --> 00:00:04.000
Hello, world!

00:00:04.000 --> 00:00:08.000
This is a subtitle.
"#;

        let cues = WebVttParser::parse(vtt).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Hello, world!");
        assert_eq!(cues[0].start_time, 0.0);
        assert_eq!(cues[0].end_time, 4.0);
    }

    #[test]
    fn test_parse_webvtt_with_settings() {
        let vtt = r#"WEBVTT

00:00:00.000 --> 00:00:04.000 align:center position:50%
Centered text
"#;

        let cues = WebVttParser::parse(vtt).unwrap();
        assert_eq!(cues.len(), 1);
        let settings = cues[0].settings.as_ref().unwrap();
        assert_eq!(settings.align, Some(CueAlignment::Center));
        assert_eq!(settings.position, Some(50.0));
    }

    #[test]
    fn test_parse_srt() {
        let srt = r#"1
00:00:00,000 --> 00:00:04,000
Hello, world!

2
00:00:04,000 --> 00:00:08,000
This is a subtitle.
"#;

        let cues = SrtParser::parse(srt).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "Hello, world!");
    }

    #[test]
    fn test_timestamp_parsing() {
        assert_eq!(WebVttParser::parse_timestamp("00:00:05.500").unwrap(), 5.5);
        assert_eq!(WebVttParser::parse_timestamp("01:30:00.000").unwrap(), 5400.0);
        assert_eq!(WebVttParser::parse_timestamp("05:30.000").unwrap(), 330.0);
    }

    #[test]
    fn test_strip_tags() {
        let text = "<v Speaker>Hello, <b>world</b>!</v>";
        assert_eq!(WebVttParser::strip_tags(text), "Hello, world!");
    }

    #[test]
    fn test_srt_to_vtt() {
        let srt = "1\n00:00:00,000 --> 00:00:04,000\nHello!";
        let vtt = srt_to_vtt(srt);
        assert!(vtt.starts_with("WEBVTT"));
        assert!(vtt.contains("00:00:00.000 --> 00:00:04.000"));
    }
}
