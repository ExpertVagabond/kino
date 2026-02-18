//! Basic playback example
//!
//! Demonstrates the PSM Player types and configuration.
//!
//! Run with: cargo run -p psm-player-core --example basic_playback

use psm_player_core::{
    PlayerConfig, PlayerState, Resolution,
    AbrAlgorithmType,
};

fn main() {
    println!("PSM Player Core - Basic Playback Example");
    println!("=========================================\n");

    // Create player configuration
    let config = PlayerConfig {
        min_buffer_time: 10.0,
        max_buffer_time: 30.0,
        abr_algorithm: AbrAlgorithmType::Bola,
        prefetch_enabled: true,
        ..Default::default()
    };

    println!("Configuration:");
    println!("  - ABR Algorithm: {:?}", config.abr_algorithm);
    println!("  - Buffer: {:.1}s - {:.1}s", config.min_buffer_time, config.max_buffer_time);
    println!("  - Prefetch: {}", config.prefetch_enabled);
    println!("  - Analytics: {}\n", config.analytics_enabled);

    // Simulate available quality levels
    let levels = vec![
        Resolution::new(854, 480),
        Resolution::new(1280, 720),
        Resolution::new(1920, 1080),
        Resolution::new(3840, 2160),
    ];

    println!("Available quality levels:");
    for level in &levels {
        println!("  - {} ({}x{})", level.quality_name(), level.width, level.height);
    }
    println!();

    // Demonstrate state transitions
    println!("Player State Transitions:");
    println!("--------------------------");

    let transitions = [
        (PlayerState::Idle, PlayerState::Loading),
        (PlayerState::Loading, PlayerState::Buffering),
        (PlayerState::Buffering, PlayerState::Playing),
        (PlayerState::Playing, PlayerState::Paused),
        (PlayerState::Paused, PlayerState::Playing),
        (PlayerState::Playing, PlayerState::Buffering),
        (PlayerState::Buffering, PlayerState::Playing),
        (PlayerState::Playing, PlayerState::Ended),
    ];

    for (from, to) in transitions {
        let can_transition = from.can_transition_to(to);
        let symbol = if can_transition { "✓" } else { "✗" };
        println!("  {} {:?} -> {:?}", symbol, from, to);
    }
    println!();

    // Show invalid transitions
    println!("Invalid Transitions (blocked by state machine):");
    let invalid = [
        (PlayerState::Idle, PlayerState::Playing),
        (PlayerState::Playing, PlayerState::Idle),
        (PlayerState::Ended, PlayerState::Playing),
    ];

    for (from, to) in invalid {
        println!("  ✗ {:?} -> {:?}", from, to);
    }

    println!("\nExample complete!");
}
