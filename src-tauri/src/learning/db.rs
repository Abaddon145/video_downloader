use crate::domain::AppResult;
use rusqlite::Connection;
use std::{path::Path, time::Duration};
pub(crate) fn error(e: rusqlite::Error) -> String {
    format!("学习库数据库错误：{e}")
}
pub fn open(path: &Path) -> AppResult<Connection> {
    let mut c = Connection::open(path).map_err(error)?;
    c.busy_timeout(Duration::from_secs(3)).map_err(error)?;
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;").map_err(error)?;
    let exists:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version')",[],|r|r.get(0)).map_err(error)?;
    let version = if exists {
        c.query_row("SELECT version FROM schema_version", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(error)?
    } else {
        0
    };
    if version > 6 || version < 0 {
        return Err("学习库版本高于本软件或版本记录无效，请保留数据库并使用对应版本".into());
    }
    let tx = c.transaction().map_err(error)?;
    if version == 0 {
        tx.execute_batch(include_str!("schema-v1.sql"))
            .map_err(error)?;
    }
    if version < 2 {
        tx.execute_batch("ALTER TABLE series ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0;
 ALTER TABLE series ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
 ALTER TABLE series_items ADD COLUMN file_key TEXT;
 ALTER TABLE series_items ADD COLUMN source_key TEXT;
 ALTER TABLE series_items ADD COLUMN download_request TEXT;
 ALTER TABLE series_items ADD COLUMN download_state TEXT NOT NULL DEFAULT 'waiting';
 CREATE UNIQUE INDEX idx_items_file_key ON series_items(series_id,file_key) WHERE file_key IS NOT NULL;
 CREATE UNIQUE INDEX idx_items_source_key ON series_items(series_id,source_key) WHERE source_key IS NOT NULL;
 CREATE INDEX idx_items_task ON series_items(download_task_id);
 CREATE TABLE learning_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
 UPDATE schema_version SET version=2;").map_err(error)?;
    }
    if version < 3 {
        tx.execute_batch(include_str!("schema-v3.sql"))
            .map_err(error)?;
    }
    if version < 4 {
        tx.execute_batch("ALTER TABLE series_items ADD COLUMN request_key TEXT; CREATE INDEX idx_learning_request_key ON series_items(request_key); UPDATE schema_version SET version=4;").map_err(error)?;
    }
    if version < 5 {
        tx.execute_batch(include_str!("schema-v5.sql"))
            .map_err(error)?;
    }
    if version < 6 {
        tx.execute_batch(include_str!("schema-v6.sql"))
            .map_err(error)?;
    }
    // Preserve databases written by the early v0.5 development builds.
    let has_options = tx
        .prepare("PRAGMA table_info(video_analyses)")
        .map_err(error)?
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?
        .iter()
        .any(|name| name == "options_json");
    if !has_options {
        tx.execute_batch(
            "ALTER TABLE video_analyses ADD COLUMN options_json TEXT NOT NULL DEFAULT '{}';",
        )
        .map_err(error)?;
    }
    // Idempotently complete the unreleased v6 schema without changing paid analysis cache keys.
    for (table, column) in [
        ("video_analyses", "note_target_vault_id"),
        ("video_analyses", "note_warning"),
        ("video_notes", "markdown_vault_id"),
        ("video_notes", "markdown_path"),
    ] {
        let columns = tx
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(error)?
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(error)?;
        if !columns.iter().any(|name| name == column) {
            tx.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} TEXT;"))
                .map_err(error)?;
        }
    }
    tx.execute_batch("PRAGMA user_version=6;").map_err(error)?;
    tx.commit().map_err(error)?;
    enable_search(&c);
    Ok(c)
}
// Search is an optional acceleration; LIKE remains available on every build.
pub(crate) fn enable_search(c: &Connection) -> bool {
    let supported = c
        .query_row("SELECT sqlite_compileoption_used('ENABLE_FTS5')", [], |r| {
            r.get::<_, bool>(0)
        })
        .unwrap_or(false);
    supported && c.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS transcript_fts USING fts5(id UNINDEXED,analysis_id UNINDEXED,text);
       CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(path UNINDEXED,title,content);").is_ok()
}
