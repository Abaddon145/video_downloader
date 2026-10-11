use crate::{
    domain::AppResult,
    ffmpeg::{
        command::{self, valid_path, BuiltCommand},
        models::*,
    },
};
use std::path::Path;
pub fn extension(path: &str) -> AppResult<String> {
    valid_path(path)?;
    let e = Path::new(path)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    if !["srt", "ass", "vtt"].contains(&e.as_str()) {
        return Err("字幕仅支持 SRT、ASS、VTT".into());
    }
    Ok(e)
}
pub fn stage(path: &str, directory: &Path) -> AppResult<()> {
    let p = crate::ffmpeg::output::input(path)?;
    if std::fs::metadata(&p).map_err(|_| "字幕无法读取")?.len() > 8 * 1024 * 1024 {
        return Err("字幕文件不能超过8MiB".into());
    }
    let e = extension(path)?;
    std::fs::copy(p, directory.join(format!("subtitle.{e}"))).map_err(|_| "无法准备字幕文件")?;
    Ok(())
}
pub fn style(r: &MediaRequest) -> AppResult<String> {
    if r.subtitle_font.is_empty()
        || r.subtitle_font.chars().count() > 80
        || !r
            .subtitle_font
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
        || !(8..=96).contains(&r.subtitle_size)
        || ![2, 5, 8].contains(&r.subtitle_position)
        || r.subtitle_color.len() != 7
        || !r.subtitle_color.starts_with('#')
        || !r.subtitle_color[1..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("字幕样式参数无效".into());
    }
    let c = &r.subtitle_color;
    Ok(format!(
        "FontName={},FontSize={},Alignment={},PrimaryColour=&H00{}{}{}",
        r.subtitle_font,
        r.subtitle_size,
        r.subtitle_position,
        &c[5..7],
        &c[3..5],
        &c[1..3]
    ))
}
pub fn build(r: &MediaRequest, i: &MediaInfo, out: &Path) -> AppResult<BuiltCommand> {
    if r.operation == MediaOperation::SubtitleBurn {
        let e = extension(r.subtitle_path.as_deref().ok_or("请选择外挂字幕文件")?)?;
        let style = style(r)?;
        let mut base = r.clone();
        base.operation = MediaOperation::Transcode;
        if base.video_codec == VideoCodec::Copy {
            return Err("硬字幕需要重新编码视频".into());
        }
        let mut c = command::build(&base, i, out)?;
        let filter = format!("subtitles=subtitle.{e}:force_style='{style}'");
        if let Some(index) = c.args.iter().position(|a| a == "-vf") {
            c.args[index + 1] = format!("{},{}", c.args[index + 1], filter);
        } else {
            c.args
                .splice(c.args.len() - 1..c.args.len() - 1, ["-vf".into(), filter]);
        }
        return Ok(c);
    }
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
        crate::ffmpeg::probe::DEMUXERS,
        "-i",
        &r.input_path,
    ]
    .map(str::to_string)
    .to_vec();
    if r.operation == MediaOperation::SubtitleConvert {
        if !matches!(
            r.output_format,
            OutputFormat::Srt | OutputFormat::Ass | OutputFormat::Vtt
        ) || i.subtitles.is_empty()
        {
            return Err("请选择文本字幕输入与 SRT/ASS/VTT 输出".into());
        }
        if !["subrip", "ass", "ssa", "webvtt", "mov_text"].contains(&i.subtitles[0].codec.as_str())
        {
            return Err("不支持位图字幕转换".into());
        }
        args.extend([
            "-map".into(),
            "0:s:0".into(),
            "-c:s".into(),
            r.output_format.muxer().into(),
        ]);
    } else {
        if !matches!(
            r.output_format,
            OutputFormat::Mkv | OutputFormat::Mp4 | OutputFormat::Mov
        ) || i.other_streams > 0
            || i.videos
                .iter()
                .any(|s| !command::video_compatible(r.output_format, &s.codec))
            || i.audios
                .iter()
                .any(|s| !command::audio_compatible(r.output_format, &s.codec))
            || i.subtitles.iter().any(|s| {
                !["subrip", "ass", "ssa", "webvtt", "mov_text"].contains(&s.codec.as_str())
            })
        {
            return Err("媒体轨道无法安全封装，请选择兼容容器；位图字幕需其他工具".into());
        }
        if let Some(p) = &r.subtitle_path {
            let e = extension(p)?;
            args.extend([
                "-protocol_whitelist".into(),
                "file,pipe".into(),
                "-format_whitelist".into(),
                crate::ffmpeg::probe::DEMUXERS.into(),
                "-i".into(),
                format!("subtitle.{e}"),
            ]);
        } else if i.subtitles.is_empty() {
            return Err("没有找到已有文本字幕，请选择字幕文件".into());
        }
        args.extend(["-map".into(), "0".into()]);
        if r.subtitle_path.is_some() {
            args.extend(["-map".into(), "1:s:0".into()]);
        }
        args.extend([
            "-c".into(),
            "copy".into(),
            "-c:s".into(),
            if r.output_format == OutputFormat::Mkv {
                "ass".into()
            } else {
                "mov_text".into()
            },
        ]);
    }
    args.extend([
        "-f".into(),
        r.output_format.muxer().into(),
        out.to_string_lossy().into(),
    ]);
    Ok(BuiltCommand {
        args,
        duration: i.duration,
    })
}
