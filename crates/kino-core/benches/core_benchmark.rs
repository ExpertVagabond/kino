//! Benchmark tests for kino-core operations
//!
//! Run with: cargo bench -p kino-core

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use url::Url;
use bytes::Bytes;

use kino_core::abr::{AbrEngine, AbrContext};
use kino_core::buffer::{BufferConfig, BufferManager};
use kino_core::branding::{CssVariables, KinoColors, KinoTheme, JsTheme};
use kino_core::types::*;
use kino_core::analytics::QoeCalculator;
use kino_core::manifest::detect_manifest_type;

// ============================================================================
// Helpers
// ============================================================================

fn create_test_segment(num: u64) -> Segment {
    Segment {
        number: num,
        uri: Url::parse(&format!("https://cdn.example.com/stream/seg{}.ts", num)).unwrap(),
        duration: Duration::from_secs(4),
        byte_range: None,
        encryption: None,
        discontinuity_sequence: 0,
        program_date_time: None,
    }
}

fn create_test_renditions() -> Vec<Rendition> {
    vec![
        Rendition {
            id: "240p".to_string(),
            bandwidth: 400_000,
            resolution: Some(Resolution::new(426, 240)),
            frame_rate: None,
            video_codec: Some(VideoCodec::H264),
            audio_codec: Some(AudioCodec::Aac),
            uri: Url::parse("https://cdn.example.com/240p.m3u8").unwrap(),
            hdr: None,
            language: None,
            name: None,
        },
        Rendition {
            id: "360p".to_string(),
            bandwidth: 800_000,
            resolution: Some(Resolution::new(640, 360)),
            frame_rate: None,
            video_codec: Some(VideoCodec::H264),
            audio_codec: Some(AudioCodec::Aac),
            uri: Url::parse("https://cdn.example.com/360p.m3u8").unwrap(),
            hdr: None,
            language: None,
            name: None,
        },
        Rendition {
            id: "480p".to_string(),
            bandwidth: 1_400_000,
            resolution: Some(Resolution::new(854, 480)),
            frame_rate: None,
            video_codec: Some(VideoCodec::H264),
            audio_codec: Some(AudioCodec::Aac),
            uri: Url::parse("https://cdn.example.com/480p.m3u8").unwrap(),
            hdr: None,
            language: None,
            name: None,
        },
        Rendition {
            id: "720p".to_string(),
            bandwidth: 2_800_000,
            resolution: Some(Resolution::new(1280, 720)),
            frame_rate: Some(30.0),
            video_codec: Some(VideoCodec::H264),
            audio_codec: Some(AudioCodec::Aac),
            uri: Url::parse("https://cdn.example.com/720p.m3u8").unwrap(),
            hdr: None,
            language: None,
            name: None,
        },
        Rendition {
            id: "1080p".to_string(),
            bandwidth: 5_000_000,
            resolution: Some(Resolution::new(1920, 1080)),
            frame_rate: Some(30.0),
            video_codec: Some(VideoCodec::H264),
            audio_codec: Some(AudioCodec::Aac),
            uri: Url::parse("https://cdn.example.com/1080p.m3u8").unwrap(),
            hdr: None,
            language: None,
            name: None,
        },
        Rendition {
            id: "1080p60".to_string(),
            bandwidth: 7_500_000,
            resolution: Some(Resolution::new(1920, 1080)),
            frame_rate: Some(60.0),
            video_codec: Some(VideoCodec::H264),
            audio_codec: Some(AudioCodec::Aac),
            uri: Url::parse("https://cdn.example.com/1080p60.m3u8").unwrap(),
            hdr: None,
            language: None,
            name: None,
        },
        Rendition {
            id: "4k".to_string(),
            bandwidth: 15_000_000,
            resolution: Some(Resolution::new(3840, 2160)),
            frame_rate: Some(30.0),
            video_codec: Some(VideoCodec::H265),
            audio_codec: Some(AudioCodec::Aac),
            uri: Url::parse("https://cdn.example.com/4k.m3u8").unwrap(),
            hdr: Some(HdrFormat::Hdr10),
            language: None,
            name: None,
        },
    ]
}

/// Generate a realistic HLS master playlist string with N variants
fn generate_hls_master(variant_count: usize) -> String {
    let mut m3u8 = String::from("#EXTM3U\n");
    let bandwidths = [400_000u64, 800_000, 1_400_000, 2_800_000, 5_000_000, 7_500_000, 15_000_000];
    let resolutions = ["426x240", "640x360", "854x480", "1280x720", "1920x1080", "1920x1080", "3840x2160"];
    let codecs = "avc1.640028,mp4a.40.2";

    for i in 0..variant_count {
        let idx = i % bandwidths.len();
        m3u8.push_str(&format!(
            "#EXT-X-STREAM-INF:BANDWIDTH={},RESOLUTION={},CODECS=\"{}\"\n",
            bandwidths[idx], resolutions[idx], codecs
        ));
        m3u8.push_str(&format!("variant_{}/playlist.m3u8\n", i));
    }

    m3u8
}

/// Generate a realistic HLS media playlist string with N segments
fn generate_hls_media(segment_count: usize) -> String {
    let mut m3u8 = String::from("#EXTM3U\n");
    m3u8.push_str("#EXT-X-VERSION:3\n");
    m3u8.push_str("#EXT-X-TARGETDURATION:6\n");
    m3u8.push_str("#EXT-X-MEDIA-SEQUENCE:0\n");

    for i in 0..segment_count {
        let dur = 4.0 + (i % 3) as f32 * 0.5; // 4.0, 4.5, 5.0 rotation
        m3u8.push_str(&format!("#EXTINF:{:.3},\n", dur));
        m3u8.push_str(&format!("segment_{:05}.ts\n", i));
    }

    m3u8.push_str("#EXT-X-ENDLIST\n");
    m3u8
}

// ============================================================================
// Buffer Benchmarks
// ============================================================================

fn bench_buffer_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Buffer Allocation");

    for &count in &[1, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("BufferManager::new", count),
            &count,
            |b, &_count| {
                b.iter(|| {
                    let config = BufferConfig {
                        max_memory_bytes: 512 * 1024 * 1024,
                        ..Default::default()
                    };
                    black_box(BufferManager::new(config))
                });
            },
        );
    }

    group.finish();
}

fn bench_buffer_segment_insertion(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("Buffer Segment Insertion");

    for &segment_count in &[1, 10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("add_segment", segment_count),
            &segment_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let buffer = BufferManager::new(BufferConfig::default());
                        for i in 0..count {
                            let segment = create_test_segment(i as u64);
                            let data = Bytes::from(vec![0u8; 64 * 1024]); // 64KB per segment
                            buffer.add_segment(segment, data).await.unwrap();
                        }
                        black_box(buffer.buffer_level().await)
                    })
                });
            },
        );
    }

    group.finish();
}

fn bench_buffer_segment_sizes(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("Buffer Segment Sizes");

    // Typical segment sizes: 64KB (low quality), 512KB (medium), 2MB (high), 8MB (4K)
    for &size_kb in &[64, 512, 2048, 8192] {
        group.bench_with_input(
            BenchmarkId::new("add_segment", format!("{}KB", size_kb)),
            &size_kb,
            |b, &size_kb| {
                let data = Bytes::from(vec![0u8; size_kb * 1024]);
                b.iter(|| {
                    rt.block_on(async {
                        let config = BufferConfig {
                            max_memory_bytes: 512 * 1024 * 1024,
                            ..Default::default()
                        };
                        let buffer = BufferManager::new(config);
                        let segment = create_test_segment(1);
                        buffer.add_segment(segment, data.clone()).await.unwrap();
                        black_box(buffer.buffer_level().await)
                    })
                });
            },
        );
    }

    group.finish();
}

fn bench_buffer_queries(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // Pre-fill buffer with 50 segments
    let buffer = rt.block_on(async {
        let config = BufferConfig {
            max_memory_bytes: 512 * 1024 * 1024,
            ..Default::default()
        };
        let buffer = BufferManager::new(config);
        for i in 0..50u64 {
            let segment = create_test_segment(i);
            let data = Bytes::from(vec![0u8; 64 * 1024]);
            buffer.add_segment(segment, data).await.unwrap();
        }
        buffer
    });

    let mut group = c.benchmark_group("Buffer Queries");

    group.bench_function("buffer_level", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(buffer.buffer_level().await)
            })
        });
    });

    group.bench_function("is_buffer_healthy", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(buffer.is_buffer_healthy().await)
            })
        });
    });

    group.bench_function("needs_data", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(buffer.needs_data().await)
            })
        });
    });

    group.bench_function("buffered_ranges", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(buffer.buffered_ranges().await)
            })
        });
    });

    group.bench_function("get_segment_at_mid", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Seek to middle of buffer (50 segments * 4s = 200s, mid = 100s)
                black_box(buffer.get_segment_at(100.0).await)
            })
        });
    });

    group.bench_function("get_next_segment", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(buffer.get_next_segment().await)
            })
        });
    });

    group.bench_function("stats", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(buffer.stats().await)
            })
        });
    });

    group.finish();
}

// ============================================================================
// HLS Manifest Parsing Benchmarks
// ============================================================================

fn bench_hls_master_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("HLS Master Parsing");

    for &variant_count in &[3, 7, 12, 20] {
        let manifest = generate_hls_master(variant_count);
        let _base_url = Url::parse("https://cdn.example.com/live/master.m3u8").unwrap();

        group.bench_with_input(
            BenchmarkId::new("parse_master", format!("{}_variants", variant_count)),
            &manifest,
            |b, manifest| {
                b.iter(|| {
                    // Use m3u8_rs directly since HlsParser::parse_master is private
                    // and HlsParser::parse requires async HTTP fetch
                    let parsed = m3u8_rs::parse_master_playlist_res(
                        black_box(manifest.as_bytes()),
                    );
                    black_box(parsed.unwrap())
                });
            },
        );
    }

    group.finish();
}

fn bench_hls_media_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("HLS Media Parsing");

    for &segment_count in &[10, 50, 200, 1000] {
        let manifest = generate_hls_media(segment_count);

        group.bench_with_input(
            BenchmarkId::new("parse_media", format!("{}_segments", segment_count)),
            &manifest,
            |b, manifest| {
                b.iter(|| {
                    let parsed = m3u8_rs::parse_media_playlist_res(
                        black_box(manifest.as_bytes()),
                    );
                    black_box(parsed.unwrap())
                });
            },
        );
    }

    group.finish();
}

fn bench_manifest_type_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("Manifest Type Detection");

    group.bench_function("detect_hls_by_url", |b| {
        let url = Url::parse("https://cdn.example.com/live/master.m3u8").unwrap();
        b.iter(|| {
            black_box(detect_manifest_type(black_box(&url), None))
        });
    });

    group.bench_function("detect_dash_by_url", |b| {
        let url = Url::parse("https://cdn.example.com/live/manifest.mpd").unwrap();
        b.iter(|| {
            black_box(detect_manifest_type(black_box(&url), None))
        });
    });

    group.bench_function("detect_hls_by_content", |b| {
        let url = Url::parse("https://cdn.example.com/live/stream").unwrap();
        let content = "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-STREAM-INF:BANDWIDTH=800000\nv.m3u8";
        b.iter(|| {
            black_box(detect_manifest_type(black_box(&url), Some(black_box(content))))
        });
    });

    group.bench_function("detect_dash_by_content", |b| {
        let url = Url::parse("https://cdn.example.com/live/stream").unwrap();
        let content = "<?xml version=\"1.0\"?><MPD xmlns=\"urn:mpeg:dash:schema:mpd:2011\">";
        b.iter(|| {
            black_box(detect_manifest_type(black_box(&url), Some(black_box(content))))
        });
    });

    group.finish();
}

// ============================================================================
// ABR Algorithm Benchmarks
// ============================================================================

fn bench_abr_engine_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ABR Engine Creation");

    for algo in &[AbrAlgorithmType::Throughput, AbrAlgorithmType::Bola, AbrAlgorithmType::Hybrid] {
        group.bench_with_input(
            BenchmarkId::new("new", format!("{:?}", algo)),
            algo,
            |b, &algo| {
                b.iter(|| {
                    black_box(AbrEngine::new(algo))
                });
            },
        );
    }

    group.finish();
}

fn bench_abr_rendition_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("ABR Rendition Selection");
    let renditions = create_test_renditions();

    // Scenario: high bandwidth, healthy buffer
    group.bench_function("throughput/high_bw", |b| {
        let mut engine = AbrEngine::new(AbrAlgorithmType::Throughput);
        // Seed with measurements
        for _ in 0..5 {
            engine.record_measurement(1_000_000, Duration::from_millis(100));
        }
        let context = AbrContext {
            buffer_level: 20.0,
            target_buffer: 30.0,
            playback_rate: 1.0,
            is_live: false,
            screen_width: Some(1920),
            max_bitrate: 0,
            network: NetworkInfo {
                bandwidth_estimate: 20_000_000,
                rtt_ms: 50,
                connection_type: Some(ConnectionType::Wifi),
                metered: false,
            },
        };
        b.iter(|| {
            black_box(engine.select_rendition(black_box(&renditions), black_box(&context)))
        });
    });

    // Scenario: low bandwidth, low buffer
    group.bench_function("throughput/low_bw", |b| {
        let mut engine = AbrEngine::new(AbrAlgorithmType::Throughput);
        for _ in 0..5 {
            engine.record_measurement(50_000, Duration::from_millis(500));
        }
        let context = AbrContext {
            buffer_level: 3.0,
            target_buffer: 30.0,
            playback_rate: 1.0,
            is_live: false,
            screen_width: Some(1280),
            max_bitrate: 0,
            network: NetworkInfo {
                bandwidth_estimate: 800_000,
                rtt_ms: 200,
                connection_type: Some(ConnectionType::Cellular4G),
                metered: true,
            },
        };
        b.iter(|| {
            black_box(engine.select_rendition(black_box(&renditions), black_box(&context)))
        });
    });

    // BOLA with various buffer levels
    for &buffer_level in &[2.0, 10.0, 25.0] {
        group.bench_with_input(
            BenchmarkId::new("bola", format!("buf_{}s", buffer_level as u32)),
            &buffer_level,
            |b, &buffer_level| {
                let mut engine = AbrEngine::new(AbrAlgorithmType::Bola);
                let context = AbrContext {
                    buffer_level,
                    target_buffer: 30.0,
                    playback_rate: 1.0,
                    is_live: false,
                    screen_width: None,
                    max_bitrate: 0,
                    network: NetworkInfo {
                        bandwidth_estimate: 5_000_000,
                        ..Default::default()
                    },
                };
                b.iter(|| {
                    black_box(engine.select_rendition(black_box(&renditions), black_box(&context)))
                });
            },
        );
    }

    // Hybrid algorithm
    group.bench_function("hybrid/mid_scenario", |b| {
        let mut engine = AbrEngine::new(AbrAlgorithmType::Hybrid);
        for _ in 0..5 {
            engine.record_measurement(500_000, Duration::from_millis(200));
        }
        let context = AbrContext {
            buffer_level: 12.0,
            target_buffer: 30.0,
            playback_rate: 1.0,
            is_live: true,
            screen_width: Some(1920),
            max_bitrate: 10_000_000,
            network: NetworkInfo {
                bandwidth_estimate: 8_000_000,
                rtt_ms: 80,
                connection_type: Some(ConnectionType::Wifi),
                metered: false,
            },
        };
        b.iter(|| {
            black_box(engine.select_rendition(black_box(&renditions), black_box(&context)))
        });
    });

    group.finish();
}

fn bench_abr_bandwidth_recording(c: &mut Criterion) {
    let mut group = c.benchmark_group("ABR Bandwidth Recording");

    group.bench_function("record_measurement", |b| {
        let mut engine = AbrEngine::new(AbrAlgorithmType::Throughput);
        b.iter(|| {
            engine.record_measurement(
                black_box(500_000),
                black_box(Duration::from_millis(200)),
            );
            black_box(engine.bandwidth_estimate())
        });
    });

    // Measure recording under heavy history (near max capacity)
    group.bench_function("record_measurement_full_history", |b| {
        let mut engine = AbrEngine::new(AbrAlgorithmType::Throughput);
        // Fill history to capacity
        for i in 0..25 {
            engine.record_measurement(100_000 * (i + 1), Duration::from_millis(100));
        }
        b.iter(|| {
            engine.record_measurement(
                black_box(750_000),
                black_box(Duration::from_millis(150)),
            );
            black_box(engine.bandwidth_estimate())
        });
    });

    group.finish();
}

// ============================================================================
// Branding / CSS Generation Benchmarks
// ============================================================================

fn bench_branding(c: &mut Criterion) {
    let mut group = c.benchmark_group("Branding");

    group.bench_function("KinoColors::default", |b| {
        b.iter(|| {
            black_box(KinoColors::default())
        });
    });

    group.bench_function("KinoTheme::default", |b| {
        b.iter(|| {
            black_box(KinoTheme::default())
        });
    });

    group.bench_function("CssVariables::generate", |b| {
        b.iter(|| {
            black_box(CssVariables::generate())
        });
    });

    group.bench_function("CssVariables::player_css", |b| {
        b.iter(|| {
            black_box(CssVariables::player_css())
        });
    });

    group.bench_function("KinoTheme::to_css", |b| {
        let theme = KinoTheme::default();
        b.iter(|| {
            black_box(theme.to_css())
        });
    });

    group.bench_function("KinoTheme::to_json", |b| {
        let theme = KinoTheme::default();
        b.iter(|| {
            black_box(theme.to_json())
        });
    });

    group.bench_function("JsTheme::to_json", |b| {
        let js_theme = JsTheme::default();
        b.iter(|| {
            black_box(js_theme.to_json())
        });
    });

    group.bench_function("primary_rgba", |b| {
        let colors = KinoColors::default();
        b.iter(|| {
            black_box(colors.primary_rgba(black_box(0.7)))
        });
    });

    group.bench_function("background_rgba", |b| {
        let colors = KinoColors::default();
        b.iter(|| {
            black_box(colors.background_rgba(black_box(0.9)))
        });
    });

    group.finish();
}

// ============================================================================
// QoE Calculator Benchmarks
// ============================================================================

fn bench_qoe(c: &mut Criterion) {
    let mut group = c.benchmark_group("QoE Calculator");

    group.bench_function("perfect_score", |b| {
        let calc = QoeCalculator::new();
        b.iter(|| {
            black_box(calc.calculate_qoe())
        });
    });

    group.bench_function("degraded_score", |b| {
        let mut calc = QoeCalculator::new();
        calc.record_initial_buffer(4.0);
        calc.record_rebuffer(1.5);
        calc.record_rebuffer(2.0);
        calc.record_quality_switch(10.0, 2_800_000);
        calc.record_quality_switch(25.0, 800_000);
        calc.record_quality_switch(40.0, 5_000_000);
        for i in 0..20 {
            calc.record_bitrate(5.0, (i % 3 + 1) as u64 * 1_000_000);
        }
        b.iter(|| {
            black_box(calc.calculate_qoe())
        });
    });

    group.bench_function("breakdown", |b| {
        let mut calc = QoeCalculator::new();
        calc.record_initial_buffer(3.0);
        calc.record_rebuffer(1.0);
        calc.record_quality_switch(15.0, 5_000_000);
        for i in 0..50 {
            calc.record_bitrate(2.0, (i % 5 + 1) as u64 * 1_000_000);
        }
        b.iter(|| {
            black_box(calc.breakdown())
        });
    });

    group.finish();
}

// ============================================================================
// Type / State Machine Benchmarks
// ============================================================================

fn bench_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("Types");

    group.bench_function("PlayerState::can_transition_to", |b| {
        let states = [
            PlayerState::Idle,
            PlayerState::Loading,
            PlayerState::Buffering,
            PlayerState::Playing,
            PlayerState::Paused,
            PlayerState::Seeking,
            PlayerState::Ended,
            PlayerState::Error,
        ];
        b.iter(|| {
            let mut valid_count = 0u32;
            for from in &states {
                for to in &states {
                    if from.can_transition_to(*to) {
                        valid_count += 1;
                    }
                }
            }
            black_box(valid_count)
        });
    });

    group.bench_function("QualityMetrics::qoe_score", |b| {
        let metrics = QualityMetrics {
            bitrate: 5_000_000,
            resolution: Some(Resolution::new(1920, 1080)),
            dropped_frames: 12,
            decoded_frames: 9000,
            buffer_level: 15.0,
            stall_count: 2,
            stall_duration: 1.5,
            quality_switches: 4,
            throughput: 8_000_000,
        };
        b.iter(|| {
            black_box(metrics.qoe_score())
        });
    });

    group.bench_function("Rendition::quality_score", |b| {
        let renditions = create_test_renditions();
        b.iter(|| {
            let mut total = 0u32;
            for r in &renditions {
                total += r.quality_score();
            }
            black_box(total)
        });
    });

    group.bench_function("Resolution::quality_name", |b| {
        let resolutions = [
            Resolution::new(426, 240),
            Resolution::new(640, 360),
            Resolution::new(854, 480),
            Resolution::new(1280, 720),
            Resolution::new(1920, 1080),
            Resolution::new(2560, 1440),
            Resolution::new(3840, 2160),
        ];
        b.iter(|| {
            for r in &resolutions {
                black_box(r.quality_name());
            }
        });
    });

    group.finish();
}

// ============================================================================
// Memory Footprint Estimation Benchmarks
// ============================================================================

fn bench_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("Memory Footprint");

    group.bench_function("allocate_100_renditions", |b| {
        b.iter(|| {
            let mut renditions = Vec::with_capacity(100);
            for i in 0..100 {
                renditions.push(Rendition {
                    id: format!("variant_{}", i),
                    bandwidth: (i as u64 + 1) * 500_000,
                    resolution: Some(Resolution::new(
                        640 + (i as u32 * 128),
                        360 + (i as u32 * 72),
                    )),
                    frame_rate: Some(30.0),
                    video_codec: Some(VideoCodec::H264),
                    audio_codec: Some(AudioCodec::Aac),
                    uri: Url::parse(&format!("https://cdn.example.com/v{}/playlist.m3u8", i)).unwrap(),
                    hdr: None,
                    language: None,
                    name: Some(format!("Variant {}", i)),
                });
            }
            black_box(renditions)
        });
    });

    group.bench_function("allocate_1000_segments", |b| {
        b.iter(|| {
            let mut segments = Vec::with_capacity(1000);
            for i in 0..1000u64 {
                segments.push(Segment {
                    number: i,
                    uri: Url::parse(&format!("https://cdn.example.com/seg/{:05}.ts", i)).unwrap(),
                    duration: Duration::from_millis(4000 + (i % 500)),
                    byte_range: if i % 4 == 0 {
                        Some(ByteRange {
                            start: i * 100_000,
                            length: 100_000,
                        })
                    } else {
                        None
                    },
                    encryption: None,
                    discontinuity_sequence: (i / 100) as u32,
                    program_date_time: None,
                });
            }
            black_box(segments)
        });
    });

    group.bench_function("allocate_player_config", |b| {
        b.iter(|| {
            black_box(PlayerConfig::default())
        });
    });

    group.bench_function("allocate_media_tracks", |b| {
        b.iter(|| {
            let mut tracks = MediaTracks::new();

            // Add video renditions
            for i in 0..7 {
                tracks.video.push(Rendition {
                    id: format!("v{}", i),
                    bandwidth: (i as u64 + 1) * 1_000_000,
                    resolution: Some(Resolution::new(1920, 1080)),
                    frame_rate: Some(30.0),
                    video_codec: Some(VideoCodec::H264),
                    audio_codec: Some(AudioCodec::Aac),
                    uri: Url::parse(&format!("https://cdn.example.com/v{}.m3u8", i)).unwrap(),
                    hdr: None,
                    language: None,
                    name: None,
                });
            }

            // Add text tracks
            for lang in &["en", "es", "fr", "de", "ja"] {
                tracks.add_text_track(TextTrack::captions(
                    *lang,
                    *lang,
                    Url::parse(&format!("https://cdn.example.com/cc/{}.vtt", lang)).unwrap(),
                ));
            }

            // Add chapters
            for i in 0..10 {
                tracks.add_chapter(Chapter::new(
                    format!("ch{}", i),
                    format!("Chapter {}", i + 1),
                    i as f64 * 60.0,
                    (i + 1) as f64 * 60.0,
                ));
            }

            black_box(tracks)
        });
    });

    group.bench_function("serialize_quality_metrics", |b| {
        let metrics = QualityMetrics {
            bitrate: 5_000_000,
            resolution: Some(Resolution::new(1920, 1080)),
            dropped_frames: 12,
            decoded_frames: 9000,
            buffer_level: 15.0,
            stall_count: 2,
            stall_duration: 1.5,
            quality_switches: 4,
            throughput: 8_000_000,
        };
        b.iter(|| {
            black_box(serde_json::to_string(black_box(&metrics)).unwrap())
        });
    });

    group.finish();
}

// ============================================================================
// Group Registration
// ============================================================================

criterion_group!(
    buffer_benches,
    bench_buffer_allocation,
    bench_buffer_segment_insertion,
    bench_buffer_segment_sizes,
    bench_buffer_queries,
);

criterion_group!(
    hls_benches,
    bench_hls_master_parsing,
    bench_hls_media_parsing,
    bench_manifest_type_detection,
);

criterion_group!(
    abr_benches,
    bench_abr_engine_creation,
    bench_abr_rendition_selection,
    bench_abr_bandwidth_recording,
);

criterion_group!(
    branding_benches,
    bench_branding,
);

criterion_group!(
    qoe_benches,
    bench_qoe,
);

criterion_group!(
    type_benches,
    bench_types,
);

criterion_group!(
    memory_benches,
    bench_memory_footprint,
);

criterion_main!(
    buffer_benches,
    hls_benches,
    abr_benches,
    branding_benches,
    qoe_benches,
    type_benches,
    memory_benches,
);
