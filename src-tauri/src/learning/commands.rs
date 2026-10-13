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
