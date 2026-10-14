use super::models::PlaybackPreference;
use crate::{domain::AppResult, ffmpeg::models::*};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BrowserPlaybackSupport {
    pub hevc: bool,
    pub hevc10: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackMode {
    Direct,
    Remux,
    Transcode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPlan {
    pub mode: PlaybackMode,
    pub format: OutputFormat,
    pub reason: String,
    pub cached: bool,
}
pub fn plan(
    info: &MediaInfo,
    support: BrowserPlaybackSupport,
    preference: PlaybackPreference,
    fallback: u8,
) -> AppResult<PlaybackPlan> {
    if fallback > 2 {
        return Err("播放兼容回退已停止，请查看技术详情".into());
    }
    let v = info
        .videos
        .iter()
        .find(|v| !v.attached_picture)
        .ok_or("文件没有视频流")?;
    let eight = v.bit_depth.unwrap_or(8) <= 8
        && v.pixel_format
            .as_deref()
            .is_none_or(|p| ["yuv420p", "yuvj420p", "nv12"].contains(&p));
    let mp4_video = v.codec == "h264" && eight
        || v.codec == "hevc"
            && if v.bit_depth.unwrap_or(8) > 8 {
                support.hevc10
            } else {
                support.hevc
            };
    let mp4_audio = info
        .audios
        .first()
        .is_none_or(|a| ["aac", "mp3"].contains(&a.codec.as_str()));
    let webm_video = ["vp8", "vp9"].contains(&v.codec.as_str()) && eight;
    let webm_audio = info
        .audios
        .first()
        .is_none_or(|a| ["opus", "vorbis"].contains(&a.codec.as_str()));
    let mp4 = info
        .container
        .split(',')
        .any(|c| ["mov", "mp4"].contains(&c));
    // ffprobe reports Matroska/WebM together; a WebM extension plus compatible tracks is the safe direct path.
    let webm = std::path::Path::new(&info.path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("webm"));
    let (copy, format, direct) = if mp4_video && mp4_audio {
        (true, OutputFormat::Mp4, mp4)
    } else if webm_video && webm_audio {
        (true, OutputFormat::Webm, webm)
    } else {
        (false, OutputFormat::Mp4, false)
    };
    let mode = if fallback == 2 || preference == PlaybackPreference::Compatible {
        PlaybackMode::Transcode
    } else if fallback == 0 && (preference == PlaybackPreference::Original || direct) {
        PlaybackMode::Direct
    } else if copy {
        PlaybackMode::Remux
    } else {
        PlaybackMode::Transcode
    };
    Ok(PlaybackPlan {
        mode,
        format: if mode == PlaybackMode::Transcode {
            OutputFormat::Mp4
        } else {
            format
        },
        cached: false,
        reason: match mode {
            PlaybackMode::Direct => "当前浏览器支持原始视频",
            PlaybackMode::Remux => "保留视频和音频编码，仅调整容器",
            PlaybackMode::Transcode => "转换为 H.264 / AAC 兼容视频",
        }
        .into(),
    })
}
pub fn request(source: &str, dir: &str, plan: &PlaybackPlan) -> MediaRequest {
    MediaRequest {
        input_path: source.into(),
        output_dir: dir.into(),
        operation: if plan.mode == PlaybackMode::Remux {
            MediaOperation::Remux
        } else {
            MediaOperation::Transcode
        },
        output_format: plan.format,
        crf: Some(22),
        audio_bitrate: 192,
        learning_playback: true,
        ..Default::default()
    }
}
