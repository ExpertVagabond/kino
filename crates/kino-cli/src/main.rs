//! Kino CLI - Headless Video Player and QC Tool
//!
//! Features:
//! - Manifest validation
//! - Stream QC (segment accessibility, continuity)
//! - Analytics extraction
//! - ABR ladder analysis
//! - DRM testing
//! - FFmpeg encoding pipeline

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod encoding;
mod frequency;
mod output;

/// Kino CLI - Video streaming toolkit
#[derive(Parser)]
#[command(name = "kino-cli")]
#[command(author = "Purple Squirrel Media")]
#[command(version)]
#[command(about = "Video streaming analysis and QC toolkit", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Output format (text, json, table)
    #[arg(short, long, default_value = "text")]
    format: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a manifest (HLS/DASH)
    Analyze {
        /// URL or path to manifest
        manifest: String,
    },

    /// Validate stream accessibility
    Validate {
        /// URL to manifest
        manifest: String,

        /// Number of segments to test
        #[arg(short, long, default_value = "10")]
        segments: usize,

        /// Test all renditions
        #[arg(short, long)]
        all_renditions: bool,
    },

    /// Run QC checks on a stream
    Qc {
        /// URL to manifest
        manifest: String,

        /// Output QC report to file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Fail on warnings
        #[arg(long)]
        strict: bool,
    },

    /// Extract analytics/metadata
    Extract {
        /// URL to manifest
        manifest: String,

        /// What to extract (bitrates, durations, segments, all)
        #[arg(short, long, default_value = "all")]
        what: String,
    },

    /// Compare two streams
    Compare {
        /// First manifest URL
        manifest1: String,

        /// Second manifest URL
        manifest2: String,
    },

    /// Monitor a live stream
    Monitor {
        /// URL to manifest
        manifest: String,

        /// Refresh interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,

        /// Duration to monitor (0 = indefinite)
        #[arg(short, long, default_value = "0")]
        duration: u64,
    },

    /// Encode video to HLS/DASH
    Encode {
        /// Input video file
        input: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,

        /// Output format (hls, dash, both)
        #[arg(short = 'f', long, default_value = "hls")]
        format: String,

        /// Encoding preset (web, mobile, premium, live, archive)
        #[arg(short, long, default_value = "web")]
        preset: String,

        /// Segment duration in seconds
        #[arg(short, long)]
        segment_duration: Option<f64>,
    },

    /// Show encoding presets
    Preset {
        /// Preset name to show details (or 'list' for all)
        #[arg(default_value = "list")]
        name: String,
    },

    // =========================================================================
    // Frequency Analysis Commands
    // =========================================================================

    /// Analyze audio frequencies in a video
    Frequency {
        /// Input video file
        input: PathBuf,

        /// Number of top frequencies to show
        #[arg(short = 'k', long, default_value = "10")]
        top_k: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Generate or verify audio fingerprint
    Fingerprint {
        /// Input video file
        input: PathBuf,

        /// Output fingerprint to file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Verify against existing hash
        #[arg(long)]
        verify: Option<String>,
    },

    /// Auto-tag content based on audio analysis
    Autotag {
        /// Input video file
        input: PathBuf,

        /// Maximum number of tags
        #[arg(short, long, default_value = "5")]
        max_tags: usize,

        /// Minimum confidence threshold (0-1)
        #[arg(short = 'c', long, default_value = "0.3")]
        min_confidence: f32,
    },

    /// Select optimal thumbnail timestamp
    Thumbnail {
        /// Input video file
        input: PathBuf,

        /// Output thumbnail file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Number of candidates to show
        #[arg(short = 'n', long, default_value = "1")]
        candidates: usize,
    },

    /// Find similar content in a library
    Similar {
        /// Input video file to match
        input: PathBuf,

        /// Directory containing video library
        #[arg(short, long)]
        library: PathBuf,

        /// Number of results to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Process video through complete frequency pipeline
    Process {
        /// Input video file
        input: PathBuf,

        /// Output directory for results
        #[arg(short, long)]
        output: PathBuf,

        /// Skip fingerprint generation
        #[arg(long)]
        skip_fingerprint: bool,

        /// Skip auto-tagging
        #[arg(long)]
        skip_tags: bool,

        /// Skip thumbnail selection
        #[arg(long)]
        skip_thumbnail: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(level)
        .init();

    match cli.command {
        Commands::Analyze { manifest } => {
            commands::analyze(&manifest, &cli.format).await?;
        }
        Commands::Validate { manifest, segments, all_renditions } => {
            commands::validate(&manifest, segments, all_renditions, &cli.format).await?;
        }
        Commands::Qc { manifest, output, strict } => {
            commands::qc(&manifest, output, strict, &cli.format).await?;
        }
        Commands::Extract { manifest, what } => {
            commands::extract(&manifest, &what, &cli.format).await?;
        }
        Commands::Compare { manifest1, manifest2 } => {
            commands::compare(&manifest1, &manifest2, &cli.format).await?;
        }
        Commands::Monitor { manifest, interval, duration } => {
            commands::monitor(&manifest, interval, duration, &cli.format).await?;
        }
        Commands::Encode { input, output, format, preset, segment_duration } => {
            // Check FFmpeg
            match encoding::check_ffmpeg() {
                Ok(version) => println!("Using: {}", version),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }

            let enc_preset = encoding::EncodingPreset::from_str(&preset)
                .unwrap_or_else(|| {
                    eprintln!("Unknown preset '{}', using 'web'", preset);
                    encoding::EncodingPreset::Web
                });

            let seg_dur = segment_duration.unwrap_or(enc_preset.segment_duration());

            let output_format = encoding::OutputFormat::from_str(&format)
                .unwrap_or(encoding::OutputFormat::Hls);

            match output_format {
                encoding::OutputFormat::Hls => {
                    encoding::encode_hls(&input, &output, enc_preset, seg_dur, None)?;
                }
                encoding::OutputFormat::Dash => {
                    encoding::encode_dash(&input, &output, enc_preset, seg_dur)?;
                }
                encoding::OutputFormat::Both => {
                    let hls_dir = output.join("hls");
                    let dash_dir = output.join("dash");
                    encoding::encode_hls(&input, &hls_dir, enc_preset, seg_dur, None)?;
                    encoding::encode_dash(&input, &dash_dir, enc_preset, seg_dur)?;
                }
            }
        }
        Commands::Preset { name } => {
            if name == "list" {
                encoding::list_presets();
            } else {
                encoding::show_preset(&name);
            }
        }

        // Frequency analysis commands
        Commands::Frequency { input, top_k, json } => {
            frequency::analyze_frequency(&input, top_k, json).await?;
        }
        Commands::Fingerprint { input, output, verify } => {
            frequency::fingerprint(&input, output, verify).await?;
        }
        Commands::Autotag { input, max_tags, min_confidence } => {
            frequency::autotag(&input, max_tags, min_confidence).await?;
        }
        Commands::Thumbnail { input, output, candidates } => {
            frequency::thumbnail(&input, output, candidates).await?;
        }
        Commands::Similar { input, library, limit } => {
            frequency::similar(&input, &library, limit).await?;
        }
        Commands::Process { input, output, skip_fingerprint, skip_tags, skip_thumbnail } => {
            frequency::process(&input, &output, skip_fingerprint, skip_tags, skip_thumbnail).await?;
        }
    }

    Ok(())
}
