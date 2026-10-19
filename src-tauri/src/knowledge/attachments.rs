use super::{
    generated::hash,
    models::*,
    service::{ensure_directory, KnowledgeService},
    vault,
};
use crate::{domain::AppResult, learning::db};
use rusqlite::OptionalExtension;
use std::{
    fs::OpenOptions,
    io::Read,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
};

/// Only the Rust native picker feeds this source path; never expose it as an IPC argument.
pub(super) fn read_selected_attachment(source: &Path) -> AppResult<(String, Vec<u8>)> {
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("附件名称不是有效 Unicode")?
        .to_string();
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(source)
        .map_err(|_| "无法读取所选附件，请检查权限或占用")?;
    let metadata = file.metadata().map_err(|_| "无法检查所选附件")?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err("请选择本地普通附件文件，不支持目录或重解析点".into());
    }
    let mut bytes = Vec::new();
    file.take(20 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "所选附件读取未完成")?;
    if bytes.len() > 20 * 1024 * 1024 {
        return Err("附件超过20MiB限制".into());
    }
    Ok((name, bytes))
}

fn registered(db: &rusqlite::Connection, id: &str, path: &str, folder: &str) -> AppResult<String> {
    let path = vault::validate_relative(path, false)?;
    if !path
        .split_once('/')
        .is_some_and(|(parent, _)| parent.eq_ignore_ascii_case(folder))
    {
        return Err("附件不在当前知识库附件目录".into());
    }
    let exists: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM knowledge_attachments WHERE vault_id=?1 AND path=?2)",
            rusqlite::params![id, path],
            |r| r.get(0),
        )
        .map_err(db::error)?;
    if !exists {
        return Err("该文件不属于当前知识库的映流导入附件".into());
    }
    Ok(path)
}

impl KnowledgeService {
    pub(crate) fn import_attachment(
        &self,
        id: &str,
        name: &str,
        bytes: &[u8],
    ) -> AppResult<KnowledgeView<NoteAttachment>> {
        if name.is_empty()
            || name.len() > 1024
            || name.chars().any(|c| c.is_control() || "/\\".contains(c))
        {
            return Err("附件名称无效".into());
        }
        self.with_vault(id,|s,v| {
            let folder=vault::validate_relative(&s.settings.attachments_folder,false)?;
            if folder.contains('/') { return Err("附件目录设置无效".into()); }
            ensure_directory(v,&folder)?;
            let extension=Path::new(name).extension().and_then(|s|s.to_str()).filter(|s|s.len()<=16 && s.bytes().all(|c|c.is_ascii_alphanumeric()));
            let filename=match extension { Some(ext)=>format!("{}.{}",crate::ffmpeg::runner::new_id(),ext),None=>format!("{}.bin",crate::ffmpeg::runner::new_id()) };
            let path=format!("{folder}/{filename}");
            v.create_attachment(&path,bytes)?;
            let hash=hash(bytes); let imported_at=crate::service::now() as i64;
            let result=s.db.execute("INSERT INTO knowledge_attachments(vault_id,path,name,content_hash,bytes,imported_at) VALUES(?1,?2,?3,?4,?5,?6)",rusqlite::params![id,path,name,hash,bytes.len() as i64,imported_at]).map_err(db::error);
            if let Err(error)=result {
                return Err(match v.trash_attachment(&path,&hash) { Ok(_)=>format!("附件记录保存失败，复制文件已移入知识库回收目录，原文件未修改：{error}"),Err(restore)=>format!("附件记录保存失败：{error}；复制文件回收失败：{restore}。请检查“{}”，不要重复导入。",v.root.join(&path).display()) });
            }
            Ok(KnowledgeView {vault_id:id.into(),data:NoteAttachment {path,name:name.into(),hash,bytes:bytes.len() as u64,imported_at,state:"ready".into()}})
        })
    }
    pub fn attachments(&self, id: &str, page: u32) -> AppResult<KnowledgeView<NoteAttachmentPage>> {
        self.with_vault(id,|s,v| {
            let total=s.db.query_row("SELECT count(*) FROM knowledge_attachments WHERE vault_id=?1",[id],|r|r.get(0)).map_err(db::error)?;
            let page=page.max(1);
            let mut q=s.db.prepare("SELECT path,name,content_hash,bytes,imported_at FROM knowledge_attachments WHERE vault_id=?1 ORDER BY imported_at DESC,path LIMIT 50 OFFSET ?2").map_err(db::error)?;
            let mut items=q.query_map(rusqlite::params![id,u64::from(page-1)*50],|r|Ok(NoteAttachment {path:r.get(0)?,name:r.get(1)?,hash:r.get(2)?,bytes:r.get(3)?,imported_at:r.get(4)?,state:"missing".into()})).map_err(db::error)?.collect::<Result<Vec<_>,_>>().map_err(db::error)?;
            for item in &mut items {
                if !item.path.split_once('/').is_some_and(|(parent,_)|parent.eq_ignore_ascii_case(&s.settings.attachments_folder)) { continue; }
                if let Ok(bytes)=v.read_attachment(&item.path) {
                    let actual=hash(&bytes);
                    item.state=if item.hash==actual {"ready"} else {"changed"}.into();
                    item.hash=actual; item.bytes=bytes.len() as u64;
                }
            }
            Ok(KnowledgeView {vault_id:id.into(),data:NoteAttachmentPage {items,total,page}})
        })
    }
    pub(crate) fn attachment_bytes(&self, id: &str, path: &str) -> AppResult<Vec<u8>> {
        self.with_vault(id, |s, v| {
            v.read_attachment(&registered(
                &s.db,
                id,
                path,
                &s.settings.attachments_folder,
            )?)
        })
    }
    pub fn delete_attachment(
        &self,
        id: &str,
        path: &str,
        expected_hash: &str,
    ) -> AppResult<KnowledgeMutation> {
        self.with_vault(id, |s, v| {
            let path = registered(&s.db, id, path, &s.settings.attachments_folder)?;
            v.trash_attachment(&path, expected_hash)?;
            let warning =
                s.db.execute(
                    "DELETE FROM knowledge_attachments WHERE vault_id=?1 AND path=?2",
                    rusqlite::params![id, path],
                )
                .err()
                .map(|_| "附件已移入知识库回收目录，但附件列表记录未能清理".into());
            Ok(KnowledgeMutation {
                vault_id: id.into(),
                path,
                index_warning: warning,
            })
        })
    }
    pub(crate) fn attachment_name(&self, id: &str, path: &str) -> AppResult<String> {
        self.with_vault(id, |s, _| {
            let path = registered(&s.db, id, path, &s.settings.attachments_folder)?;
            s.db.query_row(
                "SELECT name FROM knowledge_attachments WHERE vault_id=?1 AND path=?2",
                rusqlite::params![id, path],
                |r| r.get(0),
            )
            .optional()
            .map_err(db::error)?
            .ok_or("附件记录已变化".into())
        })
    }
}
