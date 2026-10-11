use crate::{
    domain::AppResult,
    ffmpeg::{
        command::{self, BuiltCommand},
        models::*,
    },
};
use std::path::Path;
pub fn validate(r: &MediaRequest) -> AppResult<()> {
    if r.metadata.len() > 8
        || r.metadata.iter().any(|(k, v)| {
            ![
                "title",
                "artist",
                "author",
                "date",
                "description",
                "album",
                "genre",
                "comment",
            ]
            .contains(&k.as_str())
                || v.chars().count() > 4096
                || v.chars().any(|c| c == '\0' || c.is_control() && c != '\n')
        })
    {
        return Err(
            "元数据仅支持标题、作者、日期、描述、Artist、Album、Genre、Comment（每项4096字以内）"
                .into(),
        );
    }
    Ok(())
}
pub fn cover_extension(path: &str) -> AppResult<String> {
    crate::ffmpeg::command::valid_path(path)?;
    let e = Path::new(path)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if !["jpg", "jpeg", "png"].contains(&e.as_str()) {
        return Err("封面仅支持JPG/PNG".into());
    }
    Ok(if e == "jpeg" { "jpg".into() } else { e })
}
pub fn stage_cover(path: &str, dir: &Path) -> AppResult<()> {
    let file = crate::ffmpeg::output::input(path)?;
    if std::fs::metadata(&file).map_err(|_| "无法读取封面")?.len() > 50 * 1024 * 1024 {
        return Err("封面不能超过50MiB".into());
    }
    std::fs::copy(file, dir.join(format!("cover.{}", cover_extension(path)?)))
        .map_err(|_| "无法准备封面")?;
    Ok(())
}
pub fn build(r: &MediaRequest, info: &MediaInfo, out: &Path) -> AppResult<BuiltCommand> {
    validate(r)?;
    let cover = r.operation == MediaOperation::CoverSet;
    if cover
        && !matches!(
            r.output_format,
            OutputFormat::Mp4 | OutputFormat::M4a | OutputFormat::Mp3 | OutputFormat::Flac
        )
    {
        return Err("设置封面支持MP4、M4A、MP3、FLAC".into());
    }
    let mut i = info.clone();
    let pictures: Vec<_> = info.videos.iter().filter(|v| v.attached_picture).collect();
    if !pictures.is_empty()
        && !matches!(
            r.output_format,
            OutputFormat::Mp4
                | OutputFormat::Mov
                | OutputFormat::Mkv
                | OutputFormat::M4a
                | OutputFormat::Mp3
                | OutputFormat::Flac
        )
    {
        return Err("此目标格式无法保留内嵌封面，请选择兼容容器".into());
    }
    i.videos.retain(|v| !v.attached_picture);
    let mut base = r.clone();
    let audio = matches!(
        r.output_format,
        OutputFormat::Mp3
            | OutputFormat::M4a
            | OutputFormat::Flac
            | OutputFormat::Wav
            | OutputFormat::Aac
            | OutputFormat::Opus
    );
    if audio {
        if !i.videos.is_empty()
            || i.audios.len() != 1
            || !i.subtitles.is_empty()
            || i.other_streams > 0
        {
            return Err("音频标签/封面输出需要单音轨文件；视频请选择视频容器".into());
        }
        base.operation = MediaOperation::ExtractAudio;
        base.copy_audio = true;
    } else {
        base.operation = MediaOperation::Remux;
    }
    let mut c = command::build(&base, &i, out)?;
    let output = c.args.pop().ok_or("输出参数缺失")?;
    if cover {
        let path = r.cover_path.as_deref().ok_or("请选择封面图片")?;
        let ext = cover_extension(path)?;
        let input_end = c
            .args
            .iter()
            .position(|a| a == "-map")
            .ok_or("映射参数缺失")?;
        c.args.splice(
            input_end..input_end,
            [
                "-f".into(),
                "image2".into(),
                "-pattern_type".into(),
                "none".into(),
                "-protocol_whitelist".into(),
                "file,pipe".into(),
                "-i".into(),
                format!("cover.{ext}"),
            ],
        );
        for v in info.videos.iter().filter(|v| v.attached_picture) {
            if !audio {
                c.args.extend(["-map".into(), format!("-0:{}", v.index)]);
            }
        }
        c.args.retain(|a| a != "-vn");
        let index = i.videos.len();
        c.args.extend([
            "-map".into(),
            "1:v:0".into(),
            format!("-c:v:{index}"),
            "copy".into(),
            format!("-disposition:v:{index}"),
            "attached_pic".into(),
        ]);
    }
    if !cover && audio && !pictures.is_empty() {
        c.args.retain(|a| a != "-vn");
        c.args.extend(["-c:v".into(), "copy".into()]);
        for (index, picture) in pictures.iter().enumerate() {
            c.args.extend([
                "-map".into(),
                format!("0:{}", picture.index),
                format!("-disposition:v:{index}"),
                "attached_pic".into(),
            ]);
        }
    }
    c.args.extend(["-map_metadata".into(), "0".into()]);
    for (key, value) in &r.metadata {
        c.args
            .extend(["-metadata".into(), format!("{key}={value}")]);
    }
    c.args.push(output);
    Ok(c)
}
