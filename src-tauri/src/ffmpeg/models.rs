use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoStreamInfo {
    pub index: u32,
    pub codec: String,
    pub profile: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub pixel_format: Option<String>,
    pub bit_depth: Option<u32>,
    pub bitrate: Option<u64>,
    pub attached_picture: bool,
    pub metadata: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStreamInfo {
    pub index: u32,
    pub codec: String,
    pub profile: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub channel_layout: Option<String>,
    pub bitrate: Option<u64>,
    pub metadata: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleStreamInfo {
    pub index: u32,
    pub codec: String,
    pub language: Option<String>,
    pub metadata: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub file_name: String,
    pub path: String,
    pub size: Option<u64>,
    pub duration: Option<f64>,
    pub container: String,
    pub bitrate: Option<u64>,
    pub videos: Vec<VideoStreamInfo>,
    pub audios: Vec<AudioStreamInfo>,
    pub subtitles: Vec<SubtitleStreamInfo>,
    pub other_streams: usize,
    pub metadata: BTreeMap<String, String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaOperation {
    Remux,
    #[default]
    Transcode,
    Compress,
    Trim,
    ExtractAudio,
    Screenshot,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Mp4,
    Mkv,
    Mov,
    Webm,
    Mp3,
    M4a,
    Aac,
    Wav,
    Flac,
    Opus,
    Png,
    Jpg,
    Webp,
}
impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::Mov => "mov",
            Self::Webm => "webm",
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Aac => "aac",
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::Opus => "opus",
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Webp => "webp",
        }
    }
    pub fn muxer(self) -> &'static str {
        match self {
            Self::Mkv => "matroska",
            Self::M4a => "ipod",
            Self::Aac => "adts",
            Self::Png | Self::Jpg | Self::Webp => "image2",
            _ => self.extension(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoCodec {
    #[default]
    H264,
    Hevc,
    Copy,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioCodec {
    #[default]
    Aac,
    Mp3,
    Opus,
    Copy,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    High,
    #[default]
    Balanced,
    Small,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrimMode {
    Fast,
    #[default]
    Accurate,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct MediaRequest {
    pub input_path: String,
    pub output_dir: String,
    pub operation: MediaOperation,
    pub output_format: OutputFormat,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub quality: Quality,
    pub crf: Option<u8>,
    pub height: Option<u32>,
    pub fps: Option<u32>,
    pub audio_bitrate: u32,
    pub start: f64,
    pub end: Option<f64>,
    pub trim_mode: TrimMode,
    pub copy_audio: bool,
    pub hardware_acceleration: crate::media::preset::HardwareAcceleration,
    pub generate_thumbnails: bool,
    pub source_download_id: Option<String>,
}
impl Default for MediaRequest {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            output_dir: String::new(),
            operation: MediaOperation::Transcode,
            output_format: OutputFormat::Mp4,
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::Aac,
            quality: Quality::Balanced,
            crf: None,
            height: None,
            fps: None,
            audio_bitrate: 320,
            start: 0.,
            end: None,
            trim_mode: TrimMode::Accurate,
            copy_audio: false,
            hardware_acceleration: Default::default(),
            generate_thumbnails: false,
            source_download_id: None,
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProgress {
    pub processed_time: Option<f64>,
    pub percent: Option<f64>,
    pub speed: Option<f64>,
    pub eta: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaTaskStatus {
    #[default]
    Queued,
    Probing,
    Processing,
    Interrupted,
    Completed,
    Failed,
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaTask {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: MediaOperation,
    pub input_path: String,
    pub output_path: Option<String>,
    pub request: MediaRequest,
    pub status: MediaTaskStatus,
    pub phase: String,
    pub progress: Option<f64>,
    pub speed: Option<f64>,
    pub processed_time: Option<f64>,
    pub total_duration: Option<f64>,
    pub eta: Option<f64>,
    pub error: Option<String>,
    pub logs: Vec<String>,
    pub created_at: u64,
    pub finished_at: Option<u64>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSnapshot {
    pub tasks: Vec<MediaTask>,
    pub ready: bool,
    pub error: Option<String>,
}
