/**
 * PSM Player Embed SDK
 *
 * A lightweight SDK for embedding and controlling PSM Player widgets.
 *
 * Usage:
 *   const player = new PSMPlayer('#container', {
 *       src: 'https://example.com/video.m3u8',
 *       autoplay: false,
 *       width: 640,
 *       height: 360
 *   });
 *
 *   player.on('ready', () => console.log('Player ready'));
 *   player.play();
 */

(function(global) {
    'use strict';

    const DEFAULT_WIDGET_URL = 'https://player.purplesquirrel.media/embed/widget.html';

    class PSMPlayer {
        /**
         * Create a new PSM Player instance
         * @param {string|HTMLElement} container - CSS selector or DOM element
         * @param {Object} options - Player configuration
         */
        constructor(container, options = {}) {
            this.container = typeof container === 'string'
                ? document.querySelector(container)
                : container;

            if (!this.container) {
                throw new Error('PSMPlayer: Container element not found');
            }

            this.options = {
                src: '',
                poster: '',
                autoplay: false,
                loop: false,
                muted: false,
                controls: true,
                color: '#9b30ff',
                start: 0,
                width: '100%',
                height: '100%',
                widgetUrl: DEFAULT_WIDGET_URL,
                responsive: true,
                ...options
            };

            this._listeners = {};
            this._ready = false;
            this._state = {
                currentTime: 0,
                duration: 0,
                paused: true,
                muted: false,
                volume: 1
            };

            this._boundMessageHandler = this._handleMessage.bind(this);
            window.addEventListener('message', this._boundMessageHandler);

            this._createIframe();
        }

        /**
         * Build the widget URL with parameters
         * @private
         */
        _buildUrl() {
            const params = new URLSearchParams();

            if (this.options.src) params.set('src', this.options.src);
            if (this.options.poster) params.set('poster', this.options.poster);
            if (this.options.autoplay) params.set('autoplay', 'true');
            if (this.options.loop) params.set('loop', 'true');
            if (this.options.muted) params.set('muted', 'true');
            if (!this.options.controls) params.set('controls', 'false');
            if (this.options.color !== '#9b30ff') params.set('color', this.options.color);
            if (this.options.start > 0) params.set('start', this.options.start.toString());

            return `${this.options.widgetUrl}?${params.toString()}`;
        }

        /**
         * Create and insert the iframe
         * @private
         */
        _createIframe() {
            this.iframe = document.createElement('iframe');
            this.iframe.src = this._buildUrl();
            this.iframe.frameBorder = '0';
            this.iframe.allow = 'autoplay; fullscreen; picture-in-picture';
            this.iframe.allowFullscreen = true;
            this.iframe.style.border = 'none';

            if (this.options.responsive) {
                this.container.style.position = 'relative';
                this.container.style.paddingBottom = '56.25%'; // 16:9 aspect ratio
                this.container.style.height = '0';
                this.container.style.overflow = 'hidden';

                this.iframe.style.position = 'absolute';
                this.iframe.style.top = '0';
                this.iframe.style.left = '0';
                this.iframe.style.width = '100%';
                this.iframe.style.height = '100%';
            } else {
                this.iframe.width = this.options.width;
                this.iframe.height = this.options.height;
            }

            this.container.appendChild(this.iframe);
        }

        /**
         * Handle messages from the iframe
         * @private
         */
        _handleMessage(event) {
            if (!event.data || event.data.source !== 'psm-player') return;
            if (event.source !== this.iframe.contentWindow) return;

            const { type, ...data } = event.data;

            switch (type) {
                case 'ready':
                    this._ready = true;
                    break;
                case 'timeupdate':
                    this._state.currentTime = data.currentTime;
                    this._state.duration = data.duration;
                    break;
                case 'play':
                    this._state.paused = false;
                    break;
                case 'pause':
                    this._state.paused = true;
                    break;
                case 'state':
                    Object.assign(this._state, data);
                    break;
            }

            this._emit(type, data);
        }

        /**
         * Send a command to the player
         * @private
         */
        _postMessage(action, data = {}) {
            if (!this.iframe || !this.iframe.contentWindow) return;

            this.iframe.contentWindow.postMessage({
                target: 'psm-player',
                action,
                ...data
            }, '*');
        }

        /**
         * Emit an event to listeners
         * @private
         */
        _emit(event, data) {
            const listeners = this._listeners[event] || [];
            listeners.forEach(callback => {
                try {
                    callback(data);
                } catch (e) {
                    console.error('PSMPlayer event handler error:', e);
                }
            });
        }

        // ========== Public API ==========

        /**
         * Start playback
         */
        play() {
            this._postMessage('play');
            return this;
        }

        /**
         * Pause playback
         */
        pause() {
            this._postMessage('pause');
            return this;
        }

        /**
         * Toggle play/pause
         */
        togglePlay() {
            if (this._state.paused) {
                this.play();
            } else {
                this.pause();
            }
            return this;
        }

        /**
         * Seek to a specific time
         * @param {number} time - Time in seconds
         */
        seek(time) {
            this._postMessage('seek', { time });
            return this;
        }

        /**
         * Set the volume
         * @param {number} volume - Volume level (0-1)
         */
        setVolume(volume) {
            this._postMessage('setVolume', { volume: Math.max(0, Math.min(1, volume)) });
            this._state.volume = volume;
            return this;
        }

        /**
         * Mute the audio
         */
        mute() {
            this._postMessage('mute');
            this._state.muted = true;
            return this;
        }

        /**
         * Unmute the audio
         */
        unmute() {
            this._postMessage('unmute');
            this._state.muted = false;
            return this;
        }

        /**
         * Toggle mute state
         */
        toggleMute() {
            if (this._state.muted) {
                this.unmute();
            } else {
                this.mute();
            }
            return this;
        }

        /**
         * Load a new video
         * @param {string} src - Video URL
         */
        load(src) {
            this._postMessage('load', { src });
            this.options.src = src;
            return this;
        }

        /**
         * Request current player state
         * @returns {Promise<Object>} Player state
         */
        getState() {
            return new Promise((resolve) => {
                const handler = (data) => {
                    this.off('state', handler);
                    resolve(data);
                };
                this.on('state', handler);
                this._postMessage('getState');

                // Timeout fallback
                setTimeout(() => {
                    this.off('state', handler);
                    resolve(this._state);
                }, 1000);
            });
        }

        /**
         * Get current playback time
         * @returns {number}
         */
        getCurrentTime() {
            return this._state.currentTime;
        }

        /**
         * Get video duration
         * @returns {number}
         */
        getDuration() {
            return this._state.duration;
        }

        /**
         * Check if video is paused
         * @returns {boolean}
         */
        isPaused() {
            return this._state.paused;
        }

        /**
         * Check if audio is muted
         * @returns {boolean}
         */
        isMuted() {
            return this._state.muted;
        }

        /**
         * Check if player is ready
         * @returns {boolean}
         */
        isReady() {
            return this._ready;
        }

        /**
         * Add an event listener
         * @param {string} event - Event name
         * @param {Function} callback - Event handler
         */
        on(event, callback) {
            if (!this._listeners[event]) {
                this._listeners[event] = [];
            }
            this._listeners[event].push(callback);
            return this;
        }

        /**
         * Remove an event listener
         * @param {string} event - Event name
         * @param {Function} callback - Event handler
         */
        off(event, callback) {
            if (!this._listeners[event]) return this;

            if (callback) {
                this._listeners[event] = this._listeners[event].filter(cb => cb !== callback);
            } else {
                delete this._listeners[event];
            }
            return this;
        }

        /**
         * Add a one-time event listener
         * @param {string} event - Event name
         * @param {Function} callback - Event handler
         */
        once(event, callback) {
            const wrapper = (data) => {
                this.off(event, wrapper);
                callback(data);
            };
            return this.on(event, wrapper);
        }

        /**
         * Wait for player to be ready
         * @returns {Promise}
         */
        ready() {
            if (this._ready) {
                return Promise.resolve();
            }
            return new Promise((resolve) => {
                this.once('ready', resolve);
            });
        }

        /**
         * Set the aspect ratio for responsive mode
         * @param {number} width - Width ratio
         * @param {number} height - Height ratio
         */
        setAspectRatio(width, height) {
            if (this.options.responsive) {
                this.container.style.paddingBottom = `${(height / width) * 100}%`;
            }
            return this;
        }

        /**
         * Destroy the player instance
         */
        destroy() {
            window.removeEventListener('message', this._boundMessageHandler);

            if (this.iframe && this.iframe.parentNode) {
                this.iframe.parentNode.removeChild(this.iframe);
            }

            this._listeners = {};
            this.iframe = null;
            this.container = null;
        }
    }

    // Static factory method
    PSMPlayer.create = function(container, options) {
        return new PSMPlayer(container, options);
    };

    // Export
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = PSMPlayer;
    } else {
        global.PSMPlayer = PSMPlayer;
    }

})(typeof window !== 'undefined' ? window : this);
