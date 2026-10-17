use crate::{
    domain::AppResult,
    native::{self, Job},
};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
pub struct Control {
    cancelled: AtomicBool,
    signal: tokio::sync::watch::Sender<bool>,
    job: Mutex<Option<Arc<Job>>>,
}
impl Default for Control {
    fn default() -> Self {
        let (signal, _) = tokio::sync::watch::channel(false);
        Self {
            cancelled: AtomicBool::new(false),
            signal,
            job: Mutex::new(None),
        }
    }
}
impl Control {
    pub async fn reply(
        &self,
        future: impl std::future::Future<Output = AppResult<super::client::ChatReply>>,
    ) -> AppResult<super::client::ChatReply> {
        self.check()?;
        let mut receiver = self.signal.subscribe();
        tokio::select! {biased;result=future=>result,_=receiver.wait_for(|cancelled|*cancelled)=>Err("分析已取消".into())}
    }

    #[cfg(test)]
    pub(crate) fn has_job(&self) -> bool {
        self.job.lock().unwrap().is_some()
    }
    pub fn check(&self) -> AppResult<()> {
        if self.cancelled.load(Ordering::SeqCst) {
            Err("分析已取消".into())
        } else {
            Ok(())
        }
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.signal.send_replace(true);
        if let Ok(job) = self.job.lock() {
            if let Some(job) = job.as_ref() {
                let _ = job.terminate();
            }
        }
    }
    pub fn capture(
        &self,
        exe: &Path,
        args: &[String],
        directory: Option<&Path>,
        timeout: Duration,
    ) -> AppResult<Vec<u8>> {
        self.check()?;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut timer = None;
        let mut registered = Ok(());
        let result = native::capture_bytes_in(exe, args, 32 * 1024 * 1024, directory, |job| {
            registered = self
                .job
                .lock()
                .map_err(|_| "分析进程状态不可用".to_owned())
                .map(|mut active| {
                    *active = Some(job.clone());
                    if self.check().is_err() {
                        let _ = job.terminate();
                    }
                });
            timer = Some(std::thread::spawn(move || {
                if rx.recv_timeout(timeout).is_err() {
                    let _ = job.terminate();
                }
            }));
        });
        let _ = tx.send(());
        if let Some(t) = timer {
            let _ = t.join();
        }
        if let Ok(mut job) = self.job.lock() {
            *job = None;
        }
        registered?;
        self.check()?;
        result.map_err(|_| "本地媒体处理失败或超时，请检查组件、源文件及可用磁盘空间".into())
    }
}
#[derive(Default)]
pub struct AiRuntime {
    pub active: Option<(String, Arc<Control>)>,
    pub exiting: bool,
    pub pending: std::collections::VecDeque<String>,
}

use super::{client::DeepSeekClient, models::*, service::AiService, transcription as t};
use crate::learning::db;
use rusqlite::{params, OptionalExtension};
const MAX_QUEUED: usize = 100;
impl AiService {
    pub fn start_transcription(
        self: &Arc<Self>,
        path: &str,
        item_id: Option<String>,
    ) -> AppResult<VideoAnalysis> {
        let settings = self.settings()?.settings;
        self.enqueue(
            path,
            item_id,
            settings.default_mode,
            false,
            AnalysisOptions {
                whisper_model: settings.whisper_model,
                ..Default::default()
            },
        )
    }
    pub fn start_analysis(self: &Arc<Self>, request: AnalysisRequest) -> AppResult<VideoAnalysis> {
        let settings = self.settings()?.settings;
        let options = AnalysisOptions {
            local_only: false,
            transcribe_audio: request.transcribe_audio,
            allow_transcript: request.allow_transcript,
            allow_images: request.allow_images,
            whisper_model: settings.whisper_model.clone(),
            auto_generate_note: settings.auto_generate_notes,
        };
        self.check_consent(&options)?;
        if !options.allow_transcript && !options.allow_images {
            return Err("至少需要允许发送字幕或关键帧中的一种；仍可单独使用本地转录".into());
        }
        if options.allow_images && settings.model != super::settings::DEFAULT_MODEL {
            return Err("当前模型不支持画面分析，请选择 DeepSeek Flash".into());
        }
        self.key()?;
        self.enqueue(
            &request.path,
            request.item_id,
            request.mode,
            !request.force,
            options,
        )
    }
    fn enqueue(
        self: &Arc<Self>,
        path: &str,
        item_id: Option<String>,
        mode: AnalysisMode,
        use_cache: bool,
        options: AnalysisOptions,
    ) -> AppResult<VideoAnalysis> {
        let source = crate::learning::importer::regular_file(path)?;
        let fingerprint = t::fingerprint(&source)?;
        let settings = self.settings()?.settings;
        let options_json = serde_json::to_string(&options).map_err(|_| "无法保存分析选项")?;
        let mut runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
        if runtime.exiting {
            return Err("软件正在退出".into());
        }
        if runtime.pending.len() >= MAX_QUEUED {
            return Err("分析队列已满，请等待部分任务完成".into());
        }
        let id=self.with(|r|{
            if let Some(item)=&item_id {
                let saved:Option<String>=r.conn.query_row("SELECT file_path FROM series_items WHERE id=?1",[item],|row|row.get(0)).map_err(|_|"关联课程不存在")?;
                if saved.as_deref().and_then(|p|crate::learning::importer::regular_file(p).ok()).as_ref()!=Some(&source){return Err("关联课程与源文件不一致".into());}
            }
            if use_cache && !options.local_only {
                let cached:Option<String>=r.conn.query_row("SELECT id FROM video_analyses WHERE source_fingerprint=?1 AND source_path=?2 AND item_id IS ?3 AND analysis_mode=?4 AND model=?5 AND prompt_version=?6 AND options_json=?7 AND status NOT IN ('failed','cancelled','interrupted') ORDER BY created_at DESC LIMIT 1",params![fingerprint,source.to_string_lossy(),item_id,mode.as_str(),settings.model,super::prompts::ANALYSIS_PROMPT_VERSION,options_json],|r|r.get(0)).optional().map_err(db::error)?;
                if let Some(id)=cached{return Ok((id,false));}
            }
            let id=crate::ffmpeg::runner::new_id().replace("media-","ai-");let now=crate::service::now();
            r.conn.execute("INSERT INTO video_analyses(id,item_id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,options_json,created_at,updated_at,stage) VALUES(?1,?2,?3,?4,'queued',?5,?6,?7,?8,?9,?9,'等待分析')",params![id,item_id,source.to_string_lossy(),fingerprint,mode.as_str(),settings.model,super::prompts::ANALYSIS_PROMPT_VERSION,options_json,now]).map_err(db::error)?;
            Ok((id,true))
        })?;
        let analysis = self.analysis(&id.0)?;
        if id.1 {
            runtime.pending.push_back(id.0.clone());
        }
        drop(runtime);
        if id.1 {
            self.emit_analysis(&id.0, "ai-analysis-updated");
            self.kick_queue();
        }
        Ok(analysis)
    }
    pub fn retry_analysis(
        self: &Arc<Self>,
        id: &str,
        regenerate: bool,
    ) -> AppResult<VideoAnalysis> {
        let mut runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
        if runtime.exiting {
            return Err("软件正在退出".into());
        }
        if runtime
            .active
            .as_ref()
            .is_some_and(|(active, _)| active == id)
            || runtime.pending.iter().any(|p| p == id)
        {
            return Err("该分析已经在队列中".into());
        }
        if runtime.pending.len() >= MAX_QUEUED {
            return Err("分析队列已满".into());
        }
        let analysis = self.analysis(id)?;
        if !["failed", "interrupted", "cancelled"].contains(&analysis.status.as_str())
            && !(regenerate && analysis.status == "completed")
        {
            return Err("该分析当前不能重试".into());
        }
        Self::check_source(
            Path::new(&analysis.source_path),
            &analysis.source_fingerprint,
        )?;
        if !regenerate && analysis.prompt_version != super::prompts::ANALYSIS_PROMPT_VERSION {
            return Err("AI模板已更新，请选择重新生成总结；无需重新识别语音".into());
        }
        if !analysis.options.local_only {
            self.check_consent(&analysis.options)?;
            self.key()?;
        }
        if regenerate {
            // Reuse local transcript and frames; the explicit action requests new AI results.
            let cache = self.data.join("ai/cache").join(id).join("summary-cache");
            if cache.exists() {
                let owned =
                    crate::learning::cache::owned(&self.data.join("ai/cache").join(id), &cache)?;
                std::fs::remove_dir_all(owned).map_err(|_| "无法清理旧总结缓存")?;
            }
        }
        self.with(|r|{
            let tx=r.conn.transaction().map_err(db::error)?;
            if regenerate {tx.execute("DELETE FROM analysis_chunks WHERE analysis_id=?1",[id]).map_err(db::error)?;}
            tx.execute("UPDATE video_analyses SET status='queued',stage='等待继续分析',error=NULL,completed_at=NULL,updated_at=?2,summary_json=CASE WHEN ?3 THEN NULL ELSE summary_json END,prompt_version=CASE WHEN ?3 THEN ?4 ELSE prompt_version END WHERE id=?1",params![id,crate::service::now(),regenerate,super::prompts::ANALYSIS_PROMPT_VERSION]).map_err(db::error)?;
            tx.commit().map_err(db::error)
        })?;
        runtime.pending.push_back(id.into());
        let a = self.analysis(id)?;
        drop(runtime);
        self.emit_analysis(id, "ai-analysis-updated");
        self.kick_queue();
        Ok(a)
    }
    fn kick_queue(self: &Arc<Self>) {
        let Ok(mut runtime) = self.runtime.lock() else {
            return;
        };
        if runtime.exiting || runtime.active.is_some() {
            return;
        }
        let Some(id) = runtime.pending.pop_front() else {
            return;
        };
        let control = Arc::new(Control::default());
        runtime.active = Some((id.clone(), control.clone()));
        let service = self.clone();
        let task_id = id.clone();
        let worker_control = control.clone();
        let started = std::thread::Builder::new()
            .name("yingliu-ai-analysis".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    service.run_analysis(&task_id, &worker_control, None)
                }))
                .unwrap_or_else(|_| Err("分析处理意外中断，可重试".into()));
                service.finish_analysis(&task_id, &worker_control, result);
            });
        drop(runtime);
        if started.is_err() {
            self.finish_analysis(&id, &control, Err("无法启动分析线程".into()));
        }
    }
    pub(crate) fn run_analysis(
        self: &Arc<Self>,
        id: &str,
        control: &Arc<Control>,
        client: Option<&DeepSeekClient>,
    ) -> AppResult<()> {
        let analysis = self.analysis(id)?;
        let source = Path::new(&analysis.source_path);
        let _source = t::lock_source(source)?;
        Self::check_source(source, &analysis.source_fingerprint)?;
        let settings = self.settings()?.settings;
        if !analysis.options.local_only {
            self.check_consent(&analysis.options)?;
            self.key()?;
        }
        self.stage(id, control, "extracting_audio", "读取媒体信息", 0.02)?;
        let info = t::probe_media(&self.resources, source, control)?;
        let duration = info
            .duration
            .filter(|d| d.is_finite() && *d > 0. && *d <= 7. * 24. * 3600.)
            .ok_or("无法确定媒体时长或时长超出7天限制")?;
        let duration_ms = (duration * 1000.).round() as i64;
        if analysis.duration_ms.is_some_and(|d| d != duration_ms) {
            return Err("媒体时长已变化，请重新完整分析".into());
        }
        self.owned(id, control, |r| {
            r.conn
                .execute(
                    "UPDATE video_analyses SET duration_ms=?2 WHERE id=?1",
                    params![id, duration_ms],
                )
                .map_err(db::error)?;
            Ok(())
        })?;
        let cache = self.data.join("ai/cache").join(id);
        if !analysis.transcript_done {
            let (existing, warning) =
                if analysis.options.local_only || analysis.options.allow_transcript {
                    super::subtitles::existing(
                        &self.resources,
                        &self.data,
                        source,
                        &info,
                        id,
                        duration_ms,
                        control,
                    )?
                } else {
                    (vec![], None)
                };
            let subtitle = !existing.is_empty();
            let (language, segments) = if subtitle {
                self.stage(id, control, "transcribing", "正在读取已有字幕", 0.2)?;
                (None, existing)
            } else if !analysis.options.transcribe_audio || info.audios.is_empty() {
                (None, vec![])
            } else {
                t::verify_model(&self.data, &analysis.options.whisper_model).map_err(|error| {
                    format!(
                        "{}；{error}",
                        warning.as_deref().unwrap_or("未找到可用字幕")
                    )
                })?;
                t::whisper_binary(&self.resources)?;
                self.stage(
                    id,
                    control,
                    "extracting_audio",
                    "未找到可用字幕，正在提取音频",
                    0.08,
                )?;
                t::extract_audio(&self.resources, source, &cache, &info, control)?;
                Self::check_source(source, &analysis.source_fingerprint)?;
                self.stage(id, control, "transcribing", "正在本地识别语音", 0.2)?;
                t::transcribe(
                    &self.resources,
                    &self.data,
                    &cache,
                    &analysis.options.whisper_model,
                    id,
                    duration_ms,
                    control,
                )?
            };
            control.check()?;
            std::fs::create_dir_all(&cache).map_err(|_| "无法保存字幕来源")?;
            crate::native::save_atomic(
                &cache.join("transcript-source.txt"),
                if subtitle { b"subtitle" } else { b"whisper" },
            )?;
            Self::check_source(source, &analysis.source_fingerprint)?;
            self.owned(id, control, |r| {
                r.save_transcript(id, language.as_deref(), &segments)
            })?;
        }
        if settings.clean_temporary_files {
            for file in ["audio.wav", "transcript.json"] {
                let _ = std::fs::remove_file(cache.join(file));
            }
        }
        if analysis.options.local_only {
            return self.stage(
                id,
                control,
                "completed",
                if info.audios.is_empty() {
                    "本地检查完成：无音轨，可进行画面分析"
                } else {
                    "本地转录完成，未发送任何内容给AI"
                },
                1.,
            );
        }
        if !analysis.keyframes_done {
            self.stage(id, control, "extracting_keyframes", "正在分析画面", 0.38)?;
            let frames = if analysis.options.allow_images {
                super::keyframes::extract(
                    &self.resources,
                    source,
                    &cache,
                    id,
                    analysis.analysis_mode,
                    &info,
                    control,
                )?
            } else {
                vec![]
            };
            Self::check_source(source, &analysis.source_fingerprint)?;
            self.owned(id, control, |r| r.save_keyframes(id, &frames))?;
        }
        let own_client;
        let client = match client {
            Some(c) => c,
            None => {
                own_client = DeepSeekClient::new()?;
                &own_client
            }
        };
        tauri::async_runtime::block_on(async {
            self.stage(id, control, "analyzing_chunks", "正在理解视频内容", 0.5)?;
            let results = self.analyze_chunks(id, control, client).await?;
            self.stage(id, control, "finalizing", "正在整理全片总结", 0.9)?;
            let summary = self.summarize(id, results, control, client).await?;
            Self::check_source(source, &analysis.source_fingerprint)?;
            self.check_consent(&analysis.options)?;
            let json = serde_json::to_string(&summary).map_err(|_| "无法保存全片总结")?;
            self.owned(id, control, |r| {
                r.conn
                    .execute(
                        "UPDATE video_analyses SET summary_json=?2 WHERE id=?1",
                        params![id, json],
                    )
                    .map_err(db::error)?;
                Ok(())
            })?;
            self.stage(id, control, "completed", "视频分析完成", 1.)
        })
    }
    pub(crate) fn finish_analysis(
        self: &Arc<Self>,
        id: &str,
        control: &Arc<Control>,
        result: AppResult<()>,
    ) {
        let Ok(mut runtime) = self.runtime.lock() else {
            return;
        };
        if !runtime
            .active
            .as_ref()
            .is_some_and(|(active, c)| active == id && Arc::ptr_eq(c, control))
        {
            return;
        }
        let result = if runtime.exiting || control.check().is_err() {
            Err("分析已取消或中断".into())
        } else {
            result
        };
        let _=self.with(|r|{
            if let Err(error)=result{
                let status=if runtime.exiting{"interrupted"}else if control.check().is_err(){"cancelled"}else{"failed"};
                r.conn.execute("UPDATE video_analyses SET status=?2,stage=?3,error=?4,updated_at=?5 WHERE id=?1",params![id,status,if status=="interrupted"{"分析被中断，可继续分析"}else if status=="cancelled"{"分析已取消"}else{"分析失败，可重试"},if status=="failed"{Some(error)}else{None},crate::service::now()]).map_err(db::error)?;
            }else{r.conn.execute("UPDATE video_analyses SET completed_at=?2 WHERE id=?1",params![id,crate::service::now()]).map_err(db::error)?;}
            Ok(())
        });
        runtime.active = None;
        drop(runtime);
        self.emit_analysis(
            id,
            if self.analysis(id).is_ok_and(|a| a.status == "completed") {
                "ai-analysis-completed"
            } else {
                "ai-analysis-failed"
            },
        );
        self.kick_queue();
    }
    pub fn cancel_analysis(&self, id: &str) -> AppResult<()> {
        let mut runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
        if let Some((_, c)) = runtime.active.as_ref().filter(|(active, _)| active == id) {
            c.cancel();
        } else if let Some(index) = runtime.pending.iter().position(|p| p == id) {
            runtime.pending.remove(index);
        } else {
            return Err("该任务当前没有运行或排队".into());
        }
        self.with(|r|{r.conn.execute("UPDATE video_analyses SET status='cancelled',stage='分析已取消',updated_at=?2 WHERE id=?1",params![id,crate::service::now()]).map_err(db::error)?;Ok(())})?;
        drop(runtime);
        self.emit_analysis(id, "ai-analysis-updated");
        Ok(())
    }
    pub fn shutdown(&self) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.exiting = true;
            if let Some((_, control)) = &runtime.active {
                control.cancel();
            }
            let active = runtime.active.as_ref().map(|(id, _)| id.clone());
            let ids = runtime.pending.drain(..).chain(active).collect::<Vec<_>>();
            let _=self.with(|r|{for id in ids{r.conn.execute("UPDATE video_analyses SET status='interrupted',stage='分析被中断，可继续分析',updated_at=?2 WHERE id=?1",params![id,crate::service::now()]).map_err(db::error)?;}Ok(())});
        }
        if let Ok(mut models) = self.models.lock() {
            if let Some(task) = models.task.take() {
                task.abort();
            }
        }
    }
}
