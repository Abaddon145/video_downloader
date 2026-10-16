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
    support: Option<super::compatibility::BrowserPlaybackSupport>,
    fallback: Option<u8>,
    force: Option<bool>,
    native: Option<bool>,
) -> AppResult<PlayerMedia> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        l.prepare_player(
            &item_id,
            &session,
            proxy,
            support.unwrap_or_default(),
            fallback.unwrap_or(0),
            force.unwrap_or(false),
            native.unwrap_or(false),
        )
    })
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

#[tauri::command]
pub fn learning_cancel_player(learning: Learning<'_>, session: String) -> AppResult<()> {
    learning.cancel_player(&session)
}
#[tauri::command]
pub async fn learning_cache_info(learning: Learning<'_>) -> AppResult<super::cache::CacheInfo> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.cache_info())
        .await
        .map_err(|_| "缓存检查线程异常")?
}
#[tauri::command]
pub async fn learning_clean_cache(
    learning: Learning<'_>,
    kind: String,
) -> AppResult<super::cache::CacheInfo> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.clean_cache(&kind, true))
        .await
        .map_err(|_| "缓存清理线程异常")?
}

#[tauri::command]
pub async fn learning_set_thumbnail(
    learning: Learning<'_>,
    id: String,
    path: String,
) -> AppResult<LearningItem> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.set_thumbnail(&id, &path))
        .await
        .map_err(|_| "图片设置线程异常")?
}
#[tauri::command]
pub async fn learning_regenerate_thumbnail(
    learning: Learning<'_>,
    id: String,
    confirmed: bool,
) -> AppResult<()> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.regenerate_thumbnail(&id, confirmed))
        .await
        .map_err(|_| "图片生成线程异常")?
}

use super::native_player::{NativeAction, NativeSnapshot, NativeViewport};
#[tauri::command]
pub async fn learning_native_start(
    learning: Learning<'_>,
    item_id: String,
    session: String,
    viewport: NativeViewport,
    autoplay: bool,
) -> AppResult<NativeSnapshot> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        l.native_start(&item_id, &session, viewport, autoplay)
    })
    .await
    .map_err(|_| "播放线程异常")?
}
#[tauri::command]
pub async fn learning_native_control(
    learning: Learning<'_>,
    session: String,
    action: NativeAction,
) -> AppResult<NativeSnapshot> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.native_control(&session, action))
        .await
        .map_err(|_| "播放控制线程异常")?
}
#[tauri::command]
pub fn learning_native_viewport(
    learning: Learning<'_>,
    session: String,
    viewport: NativeViewport,
) -> AppResult<()> {
    learning.native_viewport(&session, viewport)
}
#[tauri::command]
pub async fn learning_native_save_progress(
    learning: Learning<'_>,
    session: String,
    ended: Option<bool>,
) -> AppResult<PlaybackProgress> {
    let l = learning.inner().clone();
    tauri::async_runtime::spawn_blocking(move || l.native_save(&session, ended.unwrap_or(false)))
        .await
        .map_err(|_| "播放保存线程异常")?
}
