export type TaskStatus = 'queued' | 'resolving' | 'downloading' | 'processing' | 'paused' | 'completed' | 'failed' | 'cancelled';
export type MediaKind = 'video' | 'audio' | 'subtitles';
export type VideoMode = 'compatible' | 'source';
export interface DownloadRequest {
  url: string; title: string; thumbnail: string | null; kind: MediaKind;
  videoMode: VideoMode; maxHeight: number; subtitleLanguages: string[];
}
export interface DownloadTask {
  id: string; request: DownloadRequest; status: TaskStatus; phase: string;
  progress: { downloaded: number | null; total: number | null; speed: number | null; eta: number | null; percent: number | null };
  outputDir: string; files: string[]; error: string | null; logs: string[]; createdAt: number; finishedAt: number | null; queueOrder: number;
}
export interface AppSettings {
  downloadDir: string; concurrency: number; cookieMode: string; browser: string;
  browserProfile: string; hasCookieFile: boolean; proxyEnabled: boolean; proxyUrl: string;
  autoCheckCoreUpdate: boolean; lastCoreUpdateCheck: number | null;
}
export interface EngineInfo { version: string; ready: boolean; ffmpegVersion: string; denoVersion: string; error: string | null }
export interface AppSnapshot { settings: AppSettings; tasks: DownloadTask[]; engine: EngineInfo; updating: boolean; previewing: boolean; notice: string | null; coreUpdate: EngineUpdate | null }
export interface PlaylistEntry { id: string; url: string; title: string; duration: number | null; thumbnail: string | null }
export interface MediaPreview {
  url: string; title: string; thumbnail: string | null; duration: number | null; uploader: string; site: string; isPlaylist: boolean;
  entries: PlaylistEntry[];
  formats: { id: string; extension: string; height: number | null; videoCodec: string; audioCodec: string; filesize: number | null }[];
  subtitles: { language: string; automatic: boolean }[];
  videoCodec: string | null; audioCodec: string | null; compatible: boolean; filesize: number | null;
}
export interface PreviewResult { url: string; preview: MediaPreview | null; error: string | null }
export interface EngineUpdate { version: string; currentVersion: string; available: boolean; publishedAt: string }
export type BatchAction = 'pause' | 'resume' | 'cancel' | 'retry' | 'pin' | 'copy' | 'remove';
export interface BatchResult { succeeded: string[]; skipped: {id: string; reason: string}[]; failed: {id: string; error: string}[] }
