use crate::{
    domain::AppResult,
    ffmpeg::{
        command::{build, BuiltCommand},
        models::*,
    },
};
use std::path::Path;
pub fn sampling(r: &MediaRequest, duration: f64) -> AppResult<(f64, u32)> {
    if !duration.is_finite()
        || duration <= 0.
        || !r.frame_interval.is_finite()
        || r.frame_interval < 0.04
        || !r.frame_fps.is_finite()
        || r.frame_fps <= 0.
        || r.frame_fps > 60.
        || r.frame_count == 0
        || r.frame_count > 10000
    {
        return Err("抽帧参数无效：最多10000张，FPS不超过60".into());
    }
    let fps = match r.frame_mode {
        FrameMode::Interval => 1. / r.frame_interval,
        FrameMode::Count => r.frame_count as f64 / duration,
        FrameMode::Fps => r.frame_fps,
    };
    let count = if matches!(r.frame_mode, FrameMode::Count) {
        r.frame_count
    } else {
        (fps * duration).ceil() as u32
    };
    if count == 0 || count > 10000 {
        return Err("此设置会生成超过10000张图片，请增大间隔或减少FPS".into());
    }
    Ok((fps, count))
}
pub fn build_frames(r: &MediaRequest, info: &MediaInfo, out: &Path) -> AppResult<BuiltCommand> {
    if !matches!(
        r.output_format,
        OutputFormat::Png | OutputFormat::Jpg | OutputFormat::Webp
    ) {
        return Err("抽帧请选择图片格式".into());
    }
    let duration = info.duration.ok_or("无法确定视频时长")?;
    let (fps, count) = sampling(r, duration)?;
    let mut base = r.clone();
    base.operation = MediaOperation::Screenshot;
    base.start = 0.;
    let mut c = build(
        &base,
        info,
        &out.join(format!("frame-%06d.{}", r.output_format.extension())),
    )?;
    let n = c
        .args
        .iter()
        .position(|a| a == "-frames:v")
        .ok_or("抽帧参数缺失")?;
    c.args[n + 1] = count.to_string();
    c.args.splice(
        c.args.len() - 1..c.args.len() - 1,
        ["-vf".into(), format!("fps={fps:.9}:start_time=0")],
    );
    c.duration = Some(duration);
    Ok(c)
}
pub fn build_cover(r: &MediaRequest, info: &MediaInfo, out: &Path) -> AppResult<BuiltCommand> {
    let mut base = r.clone();
    base.operation = MediaOperation::Screenshot;
    let mut i = info.clone();
    if let Some(p) = i.videos.iter().find(|v| v.attached_picture).cloned() {
        i.videos = vec![VideoStreamInfo {
            attached_picture: false,
            ..p
        }];
        base.start = 0.;
        i.duration = None;
    }
    build(&base, &i, out)
}
