//! CLI command implementations

use kino_core::manifest::create_parser;
use std::path::PathBuf;
use url::Url;

/// Analyze a manifest
pub async fn analyze(manifest_url: &str, _format: &str) -> anyhow::Result<()> {
    println!("Analyzing manifest: {}", manifest_url);

    let url = Url::parse(manifest_url)?;
    let parser = create_parser(&url);
    let manifest = parser.parse(&url).await?;

    println!("\nManifest Analysis:");
    println!("  Type: {:?}", manifest.manifest_type);
    println!("  Live: {}", manifest.is_live);
    println!("  Duration: {:?}", manifest.duration);
    println!("  Renditions: {}", manifest.renditions.len());

    println!("\nRenditions:");
    for (i, r) in manifest.renditions.iter().enumerate() {
        println!("  {}. {} - {}bps {:?}",
            i + 1,
            r.id,
            r.bandwidth,
            r.resolution
        );
    }

    Ok(())
}

/// Validate stream accessibility
pub async fn validate(
    manifest_url: &str,
    segments: usize,
    all_renditions: bool,
    _format: &str,
) -> anyhow::Result<()> {
    println!("Validating stream: {}", manifest_url);
    println!("  Testing {} segments", segments);
    println!("  All renditions: {}", all_renditions);

    let url = Url::parse(manifest_url)?;
    let parser = create_parser(&url);
    let manifest = parser.parse(&url).await?;

    let renditions_to_test = if all_renditions {
        manifest.renditions.clone()
    } else {
        // Just test highest and lowest
        let mut r = Vec::new();
        if let Some(first) = manifest.renditions.first() {
            r.push(first.clone());
        }
        if manifest.renditions.len() > 1 {
            if let Some(last) = manifest.renditions.last() {
                r.push(last.clone());
            }
        }
        r
    };

    let mut passed = 0;
    let mut failed = 0;

    for rendition in &renditions_to_test {
        print!("  Testing {} ({})... ", rendition.id, rendition.bandwidth);

        // Fetch segment playlist
        match parser.parse_variant(&rendition.uri).await {
            Ok(segments_list) => {
                let test_count = segments.min(segments_list.len());
                let mut seg_passed = 0;

                let client = reqwest::Client::new();
                for seg in segments_list.iter().take(test_count) {
                    // Try to HEAD request each segment
                    if let Ok(resp) = client.head(seg.uri.as_str()).send().await {
                        if resp.status().is_success() {
                            seg_passed += 1;
                        }
                    }
                }

                if seg_passed == test_count {
                    println!("PASS ({}/{})", seg_passed, test_count);
                    passed += 1;
                } else {
                    println!("PARTIAL ({}/{})", seg_passed, test_count);
                    failed += 1;
                }
            }
            Err(e) => {
                println!("FAIL ({})", e);
                failed += 1;
            }
        }
    }

    println!("\nResults: {} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Run QC checks
pub async fn qc(
    manifest_url: &str,
    output: Option<PathBuf>,
    strict: bool,
    _format: &str,
) -> anyhow::Result<()> {
    println!("Running QC on: {}", manifest_url);

    let url = Url::parse(manifest_url)?;
    let parser = create_parser(&url);
    let manifest = parser.parse(&url).await?;

    let mut warnings: Vec<&str> = Vec::new();
    let errors: Vec<&str> = Vec::new();

    // Check: Must have at least 2 renditions for ABR
    if manifest.renditions.len() < 2 {
        warnings.push("Less than 2 renditions - ABR not possible");
    }

    // Check: Bitrate ladder should have reasonable gaps
    for window in manifest.renditions.windows(2) {
        let ratio = window[1].bandwidth as f64 / window[0].bandwidth as f64;
        if ratio > 3.0 {
            warnings.push("Large bitrate gap between adjacent renditions");
        }
        if ratio < 1.3 {
            warnings.push("Small bitrate gap - may cause ABR oscillation");
        }
    }

    // Check: Should have HD rendition
    let has_hd = manifest.renditions.iter().any(|r| {
        r.resolution.map(|res| res.height >= 720).unwrap_or(false)
    });
    if !has_hd {
        warnings.push("No HD rendition (720p+)");
    }

    // Check: Should have mobile-friendly rendition
    let has_low = manifest.renditions.iter().any(|r| r.bandwidth < 1_000_000);
    if !has_low {
        warnings.push("No low-bitrate rendition for mobile");
    }

    println!("\nQC Report:");
    println!("  Renditions: {}", manifest.renditions.len());
    println!("  Errors: {}", errors.len());
    println!("  Warnings: {}", warnings.len());

    if !warnings.is_empty() {
        println!("\nWarnings:");
        for w in &warnings {
            println!("  - {}", w);
        }
    }

    if !errors.is_empty() {
        println!("\nErrors:");
        for e in &errors {
            println!("  - {}", e);
        }
    }

    // Save report if output specified
    if let Some(path) = output {
        let report = serde_json::json!({
            "url": manifest_url,
            "renditions": manifest.renditions.len(),
            "errors": errors,
            "warnings": warnings,
        });
        std::fs::write(path, serde_json::to_string_pretty(&report)?)?;
    }

    if !errors.is_empty() || (strict && !warnings.is_empty()) {
        std::process::exit(1);
    }

    println!("\nQC: PASSED");
    Ok(())
}

/// Extract metadata
pub async fn extract(manifest_url: &str, what: &str, _format: &str) -> anyhow::Result<()> {
    let url = Url::parse(manifest_url)?;
    let parser = create_parser(&url);
    let manifest = parser.parse(&url).await?;

    match what {
        "bitrates" => {
            println!("Bitrates (bps):");
            for r in &manifest.renditions {
                println!("  {}: {}", r.id, r.bandwidth);
            }
        }
        "durations" => {
            println!("Duration: {:?}", manifest.duration);
            println!("Target segment: {:?}", manifest.target_duration);
        }
        "segments" => {
            for r in &manifest.renditions {
                println!("Segments for {}:", r.id);
                let segments = parser.parse_variant(&r.uri).await?;
                for s in segments.iter().take(10) {
                    println!("  #{}: {:?}", s.number, s.duration);
                }
                if segments.len() > 10 {
                    println!("  ... and {} more", segments.len() - 10);
                }
            }
        }
        _ => {
            println!("Full manifest data:");
            println!("{:#?}", manifest);
        }
    }

    Ok(())
}

/// Compare two streams
pub async fn compare(manifest1: &str, manifest2: &str, _format: &str) -> anyhow::Result<()> {
    println!("Comparing streams:");
    println!("  1: {}", manifest1);
    println!("  2: {}", manifest2);

    let url1 = Url::parse(manifest1)?;
    let url2 = Url::parse(manifest2)?;

    let parser1 = create_parser(&url1);
    let parser2 = create_parser(&url2);

    let m1 = parser1.parse(&url1).await?;
    let m2 = parser2.parse(&url2).await?;

    println!("\nComparison:");
    println!("  {:20} {:>15} {:>15}", "Property", "Stream 1", "Stream 2");
    println!("  {:20} {:>15} {:>15}", "Type", format!("{:?}", m1.manifest_type), format!("{:?}", m2.manifest_type));
    println!("  {:20} {:>15} {:>15}", "Live", m1.is_live, m2.is_live);
    println!("  {:20} {:>15} {:>15}", "Renditions", m1.renditions.len(), m2.renditions.len());

    let max_br1 = m1.renditions.iter().map(|r| r.bandwidth).max().unwrap_or(0);
    let max_br2 = m2.renditions.iter().map(|r| r.bandwidth).max().unwrap_or(0);
    println!("  {:20} {:>15} {:>15}", "Max Bitrate", format!("{}Mbps", max_br1 / 1_000_000), format!("{}Mbps", max_br2 / 1_000_000));

    Ok(())
}

/// Monitor a live stream
pub async fn monitor(
    manifest_url: &str,
    interval: u64,
    duration: u64,
    _format: &str,
) -> anyhow::Result<()> {
    println!("Monitoring: {}", manifest_url);
    println!("  Interval: {}s", interval);
    println!("  Duration: {}", if duration == 0 { "indefinite".to_string() } else { format!("{}s", duration) });

    let url = Url::parse(manifest_url)?;
    let parser = create_parser(&url);

    let start = std::time::Instant::now();
    let mut last_sequence = 0u64;

    loop {
        // Check duration limit
        if duration > 0 && start.elapsed().as_secs() >= duration {
            println!("\nMonitoring complete.");
            break;
        }

        // Fetch and check manifest
        match parser.parse(&url).await {
            Ok(manifest) => {
                if manifest.is_live {
                    // For live, check for new segments
                    if let Some(r) = manifest.renditions.first() {
                        if let Ok(segments) = parser.parse_variant(&r.uri).await {
                            let max_seq = segments.iter().map(|s| s.number).max().unwrap_or(0);
                            if max_seq > last_sequence {
                                println!("[{}] New segments: {} -> {}",
                                    chrono::Utc::now().format("%H:%M:%S"),
                                    last_sequence,
                                    max_seq
                                );
                                last_sequence = max_seq;
                            }
                        }
                    }
                }
                println!("[{}] OK - {} renditions",
                    chrono::Utc::now().format("%H:%M:%S"),
                    manifest.renditions.len()
                );
            }
            Err(e) => {
                println!("[{}] ERROR: {}",
                    chrono::Utc::now().format("%H:%M:%S"),
                    e
                );
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }

    Ok(())
}
