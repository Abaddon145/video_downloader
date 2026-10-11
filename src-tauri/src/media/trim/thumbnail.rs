use crate::{
    domain::AppResult,
    ffmpeg::{output, probe, runner::MediaService},
};
use serde::Serialize;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineThumbnail {
    pub time: f64,
    pub path: String,
}
impl MediaService {
    pub fn editor_thumbnails(
        &self,
        path: &str,
        session: &str,
        offset: u32,
    ) -> AppResult<Vec<TimelineThumbnail>> {
        self.ensure_ready()?;
        super::timeline::validate_session(session)?;
        let input = output::input(path)?;
        let info = self.probe_file(&input, &format!("{session}-thumb-probe"), false)?;
        let duration = super::processor::validate_duration(info.duration)?;
        let step = super::timeline::interval(duration);
        let start = offset as f64 * step;
        if start >= duration || offset > 50000 {
            return Err("缩略图位置超出视频范围".into());
        }
        let dir = self
            .preview_directory(session)?
            .join(crate::ffmpeg::runner::new_id());
        std::fs::create_dir(&dir).map_err(|_| "无法创建缩略图目录")?;
        let count = ((duration - start) / step).ceil().min(60.) as usize;
        let args = vec![
            "-hide_banner".into(),
            "-nostdin".into(),
            "-n".into(),
            "-loglevel".into(),
            "error".into(),
            "-ss".into(),
            format!("{start:.3}"),
            "-protocol_whitelist".into(),
            "file,pipe".into(),
            "-format_whitelist".into(),
            probe::DEMUXERS.into(),
            "-i".into(),
            input.to_string_lossy().into_owned(),
            "-map".into(),
            format!(
                "0:{}",
                info.videos
                    .iter()
                    .find(|v| !v.attached_picture)
                    .ok_or("文件没有视频流")?
                    .index
            ),
            "-vf".into(),
            format!("fps=1/{step}:start_time=0,scale=160:-2"),
            "-frames:v".into(),
            count.to_string(),
            "-q:v".into(),
            "5".into(),
            dir.join("frame-%04d.jpg").to_string_lossy().into_owned(),
        ];
        self.auxiliary(session, &args, 120)?;
        let mut images = Vec::new();
        for i in 1..=count {
            let p = dir.join(format!("frame-{i:04}.jpg"));
            if p.is_file() {
                self.authorize_asset(session, &p)?;
                images.push(TimelineThumbnail {
                    time: start + (i - 1) as f64 * step,
                    path: p.to_string_lossy().into_owned(),
                });
            }
        }
        if images.is_empty() {
            return Err("没有生成可用缩略图".into());
        }
        Ok(images)
    }
}
