//! Buffer management for video playback
//!
//! Handles:
//! - Segment prefetching
//! - Buffer level monitoring
//! - Seek buffer management
//! - Memory-efficient storage

use crate::{
    types::*,
    Result,
};
use bytes::Bytes;
use std::collections::{BTreeMap, VecDeque};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, warn, instrument};

/// Buffered segment data
#[derive(Debug, Clone)]
pub struct BufferedSegment {
    /// Segment metadata
    pub segment: Segment,
    /// Raw segment data
    pub data: Bytes,
    /// Start time in the timeline
    pub start_time: f64,
    /// End time in the timeline
    pub end_time: f64,
    /// Has this segment been consumed
    pub consumed: bool,
}

/// Buffer configuration
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// Minimum buffer before playback (seconds)
    pub min_buffer_time: f64,
    /// Maximum buffer level (seconds)
    pub max_buffer_time: f64,
    /// Rebuffer threshold (seconds)
    pub rebuffer_threshold: f64,
    /// Maximum memory usage (bytes)
    pub max_memory_bytes: usize,
    /// Enable lookahead prefetching
    pub prefetch_enabled: bool,
    /// Number of segments to prefetch
    pub prefetch_count: usize,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            min_buffer_time: 10.0,
            max_buffer_time: 30.0,
            rebuffer_threshold: 2.0,
            max_memory_bytes: 256 * 1024 * 1024, // 256 MB
            prefetch_enabled: true,
            prefetch_count: 3,
        }
    }
}

/// Buffer manager for video playback
pub struct BufferManager {
    /// Configuration
    config: BufferConfig,
    /// Buffered segments indexed by sequence number
    segments: RwLock<BTreeMap<u64, BufferedSegment>>,
    /// Current playback position
    playback_position: RwLock<f64>,
    /// Total buffered duration
    buffered_duration: RwLock<f64>,
    /// Total memory used
    memory_used: RwLock<usize>,
    /// Pending fetch queue
    fetch_queue: Mutex<VecDeque<Segment>>,
}

impl BufferManager {
    /// Create a new buffer manager
    pub fn new(config: BufferConfig) -> Self {
        Self {
            config,
            segments: RwLock::new(BTreeMap::new()),
            playback_position: RwLock::new(0.0),
            buffered_duration: RwLock::new(0.0),
            memory_used: RwLock::new(0),
            fetch_queue: Mutex::new(VecDeque::new()),
        }
    }

    /// Add a segment to the buffer
    #[instrument(skip(self, data))]
    pub async fn add_segment(&self, segment: Segment, data: Bytes) -> Result<()> {
        let segment_duration = segment.duration.as_secs_f64();
        let segment_size = data.len();

        // Check memory limit
        let current_memory = *self.memory_used.read().await;
        if current_memory + segment_size > self.config.max_memory_bytes {
            // Evict old segments
            self.evict_segments(segment_size).await?;
        }

        let segments = self.segments.read().await;
        let start_time = if let Some((_, last)) = segments.iter().last() {
            last.end_time
        } else {
            0.0
        };
        drop(segments);

        let buffered_segment = BufferedSegment {
            segment: segment.clone(),
            data,
            start_time,
            end_time: start_time + segment_duration,
            consumed: false,
        };

        // Add to buffer
        let mut segments = self.segments.write().await;
        segments.insert(segment.number, buffered_segment);

        // Update stats
        *self.buffered_duration.write().await += segment_duration;
        *self.memory_used.write().await += segment_size;

        debug!(
            segment = segment.number,
            duration = segment_duration,
            buffer_level = *self.buffered_duration.read().await,
            "Segment added to buffer"
        );

        Ok(())
    }

    /// Get the next segment to play
    pub async fn get_next_segment(&self) -> Option<BufferedSegment> {
        let playback_pos = *self.playback_position.read().await;

        let segments = self.segments.read().await;
        for (_, segment) in segments.iter() {
            if !segment.consumed && segment.end_time > playback_pos {
                return Some(segment.clone());
            }
        }
        None
    }

    /// Get segment at specific time
    pub async fn get_segment_at(&self, time: f64) -> Option<BufferedSegment> {
        let segments = self.segments.read().await;
        for (_, segment) in segments.iter() {
            if time >= segment.start_time && time < segment.end_time {
                return Some(segment.clone());
            }
        }
        None
    }

    /// Mark segment as consumed
    pub async fn consume_segment(&self, sequence: u64) {
        let mut segments = self.segments.write().await;
        if let Some(segment) = segments.get_mut(&sequence) {
            segment.consumed = true;
        }
    }

    /// Update playback position
    pub async fn update_position(&self, position: f64) {
        *self.playback_position.write().await = position;

        // Clean up consumed segments that are far behind
        self.cleanup_consumed(position).await;
    }

    /// Get current buffer level in seconds
    pub async fn buffer_level(&self) -> f64 {
        let playback_pos = *self.playback_position.read().await;
        let segments = self.segments.read().await;

        let mut buffered = 0.0;
        for (_, segment) in segments.iter() {
            if segment.end_time > playback_pos && !segment.consumed {
                let start = segment.start_time.max(playback_pos);
                buffered += segment.end_time - start;
            }
        }
        buffered
    }

    /// Check if buffer is healthy for playback
    pub async fn is_buffer_healthy(&self) -> bool {
        self.buffer_level().await >= self.config.rebuffer_threshold
    }

    /// Check if we need more data
    pub async fn needs_data(&self) -> bool {
        self.buffer_level().await < self.config.max_buffer_time
    }

    /// Can start playback
    pub async fn can_start_playback(&self) -> bool {
        self.buffer_level().await >= self.config.min_buffer_time
    }

    /// Get buffered time ranges
    pub async fn buffered_ranges(&self) -> Vec<(f64, f64)> {
        let segments = self.segments.read().await;
        let mut ranges = Vec::new();

        let mut current_start: Option<f64> = None;
        let mut current_end: f64 = 0.0;

        for (_, segment) in segments.iter() {
            if !segment.consumed {
                match current_start {
                    None => {
                        current_start = Some(segment.start_time);
                        current_end = segment.end_time;
                    }
                    Some(_) => {
                        // Check for gap
                        if (segment.start_time - current_end).abs() < 0.1 {
                            // Contiguous
                            current_end = segment.end_time;
                        } else {
                            // Gap - start new range
                            ranges.push((current_start.unwrap(), current_end));
                            current_start = Some(segment.start_time);
                            current_end = segment.end_time;
                        }
                    }
                }
            }
        }

        if let Some(start) = current_start {
            ranges.push((start, current_end));
        }

        ranges
    }

    /// Seek to position - returns true if position is buffered
    pub async fn seek(&self, position: f64) -> Result<bool> {
        *self.playback_position.write().await = position;

        // Check if position is buffered
        let is_buffered = self.get_segment_at(position).await.is_some();

        if !is_buffered {
            // Clear buffer for fresh fetch
            self.clear().await;
        }

        Ok(is_buffered)
    }

    /// Clear all buffered data
    pub async fn clear(&self) {
        let mut segments = self.segments.write().await;
        segments.clear();

        *self.buffered_duration.write().await = 0.0;
        *self.memory_used.write().await = 0;

        let mut queue = self.fetch_queue.lock().await;
        queue.clear();

        debug!("Buffer cleared");
    }

    /// Evict old segments to free memory
    async fn evict_segments(&self, needed_bytes: usize) -> Result<()> {
        let playback_pos = *self.playback_position.read().await;
        let mut segments = self.segments.write().await;
        let mut memory = self.memory_used.write().await;
        let mut duration = self.buffered_duration.write().await;

        let mut freed = 0;
        let mut to_remove = Vec::new();

        // Find segments to remove (oldest first, already consumed, behind playback)
        for (&seq, segment) in segments.iter() {
            if freed >= needed_bytes {
                break;
            }
            if segment.consumed || segment.end_time < playback_pos - 5.0 {
                to_remove.push(seq);
                freed += segment.data.len();
            }
        }

        // Remove segments
        for seq in to_remove {
            if let Some(segment) = segments.remove(&seq) {
                *memory -= segment.data.len();
                *duration -= segment.segment.duration.as_secs_f64();
                debug!(segment = seq, "Evicted segment from buffer");
            }
        }

        if freed < needed_bytes {
            warn!(
                needed = needed_bytes,
                freed = freed,
                "Could not free enough memory"
            );
        }

        Ok(())
    }

    /// Clean up consumed segments behind playback
    async fn cleanup_consumed(&self, playback_pos: f64) {
        let threshold = playback_pos - 10.0; // Keep 10s behind

        let mut segments = self.segments.write().await;
        let mut memory = self.memory_used.write().await;
        let mut duration = self.buffered_duration.write().await;

        let to_remove: Vec<_> = segments
            .iter()
            .filter(|(_, s)| s.consumed && s.end_time < threshold)
            .map(|(&seq, _)| seq)
            .collect();

        for seq in to_remove {
            if let Some(segment) = segments.remove(&seq) {
                *memory -= segment.data.len();
                *duration -= segment.segment.duration.as_secs_f64();
            }
        }
    }

    /// Get buffer statistics
    pub async fn stats(&self) -> BufferStats {
        let segments = self.segments.read().await;
        let ranges = self.buffered_ranges().await;

        BufferStats {
            segment_count: segments.len(),
            buffer_level: self.buffer_level().await,
            memory_used: *self.memory_used.read().await,
            buffered_ranges: ranges,
            playback_position: *self.playback_position.read().await,
        }
    }

    /// Queue segments for fetching
    pub async fn queue_fetch(&self, segments: Vec<Segment>) {
        let mut queue = self.fetch_queue.lock().await;
        for segment in segments {
            queue.push_back(segment);
        }
    }

    /// Get next segment to fetch
    pub async fn next_fetch(&self) -> Option<Segment> {
        let mut queue = self.fetch_queue.lock().await;
        queue.pop_front()
    }
}

/// Buffer statistics
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub segment_count: usize,
    pub buffer_level: f64,
    pub memory_used: usize,
    pub buffered_ranges: Vec<(f64, f64)>,
    pub playback_position: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use url::Url;

    fn create_test_segment(num: u64) -> Segment {
        Segment {
            number: num,
            uri: Url::parse(&format!("https://example.com/seg{}.ts", num)).unwrap(),
            duration: Duration::from_secs(4),
            byte_range: None,
            encryption: None,
            discontinuity_sequence: 0,
            program_date_time: None,
        }
    }

    #[tokio::test]
    async fn test_add_segment() {
        let buffer = BufferManager::new(BufferConfig::default());

        let segment = create_test_segment(1);
        let data = Bytes::from(vec![0u8; 1024]);

        buffer.add_segment(segment, data).await.unwrap();

        assert_eq!(buffer.buffer_level().await, 4.0);
    }

    #[tokio::test]
    async fn test_buffer_level() {
        let buffer = BufferManager::new(BufferConfig::default());

        for i in 1..=5 {
            let segment = create_test_segment(i);
            let data = Bytes::from(vec![0u8; 1024]);
            buffer.add_segment(segment, data).await.unwrap();
        }

        assert_eq!(buffer.buffer_level().await, 20.0);

        buffer.update_position(8.0).await;
        assert!((buffer.buffer_level().await - 12.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_seek_buffered() {
        let buffer = BufferManager::new(BufferConfig::default());

        for i in 1..=5 {
            let segment = create_test_segment(i);
            let data = Bytes::from(vec![0u8; 1024]);
            buffer.add_segment(segment, data).await.unwrap();
        }

        // Seek within buffered range
        let is_buffered = buffer.seek(10.0).await.unwrap();
        assert!(is_buffered);

        // Seek outside buffered range
        let is_buffered = buffer.seek(100.0).await.unwrap();
        assert!(!is_buffered);
    }
}
