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

#[tauri::command]
pub async fn ai_get_settings(ai: Ai<'_>) -> AppResult<super::settings::AiSettingsView> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.settings())
        .await
        .map_err(|_| "读取AI设置失败")?
}
#[tauri::command]
pub async fn ai_save_settings(
    ai: Ai<'_>,
    settings: super::settings::AiSettings,
) -> AppResult<super::settings::AiSettingsView> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.save_settings(settings))
        .await
        .map_err(|_| "保存AI设置失败")?
}
#[tauri::command]
pub async fn ai_set_deepseek_key(ai: Ai<'_>) -> AppResult<bool> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.set_key())
        .await
        .map_err(|_| "密钥录入未完成")?
}
#[tauri::command]
pub async fn ai_clear_deepseek_key(ai: Ai<'_>) -> AppResult<()> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.clear_key())
        .await
        .map_err(|_| "密钥删除未完成")?
}
#[tauri::command]
pub async fn ai_test_deepseek(ai: Ai<'_>) -> AppResult<String> {
    ai.test_connection().await
}
