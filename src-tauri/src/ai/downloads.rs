use super::{models::AnalysisRequest, service::AiService, settings::AiSettings};
use crate::{
    domain::AppResult,
    learning::{importer, service::LearningService},
    models::{DownloadTask, MediaKind, TaskStatus},
};
use std::{path::PathBuf, sync::Arc};
pub fn eligible(settings: &AiSettings, task: &DownloadTask) -> bool {
    settings.auto_analyze_downloads
        && task.status == TaskStatus::Completed
        && task.request.kind != MediaKind::Subtitles
        && (settings.allow_transcript || settings.allow_images)
}
fn download_source(task: &DownloadTask) -> AppResult<Option<PathBuf>> {
    let root = std::fs::canonicalize(&task.output_dir).map_err(|_| "下载目录不可访问")?;
    for file in task.files.iter().take(1000) {
        if !importer::video_path(file) {
            continue;
        }
        let source = importer::regular_file(file)?;
        if !source.starts_with(&root) {
            return Err("自动分析输入不在下载目录中".into());
        }
        return Ok(Some(source));
    }
    Ok(None)
}
impl AiService {
    pub fn after_download(
        self: &Arc<Self>,
        task: &DownloadTask,
        learning: Arc<LearningService>,
    ) -> AppResult<()> {
        if !eligible(&self.settings()?.settings, task) {
            return Ok(());
        }
        let ai = self.clone();
        let task = task.clone();
        std::thread::Builder::new()
            .name("ai-download-import".into())
            .spawn(move || {
                let result: AppResult<()> = (|| {
                    let view = ai.settings()?;
                    if !eligible(&view.settings, &task) {
                        return Ok(());
                    }
                    if !view.has_api_key {
                        return Err("自动分析未开始，请先在AI设置中配置密钥".into());
                    }
                    let Some(source) = download_source(&task)? else {
                        return Ok(());
                    };
                    let item = learning.import_for_analysis(&source.to_string_lossy())?;
                    // Recheck opt-in after the local import; it may have been disabled while probing.
                    let view = ai.settings()?;
                    if !eligible(&view.settings, &task) {
                        return Ok(());
                    }
                    ai.start_analysis(AnalysisRequest {
                        path: source.to_string_lossy().into_owned(),
                        item_id: Some(item.id),
                        mode: view.settings.default_mode,
                        transcribe_audio: true,
                        allow_transcript: view.settings.allow_transcript,
                        allow_images: view.settings.allow_images,
                        force: false,
                    })?;
                    Ok(())
                })();
                if let Err(error) = result {
                    ai.emit(
                        "ai-auto-analysis-warning",
                        serde_json::json!({"id":task.id,"error":crate::domain::redact(&error)}),
                    );
                }
            })
            .map_err(|_| "无法启动自动分析导入线程")?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_ai_requires_explicit_opt_in_and_new_completed_media() {
        let mut settings = AiSettings::default();
        let mut task = DownloadTask {
            status: TaskStatus::Completed,
            ..Default::default()
        };
        assert!(!eligible(&settings, &task));
        settings.auto_analyze_downloads = true;
        assert!(eligible(&settings, &task));
        task.status = TaskStatus::Failed;
        assert!(!eligible(&settings, &task));
        task.status = TaskStatus::Completed;
        task.request.kind = MediaKind::Subtitles;
        assert!(!eligible(&settings, &task));
        task.request.kind = MediaKind::Audio;
        assert!(eligible(&settings, &task));
        settings.allow_transcript = false;
        settings.allow_images = false;
        assert!(!eligible(&settings, &task));
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;
    #[test]
    fn automatic_ai_only_reads_original_media_inside_download_root() {
        let dir = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
        let root = dir.join("下载 #&");
        std::fs::create_dir_all(&root).unwrap();
        let media = root.join("课程.wmv");
        let outside = dir.join("outside.mp4");
        std::fs::write(&media, b"unchanged").unwrap();
        std::fs::write(&outside, b"private").unwrap();
        let mut task = DownloadTask {
            output_dir: root.to_string_lossy().into_owned(),
            files: vec![
                root.join("课程.srt").to_string_lossy().into_owned(),
                media.to_string_lossy().into_owned(),
            ],
            ..Default::default()
        };
        assert_eq!(
            download_source(&task).unwrap().unwrap(),
            std::fs::canonicalize(&media).unwrap()
        );
        task.files = vec![outside.to_string_lossy().into_owned()];
        assert!(download_source(&task).is_err());
        assert_eq!(std::fs::read(media).unwrap(), b"unchanged");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
