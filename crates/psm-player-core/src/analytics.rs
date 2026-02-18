//! Analytics event emission
//!
//! Captures playback events for:
//! - Quality of Experience (QoE) metrics
//! - Error tracking
//! - Usage analytics
//! - A/B testing

use crate::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};
use uuid::Uuid;

/// Analytics event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AnalyticsEvent {
    /// Content loaded
    Load {
        url: String,
        is_live: bool,
    },

    /// Playback started
    Play {
        position: f64,
    },

    /// Playback paused
    Pause {
        position: f64,
    },

    /// Seek performed
    Seek {
        from: f64,
        to: f64,
    },

    /// Rebuffering started
    Rebuffer {
        position: f64,
        buffer_level: f64,
    },

    /// Rebuffering ended
    RebufferEnd {
        position: f64,
        duration: f64,
    },

    /// Quality change
    QualityChange {
        from_bitrate: u64,
        to_bitrate: u64,
        from_resolution: Option<Resolution>,
        to_resolution: Option<Resolution>,
        reason: QualityChangeReason,
    },

    /// State change
    StateChange {
        from: PlayerState,
        to: PlayerState,
        position: f64,
    },

    /// Playback ended
    End {
        position: f64,
        watch_time: f64,
    },

    /// Error occurred
    Error {
        code: String,
        message: String,
        fatal: bool,
        position: f64,
    },

    /// Heartbeat (periodic)
    Heartbeat {
        position: f64,
        buffer_level: f64,
        bitrate: u64,
        dropped_frames: u64,
        decoded_frames: u64,
    },

    /// Custom event
    Custom {
        name: String,
        data: serde_json::Value,
    },
}

/// Reason for quality change
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityChangeReason {
    /// ABR algorithm decision
    Abr,
    /// User manual selection
    Manual,
    /// Buffer-based downgrade
    Buffer,
    /// Initial selection
    Initial,
}

/// Analytics event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEventRecord {
    /// Unique event ID
    pub id: Uuid,
    /// Session ID
    pub session_id: SessionId,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Sequence number
    pub sequence: u64,
    /// The event
    #[serde(flatten)]
    pub event: AnalyticsEvent,
}

/// Analytics emitter
pub struct AnalyticsEmitter {
    /// Session ID
    session_id: SessionId,
    /// Event sequence counter
    sequence: RwLock<u64>,
    /// Event buffer
    buffer: RwLock<Vec<AnalyticsEventRecord>>,
    /// Maximum buffer size before flush
    max_buffer_size: usize,
    /// Event channel for async processing
    event_tx: mpsc::Sender<AnalyticsEventRecord>,
    /// Beacon endpoint (if configured)
    beacon_url: Option<String>,
}

impl AnalyticsEmitter {
    /// Create a new analytics emitter
    pub fn new() -> Self {
        let (event_tx, mut event_rx) = mpsc::channel::<AnalyticsEventRecord>(1000);

        // Spawn background processor
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                // In production, batch and send to analytics endpoint
                debug!(
                    event_id = %event.id,
                    event = ?event.event,
                    "Analytics event"
                );
            }
        });

        Self {
            session_id: SessionId::new(),
            sequence: RwLock::new(0),
            buffer: RwLock::new(Vec::new()),
            max_buffer_size: 50,
            event_tx,
            beacon_url: None,
        }
    }

    /// Create with beacon endpoint
    pub fn with_beacon(beacon_url: String) -> Self {
        let mut emitter = Self::new();
        emitter.beacon_url = Some(beacon_url);
        emitter
    }

    /// Emit an analytics event
    pub async fn emit(&self, event: AnalyticsEvent) {
        let mut seq = self.sequence.write().await;
        *seq += 1;
        let sequence = *seq;

        let record = AnalyticsEventRecord {
            id: Uuid::new_v4(),
            session_id: self.session_id,
            timestamp: Utc::now(),
            sequence,
            event,
        };

        // Add to buffer
        let mut buffer = self.buffer.write().await;
        buffer.push(record.clone());

        // Flush if buffer is full
        if buffer.len() >= self.max_buffer_size {
            let events: Vec<_> = buffer.drain(..).collect();
            drop(buffer);
            self.flush_events(events).await;
        }

        // Send to channel for async processing
        let _ = self.event_tx.send(record).await;
    }

    /// Flush buffered events
    async fn flush_events(&self, events: Vec<AnalyticsEventRecord>) {
        if events.is_empty() {
            return;
        }

        info!(count = events.len(), "Flushing analytics events");

        // In production, send to analytics endpoint
        if let Some(ref url) = self.beacon_url {
            // Use reqwest to send events
            // This is fire-and-forget for beacons
            let client = reqwest::Client::new();
            let _ = client.post(url)
                .json(&events)
                .send()
                .await;
        }
    }

    /// Get all buffered events
    pub async fn get_events(&self) -> Vec<AnalyticsEventRecord> {
        self.buffer.read().await.clone()
    }

    /// Clear buffer
    pub async fn clear(&self) {
        self.buffer.write().await.clear();
    }

    /// Set beacon endpoint
    pub fn set_beacon_url(&mut self, url: String) {
        self.beacon_url = Some(url);
    }
}

impl Default for AnalyticsEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// QoE (Quality of Experience) calculator
pub struct QoeCalculator {
    /// Initial buffer time
    initial_buffer_time: f64,
    /// Total rebuffer count
    rebuffer_count: u32,
    /// Total rebuffer duration
    rebuffer_duration: f64,
    /// Playback start time
    _start_time: f64,
    /// Quality switches
    quality_switches: Vec<(f64, u64)>, // (timestamp, bitrate)
    /// Average bitrate (weighted by time)
    bitrate_samples: Vec<(f64, u64)>, // (duration, bitrate)
}

impl QoeCalculator {
    pub fn new() -> Self {
        Self {
            initial_buffer_time: 0.0,
            rebuffer_count: 0,
            rebuffer_duration: 0.0,
            _start_time: 0.0,
            quality_switches: Vec::new(),
            bitrate_samples: Vec::new(),
        }
    }

    /// Record initial buffering time
    pub fn record_initial_buffer(&mut self, duration: f64) {
        self.initial_buffer_time = duration;
    }

    /// Record rebuffer event
    pub fn record_rebuffer(&mut self, duration: f64) {
        self.rebuffer_count += 1;
        self.rebuffer_duration += duration;
    }

    /// Record quality switch
    pub fn record_quality_switch(&mut self, timestamp: f64, bitrate: u64) {
        self.quality_switches.push((timestamp, bitrate));
    }

    /// Record bitrate sample
    pub fn record_bitrate(&mut self, duration: f64, bitrate: u64) {
        self.bitrate_samples.push((duration, bitrate));
    }

    /// Calculate QoE score (0-100)
    pub fn calculate_qoe(&self) -> f64 {
        // MOS-like scoring based on:
        // - Initial buffer time (startup delay)
        // - Rebuffer frequency and duration
        // - Average quality
        // - Quality stability

        let mut score = 100.0;

        // Penalize initial buffer time
        // > 2s starts reducing score
        if self.initial_buffer_time > 2.0 {
            score -= (self.initial_buffer_time - 2.0) * 5.0;
        }

        // Penalize rebuffers heavily
        // Each rebuffer costs 10 points
        score -= self.rebuffer_count as f64 * 10.0;

        // Penalize rebuffer duration
        // Each second of rebuffering costs 5 points
        score -= self.rebuffer_duration * 5.0;

        // Penalize quality switches
        // Each switch costs 2 points
        score -= self.quality_switches.len() as f64 * 2.0;

        // Bonus for high average bitrate
        let avg_bitrate = self.average_bitrate();
        if avg_bitrate > 5_000_000 {
            score += 5.0;
        } else if avg_bitrate > 2_000_000 {
            score += 2.0;
        }

        score.clamp(0.0, 100.0)
    }

    /// Calculate average bitrate
    fn average_bitrate(&self) -> u64 {
        if self.bitrate_samples.is_empty() {
            return 0;
        }

        let total_duration: f64 = self.bitrate_samples.iter().map(|(d, _)| d).sum();
        if total_duration == 0.0 {
            return 0;
        }

        let weighted_sum: f64 = self.bitrate_samples
            .iter()
            .map(|(d, b)| d * *b as f64)
            .sum();

        (weighted_sum / total_duration) as u64
    }

    /// Get QoE breakdown
    pub fn breakdown(&self) -> QoeBreakdown {
        QoeBreakdown {
            score: self.calculate_qoe(),
            initial_buffer_time: self.initial_buffer_time,
            rebuffer_count: self.rebuffer_count,
            rebuffer_duration: self.rebuffer_duration,
            quality_switches: self.quality_switches.len() as u32,
            average_bitrate: self.average_bitrate(),
        }
    }
}

impl Default for QoeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// QoE score breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QoeBreakdown {
    pub score: f64,
    pub initial_buffer_time: f64,
    pub rebuffer_count: u32,
    pub rebuffer_duration: f64,
    pub quality_switches: u32,
    pub average_bitrate: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qoe_perfect() {
        let calc = QoeCalculator::new();
        assert_eq!(calc.calculate_qoe(), 100.0);
    }

    #[test]
    fn test_qoe_with_rebuffers() {
        let mut calc = QoeCalculator::new();
        calc.record_rebuffer(1.0);
        calc.record_rebuffer(2.0);

        // 100 - 2*10 - 3*5 = 65
        assert!((calc.calculate_qoe() - 65.0).abs() < 0.1);
    }

    #[test]
    fn test_qoe_with_initial_buffer() {
        let mut calc = QoeCalculator::new();
        calc.record_initial_buffer(5.0); // 3 seconds over threshold

        // 100 - 3*5 = 85
        assert!((calc.calculate_qoe() - 85.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_analytics_emitter() {
        let emitter = AnalyticsEmitter::new();

        emitter.emit(AnalyticsEvent::Play { position: 0.0 }).await;
        emitter.emit(AnalyticsEvent::Pause { position: 10.0 }).await;

        let events = emitter.get_events().await;
        assert_eq!(events.len(), 2);
    }
}
