use super::{models::*, probe::DEMUXERS};
use crate::domain::AppResult;
use std::path::{Component, Path, Prefix};
pub struct BuiltCommand {
    pub args: Vec<String>,
    pub duration: Option<f64>,
}
pub fn valid_path(path: &str) -> AppResult<()> {
    let disk_prefix = matches!(
        Path::new(path).components().next(),
        Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    );
    if path.is_empty()
        || path.len() > 30000
        || path.chars().any(|c| c.is_control())
        || !disk_prefix
        || !Path::new(path).is_absolute()
        || path.contains("://")
    {
        return Err("请选择本地磁盘上的有效文件或目录".into());
    }
    Ok(())
}
pub fn video_compatible(f: OutputFormat, c: &str) -> bool {
    match f {
        OutputFormat::Mp4 | OutputFormat::Mov => matches!(c, "h264" | "hevc" | "mpeg4" | "av1"),
        OutputFormat::Mkv => matches!(
            c,
            "h264" | "hevc" | "vp8" | "vp9" | "av1" | "mpeg4" | "mpeg2video" | "mjpeg" | "theora"
        ),
        OutputFormat::Webm => matches!(c, "vp8" | "vp9" | "av1"),
        _ => false,
    }
}
pub fn audio_compatible(f: OutputFormat, c: &str) -> bool {
    match f {
        OutputFormat::Mp4 | OutputFormat::Mov => matches!(c, "aac" | "mp3" | "ac3" | "eac3"),
        OutputFormat::Mkv => matches!(
            c,
            "aac"
                | "mp3"
                | "opus"
                | "vorbis"
                | "flac"
                | "ac3"
                | "eac3"
                | "dts"
                | "pcm_s16le"
                | "pcm_s24le"
        ),
        OutputFormat::Webm => matches!(c, "opus" | "vorbis"),
        OutputFormat::Mp3 => c == "mp3",
        OutputFormat::M4a | OutputFormat::Aac => c == "aac",
        OutputFormat::Flac => c == "flac",
        OutputFormat::Wav => matches!(c, "pcm_s16le" | "pcm_s24le" | "pcm_f32le"),
        OutputFormat::Opus => c == "opus",
        _ => false,
    }
}
fn subtitle_compatible(f: OutputFormat, c: &str) -> bool {
    match f {
        OutputFormat::Mp4 | OutputFormat::Mov => c == "mov_text",
        OutputFormat::Mkv => matches!(
            c,
            "subrip" | "ass" | "ssa" | "webvtt" | "hdmv_pgs_subtitle" | "dvd_subtitle" | "mov_text"
        ),
        OutputFormat::Webm => c == "webvtt",
        _ => false,
    }
}
fn video_name(c: VideoCodec) -> &'static str {
    match c {
        VideoCodec::H264 => "h264",
        VideoCodec::Hevc => "hevc",
        VideoCodec::Copy => "copy",
    }
}
fn audio_name(c: AudioCodec) -> &'static str {
    match c {
        AudioCodec::Aac => "aac",
        AudioCodec::Mp3 => "mp3",
        AudioCodec::Opus => "opus",
        AudioCodec::Copy => "copy",
    }
}
fn pair(a: &mut Vec<String>, k: &str, v: impl ToString) {
    a.push(k.into());
    a.push(v.to_string());
}
fn video(a: &mut Vec<String>, r: &MediaRequest) {
    pair(
        a,
        "-c:v",
        match r.video_codec {
            VideoCodec::H264 => "libx264",
            VideoCodec::Hevc => "libx265",
            VideoCodec::Copy => "copy",
        },
    );
    if r.video_codec != VideoCodec::Copy {
        let crf = r.crf.unwrap_or(crate::media::compressor::quality_value(
            r.video_codec,
            r.quality,
        ));
        pair(a, "-crf", crf);
        pair(a, "-preset", "medium");
        pair(a, "-pix_fmt", "yuv420p");
        if r.video_codec == VideoCodec::Hevc {
            pair(a, "-tag:v", "hvc1");
        }
        if let Some(h) = r.height {
            pair(a, "-vf", format!("scale=-2:min({h}\\,ih)"));
        }
        if let Some(f) = r.fps {
            pair(a, "-r", f);
        }
    }
}
fn audio(a: &mut Vec<String>, codec: &str, bitrate: u32) {
    pair(
        a,
        "-c:a",
        match codec {
            "aac" => "aac",
            "mp3" => "libmp3lame",
            "opus" => "libopus",
            "flac" => "flac",
            "pcm_s16le" => "pcm_s16le",
            _ => "copy",
        },
    );
    if matches!(codec, "aac" | "mp3" | "opus") {
        pair(a, "-b:a", format!("{bitrate}k"));
    }
}
pub fn build(r: &MediaRequest, info: &MediaInfo, output: &Path) -> AppResult<BuiltCommand> {
    valid_path(&r.input_path)?;
    valid_path(&r.output_dir)?;
    valid_path(&output.to_string_lossy())?;
    if output == Path::new(&r.input_path) {
        return Err("输出不能是原始文件".into());
    }
    if matches!(
        r.operation,
        MediaOperation::ExtractFrames
            | MediaOperation::ContactSheet
            | MediaOperation::CoverExtract
            | MediaOperation::CoverSet
            | MediaOperation::Metadata
    ) {
        return match r.operation {
            MediaOperation::ExtractFrames => crate::media::thumbnail::build_frames(r, info, output),
            MediaOperation::ContactSheet => {
                crate::media::contact_sheet::build_sheet(r, info, output)
            }
            MediaOperation::CoverExtract => crate::media::thumbnail::build_cover(r, info, output),
            _ => crate::media::metadata::build(r, info, output),
        };
    }
    if matches!(
        r.operation,
        MediaOperation::SubtitleConvert
            | MediaOperation::SubtitleMux
            | MediaOperation::SubtitleBurn
    ) {
        return crate::media::subtitle::build(r, info, output);
    }
    if !r.start.is_finite()
        || r.start < 0.
        || r.end.is_some_and(|t| !t.is_finite() || t < 0.)
        || r.crf.is_some_and(|n| n > 51)
        || r.height
            .is_some_and(|n| ![720, 1080, 1440, 2160].contains(&n))
        || r.fps.is_some_and(|n| ![24, 25, 30, 60].contains(&n))
        || ![128, 192, 256, 320].contains(&r.audio_bitrate)
    {
        return Err("媒体处理选项无效".into());
    }
    let v = info.videos.iter().find(|v| !v.attached_picture);
    let first_audio = info.audios.first();
    if r.operation != MediaOperation::ExtractAudio
        && r.operation != MediaOperation::Remux
        && v.is_none()
    {
        return Err("文件没有可处理的视频流".into());
    }
    let container_ok = matches!(
        r.output_format,
        OutputFormat::Mp4 | OutputFormat::Mkv | OutputFormat::Mov | OutputFormat::Webm
    );
    if matches!(
        r.operation,
        MediaOperation::Remux
            | MediaOperation::Transcode
            | MediaOperation::Compress
            | MediaOperation::Trim
    ) && !container_ok
    {
        return Err("请选择受支持的视频容器".into());
    }
    let copying = r.operation == MediaOperation::Remux
        || r.operation == MediaOperation::Trim && r.trim_mode == TrimMode::Fast;
    if copying {
        if info.other_streams > 0
            || info
                .videos
                .iter()
                .any(|v| !video_compatible(r.output_format, &v.codec))
            || info
                .audios
                .iter()
                .any(|v| !audio_compatible(r.output_format, &v.codec))
            || info
                .subtitles
                .iter()
                .any(|v| !subtitle_compatible(r.output_format, &v.codec))
        {
            return Err("当前媒体轨道不符合目标容器的无损封装支持范围，建议选择 MKV 或兼容转换（H.264 / AAC）".into());
        }
    }
    if matches!(
        r.operation,
        MediaOperation::Transcode | MediaOperation::Compress
    ) || r.operation == MediaOperation::Trim && !copying
    {
        let vc = if r.video_codec == VideoCodec::Copy {
            &v.ok_or("文件没有视频流")?.codec
        } else {
            video_name(r.video_codec)
        };
        if !video_compatible(r.output_format, vc)
            || first_audio.is_some_and(|s| {
                !audio_compatible(
                    r.output_format,
                    if r.audio_codec == AudioCodec::Copy {
                        &s.codec
                    } else {
                        audio_name(r.audio_codec)
                    },
                )
            })
        {
            return Err("所选编码不在该容器的首版支持范围内，请选择兼容编码".into());
        }
        if r.output_format == OutputFormat::Webm
            && (r.video_codec != VideoCodec::Copy
                || first_audio.is_some() && r.audio_codec != AudioCodec::Copy)
        {
            return Err("WebM 首版仅支持原始兼容编码的无损封装".into());
        }
        if r.video_codec == VideoCodec::Copy
            && (r.height.is_some() || r.fps.is_some() || r.crf.is_some())
        {
            return Err("保持原始编码时不能修改分辨率、帧率或 CRF".into());
        }
        if r.operation == MediaOperation::Trim && r.video_codec == VideoCodec::Copy {
            return Err("精确裁剪需要重新编码视频".into());
        }
    }
    let duration = if r.operation == MediaOperation::Trim {
        let end = r.end.ok_or("请输入结束时间")?;
        let total = info.duration.ok_or("无法确定媒体时长，不能安全裁剪")?;
        if r.start >= end || end > total {
            return Err("裁剪时间必须满足：0 ≤ 开始 < 结束 ≤ 媒体时长".into());
        }
        Some(end - r.start)
    } else if r.operation == MediaOperation::Screenshot {
        if !matches!(
            r.output_format,
            OutputFormat::Png | OutputFormat::Jpg | OutputFormat::Webp
        ) {
            return Err("请选择 PNG、JPG 或 WebP".into());
        }
        if info.duration.is_some_and(|d| r.start >= d) {
            return Err("截图时间超出视频时长".into());
        }
        None
    } else {
        info.duration
    };
    let mut args = [
        "-hide_banner",
        "-nostdin",
        "-n",
        "-loglevel",
        "warning",
        "-progress",
        "pipe:1",
        "-nostats",
        "-protocol_whitelist",
        "file,pipe",
        "-format_whitelist",
        DEMUXERS,
    ]
    .iter()
    .map(|x| x.to_string())
    .collect::<Vec<_>>();
    if copying && r.operation == MediaOperation::Trim || r.operation == MediaOperation::Screenshot {
        pair(&mut args, "-ss", format!("{:.3}", r.start));
    }
    pair(&mut args, "-i", &r.input_path);
    if r.operation == MediaOperation::Trim && !copying {
        pair(&mut args, "-ss", format!("{:.3}", r.start));
    }
    if r.operation == MediaOperation::Trim {
        pair(&mut args, "-t", format!("{:.3}", duration.unwrap()));
    }
    match r.operation {
        MediaOperation::Remux => {
            pair(&mut args, "-map", "0");
            pair(&mut args, "-c", "copy");
        }
        MediaOperation::Trim if copying => {
            pair(&mut args, "-map", "0");
            pair(&mut args, "-c", "copy");
            pair(&mut args, "-avoid_negative_ts", "make_zero");
        }
        MediaOperation::Transcode | MediaOperation::Compress | MediaOperation::Trim => {
            pair(&mut args, "-map", format!("0:{}", v.unwrap().index));
            video(&mut args, r);
            if let Some(s) = first_audio {
                pair(&mut args, "-map", format!("0:{}", s.index));
                audio(&mut args, audio_name(r.audio_codec), r.audio_bitrate);
            } else {
                args.push("-an".into());
            }
            args.extend(["-sn".into(), "-dn".into()]);
        }
        MediaOperation::ExtractAudio => {
            let s = first_audio.ok_or("文件没有音频轨道")?;
            let codec = match r.output_format {
                OutputFormat::Mp3 => "mp3",
                OutputFormat::M4a | OutputFormat::Aac => "aac",
                OutputFormat::Wav => "pcm_s16le",
                OutputFormat::Flac => "flac",
                OutputFormat::Opus => "opus",
                _ => return Err("请选择受支持的音频格式".into()),
            };
            if r.copy_audio && !audio_compatible(r.output_format, &s.codec) {
                return Err("原始音频编码与目标格式不兼容，请选择音频转码".into());
            }
            pair(&mut args, "-map", format!("0:{}", s.index));
            args.extend(["-vn".into(), "-sn".into(), "-dn".into()]);
            audio(
                &mut args,
                if r.copy_audio { "copy" } else { codec },
                r.audio_bitrate,
            );
        }
        MediaOperation::ExtractFrames
        | MediaOperation::ContactSheet
        | MediaOperation::CoverExtract
        | MediaOperation::CoverSet
        | MediaOperation::Metadata => unreachable!(),
        MediaOperation::SubtitleConvert
        | MediaOperation::SubtitleMux
        | MediaOperation::SubtitleBurn => unreachable!(),
        MediaOperation::Screenshot => {
            pair(&mut args, "-map", format!("0:{}", v.unwrap().index));
            pair(&mut args, "-frames:v", "1");
            pair(
                &mut args,
                "-c:v",
                match r.output_format {
                    OutputFormat::Png => "png",
                    OutputFormat::Jpg => "mjpeg",
                    _ => "libwebp",
                },
            );
            args.push("-an".into());
            if r.output_format == OutputFormat::Jpg {
                pair(&mut args, "-q:v", "2");
            }
        }
    }
    if matches!(
        r.output_format,
        OutputFormat::Mp4 | OutputFormat::Mov | OutputFormat::M4a
    ) {
        pair(&mut args, "-movflags", "+faststart");
    }
    pair(&mut args, "-f", r.output_format.muxer());
    args.push(output.to_string_lossy().into_owned());
    Ok(BuiltCommand { args, duration })
}
