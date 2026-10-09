use crate::domain::Progress;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Queued,
    Resolving,
    Downloading,
    Processing,
    Paused,
    Completed,
    Failed,
    Cancelled,
}
impl TaskStatus {
    pub fn active(&self) -> bool {
        matches!(self, Self::Resolving | Self::Downloading | Self::Processing)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    #[default]
    Video,
    Audio,
    Subtitles,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VideoMode {
    #[default]
    Compatible,
    Source,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadRequest {
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub kind: MediaKind,
    pub video_mode: VideoMode,
    pub max_height: u32,
    pub subtitle_languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadTask {
    pub id: String,
    pub request: DownloadRequest,
    pub status: TaskStatus,
    pub progress: Progress,
    pub phase: String,
    pub output_dir: String,
    pub files: Vec<String>,
    pub error: Option<String>,
    pub logs: Vec<String>,
    pub created_at: u64,
    pub finished_at: Option<u64>,
    #[serde(default)]
    pub queue_order: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub download_dir: String,
    pub concurrency: u8,
    pub cookie_mode: String,
    pub browser: String,
    pub browser_profile: String,
    pub has_cookie_file: bool,
    pub proxy_enabled: bool,
    pub proxy_url: String,
    #[serde(default = "enabled")]
    pub auto_check_core_update: bool,
    #[serde(default)]
    pub last_core_update_check: Option<u64>,
}
fn enabled() -> bool {
    true
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            download_dir: String::new(),
            concurrency: 2,
            cookie_mode: "none".into(),
            browser: "edge".into(),
            browser_profile: String::new(),
            has_cookie_file: false,
            proxy_enabled: false,
            proxy_url: String::new(),
            auto_check_core_update: true,
            last_core_update_check: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    pub download_dir: String,
    pub concurrency: u8,
    pub cookie_mode: String,
    pub browser: String,
    pub browser_profile: String,
    pub proxy_enabled: bool,
    pub proxy_url: Option<String>,
    #[serde(default = "enabled")]
    pub auto_check_core_update: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiskState {
    pub schema_version: u32,
    pub settings: AppSettings,
    pub tasks: Vec<DownloadTask>,
    pub cookie_secret: Vec<u8>,
    pub proxy_secret: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaFormat {
    pub id: String,
    pub extension: String,
    pub height: Option<u32>,
    pub video_codec: String,
    pub audio_codec: String,
    pub filesize: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrack {
    pub language: String,
    pub automatic: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPreview {
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub uploader: String,
    pub site: String,
    pub is_playlist: bool,
    pub entries: Vec<PlaylistEntry>,
    pub formats: Vec<MediaFormat>,
    pub subtitles: Vec<SubtitleTrack>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub compatible: bool,
    pub filesize: Option<u64>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    pub url: String,
    pub preview: Option<MediaPreview>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub version: String,
    pub ready: bool,
    pub ffmpeg_version: String,
    pub deno_version: String,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub settings: AppSettings,
    pub tasks: Vec<DownloadTask>,
    pub engine: EngineInfo,
    pub updating: bool,
    pub previewing: bool,
    pub notice: Option<String>,
    pub core_update: Option<EngineUpdate>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineUpdate {
    pub version: String,
    pub current_version: String,
    pub available: bool,
    pub published_at: String,
}
