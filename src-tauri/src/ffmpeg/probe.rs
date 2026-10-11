use super::models::*;
use crate::domain::AppResult;
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};
pub const DEMUXERS: &str =
    "srt,ass,webvtt,image2,jpeg_pipe,png_pipe,webp_pipe,mov,matroska,webm,avi,mp3,aac,flac,wav,ogg,asf,mpeg,mpegts,flv,amr,aiff,ape,ac3,eac3,dts";
pub fn arguments(path: &Path) -> Vec<String> {
    [
        "-v",
        "error",
        "-protocol_whitelist",
        "file,pipe",
        "-format_whitelist",
        DEMUXERS,
        "-show_format",
        "-show_streams",
        "-of",
        "json",
        "-i",
    ]
    .iter()
    .map(|s| s.to_string())
    .chain([path.to_string_lossy().into_owned()])
    .collect()
}
fn number(v: &Value) -> Option<f64> {
    let n = v.as_f64().or_else(|| v.as_str()?.parse().ok())?;
    (n.is_finite() && n >= 0.).then_some(n)
}
fn uint(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str()?.parse().ok())
}
fn string(v: &Value) -> Option<String> {
    v.as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(4096).collect())
}
fn tags(v: &Value) -> BTreeMap<String, String> {
    v.as_object()
        .map(|m| {
            m.iter()
                .take(128)
                .filter_map(|(k, v)| Some((k.chars().take(256).collect(), string(v)?)))
                .collect()
        })
        .unwrap_or_default()
}
fn fps(v: &Value) -> Option<f64> {
    let text = v.as_str()?;
    let (a, b) = text.split_once('/').unwrap_or((text, "1"));
    let a: f64 = a.parse().ok()?;
    let b: f64 = b.parse().ok()?;
    let f = a / b;
    (f.is_finite() && f > 0.).then_some(f)
}
pub fn parse(text: &str, path: &Path) -> AppResult<MediaInfo> {
    if text.len() > 8 * 1024 * 1024 {
        return Err("媒体信息过大，无法安全读取".into());
    }
    let v: Value = serde_json::from_str(text).map_err(|_| "媒体信息不是有效的 JSON")?;
    let streams = v["streams"].as_array().ok_or("无法识别媒体轨道")?;
    if streams.len() > 1024 {
        return Err("媒体轨道数量过多".into());
    }
    let f = &v["format"];
    let mut info = MediaInfo {
        file_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        path: path.to_string_lossy().into_owned(),
        size: std::fs::metadata(path)
            .ok()
            .map(|m| m.len())
            .or_else(|| uint(&f["size"])),
        duration: number(&f["duration"]).filter(|n| *n > 0.),
        container: string(&f["format_name"]).unwrap_or_default(),
        bitrate: uint(&f["bit_rate"]),
        metadata: tags(&f["tags"]),
        ..Default::default()
    };
    for s in streams {
        let index = uint(&s["index"]).unwrap_or(0) as u32;
        let codec = string(&s["codec_name"]).unwrap_or_else(|| "unknown".into());
        let metadata = tags(&s["tags"]);
        match s["codec_type"].as_str() {
            Some("video") => info.videos.push(VideoStreamInfo {
                index,
                codec,
                profile: string(&s["profile"]),
                width: uint(&s["width"]).and_then(|x| u32::try_from(x).ok()),
                height: uint(&s["height"]).and_then(|x| u32::try_from(x).ok()),
                fps: fps(&s["avg_frame_rate"]).or_else(|| fps(&s["r_frame_rate"])),
                pixel_format: string(&s["pix_fmt"]),
                bit_depth: uint(&s["bits_per_raw_sample"])
                    .or_else(|| uint(&s["bits_per_coded_sample"]))
                    .filter(|n| *n > 0)
                    .and_then(|x| u32::try_from(x).ok()),
                bitrate: uint(&s["bit_rate"]),
                attached_picture: s["disposition"]["attached_pic"].as_u64() == Some(1),
                metadata,
            }),
            Some("audio") => info.audios.push(AudioStreamInfo {
                index,
                codec,
                profile: string(&s["profile"]),
                sample_rate: uint(&s["sample_rate"]).and_then(|x| u32::try_from(x).ok()),
                channels: uint(&s["channels"]).and_then(|x| u32::try_from(x).ok()),
                channel_layout: string(&s["channel_layout"]),
                bitrate: uint(&s["bit_rate"]),
                metadata,
            }),
            Some("subtitle") => info.subtitles.push(SubtitleStreamInfo {
                index,
                codec,
                language: string(&s["tags"]["language"]),
                metadata,
            }),
            _ => info.other_streams += 1,
        }
    }
    if info.videos.is_empty() && info.audios.is_empty() {
        return Err("文件中没有可处理的音视频轨道".into());
    }
    Ok(info)
}
