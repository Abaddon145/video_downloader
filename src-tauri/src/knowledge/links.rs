use crate::{domain::AppResult, learning::db};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteCandidate {
    pub path: String,
    pub title: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkPage {
    pub items: Vec<NoteCandidate>,
    pub total: u64,
    pub page: u32,
}
pub fn normalize_target(target: &str) -> Option<String> {
    let target = target
        .split('|')
        .next()?
        .split('#')
        .next()?
        .trim()
        .replace('\\', "/");
    if target.is_empty()
        || target.len() > 2048
        || target.contains([':', '\0'])
        || target.chars().any(char::is_control)
        || target.split('/').any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || s.starts_with('.')
                || s.eq_ignore_ascii_case("node_modules")
        })
    {
        return None;
    }
    Some(
        match target.rsplit_once('.') {
            Some((base, ext)) if ext.eq_ignore_ascii_case("md") => base,
            _ => &target,
        }
        .into(),
    )
}
fn candidates(conn: &Connection, sql: &str, target: &str) -> AppResult<Vec<NoteCandidate>> {
    conn.prepare(sql)
        .map_err(db::error)?
        .query_map([target], |r| {
            Ok(NoteCandidate {
                path: r.get(0)?,
                title: r.get(1)?,
            })
        })
        .map_err(db::error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db::error)
}
pub fn resolve(
    conn: &Connection,
    source_path: &str,
    target: &str,
) -> AppResult<Vec<NoteCandidate>> {
    let Some(target) = normalize_target(target) else {
        return Ok(Vec::new());
    };
    let file = format!("{target}.md");
    if let Some((folder, _)) = source_path.rsplit_once('/') {
        let same = candidates(
            conn,
            "SELECT path,title FROM knowledge_index WHERE path=?1 COLLATE NOCASE ORDER BY path",
            &format!("{folder}/{file}"),
        )?;
        if !same.is_empty() {
            return Ok(same);
        }
    }
    let exact = candidates(
        conn,
        "SELECT path,title FROM knowledge_index WHERE path=?1 COLLATE NOCASE ORDER BY path",
        &file,
    )?;
    if !exact.is_empty() {
        return Ok(exact);
    }
    if target.contains('/') {
        return Ok(Vec::new());
    }
    candidates(conn, "SELECT path,title FROM knowledge_index WHERE path=?1 COLLATE NOCASE OR substr(path,-length(?1)-1)='/'||?1 COLLATE NOCASE ORDER BY path", &file)
}
pub fn backlinks(conn: &Connection, path: &str, page: u32) -> AppResult<BacklinkPage> {
    // ponytail: resolve each stored source target; add a resolved-edge cache only if measured Vault size needs it.
    let mut statement = conn.prepare("SELECT l.source_path,l.target,i.title FROM knowledge_links l JOIN knowledge_index i ON i.path=l.source_path ORDER BY l.source_path,l.target").map_err(db::error)?;
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(db::error)?;
    let wanted_page = page.max(1);
    let offset = u64::from(wanted_page - 1) * 50;
    let mut items = Vec::new();
    let mut total = 0;
    let mut previous = String::new();
    for row in rows {
        let (source, target, title) = row.map_err(db::error)?;
        if source == previous {
            continue;
        }
        let matches = resolve(conn, &source, &target)?;
        if matches.len() == 1 && matches[0].path == path {
            previous = source.clone();
            if total >= offset && items.len() < 50 {
                items.push(NoteCandidate {
                    path: source,
                    title,
                });
            }
            total += 1;
        }
    }
    Ok(BacklinkPage {
        items,
        total,
        page: wanted_page,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backlinks_resolve_same_folder_and_reject_ambiguity() {
        let c = super::super::index::tests::connection();
        for p in ["a/Note.md", "b/Note.md", "a/Source.md", "c/Source.md"] {
            c.execute("INSERT INTO knowledge_index(path,title) VALUES(?1,?1)", [p])
                .unwrap();
        }
        assert_eq!(
            resolve(&c, "a/Source.md", "Note#Head|Alias").unwrap()[0].path,
            "a/Note.md"
        );
        assert_eq!(resolve(&c, "c/Source.md", "Note").unwrap().len(), 2);
        c.execute(
            "INSERT INTO knowledge_links VALUES('a/Source.md','Note'),('c/Source.md','Note')",
            [],
        )
        .unwrap();
        let p = backlinks(&c, "a/Note.md", 1).unwrap();
        assert_eq!(p.total, 1);
        assert_eq!(p.items[0].path, "a/Source.md");
        assert!(resolve(&c, "a/Source.md", "../escape").unwrap().is_empty());
    }
}

#[cfg(test)]
mod extension_tests {
    #[test]
    fn md_extension_is_case_insensitive() {
        assert_eq!(
            super::normalize_target("Folder/Note.MD#Head|Alias").as_deref(),
            Some("Folder/Note")
        );
    }
}
