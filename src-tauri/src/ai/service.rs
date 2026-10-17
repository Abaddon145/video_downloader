use super::{models::*, repository::Repository};
use crate::domain::AppResult;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
pub struct AiService {
    pub(crate) db: Mutex<AppResult<Repository>>,
    pub(crate) data: PathBuf,
    pub(crate) settings: Mutex<AppResult<super::settings::AiSettings>>,
    pub(crate) key_guard: Mutex<()>,
    pub(crate) app: tauri::AppHandle,
    pub(crate) resources: PathBuf,
    pub(crate) runtime: Mutex<super::jobs::AiRuntime>,
    pub(crate) models: Mutex<super::whisper_models::ModelRuntime>,
}
impl AiService {
    pub fn new(app: tauri::AppHandle, data: &Path, resources: &Path) -> Arc<Self> {
        let db = Repository::open(&data.join("learning.db")).and_then(|r| {
            r.recover()?;
            Ok(r)
        });
        Arc::new(Self {
            db: Mutex::new(db),
            data: data.into(),
            settings: Mutex::new(super::settings::load_settings(data)),
            key_guard: Mutex::new(()),
            app,
            resources: resources.into(),
            runtime: Mutex::new(Default::default()),
            models: Mutex::new(Default::default()),
        })
    }
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut Repository) -> AppResult<R>) -> AppResult<R> {
        let mut db = self.db.lock().map_err(|_| "AI数据库暂时不可用")?;
        if db.is_err() {
            *db = Repository::open(&self.data.join("learning.db")).and_then(|r| {
                r.recover()?;
                Ok(r)
            });
        }
        f(db.as_mut()
            .map_err(|e| format!("AI数据库初始化失败：{e}"))?)
    }
    pub fn list(&self, status: Option<&str>, page: u32) -> AppResult<AnalysisPage> {
        self.with(|r| r.list(status, page))
    }
    pub fn analysis(&self, id: &str) -> AppResult<VideoAnalysis> {
        self.with(|r| r.get(id))
    }
}

impl AiService {
    pub fn settings(&self) -> AppResult<super::settings::AiSettingsView> {
        let settings = self
            .settings
            .lock()
            .map_err(|_| "AI设置暂时不可用")?
            .clone()?;
        let _key = self.key_guard.lock().map_err(|_| "密钥暂时不可用")?;
        Ok(super::settings::AiSettingsView {
            settings,
            has_api_key: super::settings::load_key(&self.data).is_ok(),
        })
    }
    pub fn save_settings(
        &self,
        settings: super::settings::AiSettings,
    ) -> AppResult<super::settings::AiSettingsView> {
        {
            let mut s = self.settings.lock().map_err(|_| "AI设置暂时不可用")?;
            super::settings::save_settings(&self.data, &settings)?;
            *s = Ok(settings);
        }
        self.settings()
    }
    pub fn set_key(&self) -> AppResult<bool> {
        let _guard = self.key_guard.lock().map_err(|_| "密钥暂时不可用")?;
        if let Some(key) = super::settings::prompt_key()? {
            super::settings::store_key(&self.data, key.as_bytes())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn clear_key(&self) -> AppResult<()> {
        let _guard = self.key_guard.lock().map_err(|_| "密钥暂时不可用")?;
        super::settings::clear_key(&self.data)
    }
    pub(crate) fn key(&self) -> AppResult<super::settings::Secret> {
        let _guard = self.key_guard.lock().map_err(|_| "密钥暂时不可用")?;
        super::settings::load_key(&self.data)
    }
    pub async fn test_connection(&self) -> AppResult<String> {
        let settings = self.settings()?.settings;
        let key = self.key()?;
        super::client::DeepSeekClient::new()?
            .test_connection(key.as_bytes(), &settings.model)
            .await
    }
}

impl AiService {
    pub fn start_transcription(
        self: &Arc<Self>,
        path: &str,
        item_id: Option<String>,
    ) -> AppResult<VideoAnalysis> {
        use super::transcription as t;
        let source = crate::learning::importer::regular_file(path)?;
        let fingerprint = t::fingerprint(&source)?;
        let settings = self.settings()?.settings;
        let id = crate::ffmpeg::runner::new_id().replace("media-", "ai-");
        let mut runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
        if runtime.exiting {
            return Err("软件正在退出".into());
        }
        if runtime.active.is_some() {
            return Err("已有分析任务运行中，请先完成或取消".into());
        }
        self.with(|r|{
   if let Some(item)=&item_id{
    let saved:Option<String>=r.conn.query_row("SELECT file_path FROM series_items WHERE id=?1",[item],|row|row.get(0)).map_err(|_|"关联课程不存在")?;
    if saved.as_deref().and_then(|p|crate::learning::importer::regular_file(p).ok()).as_ref()!=Some(&source){return Err("关联课程与源文件不一致".into());}
   }
   let now=crate::service::now();r.conn.execute("INSERT INTO video_analyses(id,item_id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,created_at,updated_at) VALUES(?1,?2,?3,?4,'queued',?5,?6,?7,?8,?8)",rusqlite::params![id,item_id,source.to_string_lossy(),fingerprint,settings.default_mode.as_str(),settings.model,super::prompts::ANALYSIS_PROMPT_VERSION,now]).map_err(crate::learning::db::error)?;Ok(())
  })?;
        let control = Arc::new(super::jobs::Control::default());
        runtime.active = Some((id.clone(), control.clone()));
        drop(runtime);
        let service = self.clone();
        let task_id = id.clone();
        let started = std::thread::Builder::new()
            .name("yingliu-local-transcription".into())
            .spawn(move || {
                let result =
                    service.run_transcription(&task_id, &source, &fingerprint, &settings, &control);
                service.finish_local(&task_id, &control, result);
            });
        if started.is_err() {
            if let Ok(mut runtime) = self.runtime.lock() {
                runtime.active = None;
            }
            self.with(|r|{r.conn.execute("UPDATE video_analyses SET status='failed',error='无法启动本地分析线程' WHERE id=?1",[&id]).map_err(crate::learning::db::error)?;Ok(())})?;
            return Err("无法启动本地分析线程".into());
        }
        self.analysis(&id)
    }
    fn owned(
        &self,
        id: &str,
        control: &Arc<super::jobs::Control>,
        f: impl FnOnce(&mut super::repository::Repository) -> AppResult<()>,
    ) -> AppResult<()> {
        let runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
        if runtime.exiting
            || !runtime
                .active
                .as_ref()
                .is_some_and(|(active, c)| active == id && Arc::ptr_eq(c, control))
        {
            return Err("分析已取消或中断".into());
        }
        control.check()?;
        self.with(f)
    }
    pub(crate) fn stage(
        &self,
        id: &str,
        control: &Arc<super::jobs::Control>,
        status: &str,
        stage: &str,
        progress: f64,
    ) -> AppResult<()> {
        self.owned(id,control,|r|{r.conn.execute("UPDATE video_analyses SET status=?2,stage=?3,progress=?4,updated_at=?5,error=NULL WHERE id=?1",rusqlite::params![id,status,stage,progress,crate::service::now()]).map_err(crate::learning::db::error)?;Ok(())})?;
        self.emit_analysis(id, "ai-analysis-updated");
        Ok(())
    }
    pub(crate) fn emit_analysis(&self, id: &str, event: &str) {
        use tauri::Emitter;
        if let Ok(analysis) = self.analysis(id) {
            let _ = self.app.emit(event, analysis);
        }
    }
    fn check_source(source: &Path, expected: &str) -> AppResult<()> {
        if super::transcription::fingerprint(source)? != expected {
            Err("源视频在分析过程中发生变化，请重新开始分析".into())
        } else {
            Ok(())
        }
    }
    fn run_transcription(
        &self,
        id: &str,
        source: &Path,
        expected: &str,
        settings: &super::settings::AiSettings,
        control: &Arc<super::jobs::Control>,
    ) -> AppResult<()> {
        use super::transcription as t;
        let _source = t::lock_source(source)?;
        Self::check_source(source, expected)?;
        self.stage(id, control, "extracting_audio", "读取媒体信息", 0.02)?;
        let info = t::probe_media(&self.resources, source, control)?;
        let duration = info
            .duration
            .filter(|d| d.is_finite() && *d > 0. && *d <= 7. * 24. * 3600.)
            .ok_or("无法确定媒体时长或时长超出7天限制")?;
        let duration_ms = (duration * 1000.).round() as i64;
        self.owned(id, control, |r| {
            r.conn
                .execute(
                    "UPDATE video_analyses SET duration_ms=?2 WHERE id=?1",
                    rusqlite::params![id, duration_ms],
                )
                .map_err(crate::learning::db::error)?;
            Ok(())
        })?;
        let cache = self.data.join("ai/cache").join(id);
        self.stage(
            id,
            control,
            "extracting_audio",
            if info.audios.is_empty() {
                "没有音轨，可使用画面分析"
            } else {
                "正在提取临时音频"
            },
            0.08,
        )?;
        let (language, segments) = if info.audios.is_empty() {
            (None, Vec::new())
        } else {
            t::verify_model(&self.data, &settings.whisper_model)?;
            t::whisper_binary(&self.resources)?;
            t::extract_audio(&self.resources, source, &cache, &info, control)?;
            Self::check_source(source, expected)?;
            self.stage(id, control, "transcribing", "正在本地识别语音", 0.2)?;
            t::transcribe(
                &self.resources,
                &self.data,
                &cache,
                &settings.whisper_model,
                id,
                duration_ms,
                control,
            )?
        };
        Self::check_source(source, expected)?;
        self.owned(id, control, |r| {
            r.save_transcript(id, language.as_deref(), &segments)
        })?;
        if settings.clean_temporary_files {
            let _ = std::fs::remove_file(cache.join("audio.wav"));
            let _ = std::fs::remove_file(cache.join("transcript.json"));
        }
        self.stage(
            id,
            control,
            "completed",
            if info.audios.is_empty() {
                "本地检查完成：无音轨，可进行画面分析"
            } else {
                "本地转录完成，未发送任何内容给AI"
            },
            1.,
        )
    }
    fn finish_local(&self, id: &str, control: &Arc<super::jobs::Control>, result: AppResult<()>) {
        if let Ok(mut runtime) = self.runtime.lock() {
            if !runtime
                .active
                .as_ref()
                .is_some_and(|(active, c)| active == id && Arc::ptr_eq(c, control))
            {
                return;
            }
            if let Err(error) = result {
                let status = if runtime.exiting {
                    "interrupted"
                } else if control.check().is_err() {
                    "cancelled"
                } else {
                    "failed"
                };
                let _=self.with(|r|{r.conn.execute("UPDATE video_analyses SET status=?2,stage=?3,error=?4,updated_at=?5 WHERE id=?1",rusqlite::params![id,status,if status=="interrupted"{"分析被中断，可继续分析"}else if status=="cancelled"{"分析已取消"}else{"分析失败，可重试"},if status=="failed"{Some(error)}else{None},crate::service::now()]).map_err(crate::learning::db::error)?;Ok(())});
            } else {
                let _ = self.with(|r| {
                    r.conn
                        .execute(
                            "UPDATE video_analyses SET completed_at=?2 WHERE id=?1",
                            rusqlite::params![id, crate::service::now()],
                        )
                        .map_err(crate::learning::db::error)?;
                    Ok(())
                });
            }
            runtime.active = None;
        }
        let event = if self.analysis(id).is_ok_and(|a| a.status == "completed") {
            "ai-analysis-completed"
        } else {
            "ai-analysis-failed"
        };
        self.emit_analysis(id, event);
    }
    pub fn cancel_analysis(&self, id: &str) -> AppResult<()> {
        let runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
        let (active, control) = runtime
            .active
            .as_ref()
            .filter(|(active, _)| active == id)
            .ok_or("该任务当前没有运行")?;
        control.cancel();
        self.with(|r|{r.conn.execute("UPDATE video_analyses SET status='cancelled',stage='正在结束本地处理',updated_at=?2 WHERE id=?1",rusqlite::params![active,crate::service::now()]).map_err(crate::learning::db::error)?;Ok(())})?;
        drop(runtime);
        self.emit_analysis(id, "ai-analysis-updated");
        Ok(())
    }
    pub fn shutdown(&self) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.exiting = true;
            if let Some((id, control)) = &runtime.active {
                control.cancel();
                let _=self.with(|r|{r.conn.execute("UPDATE video_analyses SET status='interrupted',stage='分析被中断，可继续分析',updated_at=?2 WHERE id=?1",rusqlite::params![id,crate::service::now()]).map_err(crate::learning::db::error)?;Ok(())});
            }
        }
        if let Ok(mut models) = self.models.lock() {
            if let Some(task) = models.task.take() {
                task.abort();
            }
        }
    }
}
