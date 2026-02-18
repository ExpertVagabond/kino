export { KinoPlayer } from './KinoPlayer';
export { useKinoPlayer } from './useKinoPlayer';
export type {
  KinoPlayerProps,
  KinoPlayerRef,
  PlayerState,
  QualityLevel,
  SubtitleTrack,
  PlayerEvents,
  QoeMetrics,
} from './types';

// Frequency analysis hooks and components
export {
  useFrequencyAnalysis,
  useMicrophoneAnalysis,
  useMediaElementAnalysis,
} from './useFrequencyAnalysis';

export {
  FrequencyVisualizer,
  BandMeter,
  DominantFrequencyDisplay,
} from './FrequencyVisualizer';

// Frequency analysis types
export type {
  BandEnergies,
  DominantFrequency,
  SpectralFeatures,
  FrequencyAnalysisResult,
  RealtimeFrequencyData,
  Fingerprint,
  FingerprintVerification,
  ContentTag,
  TagCategory,
  ContentSimilarity,
  Recommendation,
  ThumbnailCandidate,
  ThumbnailInfo,
  KinoFrequencyAnalyzer,
  KinoFingerprinter,
  KinoStreamingAnalyzer,
  AnalyzeRequest,
  AnalysisResponse,
  VerifyRequest,
  VerifyResponse,
  RecommendRequest,
  RecommendResponse,
  CompareRequest,
  CompareResponse,
  UseFrequencyAnalysisOptions,
  UseFrequencyAnalysisResult,
  VisualizationMode,
  FrequencyVisualizerProps,
} from './frequency-types';
