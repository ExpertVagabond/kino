/**
 * PSM Player Analytics Server
 *
 * Collects and serves video player analytics data including:
 * - Playback sessions
 * - QoE metrics (rebuffers, quality switches, etc.)
 * - Aggregate statistics
 *
 * Usage:
 *   npm start           # Start server
 *   npm run dev         # Start with auto-reload
 *
 * API Endpoints:
 *   POST /api/sessions           - Start a new session
 *   POST /api/sessions/:id/events - Record an event
 *   POST /api/sessions/:id/end   - End a session
 *   GET  /api/stats              - Get aggregate statistics
 *   GET  /api/stats/realtime     - Get real-time stats (SSE)
 */

import express from 'express';
import cors from 'cors';
import helmet from 'helmet';
import { v4 as uuidv4 } from 'uuid';
import Database from 'better-sqlite3';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const PORT = process.env.PORT || 3001;
const DB_PATH = process.env.DB_PATH || join(__dirname, '../data/analytics.db');

// Initialize database
const db = new Database(DB_PATH);
db.pragma('journal_mode = WAL');

// Create tables
db.exec(`
  CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    video_id TEXT,
    user_id TEXT,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    duration REAL DEFAULT 0,
    watch_time REAL DEFAULT 0,
    rebuffer_count INTEGER DEFAULT 0,
    rebuffer_duration REAL DEFAULT 0,
    quality_switches INTEGER DEFAULT 0,
    avg_bitrate REAL DEFAULT 0,
    max_bitrate REAL DEFAULT 0,
    startup_time REAL DEFAULT 0,
    qoe_score REAL DEFAULT 0,
    user_agent TEXT,
    ip_address TEXT,
    country TEXT,
    device_type TEXT,
    browser TEXT,
    os TEXT
  );

  CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    position REAL,
    data TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
  );

  CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);
  CREATE INDEX IF NOT EXISTS idx_sessions_video ON sessions(video_id);
  CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
  CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
`);

// Prepared statements
const insertSession = db.prepare(`
  INSERT INTO sessions (id, video_id, user_id, started_at, user_agent, ip_address, device_type, browser, os)
  VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
`);

const updateSession = db.prepare(`
  UPDATE sessions SET
    ended_at = ?,
    duration = ?,
    watch_time = ?,
    rebuffer_count = ?,
    rebuffer_duration = ?,
    quality_switches = ?,
    avg_bitrate = ?,
    max_bitrate = ?,
    startup_time = ?,
    qoe_score = ?
  WHERE id = ?
`);

const insertEvent = db.prepare(`
  INSERT INTO events (session_id, event_type, timestamp, position, data)
  VALUES (?, ?, ?, ?, ?)
`);

// Initialize Express
const app = express();
app.use(helmet());
app.use(cors());
app.use(express.json());

// Request logging
app.use((req, res, next) => {
  const start = Date.now();
  res.on('finish', () => {
    const duration = Date.now() - start;
    console.log(`${req.method} ${req.path} ${res.statusCode} ${duration}ms`);
  });
  next();
});

// Parse user agent
function parseUserAgent(ua) {
  const result = {
    deviceType: 'desktop',
    browser: 'unknown',
    os: 'unknown',
  };

  if (!ua) return result;

  // Device type
  if (/mobile/i.test(ua)) result.deviceType = 'mobile';
  else if (/tablet|ipad/i.test(ua)) result.deviceType = 'tablet';
  else if (/tv|smart/i.test(ua)) result.deviceType = 'tv';

  // Browser
  if (/chrome/i.test(ua)) result.browser = 'Chrome';
  else if (/firefox/i.test(ua)) result.browser = 'Firefox';
  else if (/safari/i.test(ua)) result.browser = 'Safari';
  else if (/edge/i.test(ua)) result.browser = 'Edge';

  // OS
  if (/windows/i.test(ua)) result.os = 'Windows';
  else if (/mac/i.test(ua)) result.os = 'macOS';
  else if (/linux/i.test(ua)) result.os = 'Linux';
  else if (/android/i.test(ua)) result.os = 'Android';
  else if (/ios|iphone|ipad/i.test(ua)) result.os = 'iOS';

  return result;
}

// ============================================================================
// API Routes
// ============================================================================

// Health check
app.get('/health', (req, res) => {
  res.json({ status: 'ok', timestamp: Date.now() });
});

// Start a new session
app.post('/api/sessions', (req, res) => {
  try {
    const { videoId, userId } = req.body;
    const sessionId = uuidv4();
    const ua = req.headers['user-agent'];
    const ip = req.ip || req.connection.remoteAddress;
    const { deviceType, browser, os } = parseUserAgent(ua);

    insertSession.run(
      sessionId,
      videoId || null,
      userId || null,
      Date.now(),
      ua,
      ip,
      deviceType,
      browser,
      os
    );

    res.status(201).json({ sessionId });
  } catch (error) {
    console.error('Error creating session:', error);
    res.status(500).json({ error: 'Failed to create session' });
  }
});

// Record an event
app.post('/api/sessions/:id/events', (req, res) => {
  try {
    const { id } = req.params;
    const { type, position, data } = req.body;

    insertEvent.run(
      id,
      type,
      Date.now(),
      position || null,
      data ? JSON.stringify(data) : null
    );

    res.status(201).json({ success: true });
  } catch (error) {
    console.error('Error recording event:', error);
    res.status(500).json({ error: 'Failed to record event' });
  }
});

// Batch record events
app.post('/api/sessions/:id/events/batch', (req, res) => {
  try {
    const { id } = req.params;
    const { events } = req.body;

    const insertMany = db.transaction((events) => {
      for (const event of events) {
        insertEvent.run(
          id,
          event.type,
          event.timestamp || Date.now(),
          event.position || null,
          event.data ? JSON.stringify(event.data) : null
        );
      }
    });

    insertMany(events);
    res.status(201).json({ success: true, count: events.length });
  } catch (error) {
    console.error('Error recording events:', error);
    res.status(500).json({ error: 'Failed to record events' });
  }
});

// End a session
app.post('/api/sessions/:id/end', (req, res) => {
  try {
    const { id } = req.params;
    const {
      duration,
      watchTime,
      rebufferCount,
      rebufferDuration,
      qualitySwitches,
      avgBitrate,
      maxBitrate,
      startupTime,
      qoeScore,
    } = req.body;

    updateSession.run(
      Date.now(),
      duration || 0,
      watchTime || 0,
      rebufferCount || 0,
      rebufferDuration || 0,
      qualitySwitches || 0,
      avgBitrate || 0,
      maxBitrate || 0,
      startupTime || 0,
      qoeScore || 0,
      id
    );

    res.json({ success: true });
  } catch (error) {
    console.error('Error ending session:', error);
    res.status(500).json({ error: 'Failed to end session' });
  }
});

// Get aggregate statistics
app.get('/api/stats', (req, res) => {
  try {
    const { period = '24h', videoId } = req.query;

    // Calculate time range
    const now = Date.now();
    const periodMs = {
      '1h': 60 * 60 * 1000,
      '24h': 24 * 60 * 60 * 1000,
      '7d': 7 * 24 * 60 * 60 * 1000,
      '30d': 30 * 24 * 60 * 60 * 1000,
    }[period] || 24 * 60 * 60 * 1000;

    const since = now - periodMs;

    let query = `
      SELECT
        COUNT(*) as total_sessions,
        COUNT(DISTINCT video_id) as unique_videos,
        SUM(watch_time) as total_watch_time,
        AVG(watch_time) as avg_watch_time,
        AVG(qoe_score) as avg_qoe_score,
        SUM(rebuffer_count) as total_rebuffers,
        AVG(rebuffer_count) as avg_rebuffers_per_session,
        AVG(startup_time) as avg_startup_time,
        AVG(quality_switches) as avg_quality_switches,
        AVG(avg_bitrate) as overall_avg_bitrate
      FROM sessions
      WHERE started_at >= ?
    `;

    const params = [since];

    if (videoId) {
      query += ' AND video_id = ?';
      params.push(videoId);
    }

    const stats = db.prepare(query).get(...params);

    // Get device breakdown
    const deviceStats = db.prepare(`
      SELECT device_type, COUNT(*) as count
      FROM sessions
      WHERE started_at >= ?
      GROUP BY device_type
    `).all(since);

    // Get browser breakdown
    const browserStats = db.prepare(`
      SELECT browser, COUNT(*) as count
      FROM sessions
      WHERE started_at >= ?
      GROUP BY browser
      ORDER BY count DESC
      LIMIT 5
    `).all(since);

    // Get QoE distribution
    const qoeDistribution = db.prepare(`
      SELECT
        CASE
          WHEN qoe_score >= 80 THEN 'excellent'
          WHEN qoe_score >= 60 THEN 'good'
          WHEN qoe_score >= 40 THEN 'fair'
          ELSE 'poor'
        END as quality,
        COUNT(*) as count
      FROM sessions
      WHERE started_at >= ? AND qoe_score > 0
      GROUP BY quality
    `).all(since);

    res.json({
      period,
      since: new Date(since).toISOString(),
      stats: {
        totalSessions: stats.total_sessions || 0,
        uniqueVideos: stats.unique_videos || 0,
        totalWatchTime: Math.round(stats.total_watch_time || 0),
        avgWatchTime: Math.round(stats.avg_watch_time || 0),
        avgQoeScore: Math.round((stats.avg_qoe_score || 0) * 10) / 10,
        totalRebuffers: stats.total_rebuffers || 0,
        avgRebuffersPerSession: Math.round((stats.avg_rebuffers_per_session || 0) * 100) / 100,
        avgStartupTime: Math.round(stats.avg_startup_time || 0),
        avgQualitySwitches: Math.round((stats.avg_quality_switches || 0) * 10) / 10,
        avgBitrate: Math.round(stats.overall_avg_bitrate || 0),
      },
      breakdown: {
        devices: deviceStats.reduce((acc, d) => ({ ...acc, [d.device_type]: d.count }), {}),
        browsers: browserStats.reduce((acc, b) => ({ ...acc, [b.browser]: b.count }), {}),
        qoeDistribution: qoeDistribution.reduce((acc, q) => ({ ...acc, [q.quality]: q.count }), {}),
      },
    });
  } catch (error) {
    console.error('Error getting stats:', error);
    res.status(500).json({ error: 'Failed to get statistics' });
  }
});

// Real-time stats (Server-Sent Events)
app.get('/api/stats/realtime', (req, res) => {
  res.writeHead(200, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    'Connection': 'keep-alive',
  });

  const sendStats = () => {
    const now = Date.now();
    const since = now - 60 * 1000; // Last minute

    const stats = db.prepare(`
      SELECT
        COUNT(*) as active_sessions,
        SUM(CASE WHEN started_at >= ? THEN 1 ELSE 0 END) as new_sessions
      FROM sessions
      WHERE ended_at IS NULL OR ended_at >= ?
    `).get(since, now - 5 * 60 * 1000);

    const recentEvents = db.prepare(`
      SELECT event_type, COUNT(*) as count
      FROM events
      WHERE timestamp >= ?
      GROUP BY event_type
    `).all(since);

    res.write(`data: ${JSON.stringify({
      timestamp: now,
      activeSessions: stats.active_sessions || 0,
      newSessions: stats.new_sessions || 0,
      recentEvents: recentEvents.reduce((acc, e) => ({ ...acc, [e.event_type]: e.count }), {}),
    })}\n\n`);
  };

  // Send initial stats
  sendStats();

  // Send updates every 5 seconds
  const interval = setInterval(sendStats, 5000);

  req.on('close', () => {
    clearInterval(interval);
  });
});

// Get session details
app.get('/api/sessions/:id', (req, res) => {
  try {
    const { id } = req.params;

    const session = db.prepare('SELECT * FROM sessions WHERE id = ?').get(id);
    if (!session) {
      return res.status(404).json({ error: 'Session not found' });
    }

    const events = db.prepare(`
      SELECT event_type, timestamp, position, data
      FROM events
      WHERE session_id = ?
      ORDER BY timestamp
    `).all(id);

    res.json({
      ...session,
      events: events.map(e => ({
        type: e.event_type,
        timestamp: e.timestamp,
        position: e.position,
        data: e.data ? JSON.parse(e.data) : null,
      })),
    });
  } catch (error) {
    console.error('Error getting session:', error);
    res.status(500).json({ error: 'Failed to get session' });
  }
});

// Start server
app.listen(PORT, () => {
  console.log('');
  console.log('\x1b[35m╔═══════════════════════════════════════════════════════════╗\x1b[0m');
  console.log('\x1b[35m║       Purple Squirrel Media - Analytics Server            ║\x1b[0m');
  console.log('\x1b[35m╚═══════════════════════════════════════════════════════════╝\x1b[0m');
  console.log('');
  console.log(`\x1b[32mServer running on port ${PORT}\x1b[0m`);
  console.log(`\x1b[32mDatabase: ${DB_PATH}\x1b[0m`);
  console.log('');
  console.log('Endpoints:');
  console.log('  POST /api/sessions           - Start session');
  console.log('  POST /api/sessions/:id/events - Record event');
  console.log('  POST /api/sessions/:id/end   - End session');
  console.log('  GET  /api/stats              - Get statistics');
  console.log('  GET  /api/stats/realtime     - Real-time stats (SSE)');
  console.log('');
});

// Graceful shutdown
process.on('SIGINT', () => {
  console.log('\nShutting down...');
  db.close();
  process.exit(0);
});
