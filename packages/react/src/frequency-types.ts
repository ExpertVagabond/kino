/**
 * TypeScript types for PSM Player Frequency Analysis WASM module
 * @module @purplesquirrel/player-frequency
 */

// ============================================================================
// Frequency Analysis Types
// ============================================================================

/**
 * Band energy distribution across frequency ranges
 */
export interface BandEnergies {
  /** Sub-bass: 20-60 Hz */
  subBass: number;
  /** Bass: 60-250 Hz */
  bass: number;
  /** Low-mid: 250-500 Hz */
  lowMid: number;
  /** Mid: 500-2000 Hz */
  mid: number;
  /** High-mid: 2000-4000 Hz */
  highMid: number;
  /** Presence/Brilliance: 4000-20000 Hz */
  high: number;
}

/**
 * Dominant frequency detected in audio
 */
export interface DominantFrequency {
  /** Frequency in Hz */
  frequencyHz: number;
  /** Normalized magnitude (0-1) */
  magnitude: number;
  /** Rank (1 = highest energy) */
  rank: number;
}

/**
 * Spectral features extracted from audio
 */
export interface SpectralFeatures {
  /** Center of mass of the spectrum */
  centroid: number;
  /** Frequency below which 95% of energy lies */
  rolloff: number;
  /** Flatness (0 = tonal, 1 = noisy) */
  flatness: number;
  /** Zero crossing rate */
  zcr: number;
  /** Energy distribution across bands */
  bandEnergies: BandEnergies;
}

/**
 * Result from frequency analysis
 */
export interface FrequencyAnalysisResult {
  /** Top dominant frequencies */
  dominantFrequencies: DominantFrequency[];
  /** Spectral features */
  spectralFeatures: SpectralFeatures;
  /** Processing time in milliseconds */
  processingTimeMs: number;
}

/**
 * Real-time frequency data for visualization
 */
export interface RealtimeFrequencyData {
  /** Magnitude spectrum */
  spectrum: Float32Array;
  /** Band energy values (6 bands) */
  bandEnergies: [number, number, number, number, number, number];
  /** Dominant frequency in Hz */
  dominantFrequency: number;
  /** Spectral centroid in Hz */
  spectralCentroid: number;
}

// ============================================================================
// Fingerprinting Types
// ============================================================================

/**
 * Audio fingerprint information
 */
export interface Fingerprint {
  /** Hash string (hex encoded) */
  hash: string;
  /** Audio duration in seconds */
  durationSecs: number;
  /** Number of spectral peaks detected */
  peakCount: number;
}

/**
 * Fingerprint verification result
 */
export interface FingerprintVerification {
  /** Whether the fingerprints match */
  isMatch: boolean;
  /** Expected hash */
  expectedHash: string;
  /** Computed hash */
  actualHash: string;
  /** Similarity score (0-1) */
  similarity: number;
}

// ============================================================================
// Auto-Tagging Types
// ============================================================================

/**
 * Auto-generated content tag
 */
export interface ContentTag {
  /** Tag label (e.g., "music", "energetic") */
  label: string;
  /** Tag category (e.g., "content_type", "mood") */
  category: TagCategory;
  /** Confidence score (0-1) */
  confidence: number;
}

/**
 * Tag categories
 */
export type TagCategory =
  | 'content_type'   // music, speech, ambient
  | 'mood'           // energetic, calm, melancholic
  | 'characteristic' // bass-heavy, bright, warm
  | 'genre'          // electronic, acoustic, vocal
  | 'instrument';    // guitar, drums, synth

// ============================================================================
// Recommendation Types
// ============================================================================

/**
 * Content similarity result
 */
export interface ContentSimilarity {
  /** Overall similarity score (0-1) */
  overallSimilarity: number;
  /** Mel-band based similarity */
  melSimilarity: number;
  /** MFCC-based similarity */
  mfccSimilarity: number;
  /** Spectral feature similarity */
  spectralSimilarity: number;
  /** Features that contributed to the match */
  matchingFeatures: string[];
}

/**
 * Content recommendation
 */
export interface Recommendation {
  /** Content ID */
  contentId: string;
  /** Similarity score (0-1) */
  similarity: number;
  /** Features that matched */
  matchingFeatures: string[];
  /** Optional content title */
  title?: string;
  /** Optional thumbnail URL */
  thumbnailUrl?: string;
}

// ============================================================================
// Thumbnail Selection Types
// ============================================================================

/**
 * Thumbnail candidate
 */
export interface ThumbnailCandidate {
  /** Timestamp in seconds */
  timestamp: number;
  /** Sharpness score (0-1) */
  sharpness: number;
  /** Contrast score (0-1) */
  contrast: number;
  /** Overall quality score (0-1) */
  qualityScore: number;
}

/**
 * Selected thumbnail information
 */
export interface ThumbnailInfo {
  /** Selected timestamp in seconds */
  timestamp: number;
  /** Human-readable timecode (MM:SS.mmm) */
  timecode: string;
  /** All candidates considered */
  candidates: ThumbnailCandidate[];
}

// ============================================================================
// WASM Module Types
// ============================================================================

/**
 * PsmFrequencyAnalyzer WASM class
 */
export interface PsmFrequencyAnalyzer {
  /** Analyze audio samples */
  analyze(samples: Float32Array, sampleRate: number): {
    spectral_centroid: number;
    spectral_rolloff: number;
    spectral_flatness: number;
    get_dominant_json(): string;
    get_band_energies_json(): string;
  };

  /** Get magnitude spectrum */
  getSpectrum(samples: Float32Array): Float32Array;

  /** Get dominant frequencies as array of objects */
  getDominant(
    samples: Float32Array,
    sampleRate: number,
    topK: number
  ): Array<{ frequencyHz: number; magnitude: number; rank: number }>;

  /** Free WASM memory */
  free(): void;
}

/**
 * PsmFingerprinter WASM class
 */
export interface PsmFingerprinter {
  /** Generate fingerprint hash from audio samples */
  fingerprint(samples: Float32Array, sampleRate: number): string;

  /** Compare two fingerprint hashes */
  compare(hash1: string, hash2: string): number;

  /** Free WASM memory */
  free(): void;
}

/**
 * PsmStreamingAnalyzer WASM class
 */
export interface PsmStreamingAnalyzer {
  /** Push samples and get analysis if buffer is full */
  push(samples: Float32Array): {
    dominant_frequency: number;
    spectral_centroid: number;
    get_spectrum(): Float32Array;
    get_band_energy(band: number): number;
  } | undefined;

  /** Reset the internal buffer */
  reset(): void;

  /** Free WASM memory */
  free(): void;
}

/**
 * Constructor for PsmFrequencyAnalyzer
 */
export interface PsmFrequencyAnalyzerConstructor {
  new(fftSize: number): PsmFrequencyAnalyzer;
}

/**
 * Constructor for PsmFingerprinter
 */
export interface PsmFingerprinterConstructor {
  new(): PsmFingerprinter;
}

/**
 * Constructor for PsmStreamingAnalyzer
 */
export interface PsmStreamingAnalyzerConstructor {
  new(fftSize: number, sampleRate: number): PsmStreamingAnalyzer;
}

// ============================================================================
// API Request/Response Types
// ============================================================================

/**
 * Request to analyze content
 */
export interface AnalyzeRequest {
  /** Content ID */
  contentId: string;
  /** Enable fingerprint generation */
  enableFingerprint?: boolean;
  /** Enable auto-tagging */
  enableTagging?: boolean;
  /** Enable thumbnail selection */
  enableThumbnail?: boolean;
  /** Enable signature for recommendations */
  enableSignature?: boolean;
}

/**
 * Analysis response from API
 */
export interface AnalysisResponse {
  /** Content ID */
  contentId: string;
  /** Fingerprint info (if enabled) */
  fingerprint?: Fingerprint;
  /** Auto-generated tags */
  tags: ContentTag[];
  /** Dominant frequencies */
  dominantFrequencies: DominantFrequency[];
  /** Spectral features */
  spectralFeatures: SpectralFeatures;
  /** Thumbnail info (if enabled) */
  thumbnail?: ThumbnailInfo;
  /** Processing time in milliseconds */
  processingTimeMs: number;
}

/**
 * Request to verify content fingerprint
 */
export interface VerifyRequest {
  /** Content ID */
  contentId: string;
  /** Expected fingerprint hash */
  expectedHash: string;
}

/**
 * Verification response
 */
export interface VerifyResponse {
  /** Content ID */
  contentId: string;
  /** Whether hashes match */
  isMatch: boolean;
  /** Expected hash */
  expectedHash: string;
  /** Actual computed hash */
  actualHash: string;
  /** Confidence score (0-1) */
  confidence: number;
}

/**
 * Request for recommendations
 */
export interface RecommendRequest {
  /** Content ID for content-based recommendations */
  contentId?: string;
  /** User ID for personalized recommendations */
  userId?: string;
  /** Maximum number of recommendations */
  limit?: number;
}

/**
 * Recommendations response
 */
export interface RecommendResponse {
  /** List of recommendations */
  recommendations: Recommendation[];
}

/**
 * Request to compare two content items
 */
export interface CompareRequest {
  /** First content ID */
  contentIdA: string;
  /** Second content ID */
  contentIdB: string;
}

/**
 * Comparison response
 */
export interface CompareResponse {
  /** First content ID */
  contentIdA: string;
  /** Second content ID */
  contentIdB: string;
  /** Overall similarity score */
  overallSimilarity: number;
  /** Mel-band similarity */
  melSimilarity: number;
  /** MFCC similarity */
  mfccSimilarity: number;
  /** Spectral similarity */
  spectralSimilarity: number;
  /** Matching feature names */
  matchingFeatures: string[];
}

// ============================================================================
// Hook Types
// ============================================================================

/**
 * Options for useFrequencyAnalysis hook
 */
export interface UseFrequencyAnalysisOptions {
  /** FFT size (power of 2, 256-8192) */
  fftSize?: number;
  /** Sample rate in Hz (default: 44100) */
  sampleRate?: number;
  /** Enable real-time streaming analysis */
  streaming?: boolean;
  /** Auto-start analysis when audio plays */
  autoStart?: boolean;
}

/**
 * Return value from useFrequencyAnalysis hook
 */
export interface UseFrequencyAnalysisResult {
  /** Current frequency data (when streaming) */
  frequencyData: RealtimeFrequencyData | null;
  /** Analyze a buffer of samples */
  analyze: (samples: Float32Array) => FrequencyAnalysisResult;
  /** Start streaming analysis from AudioContext */
  startStreaming: (source: AudioNode, audioContext: AudioContext) => void;
  /** Stop streaming analysis */
  stopStreaming: () => void;
  /** Whether currently streaming */
  isStreaming: boolean;
  /** Any error that occurred */
  error: Error | null;
}

// ============================================================================
// Visualization Types
// ============================================================================

/**
 * Visualization mode for frequency display
 */
export type VisualizationMode =
  | 'bars'       // Traditional bar chart
  | 'circular'   // Circular spectrum
  | 'waveform'   // Time-domain waveform
  | 'spectrogram' // 2D spectrogram
  | 'bands';     // 6-band energy display

/**
 * Props for FrequencyVisualizer component
 */
export interface FrequencyVisualizerProps {
  /** Frequency data to visualize */
  data: RealtimeFrequencyData | null;
  /** Visualization mode */
  mode?: VisualizationMode;
  /** Width in pixels */
  width?: number;
  /** Height in pixels */
  height?: number;
  /** Bar color or gradient */
  barColor?: string | CanvasGradient;
  /** Background color */
  backgroundColor?: string;
  /** Number of bars (for 'bars' mode) */
  barCount?: number;
  /** Whether to show labels */
  showLabels?: boolean;
  /** Custom class name */
  className?: string;
  /** Custom style */
  style?: React.CSSProperties;
}
