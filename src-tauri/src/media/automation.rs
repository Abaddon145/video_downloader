use super::preset::{AfterDownload, MediaProSettings};
use crate::models::{DownloadTask, TaskStatus};
pub fn eligible(settings: &MediaProSettings, task: &DownloadTask) -> bool {
    settings.after_download != AfterDownload::None
        && settings.enabled_at > 0
        && task.status == TaskStatus::Completed
        && task.finished_at.is_some_and(|t| t >= settings.enabled_at)
}
impl crate::ffmpeg::runner::MediaService {
    pub fn after_download(&self, task: &DownloadTask) -> crate::domain::AppResult<()> {
        use crate::ffmpeg::{models::*, output};
        use std::path::Path;
        let settings = self.pro_settings();
        if !eligible(&settings, task) {
            return Ok(());
        }
        let root = std::fs::canonicalize(&task.output_dir).map_err(|_| "下载目录不可访问")?;
        for file in &task.files {
            let extension = Path::new(file)
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase();
            if !["mp4", "mkv", "mov", "webm", "avi", "m4v", "ts", "flv"]
                .contains(&extension.as_str())
            {
                continue;
            }
            let input = output::input(file)?;
            if !input.starts_with(&root) {
                return Err("自动处理输入不在下载目录中".into());
            }
            let mut request = MediaRequest {
                input_path: input.to_string_lossy().into(),
                output_dir: input
                    .parent()
                    .ok_or("下载文件路径无效")?
                    .to_string_lossy()
                    .into(),
                source_download_id: Some(task.id.clone()),
                hardware_acceleration: settings.hardware_acceleration,
                ..Default::default()
            };
            match settings.after_download {
                AfterDownload::Mp4 => {}
                AfterDownload::Compress => {
                    request.operation = MediaOperation::Compress;
                    request.quality = Quality::Small;
                }
                AfterDownload::Cover => {
                    request.operation = MediaOperation::Screenshot;
                    request.output_format = OutputFormat::Jpg;
                }
                AfterDownload::Subtitles => {
                    request.operation = serde_json::from_value(serde_json::json!("subtitleMux"))
                        .map_err(|_| "字幕模块尚未就绪")?;
                    request.output_format = OutputFormat::Mkv;
                    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
                    request.subtitle_path = task
                        .files
                        .iter()
                        .filter(|p| {
                            Path::new(p)
                                .file_name()
                                .is_some_and(|f| f.to_string_lossy().starts_with(stem.as_ref()))
                        })
                        .find(|p| {
                            Path::new(p).extension().is_some_and(|e| {
                                ["srt", "ass", "vtt"]
                                    .contains(&e.to_string_lossy().to_ascii_lowercase().as_str())
                            })
                        })
                        .cloned();
                }
                AfterDownload::None => continue,
            }
            self.create(request)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_processing_only_accepts_new_completed_downloads() {
        let mut s = MediaProSettings::default();
        s.after_download = AfterDownload::Mp4;
        s.enabled_at = 100;
        let mut t = DownloadTask::default();
        t.status = TaskStatus::Completed;
        t.finished_at = Some(101);
        assert!(eligible(&s, &t));
        t.finished_at = Some(99);
        assert!(!eligible(&s, &t));
        t.finished_at = Some(101);
        t.status = TaskStatus::Cancelled;
        assert!(!eligible(&s, &t));
    }
}
