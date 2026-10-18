export type AnalysisMode = 'quick' | 'standard' | 'deep';
export interface AiSettings {
 model: string; defaultMode: AnalysisMode; whisperModel: 'base' | 'small' | 'custom';
 allowTranscript: boolean; allowImages: boolean; cleanTemporaryFiles: boolean;
 autoGenerateNotes: boolean; autoAnalyzeDownloads: boolean; chunkConcurrency: number;
}
export interface AiSettingsView { settings: AiSettings; hasApiKey: boolean }

export interface AnalysisOptions {localOnly:boolean;transcribeAudio:boolean;allowTranscript:boolean;allowImages:boolean;whisperModel:string;autoGenerateNote:boolean}
export interface VideoAnalysis {
 title?:string|null;id:string;itemId:string|null;sourcePath:string;sourceFingerprint:string;status:string;analysisMode:AnalysisMode;model:string;promptVersion:string;
 language:string|null;durationMs:number|null;progress:number;stage:string|null;summary:VideoAnalysisResult|null;markdownPath:string|null;markdownVaultId:string|null;
 error:string|null;noteWarning:string|null;noteTargetVaultId:string|null;createdAt:number;updatedAt:number;completedAt:number|null;options:AnalysisOptions;transcriptDone:boolean;keyframesDone:boolean;
}
export interface Chapter {startMs:number;endMs:number;title:string;summary:string;keywords:string[];keyframeIds:string[]}
export interface VideoAnalysisResult {
 warnings?:string[];
 contentType:string;oneSentenceSummary:string;overview:string;chapters:Chapter[];
 keyPoints:{timeMs:number;title:string;content:string;importance:number;sourceSegmentIds:string[]}[];
 tutorialSteps:{order:number;timeMs:number;title:string;description:string;notes:string[]}[];
 terms:{name:string;definition:string;timeMs:number}[];actionItems:string[];tags:string[];
}
export interface AnalysisPage {items:VideoAnalysis[];total:number;page:number}
export interface TranscriptSegment {id:string;analysisId:string;segmentIndex:number;startMs:number;endMs:number;text:string}
export interface TranscriptPage {items:TranscriptSegment[];total:number;page:number}
export interface QuestionReply {id:string;analysisId:string;answer:string;references:{startMs:number;endMs:number;label:string}[]}
export interface UsageModel {model:string;inputTokens:number;outputTokens:number;cachedTokens:number;unknownRequests:number}
export interface UsageView {analyses:number;questions:number;models:UsageModel[]}
export interface CacheInfo {bytes:number;clearableBytes:number}
export interface WhisperModel {id:string;label:string;filename:string;bytes:number;sha256:string;url:string;source:string;installed:boolean}
export interface ModelDownload {id:string;modelId:string;status:string;received:number;total:number;error:string|null}
export interface AnalysisSource {title?:string;path:string;itemId?:string|null;force?:boolean}

export interface LibraryVideo {itemId:string;title:string;seriesTitle:string;path:string|null;durationMs:number|null;available:boolean}
export interface LibraryVideoPage {items:LibraryVideo[];total:number;page:number}

export interface CourseAiState{id:string;status:string;progress:number;hasSummary:boolean;createdAt:number;updatedAt:number}

export interface CourseTranscriptPage extends TranscriptPage{source:"subtitle"|"whisper"|"none";analysisId:string|null;warning:string|null;nextStartMs:number|null}
