//! Encoding pipeline - FFmpeg integration for HLS/DASH packaging
//!
//! Provides encoding commands for:
//! - Transcoding video files to adaptive streaming formats
//! - Generating HLS/DASH manifests
//! - Applying PSM-optimized encoding presets

use std::path::Path;
use std::process::Command;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// PSM-optimized encoding presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingPreset {
    /// Web streaming - 360p to 1080p, balanced quality/size
    Web,
    /// Mobile-first - 240p to 720p, smaller file sizes
    Mobile,
    /// Premium - 360p to 4K HDR, highest quality
    Premium,
    /// Low-latency live - optimized for streaming
    Live,
    /// Archive - high quality single rendition
    Archive,
}

impl EncodingPreset {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "web" => Some(Self::Web),
            "mobile" => Some(Self::Mobile),
            "premium" => Some(Self::Premium),
            "live" => Some(Self::Live),
            "archive" => Some(Self::Archive),
            _ => None,
        }
    }

    pub fn renditions(&self) -> Vec<RenditionSpec> {
        match self {
            Self::Web => vec![
                RenditionSpec::new(360, 800_000, 30),
                RenditionSpec::new(480, 1_400_000, 30),
                RenditionSpec::new(720, 2_800_000, 30),
                RenditionSpec::new(1080, 5_000_000, 30),
            ],
            Self::Mobile => vec![
                RenditionSpec::new(240, 400_000, 24),
                RenditionSpec::new(360, 800_000, 30),
                RenditionSpec::new(480, 1_200_000, 30),
                RenditionSpec::new(720, 2_000_000, 30),
            ],
            Self::Premium => vec![
                RenditionSpec::new(360, 1_000_000, 30),
                RenditionSpec::new(480, 1_800_000, 30),
                RenditionSpec::new(720, 3_500_000, 30),
                RenditionSpec::new(1080, 6_000_000, 30),
                RenditionSpec::new(1440, 12_000_000, 30),
                RenditionSpec::new(2160, 20_000_000, 30),
            ],
            Self::Live => vec![
                RenditionSpec::new(360, 600_000, 30),
                RenditionSpec::new(480, 1_200_000, 30),
                RenditionSpec::new(720, 2_500_000, 30),
                RenditionSpec::new(1080, 4_500_000, 30),
            ],
            Self::Archive => vec![
                RenditionSpec::new(1080, 8_000_000, 30),
            ],
        }
    }

    pub fn segment_duration(&self) -> f64 {
        match self {
            Self::Live => 2.0,
            _ => 6.0,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Web => "Web streaming (360p-1080p, balanced)",
            Self::Mobile => "Mobile-first (240p-720p, smaller files)",
            Self::Premium => "Premium (360p-4K, highest quality)",
            Self::Live => "Low-latency live (2s segments)",
            Self::Archive => "Archive (single 1080p high quality)",
        }
    }
}

/// Specification for a single rendition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenditionSpec {
    pub height: u32,
    pub bitrate: u32,
    pub framerate: u32,
}

impl RenditionSpec {
    pub fn new(height: u32, bitrate: u32, framerate: u32) -> Self {
        Self { height, bitrate, framerate }
    }

    pub fn width(&self) -> u32 {
        // Calculate 16:9 width
        (self.height as f64 * 16.0 / 9.0).round() as u32
    }

    pub fn quality_name(&self) -> &'static str {
        match self.height {
            0..=240 => "240p",
            241..=360 => "360p",
            361..=480 => "480p",
            481..=720 => "720p",
            721..=1080 => "1080p",
            1081..=1440 => "1440p",
            _ => "4K",
        }
    }
}

/// Encoding output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Hls,
    Dash,
    Both,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "hls" => Some(Self::Hls),
            "dash" => Some(Self::Dash),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

/// Check if FFmpeg is available
pub fn check_ffmpeg() -> Result<String> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("FFmpeg not found. Please install FFmpeg.")?;

    let version = String::from_utf8_lossy(&output.stdout);
    let first_line = version.lines().next().unwrap_or("FFmpeg");
    Ok(first_line.to_string())
}

/// Probe input file for metadata
pub fn probe_input(input: &Path) -> Result<InputInfo> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(input)
        .output()
        .context("FFprobe failed")?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("Failed to parse ffprobe output")?;

    // Extract video stream info
    let video_stream = json["streams"]
        .as_array()
        .and_then(|streams| {
            streams.iter().find(|s| s["codec_type"] == "video")
        });

    let (width, height, framerate, duration) = if let Some(vs) = video_stream {
        let w = vs["width"].as_u64().unwrap_or(0) as u32;
        let h = vs["height"].as_u64().unwrap_or(0) as u32;

        // Parse framerate (e.g., "30000/1001" or "30")
        let fr_str = vs["r_frame_rate"].as_str().unwrap_or("30/1");
        let fr = if fr_str.contains('/') {
            let parts: Vec<&str> = fr_str.split('/').collect();
            let num: f64 = parts[0].parse().unwrap_or(30.0);
            let den: f64 = parts[1].parse().unwrap_or(1.0);
            (num / den).round() as u32
        } else {
            fr_str.parse().unwrap_or(30)
        };

        let dur = json["format"]["duration"]
            .as_str()
            .and_then(|d| d.parse::<f64>().ok())
            .unwrap_or(0.0);

        (w, h, fr, dur)
    } else {
        bail!("No video stream found in input");
    };

    // Audio info
    let audio_stream = json["streams"]
        .as_array()
        .and_then(|streams| {
            streams.iter().find(|s| s["codec_type"] == "audio")
        });

    let has_audio = audio_stream.is_some();

    Ok(InputInfo {
        width,
        height,
        framerate,
        duration,
        has_audio,
    })
}

#[derive(Debug)]
pub struct InputInfo {
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub duration: f64,
    pub has_audio: bool,
}

/// Encode video to HLS
pub fn encode_hls(
    input: &Path,
    output_dir: &Path,
    preset: EncodingPreset,
    segment_duration: f64,
    _progress_callback: Option<Box<dyn Fn(f64)>>,
) -> Result<()> {
    let input_info = probe_input(input)?;
    let renditions = preset.renditions();

    std::fs::create_dir_all(output_dir)?;

    println!("Encoding to HLS with {} preset", preset.description());
    println!("Input: {}x{} @ {}fps, {:.1}s",
        input_info.width, input_info.height, input_info.framerate, input_info.duration);

    // Build FFmpeg command for multi-rendition HLS
    let mut args: Vec<String> = vec![
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-y".to_string(),  // Overwrite
    ];

    // Filter complex for scaling
    let mut filter_complex = String::new();
    let mut map_args: Vec<String> = Vec::new();
    let mut stream_map = String::new();

    for (i, r) in renditions.iter().enumerate() {
        // Skip renditions higher than source
        if r.height > input_info.height {
            continue;
        }

        // Scale filter
        filter_complex.push_str(&format!(
            "[0:v]scale={}:{}:force_original_aspect_ratio=decrease[v{}];",
            r.width(), r.height, i
        ));

        // Video output
        map_args.extend([
            "-map".to_string(), format!("[v{}]", i),
            format!("-c:v:{}", i), "libx264".to_string(),
            format!("-b:v:{}", i), format!("{}", r.bitrate),
            format!("-maxrate:v:{}", i), format!("{}", (r.bitrate as f64 * 1.1) as u32),
            format!("-bufsize:v:{}", i), format!("{}", r.bitrate * 2),
            format!("-preset:v:{}", i), "medium".to_string(),
            format!("-g:v:{}", i), format!("{}", r.framerate * 2),  // GOP size
            format!("-keyint_min:v:{}", i), format!("{}", r.framerate),
        ]);

        // Audio (copy to all variants)
        if input_info.has_audio {
            map_args.extend([
                "-map".to_string(), "0:a".to_string(),
                format!("-c:a:{}", i), "aac".to_string(),
                format!("-b:a:{}", i), "128k".to_string(),
            ]);
        }

        if !stream_map.is_empty() {
            stream_map.push(' ');
        }
        stream_map.push_str(&format!("v:{},a:{}", i, i));
    }

    if filter_complex.is_empty() {
        bail!("Source resolution is lower than all preset renditions");
    }

    // Remove trailing semicolon
    filter_complex.pop();

    args.extend([
        "-filter_complex".to_string(),
        filter_complex,
    ]);
    args.extend(map_args);

    // HLS options
    args.extend([
        "-f".to_string(), "hls".to_string(),
        "-hls_time".to_string(), format!("{}", segment_duration as u32),
        "-hls_playlist_type".to_string(), "vod".to_string(),
        "-hls_segment_filename".to_string(),
        output_dir.join("stream_%v_%03d.ts").to_string_lossy().to_string(),
        "-master_pl_name".to_string(), "master.m3u8".to_string(),
        "-var_stream_map".to_string(), stream_map,
        output_dir.join("stream_%v.m3u8").to_string_lossy().to_string(),
    ]);

    println!("Running FFmpeg...");

    let status = Command::new("ffmpeg")
        .args(&args)
        .status()
        .context("FFmpeg execution failed")?;

    if !status.success() {
        bail!("FFmpeg encoding failed");
    }

    println!("HLS encoding complete!");
    println!("Output: {}", output_dir.display());
    println!("Master playlist: {}", output_dir.join("master.m3u8").display());

    Ok(())
}

/// Encode video to DASH
pub fn encode_dash(
    input: &Path,
    output_dir: &Path,
    preset: EncodingPreset,
    segment_duration: f64,
) -> Result<()> {
    let input_info = probe_input(input)?;
    let renditions = preset.renditions();

    std::fs::create_dir_all(output_dir)?;

    println!("Encoding to DASH with {} preset", preset.description());

    // For DASH, we encode to fragmented MP4 first, then use MP4Box or ffmpeg dash muxer
    let mut args: Vec<String> = vec![
        "-i".to_string(),
        input.to_string_lossy().to_string(),
        "-y".to_string(),
    ];

    // Filter complex for scaling
    let mut filter_complex = String::new();
    let mut map_args: Vec<String> = Vec::new();

    for (i, r) in renditions.iter().enumerate() {
        if r.height > input_info.height {
            continue;
        }

        filter_complex.push_str(&format!(
            "[0:v]scale={}:{}:force_original_aspect_ratio=decrease[v{}];",
            r.width(), r.height, i
        ));

        map_args.extend([
            "-map".to_string(), format!("[v{}]", i),
            format!("-c:v:{}", i), "libx264".to_string(),
            format!("-b:v:{}", i), format!("{}", r.bitrate),
            format!("-preset:v:{}", i), "medium".to_string(),
        ]);

        if input_info.has_audio {
            map_args.extend([
                "-map".to_string(), "0:a".to_string(),
                format!("-c:a:{}", i), "aac".to_string(),
                format!("-b:a:{}", i), "128k".to_string(),
            ]);
        }
    }

    filter_complex.pop();

    args.extend([
        "-filter_complex".to_string(),
        filter_complex,
    ]);
    args.extend(map_args);

    // DASH options
    args.extend([
        "-f".to_string(), "dash".to_string(),
        "-seg_duration".to_string(), format!("{}", segment_duration as u32),
        "-use_template".to_string(), "1".to_string(),
        "-use_timeline".to_string(), "1".to_string(),
        "-init_seg_name".to_string(), "init_$RepresentationID$.mp4".to_string(),
        "-media_seg_name".to_string(), "segment_$RepresentationID$_$Number$.m4s".to_string(),
        output_dir.join("manifest.mpd").to_string_lossy().to_string(),
    ]);

    println!("Running FFmpeg for DASH...");

    let status = Command::new("ffmpeg")
        .args(&args)
        .status()
        .context("FFmpeg execution failed")?;

    if !status.success() {
        bail!("FFmpeg DASH encoding failed");
    }

    println!("DASH encoding complete!");
    println!("Output: {}", output_dir.display());
    println!("MPD manifest: {}", output_dir.join("manifest.mpd").display());

    Ok(())
}

/// List all available presets
pub fn list_presets() {
    println!("Available PSM Encoding Presets:\n");

    for preset in [
        EncodingPreset::Web,
        EncodingPreset::Mobile,
        EncodingPreset::Premium,
        EncodingPreset::Live,
        EncodingPreset::Archive,
    ] {
        println!("  {} - {}", format!("{:?}", preset).to_lowercase(), preset.description());
        println!("    Renditions:");
        for r in preset.renditions() {
            println!("      {} - {}x{} @ {}kbps",
                r.quality_name(), r.width(), r.height, r.bitrate / 1000);
        }
        println!("    Segment duration: {}s\n", preset.segment_duration());
    }
}

/// Show details of a specific preset
pub fn show_preset(name: &str) {
    if let Some(preset) = EncodingPreset::from_str(name) {
        println!("Preset: {}", name);
        println!("Description: {}", preset.description());
        println!("Segment duration: {}s", preset.segment_duration());
        println!("\nRenditions:");
        println!("  {:>6}  {:>10}  {:>8}  {:>4}", "Quality", "Resolution", "Bitrate", "FPS");
        println!("  {:->6}  {:->10}  {:->8}  {:->4}", "", "", "", "");
        for r in preset.renditions() {
            println!("  {:>6}  {:>10}  {:>7}k  {:>4}",
                r.quality_name(),
                format!("{}x{}", r.width(), r.height),
                r.bitrate / 1000,
                r.framerate
            );
        }

        // Show FFmpeg command
        println!("\nEquivalent FFmpeg command (simplified):");
        let renditions = preset.renditions();
        println!("  ffmpeg -i input.mp4 \\");
        for r in &renditions {
            println!("    -vf scale={}:{} -b:v {}k \\",
                r.width(), r.height, r.bitrate / 1000);
        }
        println!("    -f hls -hls_time {} output/master.m3u8", preset.segment_duration() as u32);
    } else {
        println!("Unknown preset: {}", name);
        println!("Available presets: web, mobile, premium, live, archive");
    }
}
