use super::{models::*, service::LearningService};
use crate::domain::AppResult;
use std::sync::Arc;
use tauri::State;
type Learning<'a> = State<'a, Arc<LearningService>>;
#[tauri::command]
pub fn learning_home(
    learning: Learning<'_>,
    query: String,
    category: Option<String>,
    page: u32,
    favorite: bool,
) -> AppResult<LibraryHome> {
    learning.home(&query, category.as_deref(), page, favorite)
}
#[tauri::command]
pub fn learning_series(learning: Learning<'_>, id: String, page: u32) -> AppResult<SeriesDetail> {
    learning.series(&id, page)
}
#[tauri::command]
pub fn learning_item(learning: Learning<'_>, id: String) -> AppResult<LearningItem> {
    learning.with(|r| r.item(&id))
}
#[tauri::command]
pub fn learning_change(learning: Learning<'_>, change: LearningChange) -> AppResult<String> {
    learning.change(change)
}

#[tauri::command]
pub async fn learning_scan(learning: Learning<'_>, paths: Vec<String>) -> AppResult<LocalScan> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.scan(&paths))
        .await
        .map_err(|_| "导入扫描线程异常")?
}
#[tauri::command]
pub async fn learning_import(
    learning: Learning<'_>,
    input: LocalImportInput,
) -> AppResult<ImportResult> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.import_files(input))
        .await
        .map_err(|_| "导入线程异常")?
}
#[tauri::command]
pub async fn learning_relink(
    learning: Learning<'_>,
    id: String,
    path: String,
) -> AppResult<LearningItem> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.relink_file(&id, &path))
        .await
        .map_err(|_| "重新定位线程异常")?
}
#[tauri::command]
pub fn learning_delete_preview(
    learning: Learning<'_>,
    kind: String,
    id: String,
) -> AppResult<Vec<String>> {
    learning.delete_preview(&kind, &id)
}
#[tauri::command]
pub fn learning_open_file(learning: Learning<'_>, id: String) -> AppResult<()> {
    learning.open_file(&id)
}

#[tauri::command]
pub async fn learning_prepare_player(
    learning: Learning<'_>,
    item_id: String,
    session: String,
    proxy: bool,
) -> AppResult<PlayerMedia> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.prepare_player(&item_id, &session, proxy))
        .await
        .map_err(|_| "播放器准备线程异常")?
}
#[tauri::command]
pub async fn learning_load_subtitle(
    learning: Learning<'_>,
    path: String,
    session: String,
) -> AppResult<PlayerSubtitle> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.subtitle(std::path::Path::new(&path), &session))
        .await
        .map_err(|_| "字幕准备线程异常")?
}

#[tauri::command]
pub fn learning_live_progress(learning: Learning<'_>, input: ProgressInput) -> AppResult<()> {
    learning.live_progress(input, false)
}
#[tauri::command]
pub fn learning_save_progress(
    learning: Learning<'_>,
    input: ProgressInput,
) -> AppResult<PlaybackProgress> {
    let id = input.item_id.clone();
    learning.live_progress(input, true)?;
    learning.with(|r| Ok(r.item(&id)?.progress))
}
#[tauri::command]
pub fn learning_flush_progress(learning: Learning<'_>) -> AppResult<()> {
    learning.flush_progress()
}

#[tauri::command]
pub fn learning_notes(learning: Learning<'_>, item_id: String, page: u32) -> AppResult<NotePage> {
    learning.with(|r| r.notes(&item_id, page))
}
#[tauri::command]
pub fn learning_save_note(learning: Learning<'_>, input: NoteInput) -> AppResult<String> {
    let id = learning.with(|r| r.save_note(&input))?;
    learning.changed("note", &id);
    Ok(id)
}

#[tauri::command]
pub async fn learning_enqueue_downloads(
    learning: Learning<'_>,
    service: State<'_, Arc<crate::service::Service>>,
    requests: Vec<crate::models::DownloadRequest>,
    imports: Vec<PlaylistImport>,
) -> AppResult<LearningEnqueueResult> {
    let l = learning.inner().clone();
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.enqueue_playlists(&s, requests, imports))
        .await
        .map_err(|_| "播放列表入队线程异常")?
}
#[tauri::command]
pub async fn learning_download_item(
    learning: Learning<'_>,
    service: State<'_, Arc<crate::service::Service>>,
    id: String,
) -> AppResult<()> {
    let l = learning.inner().clone();
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.download_item(&s, &id))
        .await
        .map_err(|_| "课程下载线程异常")?
}

#[tauri::command]
pub async fn learning_check_updates(
    learning: Learning<'_>,
    service: State<'_, Arc<crate::service::Service>>,
    series_id: String,
) -> AppResult<SeriesUpdate> {
    let l = learning.inner().clone();
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.check_updates(&s, &series_id))
        .await
        .map_err(|_| "系列检查线程异常")?
}
