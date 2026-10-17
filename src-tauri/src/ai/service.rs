use super::{models::*, repository::Repository};
use crate::domain::AppResult;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
pub struct AiService {
    pub(crate) db: Mutex<AppResult<Repository>>,
    pub(crate) data: PathBuf,
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
