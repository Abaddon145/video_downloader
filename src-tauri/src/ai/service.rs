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
    pub(crate) app: Option<tauri::AppHandle>,
    pub(crate) resources: PathBuf,
    pub(crate) runtime: Mutex<super::jobs::AiRuntime>,
    pub(crate) models: Mutex<super::whisper_models::ModelRuntime>,
    pub(crate) question_busy: std::sync::atomic::AtomicBool,
}
impl AiService {
    pub fn new(app: tauri::AppHandle, data: &Path, resources: &Path) -> Arc<Self> {
        Self::create(Some(app), data, resources)
    }
    pub(crate) fn create(
        app: Option<tauri::AppHandle>,
        data: &Path,
        resources: &Path,
    ) -> Arc<Self> {
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
            question_busy: std::sync::atomic::AtomicBool::new(false),
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
    pub(crate) fn emit<T: serde::Serialize + Clone>(&self, event: &str, payload: T) {
        use tauri::Emitter;
        if let Some(app) = &self.app {
            let _ = app.emit(event, payload);
        }
    }
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
    pub(crate) fn owned(
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
        if let Ok(analysis) = self.analysis(id) {
            self.emit(event, analysis);
        }
    }
    pub(crate) fn check_source(source: &Path, expected: &str) -> AppResult<()> {
        if super::transcription::fingerprint(source)? != expected {
            Err("源视频在分析过程中发生变化，请重新开始分析".into())
        } else {
            Ok(())
        }
    }
}

impl AiService {
    pub fn library_videos(&self, query: &str, page: u32) -> AppResult<LibraryVideoPage> {
        let query = crate::learning::repository::text(query, 200, false)?;
        self.with(|r|{
   let filter="(?1='' OR instr(lower(i.title||' '||s.title),lower(?1))>0)";
   let total:u64=r.conn.query_row(&format!("SELECT count(*) FROM series_items i JOIN series s ON s.id=i.series_id WHERE {filter}"),[&query],|r|r.get(0)).map_err(crate::learning::db::error)?;
   let page=page.max(1).min(((total+49)/50).max(1) as u32);
   let mut stmt=r.conn.prepare(&format!("SELECT i.id,i.title,s.title,i.file_path,i.duration_ms FROM series_items i JOIN series s ON s.id=i.series_id WHERE {filter} ORDER BY s.sort_order,s.created_at DESC,s.id,i.sort_order,i.id LIMIT 50 OFFSET ?2")).map_err(crate::learning::db::error)?;
   let items=stmt.query_map(rusqlite::params![query,(page-1)*50],|row|{let path:Option<String>=row.get(3)?;let available=path.as_deref().is_some_and(|path|crate::learning::importer::regular_file(path).is_ok());Ok(LibraryVideo{item_id:row.get(0)?,title:row.get(1)?,series_title:row.get(2)?,path,duration_ms:row.get(4)?,available})}).map_err(crate::learning::db::error)?.collect::<Result<Vec<_>,_>>().map_err(crate::learning::db::error)?;
   Ok(LibraryVideoPage{items,total,page})
  })
    }
}
