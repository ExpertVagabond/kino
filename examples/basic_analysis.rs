//! Basic HLS manifest analysis using kino-core.
//!
//! ```bash
//! cargo run --example basic_analysis
//! ```

use kino_core::manifest::{create_parser, ManifestType};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = Url::parse("https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8")?;

    let parser = create_parser(&url);
    let manifest = parser.parse(&url).await?;

    println!("Type:       {:?}", manifest.manifest_type);
    println!("Live:       {}", manifest.is_live);
    println!("Duration:   {:?}", manifest.duration);
    println!("Renditions: {}", manifest.renditions.len());

    for (i, r) in manifest.renditions.iter().enumerate() {
        println!(
            "  {i}. {id} -- {bw} bps, resolution: {res:?}",
            id = r.id,
            bw = r.bandwidth,
            res = r.resolution,
        );
    }

    Ok(())
}
