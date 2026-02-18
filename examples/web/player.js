/**
 * PSM Player Demo - Web Integration Example
 *
 * This demonstrates how to use the PSM Player WASM module alongside hls.js
 * for enhanced adaptive bitrate selection and analytics.
 */

// Import the WASM module (adjust path based on your build setup)
// In production, this would be: import init, { ... } from '@purplesquirrel/player-wasm';
import init, {
    PsmBranding,
    PsmAnalytics,
    PsmBufferController,
    PsmAbrController,
    WasmConfig,
    version
} from '../../crates/psm-player-wasm/pkg/psm_player_wasm.js';

class PsmPlayer {
    constructor() {
        this.video = document.getElementById('video');
        this.hls = null;
        this.analytics = null;
        this.abrController = null;
        this.bufferController = null;
        this.initialized = false;
        this.currentLevel = -1;
        this.isBuffering = false;
    }

    /**
     * Initialize the WASM module and apply branding
     */
    async init() {
        // Initialize WASM
        await init();
        console.log(`PSM Player v${version()} initialized`);

        // Apply PSM branding CSS
        const themeStyle = document.getElementById('psm-theme');
        themeStyle.textContent = PsmBranding.get_css_variables();

        // Initialize analytics
        this.analytics = new PsmAnalytics();

        // Initialize buffer controller
        this.bufferController = new PsmBufferController();

        this.initialized = true;
        this.setupEventListeners();
    }

    /**
     * Set up UI event listeners
     */
    setupEventListeners() {
        document.getElementById('load').addEventListener('click', () => this.loadStream());

        // Video events for analytics
        this.video.addEventListener('play', () => {
            this.analytics.report_play(this.video.currentTime);
        });

        this.video.addEventListener('pause', () => {
            this.analytics.report_pause(this.video.currentTime);
        });

        this.video.addEventListener('waiting', () => {
            if (!this.isBuffering) {
                this.isBuffering = true;
                this.analytics.report_rebuffer_start(this.video.currentTime);
                this.bufferController.report_stall();
            }
        });

        this.video.addEventListener('playing', () => {
            if (this.isBuffering) {
                this.isBuffering = false;
                this.analytics.report_rebuffer_end(this.video.currentTime);
            }
        });

        this.video.addEventListener('seeking', () => {
            // Track seek events
        });

        this.video.addEventListener('seeked', () => {
            this.analytics.report_seek(0, this.video.currentTime);
        });

        this.video.addEventListener('error', (e) => {
            this.analytics.report_error('VIDEO_ERROR', e.message || 'Unknown error', true);
        });

        // Update stats periodically
        setInterval(() => this.updateStats(), 1000);
    }

    /**
     * Load an HLS stream
     */
    loadStream() {
        const url = document.getElementById('url').value;
        const abrAlgorithm = document.getElementById('abr').value;

        if (!url) {
            alert('Please enter a stream URL');
            return;
        }

        // Destroy existing HLS instance
        if (this.hls) {
            this.hls.destroy();
        }

        // Reset analytics and controllers
        this.analytics.reset();
        this.bufferController.reset();

        // Create new ABR controller with selected algorithm
        this.abrController = PsmAbrController.with_algorithm(abrAlgorithm);

        // Check for HLS support
        if (Hls.isSupported()) {
            this.hls = new Hls({
                // Let hls.js handle ABR, but we'll track it
                abrEwmaDefaultEstimate: 1000000,
                startLevel: -1,
            });

            this.setupHlsEvents();
            this.hls.loadSource(url);
            this.hls.attachMedia(this.video);
        } else if (this.video.canPlayType('application/vnd.apple.mpegurl')) {
            // Native HLS support (Safari)
            this.video.src = url;
        } else {
            alert('HLS is not supported in this browser');
        }
    }

    /**
     * Set up HLS.js event handlers
     */
    setupHlsEvents() {
        this.hls.on(Hls.Events.MANIFEST_PARSED, (event, data) => {
            console.log('Manifest parsed, quality levels:', data.levels.length);

            // Configure buffer controller for VOD
            if (this.hls.levels.length > 0) {
                const maxBitrate = Math.max(...this.hls.levels.map(l => l.bitrate));
                this.analytics.set_available_qualities(maxBitrate);
            }

            this.renderQualityLevels(data.levels);
            this.video.play().catch(e => console.log('Autoplay prevented:', e));
        });

        this.hls.on(Hls.Events.LEVEL_LOADED, (event, data) => {
            // Configure buffer controller based on content type
            if (data.details.live) {
                this.bufferController.configure_live(data.details.targetduration);
            } else {
                this.bufferController.configure_vod(data.details.totalduration);
            }
        });

        this.hls.on(Hls.Events.FRAG_LOADED, (event, data) => {
            const stats = data.frag.stats;
            const bytes = stats.loaded;
            const duration = stats.loading.end - stats.loading.start;

            // Record download for bandwidth estimation
            this.abrController.record_download(bytes, duration);

            // Report bitrate sample
            const level = this.hls.levels[data.frag.level];
            if (level) {
                this.analytics.report_bitrate_sample(level.bitrate, data.frag.duration);
            }
        });

        this.hls.on(Hls.Events.LEVEL_SWITCHED, (event, data) => {
            if (this.currentLevel !== data.level && this.currentLevel !== -1) {
                const level = this.hls.levels[data.level];
                if (level) {
                    this.analytics.report_quality_change(level.bitrate, this.video.currentTime);
                }
            }
            this.currentLevel = data.level;
            this.updateQualityDisplay();
        });

        this.hls.on(Hls.Events.ERROR, (event, data) => {
            console.error('HLS error:', data);
            this.analytics.report_error(
                data.type,
                data.details,
                data.fatal
            );

            if (data.fatal) {
                switch (data.type) {
                    case Hls.ErrorTypes.NETWORK_ERROR:
                        console.log('Network error, trying to recover...');
                        this.hls.startLoad();
                        break;
                    case Hls.ErrorTypes.MEDIA_ERROR:
                        console.log('Media error, trying to recover...');
                        this.hls.recoverMediaError();
                        break;
                    default:
                        console.log('Fatal error, destroying HLS');
                        this.hls.destroy();
                        break;
                }
            }
        });

        // Track first frame
        this.video.addEventListener('loadeddata', () => {
            this.analytics.report_first_frame();
        }, { once: true });
    }

    /**
     * Render quality level buttons
     */
    renderQualityLevels(levels) {
        const container = document.getElementById('quality-levels');
        container.innerHTML = '<h3 style="color: var(--psm-text); margin: 0 0 0.5rem 0;">Quality Levels</h3>';

        const list = document.createElement('div');
        list.style.display = 'flex';
        list.style.flexWrap = 'wrap';
        list.style.gap = '0.5rem';

        // Add Auto option
        const autoBtn = this.createQualityButton(-1, 'Auto', 0);
        autoBtn.classList.add('active');
        list.appendChild(autoBtn);

        levels.forEach((level, index) => {
            const height = level.height || 'Unknown';
            const bitrate = Math.round(level.bitrate / 1000);
            const label = `${height}p (${bitrate} kbps)`;
            list.appendChild(this.createQualityButton(index, label, level.bitrate));
        });

        container.appendChild(list);
    }

    /**
     * Create a quality level button
     */
    createQualityButton(index, label, bitrate) {
        const btn = document.createElement('button');
        btn.textContent = label;
        btn.dataset.level = index;
        btn.style.fontSize = '0.875rem';
        btn.style.padding = '0.25rem 0.75rem';

        btn.addEventListener('click', () => {
            if (this.hls) {
                this.hls.currentLevel = index;
                document.querySelectorAll('#quality-levels button').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
            }
        });

        return btn;
    }

    /**
     * Update the quality display
     */
    updateQualityDisplay() {
        const statQuality = document.getElementById('stat-quality');
        if (this.hls && this.currentLevel >= 0 && this.hls.levels[this.currentLevel]) {
            const level = this.hls.levels[this.currentLevel];
            statQuality.textContent = `${level.height}p`;
        } else {
            statQuality.textContent = 'Auto';
        }
    }

    /**
     * Update statistics display
     */
    updateStats() {
        if (!this.initialized || !this.hls) return;

        // Buffer level
        const bufferLevel = this.getBufferLevel();
        const statBuffer = document.getElementById('stat-buffer');
        statBuffer.textContent = `${bufferLevel.toFixed(1)}s`;
        statBuffer.className = 'stat-value';
        if (bufferLevel < 5) {
            statBuffer.classList.add('bad');
        } else if (bufferLevel < 15) {
            statBuffer.classList.add('warning');
        } else {
            statBuffer.classList.add('good');
        }

        // Bandwidth estimate
        const bandwidth = this.abrController.get_bandwidth_estimate();
        const statBandwidth = document.getElementById('stat-bandwidth');
        statBandwidth.textContent = this.abrController.get_bandwidth_display();

        // QoE metrics
        const qoe = this.analytics.get_qoe();
        const statQoe = document.getElementById('stat-qoe');
        statQoe.textContent = qoe.score.toFixed(0);
        statQoe.className = 'stat-value';
        if (qoe.score >= 80) {
            statQoe.classList.add('good');
        } else if (qoe.score >= 50) {
            statQoe.classList.add('warning');
        } else {
            statQoe.classList.add('bad');
        }

        // Rebuffers
        document.getElementById('stat-rebuffers').textContent = qoe.rebuffer_count;

        // Quality switches
        document.getElementById('stat-switches').textContent = qoe.quality_switches;

        // Update buffer controller position
        this.bufferController.update_position(this.video.currentTime);
    }

    /**
     * Get current buffer level in seconds
     */
    getBufferLevel() {
        if (!this.video || this.video.buffered.length === 0) return 0;

        const currentTime = this.video.currentTime;
        for (let i = 0; i < this.video.buffered.length; i++) {
            if (this.video.buffered.start(i) <= currentTime && currentTime <= this.video.buffered.end(i)) {
                return this.video.buffered.end(i) - currentTime;
            }
        }
        return 0;
    }
}

// Initialize the player when the page loads
const player = new PsmPlayer();
player.init().catch(console.error);
