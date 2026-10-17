use super::{
    models::*,
    service::KnowledgeService,
    vault::{Document, TreePage},
};
use crate::domain::AppResult;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
type Knowledge<'a> = State<'a, Arc<KnowledgeService>>;
#[tauri::command]
pub fn knowledge_default_path(app: tauri::AppHandle) -> AppResult<String> {
    Ok(app
        .path()
        .document_dir()
        .map_err(|_| "无法找到系统文档目录，请自行选择文件夹")?
        .join("映流知识库")
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
pub async fn knowledge_get_settings(knowledge: Knowledge<'_>) -> AppResult<KnowledgeSettings> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.settings())
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_set_vault(
    knowledge: Knowledge<'_>,
    path: String,
    create: bool,
) -> AppResult<KnowledgeSettings> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.set_vault(&path, create))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_tree(
    knowledge: Knowledge<'_>,
    vault_id: String,
    parent: String,
    page: u32,
) -> AppResult<KnowledgeView<TreePage>> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.tree(&vault_id, &parent, page))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_read(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
) -> AppResult<KnowledgeView<Document>> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.read(&vault_id, &path))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_write(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
    path: String,
    text: String,
    expected_hash: String,
) -> AppResult<KnowledgeWrite> {
    let k = knowledge.inner().clone();
    let event_id = path.clone();
    let event_vault = vault_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        k.write(&vault_id, &path, &text, &expected_hash)
    })
    .await
    .map_err(|_| "知识库后台操作未完成，请重试")?;
    if result.is_ok() {
        let _ = app.emit(
            "knowledge-file-changed",
            serde_json::json!({"id":event_id,"vaultId":event_vault,"kind":"write"}),
        );
    }
    result
}

#[tauri::command]
pub async fn knowledge_create(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
    path: String,
    text: String,
    directory: bool,
) -> AppResult<KnowledgeMutation> {
    let k = knowledge.inner().clone();
    let event_id = path.clone();
    let event_vault = vault_id.clone();
    let result =
        tauri::async_runtime::spawn_blocking(move || k.create(&vault_id, &path, &text, directory))
            .await
            .map_err(|_| "知识库后台操作未完成，请重试")?;
    if result.is_ok() {
        let _ = app.emit(
            "knowledge-file-changed",
            serde_json::json!({"id":event_id,"vaultId":event_vault,"kind":"create"}),
        );
    }
    result
}

#[tauri::command]
pub async fn knowledge_rename(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
    path: String,
    to: String,
) -> AppResult<KnowledgeMutation> {
    let k = knowledge.inner().clone();
    let event_id = path.clone();
    let event_vault = vault_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.move_note(&vault_id, &path, &to))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    if result.is_ok() {
        let _ = app.emit(
            "knowledge-file-changed",
            serde_json::json!({"id":event_id,"vaultId":event_vault,"kind":"rename"}),
        );
    }
    result
}

#[tauri::command]
pub async fn knowledge_move(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
    from: String,
    to: String,
) -> AppResult<KnowledgeMutation> {
    let k = knowledge.inner().clone();
    let event_id = from.clone();
    let event_vault = vault_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.move_note(&vault_id, &from, &to))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    if result.is_ok() {
        let _ = app.emit(
            "knowledge-file-changed",
            serde_json::json!({"id":event_id,"vaultId":event_vault,"kind":"move"}),
        );
    }
    result
}

#[tauri::command]
pub async fn knowledge_delete(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
    path: String,
    expected_hash: String,
) -> AppResult<KnowledgeMutation> {
    let k = knowledge.inner().clone();
    let event_id = path.clone();
    let event_vault = vault_id.clone();
    let result =
        tauri::async_runtime::spawn_blocking(move || k.delete(&vault_id, &path, &expected_hash))
            .await
            .map_err(|_| "知识库后台操作未完成，请重试")?;
    if result.is_ok() {
        let _ = app.emit(
            "knowledge-file-changed",
            serde_json::json!({"id":event_id,"vaultId":event_vault,"kind":"delete"}),
        );
    }
    result
}

#[tauri::command]
pub async fn knowledge_search(
    knowledge: Knowledge<'_>,
    vault_id: String,
    query: String,
    page: u32,
    tag: Option<String>,
) -> AppResult<KnowledgeView<super::search::SearchPage>> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        k.search(&vault_id, &query, page, tag.as_deref())
    })
    .await
    .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_backlinks(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
    page: u32,
) -> AppResult<KnowledgeView<super::links::BacklinkPage>> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.backlinks(&vault_id, &path, page))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_resolve(
    knowledge: Knowledge<'_>,
    vault_id: String,
    source: String,
    target: String,
) -> AppResult<KnowledgeView<Vec<super::links::NoteCandidate>>> {
    let k = knowledge.inner().clone();
    let result =
        tauri::async_runtime::spawn_blocking(move || k.resolve(&vault_id, &source, &target))
            .await
            .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_tags(
    knowledge: Knowledge<'_>,
    vault_id: String,
) -> AppResult<KnowledgeView<Vec<String>>> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.tags(&vault_id))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_reindex(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
) -> AppResult<KnowledgeView<IndexStats>> {
    let k = knowledge.inner().clone();
    let event_id = vault_id.clone();
    let event_vault = vault_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.reindex(&vault_id))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    if result.is_ok() {
        let _ = app.emit(
            "knowledge-index-updated",
            serde_json::json!({"id":event_id,"vaultId":event_vault,"kind":"index"}),
        );
    }
    result
}

#[tauri::command]
pub async fn knowledge_asset(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
) -> AppResult<KnowledgeView<KnowledgeAsset>> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.asset(&vault_id, &path))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_open_in_obsidian(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
) -> AppResult<String> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.open_in_obsidian(&vault_id, &path))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}
