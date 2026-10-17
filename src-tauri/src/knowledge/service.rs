use super::models::KnowledgeSettings;
use crate::domain::AppResult;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
pub struct KnowledgeService {
    pub(crate) settings: Mutex<AppResult<KnowledgeSettings>>,
}
impl KnowledgeService {
    pub fn new(data: &Path) -> Arc<Self> {
        let file = data.join("knowledge-settings.json");
        let settings = if file.exists() {
            std::fs::read(&file)
                .map_err(|_| "无法读取知识库设置".into())
                .and_then(|b| {
                    serde_json::from_slice(&b)
                        .map_err(|_| "知识库设置格式错误，请保留原文件".into())
                })
        } else {
            Ok(KnowledgeSettings::empty())
        };
        Arc::new(Self {
            settings: Mutex::new(settings),
        })
    }
    pub fn settings(&self) -> AppResult<KnowledgeSettings> {
        self.settings
            .lock()
            .map_err(|_| "知识库暂时不可用")?
            .clone()
    }
}
