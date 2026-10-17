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
}
impl AiService {
    pub fn new(data: &Path) -> Arc<Self> {
        let db = Repository::open(&data.join("learning.db")).and_then(|r| {
            r.recover()?;
            Ok(r)
        });
        Arc::new(Self {
            db: Mutex::new(db),
            data: data.into(),
            settings: Mutex::new(super::settings::load_settings(data)),
            key_guard: Mutex::new(()),
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
