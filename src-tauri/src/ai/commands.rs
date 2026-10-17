use super::{models::*, service::AiService};
use crate::domain::AppResult;
use std::sync::Arc;
use tauri::State;
type Ai<'a> = State<'a, Arc<AiService>>;
#[tauri::command]
pub async fn ai_list_analyses(
    ai: Ai<'_>,
    status: Option<String>,
    page: u32,
) -> AppResult<AnalysisPage> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.list(status.as_deref(), page))
        .await
        .map_err(|_| "读取分析记录失败")?
}
#[tauri::command]
pub async fn ai_analysis(ai: Ai<'_>, id: String) -> AppResult<VideoAnalysis> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.analysis(&id))
        .await
        .map_err(|_| "读取分析记录失败")?
}
