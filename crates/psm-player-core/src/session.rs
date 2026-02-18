//! Player Session - Main orchestrator for playback
//!
//! Coordinates:
//! - Manifest loading and parsing
//! - Segment fetching and buffering
//! - ABR selection
//! - State machine transitions
//! - Analytics events

use crate::{
    abr::{AbrContext, AbrEngine},
    analytics::{AnalyticsEmitter, AnalyticsEvent},
    buffer::{BufferConfig, BufferManager},
    Error,
    manifest::{create_parser, Manifest},
    types::*,
    Result,
};
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, watch};
use tracing::{debug, info, instrument, warn};
use url::Url;

/// Player session managing a single playback
pub struct PlayerSession {
    /// Unique session ID
    id: SessionId,
    /// Session configuration
    config: PlayerConfig,
    /// Current player state
    state: Arc<RwLock<PlayerState>>,
    /// State change broadcaster
    state_tx: watch::Sender<PlayerState>,
    /// Buffer manager
    buffer: Arc<BufferManager>,
    /// ABR engine
    abr: Arc<RwLock<AbrEngine>>,
    /// HTTP client
    client: Client,
    /// Current manifest
    manifest: Arc<RwLock<Option<Manifest>>>,
    /// Current rendition
    current_rendition: Arc<RwLock<Option<Rendition>>>,
    /// Playback position
    position: Arc<RwLock<f64>>,
    /// Content duration (if known)
    duration: Arc<RwLock<Option<f64>>>,
    /// Quality metrics
    metrics: Arc<RwLock<QualityMetrics>>,
    /// Analytics emitter
    analytics: Option<Arc<AnalyticsEmitter>>,
    /// Session start time
    start_time: Instant,
}

impl PlayerSession {
    /// Create a new player session
    pub fn new(config: PlayerConfig) -> Self {
        let (state_tx, _) = watch::channel(PlayerState::Idle);

        let buffer_config = BufferConfig {
            min_buffer_time: config.min_buffer_time,
            max_buffer_time: config.max_buffer_time,
            rebuffer_threshold: config.rebuffer_threshold,
            prefetch_enabled: config.prefetch_enabled,
            ..Default::default()
        };

        let analytics = if config.analytics_enabled {
            Some(Arc::new(AnalyticsEmitter::new()))
        } else {
            None
        };

        Self {
            id: SessionId::new(),
            config: config.clone(),
            state: Arc::new(RwLock::new(PlayerState::Idle)),
            state_tx,
            buffer: Arc::new(BufferManager::new(buffer_config)),
            abr: Arc::new(RwLock::new(AbrEngine::new(config.abr_algorithm))),
            client: Client::builder()
                .timeout(Duration::from_millis(config.request_timeout_ms))
                .build()
                .expect("Failed to create HTTP client"),
            manifest: Arc::new(RwLock::new(None)),
            current_rendition: Arc::new(RwLock::new(None)),
            position: Arc::new(RwLock::new(0.0)),
            duration: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(QualityMetrics::default())),
            analytics,
            start_time: Instant::now(),
        }
    }

    /// Get session ID
    pub fn id(&self) -> SessionId {
        self.id
    }

    /// Get current state
    pub async fn state(&self) -> PlayerState {
        *self.state.read().await
    }

    /// Subscribe to state changes
    pub fn subscribe_state(&self) -> watch::Receiver<PlayerState> {
        self.state_tx.subscribe()
    }

    /// Transition to new state
    async fn set_state(&self, new_state: PlayerState) -> Result<()> {
        let current = *self.state.read().await;

        if !current.can_transition_to(new_state) {
            return Err(Error::InvalidStateTransition {
                from: current.to_string(),
                to: new_state.to_string(),
            });
        }

        *self.state.write().await = new_state;
        let _ = self.state_tx.send(new_state);

        // Emit analytics event
        if let Some(ref analytics) = self.analytics {
            analytics.emit(AnalyticsEvent::StateChange {
                from: current,
                to: new_state,
                position: *self.position.read().await,
            }).await;
        }

        info!(from = %current, to = %new_state, "State transition");

        Ok(())
    }

    /// Load content from URL
    #[instrument(skip(self))]
    pub async fn load(&self, url: &Url) -> Result<()> {
        info!(url = %url, session_id = %self.id, "Loading content");

        self.set_state(PlayerState::Loading).await?;

        // Parse manifest
        let parser = create_parser(url);
        let manifest = parser.parse(url).await?;

        info!(
            renditions = manifest.renditions.len(),
            is_live = manifest.is_live,
            "Manifest parsed"
        );

        // Store manifest
        *self.manifest.write().await = Some(manifest.clone());

        // Set duration if VOD
        if let Some(duration) = manifest.duration {
            *self.duration.write().await = Some(duration.as_secs_f64());
        }

        // Select initial rendition
        let context = self.create_abr_context().await;
        let mut abr = self.abr.write().await;
        if let Some(rendition) = abr.select_rendition(&manifest.renditions, &context) {
            *self.current_rendition.write().await = Some(rendition.clone());
            info!(rendition = %rendition.id, bandwidth = rendition.bandwidth, "Initial rendition selected");
        }

        // Emit load event
        if let Some(ref analytics) = self.analytics {
            analytics.emit(AnalyticsEvent::Load {
                url: url.to_string(),
                is_live: manifest.is_live,
            }).await;
        }

        // Transition to buffering
        self.set_state(PlayerState::Buffering).await?;

        Ok(())
    }

    /// Start playback
    #[instrument(skip(self))]
    pub async fn play(&self) -> Result<()> {
        let current_state = self.state().await;

        match current_state {
            PlayerState::Buffering => {
                // Wait for buffer
                if self.buffer.can_start_playback().await {
                    self.set_state(PlayerState::Playing).await?;
                }
            }
            PlayerState::Paused => {
                self.set_state(PlayerState::Playing).await?;
            }
            PlayerState::Ended => {
                // Restart from beginning
                self.seek(0.0).await?;
                self.set_state(PlayerState::Playing).await?;
            }
            _ => {
                warn!(state = %current_state, "Cannot play from current state");
            }
        }

        // Emit play event
        if let Some(ref analytics) = self.analytics {
            analytics.emit(AnalyticsEvent::Play {
                position: *self.position.read().await,
            }).await;
        }

        Ok(())
    }

    /// Pause playback
    #[instrument(skip(self))]
    pub async fn pause(&self) -> Result<()> {
        if self.state().await == PlayerState::Playing {
            self.set_state(PlayerState::Paused).await?;

            // Emit pause event
            if let Some(ref analytics) = self.analytics {
                analytics.emit(AnalyticsEvent::Pause {
                    position: *self.position.read().await,
                }).await;
            }
        }
        Ok(())
    }

    /// Seek to position
    #[instrument(skip(self))]
    pub async fn seek(&self, position: f64) -> Result<()> {
        let duration = self.duration.read().await;

        // Clamp position
        let clamped = if let Some(dur) = *duration {
            position.clamp(0.0, dur)
        } else {
            position.max(0.0)
        };

        info!(from = *self.position.read().await, to = clamped, "Seeking");

        // Update state
        let was_playing = self.state().await == PlayerState::Playing;
        self.set_state(PlayerState::Seeking).await?;

        // Check if position is buffered
        let is_buffered = self.buffer.seek(clamped).await?;

        // Update position
        *self.position.write().await = clamped;

        // Emit seek event
        if let Some(ref analytics) = self.analytics {
            analytics.emit(AnalyticsEvent::Seek {
                from: *self.position.read().await,
                to: clamped,
            }).await;
        }

        if is_buffered && was_playing {
            self.set_state(PlayerState::Playing).await?;
        } else {
            self.set_state(PlayerState::Buffering).await?;
        }

        Ok(())
    }

    /// Stop playback and reset
    #[instrument(skip(self))]
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping playback");

        self.buffer.clear().await;
        *self.position.write().await = 0.0;
        *self.manifest.write().await = None;
        *self.current_rendition.write().await = None;

        // Force state to Idle
        *self.state.write().await = PlayerState::Idle;
        let _ = self.state_tx.send(PlayerState::Idle);

        // Emit end event
        if let Some(ref analytics) = self.analytics {
            analytics.emit(AnalyticsEvent::End {
                position: *self.position.read().await,
                watch_time: self.start_time.elapsed().as_secs_f64(),
            }).await;
        }

        Ok(())
    }

    /// Get current position
    pub async fn position(&self) -> f64 {
        *self.position.read().await
    }

    /// Get content duration
    pub async fn duration(&self) -> Option<f64> {
        *self.duration.read().await
    }

    /// Get current rendition
    pub async fn current_rendition(&self) -> Option<Rendition> {
        self.current_rendition.read().await.clone()
    }

    /// Get buffer level
    pub async fn buffer_level(&self) -> f64 {
        self.buffer.buffer_level().await
    }

    /// Get quality metrics
    pub async fn metrics(&self) -> QualityMetrics {
        self.metrics.read().await.clone()
    }

    /// Get buffered ranges
    pub async fn buffered_ranges(&self) -> Vec<(f64, f64)> {
        self.buffer.buffered_ranges().await
    }

    /// Create ABR context from current state
    async fn create_abr_context(&self) -> AbrContext {
        let manifest = self.manifest.read().await;
        let is_live = manifest.as_ref().map(|m| m.is_live).unwrap_or(false);

        AbrContext {
            buffer_level: self.buffer.buffer_level().await,
            target_buffer: self.config.max_buffer_time,
            playback_rate: 1.0,
            is_live,
            screen_width: None,
            max_bitrate: self.config.max_bitrate,
            network: NetworkInfo {
                bandwidth_estimate: self.abr.read().await.bandwidth_estimate(),
                ..Default::default()
            },
        }
    }

    /// Fetch next segment
    #[instrument(skip(self))]
    pub async fn fetch_segment(&self, segment: &Segment) -> Result<bytes::Bytes> {
        let start = Instant::now();

        let response = self
            .client
            .get(segment.uri.clone())
            .send()
            .await
            .map_err(|e| Error::SegmentFetch {
                url: segment.uri.to_string(),
                source: e,
            })?;

        let data = response
            .bytes()
            .await
            .map_err(|e| Error::SegmentFetch {
                url: segment.uri.to_string(),
                source: e,
            })?;

        let duration = start.elapsed();
        let bytes = data.len();

        // Record bandwidth measurement
        self.abr.write().await.record_measurement(bytes, duration);

        debug!(
            segment = segment.number,
            bytes = bytes,
            duration_ms = duration.as_millis(),
            "Segment fetched"
        );

        Ok(data)
    }

    /// Update playback position (called by renderer)
    pub async fn update_position(&self, position: f64) {
        *self.position.write().await = position;
        self.buffer.update_position(position).await;

        // Check for end of content
        if let Some(duration) = *self.duration.read().await {
            if position >= duration - 0.5 {
                let _ = self.set_state(PlayerState::Ended).await;
            }
        }

        // Check buffer health
        if self.state().await == PlayerState::Playing && !self.buffer.is_buffer_healthy().await {
            let mut metrics = self.metrics.write().await;
            metrics.stall_count += 1;
            let _ = self.set_state(PlayerState::Buffering).await;

            // Emit rebuffer event
            if let Some(ref analytics) = self.analytics {
                analytics.emit(AnalyticsEvent::Rebuffer {
                    position,
                    buffer_level: self.buffer.buffer_level().await,
                }).await;
            }
        }
    }

    /// Report dropped frame
    pub async fn report_dropped_frame(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.dropped_frames += 1;
    }

    /// Report decoded frame
    pub async fn report_decoded_frame(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.decoded_frames += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let config = PlayerConfig::default();
        let session = PlayerSession::new(config);

        assert_eq!(session.state().await, PlayerState::Idle);
        assert_eq!(session.position().await, 0.0);
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let config = PlayerConfig::default();
        let session = PlayerSession::new(config);

        // Valid: Idle -> Loading
        assert!(session.set_state(PlayerState::Loading).await.is_ok());
        assert_eq!(session.state().await, PlayerState::Loading);

        // Valid: Loading -> Buffering
        assert!(session.set_state(PlayerState::Buffering).await.is_ok());

        // Invalid: Buffering -> Ended (need to go through Playing first)
        // Actually Buffering -> Playing -> Ended is the path
    }
}
