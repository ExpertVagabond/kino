//! Caption parsing example
//!
//! Demonstrates how to parse WebVTT and SRT caption files.
//!
//! Run with: cargo run -p kino-core --example captions

use kino_core::captions::{WebVttParser, SrtParser, cues_at_time, srt_to_vtt};

fn main() {
    println!("Kino Core - Caption Parsing Example");
    println!("==========================================\n");

    // Example WebVTT content
    let webvtt = r#"WEBVTT

NOTE This is an example WebVTT file

STYLE
::cue {
  background-color: rgba(0, 0, 0, 0.8);
}

intro
00:00:00.000 --> 00:00:03.000
Welcome to Purple Squirrel Media!

00:00:03.500 --> 00:00:07.000 align:center position:50%
This video demonstrates our
amazing player capabilities.

action
00:00:08.000 --> 00:00:12.000
<v Speaker>Let's dive into the features.</v>

00:00:12.500 --> 00:00:15.000
<b>Bold text</b> and <i>italic text</i> are supported.

01:30:00.000 --> 01:30:05.000
This caption appears at 1 hour 30 minutes.
"#;

    println!("Parsing WebVTT content...\n");

    match WebVttParser::parse(webvtt) {
        Ok(cues) => {
            println!("Found {} cues:\n", cues.len());

            for (i, cue) in cues.iter().enumerate() {
                println!("Cue {}: \"{}\"", i + 1, cue.id);
                println!("  Time: {:.3}s -> {:.3}s", cue.start_time, cue.end_time);
                println!("  Duration: {:.3}s", cue.end_time - cue.start_time);

                // Show cleaned text (strip VTT tags)
                let clean_text = WebVttParser::strip_tags(&cue.text);
                println!("  Text: {}", clean_text.replace('\n', " | "));

                if let Some(ref settings) = cue.settings {
                    if settings.align.is_some() || settings.position.is_some() {
                        println!("  Settings: align={:?}, position={:?}",
                            settings.align, settings.position);
                    }
                }
                println!();
            }

            // Demonstrate finding active cues at specific times
            println!("Finding cues at specific times:");

            let test_times = [0.5, 5.0, 10.0, 90.0 * 60.0 + 2.0]; // Last one is 1:30:02
            for time in test_times {
                let active = cues_at_time(&cues, time);
                let time_str = format_time(time);
                if active.is_empty() {
                    println!("  At {}: (no caption)", time_str);
                } else {
                    for cue in active {
                        let text = WebVttParser::strip_tags(&cue.text);
                        println!("  At {}: \"{}\"", time_str, text.replace('\n', " "));
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to parse WebVTT: {}", e);
        }
    }

    println!("\n----------------------------------------\n");

    // Example SRT content
    let srt = r#"1
00:00:00,000 --> 00:00:03,000
Welcome to Purple Squirrel Media!

2
00:00:03,500 --> 00:00:07,000
This video demonstrates our
amazing player capabilities.

3
00:00:08,000 --> 00:00:12,000
<b>Bold text</b> works in SRT too.
"#;

    println!("Parsing SRT content...\n");

    match SrtParser::parse(srt) {
        Ok(cues) => {
            println!("Found {} cues:\n", cues.len());

            for cue in &cues {
                let clean_text = SrtParser::strip_tags(&cue.text);
                println!("  [{}] {:.3}s -> {:.3}s: {}",
                    cue.id,
                    cue.start_time,
                    cue.end_time,
                    clean_text.replace('\n', " | ")
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to parse SRT: {}", e);
        }
    }

    println!("\n----------------------------------------\n");

    // Demonstrate SRT to VTT conversion
    println!("Converting SRT to WebVTT...\n");

    let vtt = srt_to_vtt(srt);
    println!("Result (first 200 chars):");
    println!("{}", &vtt[..vtt.len().min(200)]);
    println!("...\n");

    // Verify the converted VTT can be parsed
    match WebVttParser::parse(&vtt) {
        Ok(cues) => {
            println!("Converted VTT successfully parsed: {} cues", cues.len());
        }
        Err(e) => {
            eprintln!("Failed to parse converted VTT: {}", e);
        }
    }
}

/// Format time in HH:MM:SS.mmm format
fn format_time(seconds: f64) -> String {
    let hours = (seconds / 3600.0) as u32;
    let minutes = ((seconds % 3600.0) / 60.0) as u32;
    let secs = seconds % 60.0;

    if hours > 0 {
        format!("{:02}:{:02}:{:06.3}", hours, minutes, secs)
    } else {
        format!("{:02}:{:06.3}", minutes, secs)
    }
}
