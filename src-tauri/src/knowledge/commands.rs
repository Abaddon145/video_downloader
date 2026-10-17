use super::{models::KnowledgeSettings, service::KnowledgeService};
use crate::domain::AppResult;
use std::sync::Arc;
#[tauri::command]
pub fn knowledge_get_settings(
    knowledge: tauri::State<'_, Arc<KnowledgeService>>,
) -> AppResult<KnowledgeSettings> {
    knowledge.settings()
}
