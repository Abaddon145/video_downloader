use super::vault::Document;
use crate::{domain::AppResult, learning::db};
use rusqlite::{params, Connection};

pub fn has_fts(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='knowledge_fts')",
        [],
        |r| r.get(0),
    )
    .unwrap_or(false)
}
pub fn upsert(conn: &mut Connection, doc: &Document) -> AppResult<Vec<String>> {
    let mut note = super::markdown::parse_note(&doc.path, &doc.text);
    let tx = conn.transaction().map_err(db::error)?;
    for (id, table) in [
        (&mut note.item_id, "series_items"),
        (&mut note.analysis_id, "video_analyses"),
    ] {
        if let Some(value) = id.as_ref() {
            let exists: bool = tx
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
                    [value],
                    |r| r.get(0),
                )
                .map_err(db::error)?;
            if !exists {
                note.warnings.push(format!(
                    "Frontmatter {table} 关联不存在，已保留原文并忽略索引关联"
                ));
                *id = None;
            }
        }
    }
    tx.execute("INSERT INTO knowledge_index(path,title,content_hash,mtime,content,analysis_id,item_id,indexed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(path) DO UPDATE SET title=excluded.title,content_hash=excluded.content_hash,mtime=excluded.mtime,content=excluded.content,analysis_id=excluded.analysis_id,item_id=excluded.item_id,indexed_at=excluded.indexed_at", params![doc.path,note.title,doc.hash,doc.mtime,doc.text,note.analysis_id,note.item_id,crate::service::now()]).map_err(db::error)?;
    tx.execute(
        "DELETE FROM knowledge_links WHERE source_path=?1",
        [&doc.path],
    )
    .map_err(db::error)?;
    tx.execute("DELETE FROM knowledge_tags WHERE path=?1", [&doc.path])
        .map_err(db::error)?;
    for target in &note.links {
        tx.execute(
            "INSERT INTO knowledge_links(source_path,target) VALUES(?1,?2)",
            params![doc.path, target],
        )
        .map_err(db::error)?;
    }
    for tag in &note.tags {
        tx.execute(
            "INSERT INTO knowledge_tags(path,tag) VALUES(?1,?2)",
            params![doc.path, tag],
        )
        .map_err(db::error)?;
    }
    if has_fts(&tx) {
        let updated = tx
            .execute("DELETE FROM knowledge_fts WHERE path=?1", [&doc.path])
            .and_then(|_| {
                tx.execute(
                    "INSERT INTO knowledge_fts(path,title,content) VALUES(?1,?2,?3)",
                    params![doc.path, note.title, doc.text],
                )
            });
        if updated.is_err() {
            note.warnings
                .push("全文加速索引暂时不可用，仍可使用普通搜索".into());
        }
    }
    tx.commit().map_err(db::error)?;
    Ok(note.warnings)
}
pub fn remove(conn: &mut Connection, path: &str) -> AppResult<()> {
    let tx = conn.transaction().map_err(db::error)?;
    tx.execute("DELETE FROM knowledge_index WHERE path=?1", [path])
        .map_err(db::error)?;
    if has_fts(&tx) {
        let _ = tx.execute("DELETE FROM knowledge_fts WHERE path=?1", [path]);
    }
    tx.commit().map_err(db::error)
}
pub fn clear(conn: &mut Connection) -> AppResult<()> {
    let tx = conn.transaction().map_err(db::error)?;
    tx.execute_batch(
        "DELETE FROM knowledge_links; DELETE FROM knowledge_tags; DELETE FROM knowledge_index;",
    )
    .map_err(db::error)?;
    if has_fts(&tx) {
        tx.execute("DELETE FROM knowledge_fts", [])
            .map_err(db::error)?;
    }
    tx.commit().map_err(db::error)
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn connection() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE series_items(id TEXT PRIMARY KEY,title TEXT NOT NULL DEFAULT '课程标题'); CREATE TABLE video_analyses(id TEXT PRIMARY KEY,item_id TEXT); CREATE TABLE transcript_segments(id TEXT PRIMARY KEY,analysis_id TEXT,start_ms INTEGER,end_ms INTEGER,text TEXT); CREATE TABLE knowledge_index(path TEXT PRIMARY KEY,title TEXT NOT NULL,content_hash TEXT,mtime INTEGER,content TEXT,analysis_id TEXT REFERENCES video_analyses(id),item_id TEXT REFERENCES series_items(id),indexed_at INTEGER); CREATE TABLE knowledge_links(source_path TEXT REFERENCES knowledge_index(path) ON DELETE CASCADE,target TEXT,PRIMARY KEY(source_path,target)); CREATE TABLE knowledge_tags(path TEXT REFERENCES knowledge_index(path) ON DELETE CASCADE,tag TEXT,PRIMARY KEY(path,tag));").unwrap();
        let _ = c.execute_batch(
            "CREATE VIRTUAL TABLE knowledge_fts USING fts5(path UNINDEXED,title,content)",
        );
        c
    }
    #[test]
    fn upsert_filters_foreign_ids_and_replaces_derived_rows() {
        let mut c = connection();
        let mut d = super::super::vault::Document {
            path: "a.md".into(),
            text: "---\nitem_id: gone\ntags: [old]\n---\n[[Old]]".into(),
            hash: "1".into(),
            mtime: 1,
        };
        assert!(!upsert(&mut c, &d).unwrap().is_empty());
        d.text = "# New\n[[New]] #new".into();
        upsert(&mut c, &d).unwrap();
        assert_eq!(
            c.query_row("SELECT target FROM knowledge_links", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "New"
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM knowledge_tags", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        if has_fts(&c) {
            assert_eq!(
                c.query_row("SELECT count(*) FROM knowledge_fts", [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
        remove(&mut c, "a.md").unwrap();
        clear(&mut c).unwrap();
    }
}
