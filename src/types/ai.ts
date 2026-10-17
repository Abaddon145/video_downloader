export type AnalysisMode = 'quick' | 'standard' | 'deep';
export interface AiSettings {
 model: string; defaultMode: AnalysisMode; whisperModel: 'base' | 'small' | 'custom';
 allowTranscript: boolean; allowImages: boolean; cleanTemporaryFiles: boolean;
 autoGenerateNotes: boolean; autoAnalyzeDownloads: boolean; chunkConcurrency: number;
}
export interface AiSettingsView { settings: AiSettings; hasApiKey: boolean }
