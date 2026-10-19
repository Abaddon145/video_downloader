use super::{jobs::Control, models::*, service::AiService, transcription as t};
use crate::{
    domain::AppResult,
    learning::{cache::owned, db},
};
use std::{path::Path, sync::Arc};

impl AiService {
    pub(crate) fn reuse_materials(
        &self,
        a: &VideoAnalysis,
        duration: i64,
        control: &Arc<Control>,
        transcript: bool,
        frames: bool,
    ) -> AppResult<()> {
        self.owned(&a.id, control, |_| Ok(()))?;
        if !a.options.local_only {
            self.check_consent(&a.options)?;
        }
        Self::check_source(Path::new(&a.source_path), &a.source_fingerprint)?;
        let previous = self.with(|r| {
            // Persisted absence means a full analysis, including after restart; only explicit template/regenerate actions reuse materials.
            let id: Option<String> = r
                .conn
                .query_row(
                    "SELECT reuse_analysis_id FROM video_analyses WHERE id=?1",
                    [&a.id],
                    |r| r.get(0),
                )
                .map_err(db::error)?;
            id.and_then(|id| r.get(&id).ok())
                .map(|old| {
                    Ok((
                        old.clone(),
                        r.all_transcript(&old.id)?,
                        r.keyframes(&old.id)?,
                    ))
                })
                .transpose()
        })?;
        let Some((old, mut segments, old_frames)) = previous else {
            return Ok(());
        };
        for id in [&a.id, &old.id] {
            if id.is_empty()
                || id.len() > 128
                || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                return Err("复用分析标识无效".into());
            }
        }
        if old.id == a.id
            || crate::learning::importer::regular_file(&old.source_path)?
                != crate::learning::importer::regular_file(&a.source_path)?
            || old.source_fingerprint != a.source_fingerprint
            || old.item_id != a.item_id
            || old.duration_ms != Some(duration)
        {
            return Ok(());
        }
        let target = self.data.join("ai/cache").join(&a.id);
        if transcript
            && !a.transcript_done
            && old.transcript_done
            && old.options.whisper_model == a.options.whisper_model
            && old.options.transcribe_audio == a.options.transcribe_audio
            && !segments.is_empty()
        {
            let old_root = self.data.join("ai/cache").join(&old.id);
            let marker = old_root.join("transcript-source.txt");
            let origin = if marker.exists() {
                let marker = owned(&old_root, &marker)?;
                if std::fs::metadata(&marker)
                    .map_err(|_| "字幕来源不可读")?
                    .len()
                    > 16
                {
                    return Err("字幕来源标记无效".into());
                }
                std::fs::read_to_string(marker).unwrap_or_else(|_| "unknown".into())
            } else {
                "unknown".into()
            };
            // Current external/embedded subtitles are read first; a removed or changed subtitle is not a reusable Whisper result.
            if origin.trim() == "whisper"
                && segments.iter().all(|s| {
                    s.analysis_id == old.id
                        && s.start_ms >= 0
                        && s.end_ms > s.start_ms
                        && s.end_ms <= duration
                        && !s.text.trim().is_empty()
                        && s.text.len() <= 16000
                })
            {
                for (index, segment) in segments.iter_mut().enumerate() {
                    segment.id = format!("{}-segment-{index}", a.id);
                    segment.analysis_id = a.id.clone();
                    segment.segment_index = index as u32;
                }
                self.owned(&a.id, control, |r| {
                    r.save_transcript(&a.id, old.language.as_deref(), &segments)
                })?;
            }
        }
        if frames
            && !a.keyframes_done
            && a.options.allow_images
            && old.options.allow_images
            && old.keyframes_done
            && old.analysis_mode == a.analysis_mode
            && !old_frames.is_empty()
            && old_frames.len() <= super::keyframes::policy(a.analysis_mode).max_frames
        {
            let root = self.data.join("ai/cache");
            let directory = target.join("keyframes");
            std::fs::create_dir_all(&directory).map_err(|_| "无法创建复用画面目录")?;
            let directory = owned(&root, &directory)?;
            let mut copied = Vec::new();
            for (index, frame) in old_frames.into_iter().enumerate() {
                control.check()?;
                if frame.analysis_id != old.id || frame.time_ms < 0 || frame.time_ms >= duration {
                    return Err("复用画面时间或来源无效".into());
                }
                let source = owned(&root.join(&old.id), Path::new(&frame.path))?;
                let _lock = t::lock_source(&source)?;
                if source
                    .extension()
                    .is_none_or(|e| !e.eq_ignore_ascii_case("jpg"))
                    || std::fs::metadata(&source)
                        .map_err(|_| "复用画面不可读")?
                        .len()
                        > super::keyframes::MAX_JPEG_BYTES
                    || t::file_hash(&source)? != frame.sha256
                {
                    return Err("复用画面已变化或超过限制".into());
                }
                let bytes = std::fs::read(source).map_err(|_| "复用画面不可读")?;
                let (w, h) = super::keyframes::jpeg_dimensions(&bytes)?;
                if w > super::keyframes::MAX_IMAGE_DIMENSION
                    || h > super::keyframes::MAX_IMAGE_DIMENSION
                {
                    return Err("复用画面尺寸无效".into());
                }
                let file = directory.join(format!("reused-{index:04}.jpg"));
                self.owned(&a.id, control, |_| {
                    crate::native::save_atomic(&file, &bytes)
                })?;
                copied.push(AnalysisKeyframe {
                    id: format!("{}-frame-{index}", a.id),
                    analysis_id: a.id.clone(),
                    path: file.to_string_lossy().into_owned(),
                    ..frame
                });
            }
            Self::check_source(Path::new(&a.source_path), &a.source_fingerprint)?;
            self.check_consent(&a.options)?;
            self.owned(&a.id, control, |r| r.save_keyframes(&a.id, &copied))?;
        }
        Self::check_source(Path::new(&a.source_path), &a.source_fingerprint)?;
        control.check()
    }
}
