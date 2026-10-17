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

#[tauri::command]
pub async fn ai_whisper_models(ai: Ai<'_>) -> AppResult<Vec<super::whisper_models::ModelView>> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.whisper_models())
        .await
        .map_err(|_| "读取语音模型失败")?
}
#[tauri::command]
pub fn ai_model_download(ai: Ai<'_>) -> AppResult<Option<super::whisper_models::ModelDownload>> {
    ai.model_download()
}
#[tauri::command]
pub fn ai_download_whisper_model(
    ai: Ai<'_>,
    model_id: String,
) -> AppResult<super::whisper_models::ModelDownload> {
    ai.inner().download_model(&model_id)
}
#[tauri::command]
pub fn ai_cancel_model_download(ai: Ai<'_>, id: String) -> AppResult<()> {
    ai.cancel_model_download(&id)
}
#[tauri::command]
pub async fn ai_import_whisper_model(
    ai: Ai<'_>,
    path: String,
    expected_sha256: Option<String>,
) -> AppResult<String> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.import_model(&path, expected_sha256.as_deref()))
        .await
        .map_err(|_| "导入语音模型未完成")?
}
#[tauri::command]
pub async fn ai_start_transcription(
    ai: Ai<'_>,
    path: String,
    item_id: Option<String>,
) -> AppResult<VideoAnalysis> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.start_transcription(&path, item_id))
        .await
        .map_err(|_| "无法启动本地转录")?
}
#[tauri::command]
pub fn ai_cancel_analysis(ai: Ai<'_>, id: String) -> AppResult<()> {
    ai.cancel_analysis(&id)
}
#[tauri::command]
pub async fn ai_transcript(ai: Ai<'_>, id: String, page: u32) -> AppResult<TranscriptPage> {
    let ai = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || ai.with(|r| r.transcript(&id, page)))
        .await
        .map_err(|_| "读取转录失败")?
}
