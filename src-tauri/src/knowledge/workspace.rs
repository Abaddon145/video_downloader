use super::{
    models::*,
    service::KnowledgeService,
    vault::{self, Document},
};
use crate::{domain::AppResult, learning::db};
use rusqlite::{Connection, OptionalExtension};

pub(super) fn checkpoint(db: &Connection, vault_id: &str, document: &Document) -> AppResult<()> {
    let previous: Option<String> = db.query_row(
        "SELECT content_hash FROM knowledge_note_versions WHERE vault_id=?1 AND path=?2 ORDER BY rowid DESC LIMIT 1",
        rusqlite::params![vault_id,document.path], |r| r.get(0),
    ).optional().map_err(db::error)?;
    if previous.as_deref() != Some(&document.hash) {
        // ponytail: retain all snapshots; add explicit user-controlled retention if storage grows.
        db.execute("INSERT INTO knowledge_note_versions(id,vault_id,path,content_hash,text,bytes,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![crate::ffmpeg::runner::new_id(),vault_id,document.path,document.hash,document.text,document.text.len() as i64,crate::service::now()]).map_err(db::error)?;
    }
    Ok(())
}

fn note_path(path: &str) -> AppResult<String> {
    let path = vault::validate_relative(path, false)?;
    if !path.to_ascii_lowercase().ends_with(".md") {
        return Err("仅支持 Markdown 笔记".into());
    }
    Ok(path)
}
fn version_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NoteVersion> {
    Ok(NoteVersion {
        id: row.get(0)?,
        path: row.get(1)?,
        hash: row.get(2)?,
        bytes: row.get(3)?,
        created_at: row.get(4)?,
    })
}
impl KnowledgeService {
    pub fn set_note_flag(
        &self,
        id: &str,
        path: &str,
        flag: &str,
        expected: bool,
        value: bool,
    ) -> AppResult<KnowledgeView<NoteMetadata>> {
        let column = match flag {
            "favorite" => "favorite",
            "template" => "template",
            _ => return Err("笔记标记类型无效".into()),
        };
        self.with_vault(id,|s,v| {
            let path=v.read(path)?.path;
            let tx=s.db.transaction().map_err(db::error)?;
            tx.execute("INSERT OR IGNORE INTO knowledge_note_metadata(vault_id,path,favorite,template) VALUES(?1,?2,0,0)",rusqlite::params![id,path]).map_err(db::error)?;
            let changed=tx.execute(&format!("UPDATE knowledge_note_metadata SET {column}=?3 WHERE vault_id=?1 AND path=?2 AND {column}=?4"),rusqlite::params![id,path,value,expected]).map_err(db::error)?;
            if changed!=1 {return Err("笔记标记已发生变化，请重新读取；未覆盖后续修改".into());}
            let (favorite,template)=tx.query_row("SELECT favorite,template FROM knowledge_note_metadata WHERE vault_id=?1 AND path=?2",rusqlite::params![id,path],|r|Ok((r.get(0)?,r.get(1)?))).map_err(db::error)?;
            tx.commit().map_err(db::error)?;
            Ok(KnowledgeView{vault_id:id.into(),data:NoteMetadata{path,favorite,template,available:true}})
        })
    }
    pub fn note_metadata(&self, id: &str, path: &str) -> AppResult<KnowledgeView<NoteMetadata>> {
        self.with_vault(id, |s,v| {
            let path=note_path(path)?;
            let flags=s.db.query_row("SELECT favorite,template FROM knowledge_note_metadata WHERE vault_id=?1 AND path=?2",rusqlite::params![id,path],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(db::error)?.unwrap_or((false,false));
            Ok(KnowledgeView { vault_id:id.into(),data:NoteMetadata { available:v.read(&path).is_ok(),path,favorite:flags.0,template:flags.1 } })
        })
    }
    pub fn set_note_metadata(
        &self,
        id: &str,
        path: &str,
        favorite: bool,
        template: bool,
    ) -> AppResult<KnowledgeView<NoteMetadata>> {
        self.with_vault(id,|s,v| {
            let path=v.read(path)?.path;
            s.db.execute("INSERT INTO knowledge_note_metadata(vault_id,path,favorite,template) VALUES(?1,?2,?3,?4) ON CONFLICT(vault_id,path) DO UPDATE SET favorite=excluded.favorite,template=excluded.template",rusqlite::params![id,path,favorite,template]).map_err(db::error)?;
            Ok(KnowledgeView { vault_id:id.into(),data:NoteMetadata { path,favorite,template,available:true } })
        })
    }
    pub fn note_collection(
        &self,
        id: &str,
        kind: &str,
        page: u32,
    ) -> AppResult<KnowledgeView<NoteMetadataPage>> {
        let column = match kind {
            "favorites" => "favorite",
            "templates" => "template",
            _ => return Err("笔记集合无效".into()),
        };
        self.with_vault(id,|s,v| {
            let total:u64=s.db.query_row(&format!("SELECT count(*) FROM knowledge_note_metadata WHERE vault_id=?1 AND {column}=1"),[id],|r|r.get(0)).map_err(db::error)?;
            let page=page.max(1);
            let mut q=s.db.prepare(&format!("SELECT path,favorite,template FROM knowledge_note_metadata WHERE vault_id=?1 AND {column}=1 ORDER BY path COLLATE NOCASE,path LIMIT 50 OFFSET ?2")).map_err(db::error)?;
            let mut items=q.query_map(rusqlite::params![id,u64::from(page-1)*50],|r|Ok(NoteMetadata { path:r.get(0)?,favorite:r.get(1)?,template:r.get(2)?,available:false })).map_err(db::error)?.collect::<Result<Vec<_>,_>>().map_err(db::error)?;
            for item in &mut items { item.available=v.read(&item.path).is_ok(); }
            Ok(KnowledgeView {vault_id:id.into(),data:NoteMetadataPage {items,total,page}})
        })
    }
    pub fn note_versions(
        &self,
        id: &str,
        path: &str,
        page: u32,
    ) -> AppResult<KnowledgeView<NoteVersionPage>> {
        self.with_vault(id,|s,_| {
            let path=note_path(path)?;
            let total=s.db.query_row("SELECT count(*) FROM knowledge_note_versions WHERE vault_id=?1 AND path=?2",rusqlite::params![id,path],|r|r.get(0)).map_err(db::error)?;
            let page=page.max(1);
            let mut q=s.db.prepare("SELECT id,path,content_hash,bytes,created_at FROM knowledge_note_versions WHERE vault_id=?1 AND path=?2 ORDER BY rowid DESC LIMIT 50 OFFSET ?3").map_err(db::error)?;
            let items=q.query_map(rusqlite::params![id,path,u64::from(page-1)*50],version_row).map_err(db::error)?.collect::<Result<Vec<_>,_>>().map_err(db::error)?;
            Ok(KnowledgeView {vault_id:id.into(),data:NoteVersionPage {items,total,page}})
        })
    }
    pub fn note_version(
        &self,
        id: &str,
        path: &str,
        version_id: &str,
    ) -> AppResult<KnowledgeView<NoteVersionContent>> {
        if version_id.is_empty() || version_id.len() > 128 {
            return Err("历史版本标识无效".into());
        }
        self.with_vault(id,|s,_| {
            let path=note_path(path)?;
            let data=s.db.query_row("SELECT id,path,content_hash,bytes,created_at,text FROM knowledge_note_versions WHERE vault_id=?1 AND path=?2 AND id=?3 AND bytes<=5242880 AND length(CAST(text AS BLOB))<=5242880",rusqlite::params![id,path,version_id],|r|Ok(NoteVersionContent { version:version_row(r)?,text:r.get(5)? })).optional().map_err(db::error)?.ok_or("未找到该笔记的历史版本")?;
            if super::generated::hash(data.text.as_bytes())!=data.version.hash { return Err("历史版本内容校验失败，原文件未修改".into()); }
            Ok(KnowledgeView {vault_id:id.into(),data})
        })
    }
    pub fn restore_note_version(
        &self,
        id: &str,
        path: &str,
        version_id: &str,
        expected_hash: &str,
    ) -> AppResult<KnowledgeWrite> {
        let version = self.note_version(id, path, version_id)?;
        self.write(id, path, &version.data.text, expected_hash)
    }
}
