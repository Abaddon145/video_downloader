use crate::{domain::AppResult, learning::db};
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub kind: String,
    pub path: Option<String>,
    pub title: String,
    pub excerpt: String,
    pub item_id: Option<String>,
    pub analysis_id: Option<String>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub items: Vec<SearchHit>,
    pub total: u64,
    pub page: u32,
}
fn fts_query(query: &str) -> String {
    query
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .take(16)
        .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}
pub fn search(
    conn: &Connection,
    query: &str,
    page: u32,
    tag: Option<&str>,
) -> AppResult<SearchPage> {
    let query = query.trim();
    let tag = tag.map(str::trim).filter(|s| !s.is_empty());
    if query.len() > 1024 || query.chars().count() > 200 || tag.is_some_and(|s| s.len() > 256) {
        return Err("搜索条件过长".into());
    }
    let page = page.max(1);
    if query.is_empty() && tag.is_none() {
        return Ok(SearchPage {
            items: Vec::new(),
            total: 0,
            page,
        });
    }
    let fts = fts_query(query);
    let note_fts = !fts.is_empty() && conn.query_row("SELECT count(*) FROM knowledge_index i WHERE path IN (SELECT path FROM knowledge_fts WHERE knowledge_fts MATCH ?1) AND (?2 IS NULL OR EXISTS(SELECT 1 FROM knowledge_tags t WHERE t.path=i.path AND t.tag=?2))",params![fts,tag],|r|r.get::<_,i64>(0)).unwrap_or(0)>0;
    let transcript_fts = tag.is_none() && !fts.is_empty() && conn.query_row("SELECT count(*) FROM transcript_segments t WHERE id IN (SELECT id FROM transcript_fts WHERE transcript_fts MATCH ?1)",[&fts],|r|r.get::<_,i64>(0)).unwrap_or(0)>0;
    let note_filter = if note_fts {
        "i.path IN (SELECT path FROM knowledge_fts WHERE knowledge_fts MATCH ?4)"
    } else {
        "(instr(lower(i.title),lower(?1))>0 OR instr(lower(i.content),lower(?1))>0)"
    };
    let transcript_filter = if transcript_fts {
        "t.id IN (SELECT id FROM transcript_fts WHERE transcript_fts MATCH ?4)"
    } else {
        "instr(lower(t.text),lower(?1))>0"
    };
    let union = format!("SELECT 'note' AS kind,i.path,i.title,substr(i.content,max(1,instr(lower(i.content),lower(?1))-80),240) AS excerpt,i.item_id,i.analysis_id,NULL AS start_ms,NULL AS end_ms,i.path AS sort_key FROM knowledge_index i WHERE {note_filter} AND (?2 IS NULL OR EXISTS(SELECT 1 FROM knowledge_tags k WHERE k.path=i.path AND k.tag=?2)) AND ?4 IS NOT NULL UNION ALL SELECT 'transcript',NULL,coalesce(p.title,'视频字幕'),substr(t.text,max(1,instr(lower(t.text),lower(?1))-80),240),a.item_id,t.analysis_id,t.start_ms,t.end_ms,t.id FROM transcript_segments t JOIN video_analyses a ON a.id=t.analysis_id LEFT JOIN series_items p ON p.id=a.item_id WHERE ?2 IS NULL AND {transcript_filter} AND ?4 IS NOT NULL");
    let offset = u64::from(page - 1) * 50;
    // Keep numbered bindings identical for count and page queries, including unused offset.
    let total: u64 = conn
        .query_row(
            &format!("SELECT count(*) FROM ({union}) WHERE ?3>=0"),
            params![query, tag, offset, fts],
            |r| r.get(0),
        )
        .map_err(db::error)?;
    let mut statement = conn.prepare(&format!("SELECT kind,path,title,excerpt,item_id,analysis_id,start_ms,end_ms FROM ({union}) ORDER BY kind,sort_key,start_ms LIMIT 50 OFFSET ?3")).map_err(db::error)?;
    let items = statement
        .query_map(params![query, tag, offset, fts], |r| {
            Ok(SearchHit {
                kind: r.get(0)?,
                path: r.get(1)?,
                title: r.get(2)?,
                excerpt: r.get(3)?,
                item_id: r.get(4)?,
                analysis_id: r.get(5)?,
                start_ms: r.get(6)?,
                end_ms: r.get(7)?,
            })
        })
        .map_err(db::error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db::error)?;
    Ok(SearchPage { items, total, page })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chinese_fallback_tags_and_transcript_seek() {
        let c = super::super::index::tests::connection();
        c.execute_batch("INSERT INTO knowledge_index(path,title,content) VALUES('中文.md','中文','这里学习中文内容'); INSERT INTO knowledge_tags VALUES('中文.md','学习'); INSERT INTO series_items(id) VALUES('item'); INSERT INTO video_analyses VALUES('analysis','item'); INSERT INTO transcript_segments VALUES('s','analysis',1200,2400,'学习中文内容');").unwrap();
        let p = search(&c, "中文", 1, None).unwrap();
        assert_eq!(p.total, 2);
        assert!(p
            .items
            .iter()
            .any(|h| h.start_ms == Some(1200) && h.item_id.as_deref() == Some("item")));
        assert_eq!(search(&c, "中文", 1, Some("学习")).unwrap().total, 1);
        assert_eq!(search(&c, "", 1, Some("学习")).unwrap().total, 1);
        assert_eq!(
            p.items
                .iter()
                .find(|h| h.kind == "transcript")
                .unwrap()
                .title,
            "课程标题"
        );
        assert_eq!(search(&c, "\" OR *", 1, None).unwrap().total, 0);
        assert!(search(&c, "", 1, None).unwrap().items.is_empty());
    }
}
