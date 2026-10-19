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
pub fn knowledge_set_pending_edits(knowledge: Knowledge<'_>, pending: bool) {
    knowledge.set_pending_edits(pending);
}

#[tauri::command]
pub async fn knowledge_import_attachment(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
) -> AppResult<Option<KnowledgeView<NoteAttachment>>> {
    use tauri_plugin_dialog::DialogExt;
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(source) = app.dialog().file().blocking_pick_file() else {
            return Ok(None);
        };
        let source = source.into_path().map_err(|_| "请选择本地附件文件")?;
        let (name, bytes) = super::attachments::read_selected_attachment(&source)?;
        k.import_attachment(&vault_id, &name, &bytes).map(Some)
    })
    .await
    .map_err(|_| "导入附件未完成")?
}

#[tauri::command]
pub async fn knowledge_attachments(
    knowledge: Knowledge<'_>,
    vault_id: String,
    page: u32,
) -> AppResult<KnowledgeView<NoteAttachmentPage>> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || k.attachments(&vault_id, page))
        .await
        .map_err(|_| "读取附件列表未完成")?
}

#[tauri::command]
pub async fn knowledge_delete_attachment(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
    expected_hash: String,
) -> AppResult<KnowledgeMutation> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        k.delete_attachment(&vault_id, &path, &expected_hash)
    })
    .await
    .map_err(|_| "回收附件未完成")?
}

#[tauri::command]
pub async fn knowledge_export_attachment(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
    path: String,
) -> AppResult<bool> {
    use std::io::Write;
    use tauri_plugin_dialog::DialogExt;
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let name = k.attachment_name(&vault_id, &path)?;
        let bytes = k.attachment_bytes(&vault_id, &path)?;
        let Some(target) = app
            .dialog()
            .file()
            .set_file_name(&name)
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let target = target.into_path().map_err(|_| "请选择本地附件保存路径")?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|_| "无法创建导出文件；目标可能已存在，请使用新的文件名")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "附件导出写入未完成，请检查目标磁盘")?;
        Ok(true)
    })
    .await
    .map_err(|_| "导出附件未完成")?
}

#[tauri::command]
pub async fn knowledge_note_metadata(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
) -> AppResult<KnowledgeView<NoteMetadata>> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || k.note_metadata(&vault_id, &path))
        .await
        .map_err(|_| "读取笔记标记未完成")?
}
#[tauri::command]
pub async fn knowledge_set_note_metadata(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
    favorite: bool,
    template: bool,
) -> AppResult<KnowledgeView<NoteMetadata>> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        k.set_note_metadata(&vault_id, &path, favorite, template)
    })
    .await
    .map_err(|_| "保存笔记标记未完成")?
}
#[tauri::command]
pub async fn knowledge_note_collection(
    knowledge: Knowledge<'_>,
    vault_id: String,
    kind: String,
    page: u32,
) -> AppResult<KnowledgeView<NoteMetadataPage>> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || k.note_collection(&vault_id, &kind, page))
        .await
        .map_err(|_| "读取笔记集合未完成")?
}
#[tauri::command]
pub async fn knowledge_note_versions(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
    page: u32,
) -> AppResult<KnowledgeView<NoteVersionPage>> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || k.note_versions(&vault_id, &path, page))
        .await
        .map_err(|_| "读取历史列表未完成")?
}
#[tauri::command]
pub async fn knowledge_note_version(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
    version_id: String,
) -> AppResult<KnowledgeView<NoteVersionContent>> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || k.note_version(&vault_id, &path, &version_id))
        .await
        .map_err(|_| "读取历史版本未完成")?
}
#[tauri::command]
pub async fn knowledge_restore_note_version(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
    path: String,
    version_id: String,
    expected_hash: String,
) -> AppResult<KnowledgeWrite> {
    let k = knowledge.inner().clone();
    let event_vault = vault_id.clone();
    let event_path = path.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        k.restore_note_version(&vault_id, &path, &version_id, &expected_hash)
    })
    .await
    .map_err(|_| "恢复历史版本未完成")??;
    if matches!(&result.outcome, super::vault::WriteOutcome::Saved(_)) {
        let _ = app.emit(
            "knowledge-file-changed",
            serde_json::json!({"vaultId":event_vault,"id":event_path,"kind":"write"}),
        );
    }
    Ok(result)
}
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
    sort: Option<String>,
) -> AppResult<KnowledgeView<TreePage>> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        k.tree_sorted(&vault_id, &parent, page, sort.as_deref().unwrap_or("name"))
    })
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
pub async fn knowledge_delete_empty_directory(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
    path: String,
) -> AppResult<KnowledgeMutation> {
    let k = knowledge.inner().clone();
    let event_id = path.clone();
    let event_vault = vault_id.clone();
    let result =
        tauri::async_runtime::spawn_blocking(move || k.delete_empty_directory(&vault_id, &path))
            .await
            .map_err(|_| "删除空笔记本未完成")?;
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
) -> AppResult<ObsidianOpenResult> {
    let k = knowledge.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || k.open_in_obsidian(&vault_id, &path))
        .await
        .map_err(|_| "知识库后台操作未完成，请重试")?;
    result
}

#[tauri::command]
pub async fn knowledge_obsidian_connection(
    knowledge: Knowledge<'_>,
    vault_id: String,
) -> AppResult<ObsidianConnection> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || k.obsidian_connection(&vault_id))
        .await
        .map_err(|_| "Obsidian 状态检查未完成")?
}

#[tauri::command]
pub async fn knowledge_configure_obsidian(
    knowledge: Knowledge<'_>,
    vault_id: String,
    kind: ObsidianKind,
    identifier: Option<String>,
    registration_confirmed: bool,
) -> AppResult<KnowledgeSettings> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        k.configure_obsidian(&vault_id, kind, identifier, registration_confirmed)
    })
    .await
    .map_err(|_| "Obsidian 设置未保存")?
}

#[tauri::command]
pub async fn knowledge_confirm_obsidian(
    knowledge: Knowledge<'_>,
    vault_id: String,
    expected: ObsidianBinding,
    path: String,
) -> AppResult<KnowledgeSettings> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || k.confirm_obsidian(&vault_id, &expected, &path))
        .await
        .map_err(|_| "Obsidian 确认未保存")?
}

#[tauri::command]
pub async fn knowledge_obsidian_manager() -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(|| crate::service::shell_open("obsidian://choose-vault"))
        .await
        .map_err(|_| "Obsidian Vault 管理器请求未完成")?
}

#[tauri::command]
pub async fn knowledge_open_entry_folder(
    knowledge: Knowledge<'_>,
    vault_id: String,
    path: String,
) -> AppResult<()> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || k.open_entry_folder(&vault_id, &path))
        .await
        .map_err(|_| "打开笔记文件夹未完成")?
}

#[tauri::command]
pub async fn knowledge_export_note(
    knowledge: Knowledge<'_>,
    app: tauri::AppHandle,
    vault_id: String,
    path: String,
) -> AppResult<bool> {
    use std::io::Write;
    use tauri_plugin_dialog::DialogExt;
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let doc = k.read(&vault_id, &path)?.data;
        let Some(target) = app
            .dialog()
            .file()
            .add_filter("Markdown", &["md"])
            .set_file_name(doc.path.rsplit('/').next().unwrap_or("笔记.md"))
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let target = target
            .into_path()
            .map_err(|_| "请选择本地 Markdown 文件路径")?;
        if !target
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            return Err("导出文件必须使用 .md 扩展名".into());
        }
        // A selected existing path must never overwrite the source note or another user file.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|_| "无法创建导出文件；目标可能已存在，请使用新的文件名")?;
        file.write_all(doc.text.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| "导出写入未完成，请检查目标磁盘")?;
        Ok(true)
    })
    .await
    .map_err(|_| "导出笔记未完成")?
}

#[tauri::command]
pub async fn knowledge_open_vault_folder(
    knowledge: Knowledge<'_>,
    vault_id: String,
) -> AppResult<()> {
    let k = knowledge.inner().clone();
    tauri::async_runtime::spawn_blocking(move || k.open_vault_folder(&vault_id))
        .await
        .map_err(|_| "知识库文件夹请求未完成")?
}

#[tauri::command]
pub async fn knowledge_generate_analysis_note(
    knowledge: Knowledge<'_>,
    ai: State<'_, Arc<crate::ai::service::AiService>>,
    app: tauri::AppHandle,
    id: String,
    vault_id: String,
) -> AppResult<super::generated::GeneratedNote> {
    let k = knowledge.inner().clone();
    let a = ai.inner().clone();
    let result = tauri::async_runtime::spawn_blocking(move || a.generate_note(&k, &id, &vault_id))
        .await
        .map_err(|_| "知识笔记生成线程未完成")??;
    let _ = app.emit(
        "knowledge-file-changed",
        serde_json::json!({"vaultId":result.vault_id,"id":result.path,"kind":"create"}),
    );
    Ok(result)
}
#[tauri::command]
pub async fn knowledge_analysis_note(
    knowledge: Knowledge<'_>,
    ai: State<'_, Arc<crate::ai::service::AiService>>,
    id: String,
) -> AppResult<Option<super::generated::NoteReference>> {
    let k = knowledge.inner().clone();
    let a = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || a.analysis_note(&k, &id))
        .await
        .map_err(|_| "读取知识笔记关联失败")?
}
#[tauri::command]
pub async fn knowledge_plan_analysis_delete(
    knowledge: Knowledge<'_>,
    ai: State<'_, Arc<crate::ai::service::AiService>>,
    id: String,
    vault_id: String,
) -> AppResult<super::generated::DeletePlan> {
    let k = knowledge.inner().clone();
    let a = ai.inner().clone();
    tauri::async_runtime::spawn_blocking(move || a.plan_note_delete(&k, &id, &vault_id))
        .await
        .map_err(|_| "预览删除清单失败")?
}
#[tauri::command]
pub async fn knowledge_delete_analysis_bundle(
    knowledge: Knowledge<'_>,
    ai: State<'_, Arc<crate::ai::service::AiService>>,
    app: tauri::AppHandle,
    id: String,
    vault_id: String,
    files: Vec<super::generated::GeneratedFile>,
) -> AppResult<super::generated::BundleDeleted> {
    let k = knowledge.inner().clone();
    let a = ai.inner().clone();
    let event_vault = vault_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        a.delete_note_bundle(&k, &id, &vault_id, &files)
    })
    .await
    .map_err(|_| "联合删除线程未完成")??;
    let _ = app.emit(
        "knowledge-index-updated",
        serde_json::json!({"vaultId":event_vault,"id":event_vault,"kind":"delete"}),
    );
    Ok(result)
}
