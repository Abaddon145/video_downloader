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
