use super::{
    index,
    models::*,
    vault::{self, Document, TreePage, Vault, WriteOutcome},
};
use crate::{domain::AppResult, learning::db};
use rusqlite::Connection;
use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

struct State {
    settings: KnowledgeSettings,
    db: Connection,
}
pub struct KnowledgeService {
    data: PathBuf,
    state: Mutex<AppResult<State>>,
    indexing: AtomicBool,
}
fn load(data: &Path) -> AppResult<State> {
    let file = data.join("knowledge-settings.json");
    let settings = if file.exists() {
        let bytes = std::fs::read(file).map_err(|_| "无法读取知识库设置")?;
        if bytes.len() > 16384 {
            return Err("知识库设置过大，请保留原文件".into());
        }
        serde_json::from_slice(&bytes).map_err(|_| "知识库设置格式错误，请保留原文件")?
    } else {
        KnowledgeSettings::default()
    };
    Ok(State {
        settings,
        db: db::open(&data.join("learning.db"))?,
    })
}
fn ensure_directory(vault: &Vault, relative: &str) -> AppResult<()> {
    let relative = vault::validate_relative(relative, false)?;
    let mut parent = String::new();
    for part in relative.split('/') {
        if !parent.is_empty() {
            parent.push('/');
        }
        parent.push_str(part);
        if vault.list(&parent, 1).is_err() {
            vault.create_dir(&parent)?;
        }
    }
    Ok(())
}
fn metadata_warning(result: AppResult<Vec<String>>) -> Option<String> {
    match result {
        Ok(w) if w.is_empty() => None,
        Ok(w) => Some(w.join("；")),
        Err(_) => Some("文件已保存，但搜索索引未能更新；请重新扫描知识库".into()),
    }
}
impl KnowledgeService {
    pub fn new(data: &Path) -> Arc<Self> {
        Arc::new(Self {
            data: data.into(),
            state: Mutex::new(load(data)),
            indexing: AtomicBool::new(false),
        })
    }
    fn with_state<T>(&self, work: impl FnOnce(&mut State) -> AppResult<T>) -> AppResult<T> {
        let mut state = self.state.lock().map_err(|_| "知识库暂时不可用")?;
        if state.is_err() {
            *state = load(&self.data);
        }
        work(
            state
                .as_mut()
                .map_err(|e| format!("知识库初始化失败：{e}"))?,
        )
    }
    fn with_vault<T>(
        &self,
        id: &str,
        work: impl FnOnce(&mut State, &Vault) -> AppResult<T>,
    ) -> AppResult<T> {
        self.with_state(|s| {
            if s.settings.vault_id.as_deref() != Some(id) {
                return Err("知识库已切换，请重新打开当前笔记".into());
            }
            let root = s
                .settings
                .vault_path
                .as_deref()
                .ok_or("请先创建或选择知识库")?;
            let vault = Vault::open(Path::new(root))?;
            if vault.id != id {
                return Err("知识库目录已被替换，请重新选择".into());
            }
            work(s, &vault)
        })
    }
    pub fn settings(&self) -> AppResult<KnowledgeSettings> {
        self.with_state(|s| Ok(s.settings.clone()))
    }
    pub fn set_vault(&self, path: &str, create: bool) -> AppResult<KnowledgeSettings> {
        self.with_state(|s| {
            let path = Path::new(path);
            if !path.exists() && create {
                let parent = path.parent().ok_or("知识库目录无效")?;
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or("知识库名称无效")?;
                Vault::open(parent)?.create_dir(name)?;
            }
            let vault = Vault::open(path)?;
            let folder = vault::validate_relative(&s.settings.root_folder, false)?;
            if folder.contains('/') {
                return Err("知识笔记根目录必须是单个文件夹名称".into());
            }
            let attachments = vault::validate_relative(&s.settings.attachments_folder, false)?;
            if attachments.contains('/') {
                return Err("附件目录必须是单个文件夹名称".into());
            }
            ensure_directory(&vault, &folder)?;
            let next = KnowledgeSettings {
                vault_path: Some(vault.root.to_string_lossy().into_owned()),
                vault_id: Some(vault.id.clone()),
                root_folder: folder,
                attachments_folder: attachments,
            };
            // Cache is cleared before publishing settings; a crash can leave an empty cache, never another Vault's data.
            if s.settings.vault_id != next.vault_id {
                index::clear(&mut s.db)?;
            }
            let bytes = serde_json::to_vec_pretty(&next).map_err(|_| "知识库设置无效")?;
            crate::native::save_atomic(&self.data.join("knowledge-settings.json"), &bytes)?;
            s.settings = next.clone();
            Ok(next)
        })
    }
    pub fn tree(&self, id: &str, parent: &str, page: u32) -> AppResult<KnowledgeView<TreePage>> {
        self.with_vault(id, |_, v| {
            Ok(KnowledgeView {
                vault_id: id.into(),
                data: v.list(parent, page)?,
            })
        })
    }
    pub fn read(&self, id: &str, path: &str) -> AppResult<KnowledgeView<Document>> {
        self.with_vault(id, |_, v| {
            Ok(KnowledgeView {
                vault_id: id.into(),
                data: v.read(path)?,
            })
        })
    }
    pub fn write(
        &self,
        id: &str,
        path: &str,
        text: &str,
        expected_hash: &str,
    ) -> AppResult<KnowledgeWrite> {
        self.with_vault(id, |s, v| {
            let outcome = v.write(path, text, expected_hash)?;
            let doc = match &outcome {
                WriteOutcome::Saved(d) | WriteOutcome::Conflict(d) => d,
            };
            let warning = metadata_warning(index::upsert(&mut s.db, doc));
            Ok(KnowledgeWrite {
                vault_id: id.into(),
                outcome,
                index_warning: warning,
            })
        })
    }
    pub fn create(
        &self,
        id: &str,
        path: &str,
        text: &str,
        directory: bool,
    ) -> AppResult<KnowledgeMutation> {
        self.with_vault(id, |s, v| {
            let path = vault::validate_relative(path, false)?;
            if let Some((parent, _)) = path.rsplit_once('/') {
                ensure_directory(v, parent)?;
            }
            let warning = if directory {
                v.create_dir(&path)?;
                None
            } else {
                let doc = v.create_file(&path, text)?;
                metadata_warning(index::upsert(&mut s.db, &doc))
            };
            Ok(KnowledgeMutation {
                vault_id: id.into(),
                path,
                index_warning: warning,
            })
        })
    }
    fn remove_paths(s: &mut State, path: &str) -> AppResult<()> {
        let paths = {
            let mut q=s.db.prepare("SELECT path FROM knowledge_index WHERE path=?1 OR substr(path,1,length(?1)+1)=?1||'/'").map_err(db::error)?;
            let rows = q
                .query_map([path], |r| r.get::<_, String>(0))
                .map_err(db::error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db::error)?;
            rows
        };
        for path in paths {
            index::remove(&mut s.db, &path)?;
        }
        Ok(())
    }
    pub fn move_note(&self, id: &str, from: &str, to: &str) -> AppResult<KnowledgeMutation> {
        self.with_vault(id, |s, v| {
            let from = vault::validate_relative(from, false)?;
            let to = vault::validate_relative(to, false)?;
            if let Some((parent, _)) = to.rsplit_once('/') {
                ensure_directory(v, parent)?;
            }
            v.move_entry(&from, &to)?;
            for table in ["video_analyses","video_notes"] {
                let _=s.db.execute(&format!("UPDATE {table} SET markdown_path=?3||substr(markdown_path,length(?2)+1) WHERE markdown_vault_id=?1 AND (markdown_path=?2 OR substr(markdown_path,1,length(?2)+1)=?2||'/')"),rusqlite::params![id,from,to]);
            }

            let warning = if Self::remove_paths(s, &from).is_err() {
                Some("文件已移动，但旧搜索索引未能清理；请重新扫描知识库".into())
            } else if to.to_ascii_lowercase().ends_with(".md") {
                metadata_warning(v.read(&to).and_then(|d| index::upsert(&mut s.db, &d)))
            } else {
                Some("目录已移动，请重新扫描以更新其中笔记的索引".into())
            };
            Ok(KnowledgeMutation {
                vault_id: id.into(),
                path: to,
                index_warning: warning,
            })
        })
    }
    pub fn delete(
        &self,
        id: &str,
        path: &str,
        expected_hash: &str,
    ) -> AppResult<KnowledgeMutation> {
        self.with_vault(id, |s, v| {
            let path = vault::validate_relative(path, false)?;
            v.trash(&path, expected_hash)?;
            let warning = Self::remove_paths(s, &path)
                .err()
                .map(|_| "笔记已移入知识库回收目录，但搜索索引未能更新".into());
            Ok(KnowledgeMutation {
                vault_id: id.into(),
                path,
                index_warning: warning,
            })
        })
    }
    pub fn search(
        &self,
        id: &str,
        query: &str,
        page: u32,
        tag: Option<&str>,
    ) -> AppResult<KnowledgeView<super::search::SearchPage>> {
        self.with_vault(id, |s, _| {
            Ok(KnowledgeView {
                vault_id: id.into(),
                data: super::search::search(&s.db, query, page, tag)?,
            })
        })
    }
    pub fn backlinks(
        &self,
        id: &str,
        path: &str,
        page: u32,
    ) -> AppResult<KnowledgeView<super::links::BacklinkPage>> {
        self.with_vault(id, |s, v| {
            let d = v.read(path)?;
            Ok(KnowledgeView {
                vault_id: id.into(),
                data: super::links::backlinks(&s.db, &d.path, page)?,
            })
        })
    }
    pub fn resolve(
        &self,
        id: &str,
        source: &str,
        target: &str,
    ) -> AppResult<KnowledgeView<Vec<super::links::NoteCandidate>>> {
        self.with_vault(id, |s, v| {
            let d = v.read(source)?;
            Ok(KnowledgeView {
                vault_id: id.into(),
                data: super::links::resolve(&s.db, &d.path, target)?,
            })
        })
    }
    pub fn tags(&self, id: &str) -> AppResult<KnowledgeView<Vec<String>>> {
        self.with_vault(id, |s, _| {
            // ponytail: 5000 tag labels; page this endpoint if a measured Vault needs more.
            let mut q =
                s.db.prepare("SELECT DISTINCT tag FROM knowledge_tags ORDER BY tag LIMIT 5000")
                    .map_err(db::error)?;
            let data = q
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(db::error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db::error)?;
            Ok(KnowledgeView {
                vault_id: id.into(),
                data,
            })
        })
    }
    pub fn asset(&self, id: &str, path: &str) -> AppResult<KnowledgeView<KnowledgeAsset>> {
        use base64::Engine;
        self.with_vault(id, |_, v| {
            let bytes = v.read_bytes(path, 8 * 1024 * 1024)?;
            let ext = Path::new(path)
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_ascii_lowercase();
            let mime = match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "avif" => "image/avif",
                "bmp" => "image/bmp",
                _ => return Err("仅支持本地图片附件".into()),
            };
            Ok(KnowledgeView {
                vault_id: id.into(),
                data: KnowledgeAsset {
                    mime: mime.into(),
                    base64: base64::engine::general_purpose::STANDARD.encode(bytes),
                },
            })
        })
    }
    pub fn open_in_obsidian(&self, id: &str, path: &str) -> AppResult<String> {
        let uri = self.with_vault(id, |_, v| {
            let d = v.read(path)?;
            super::obsidian::uri(&v.root.join(d.path))
        })?;
        match crate::service::shell_open(&uri) {
            Ok(()) => Ok("已请求在 Obsidian 中打开".into()),
            Err(_) => Ok("未能启动 Obsidian；该笔记仍可继续在映流中编辑".into()),
        }
    }
    pub fn reindex(&self, id: &str) -> AppResult<KnowledgeView<IndexStats>> {
        if self.indexing.swap(true, Ordering::AcqRel) {
            return Err("知识库正在扫描，请稍后刷新".into());
        }
        struct Finish<'a>(&'a AtomicBool);
        impl Drop for Finish<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _finish = Finish(&self.indexing);
        let v = self.with_vault(id, |_, v| Vault::open(&v.root))?;
        let mut dirs = VecDeque::from([(String::new(), 0usize)]);
        let mut seen = BTreeSet::new();
        let mut stats = IndexStats {
            complete: true,
            ..Default::default()
        };
        let mut entries = 0usize;
        let mut bytes = 0usize;
        // ponytail: bounded full scan on explicit refresh/focus; add incremental watcher only if 10k-entry Vaults need it.
        while let Some((parent, depth)) = dirs.pop_front() {
            if depth > 64 {
                stats.complete = false;
                stats.warnings.push("目录层级过深，部分笔记未扫描".into());
                continue;
            }
            let mut page = 1;
            loop {
                let listing = match v.list(&parent, page) {
                    Ok(p) => p,
                    Err(_) => {
                        stats.complete = false;
                        stats.warnings.push("部分目录无法读取，旧索引已保留".into());
                        break;
                    }
                };
                if listing.skipped > 0 {
                    stats.complete = false;
                    stats.skipped += listing.skipped;
                    if stats.warnings.len() < 50 {
                        stats
                            .warnings
                            .push("部分目录项被占用或无法安全打开，旧索引已保留".into());
                    }
                }
                for item in listing.items {
                    entries += 1;
                    if entries > 10000 || bytes > 256 * 1024 * 1024 {
                        stats.complete = false;
                        stats
                            .warnings
                            .push("扫描达到单次上限，未删除任何未检查的索引".into());
                        return Ok(KnowledgeView {
                            vault_id: id.into(),
                            data: stats,
                        });
                    }
                    if item.directory {
                        dirs.push_back((item.path, depth + 1));
                        continue;
                    }
                    if !item.path.to_ascii_lowercase().ends_with(".md") {
                        continue;
                    }
                    seen.insert(item.path.clone());
                    match v.read(&item.path) {
                        Ok(doc) => {
                            bytes += doc.text.len();
                            let result = self.with_vault(id, |s, current| {
                                let fresh = current.read(&doc.path)?;
                                index::upsert(&mut s.db, &fresh)
                            });
                            match result {
                                Ok(w) => {
                                    stats.indexed += 1;
                                    for warning in w {
                                        if stats.warnings.len() < 50 {
                                            stats.warnings.push(warning);
                                        }
                                    }
                                }
                                Err(e) => {
                                    if self.settings()?.vault_id.as_deref() != Some(id) {
                                        return Err(e);
                                    }
                                    stats.skipped += 1;
                                    stats.complete = false;
                                    if stats.warnings.len() < 50 {
                                        stats
                                            .warnings
                                            .push("部分笔记索引未能更新，原文件已保留".into());
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            stats.skipped += 1;
                            stats.complete = false;
                            if stats.warnings.len() < 50 {
                                stats.warnings.push(
                                    "部分笔记无法读取或超过5MiB，原文件与旧索引已保留".into(),
                                );
                            }
                        }
                    }
                }
                if u64::from(page) * 50 >= listing.total {
                    break;
                }
                page += 1;
            }
        }
        if stats.complete {
            self.with_vault(id, |s, _| {
                let paths = {
                    let mut q =
                        s.db.prepare("SELECT path FROM knowledge_index")
                            .map_err(db::error)?;
                    let data = q
                        .query_map([], |r| r.get::<_, String>(0))
                        .map_err(db::error)?
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(db::error)?;
                    data
                };
                for path in paths {
                    if !seen.contains(&path) {
                        index::remove(&mut s.db, &path)?;
                    }
                }
                Ok(())
            })?;
        }
        Ok(KnowledgeView {
            vault_id: id.into(),
            data: stats,
        })
    }
}

impl KnowledgeService {
    pub(crate) fn prepare_generated(
        &self,
        a: &crate::ai::models::VideoAnalysis,
        pack: &super::generated::Pack,
        id: &str,
    ) -> AppResult<super::generated::PreparedNote> {
        use super::generated::{verify_pack, PreparedNote};
        self.with_vault(id, |s, v| {
            if a.markdown_vault_id.as_deref() == Some(id) {
                if let Some(path) = a.markdown_path.as_deref() {
                    if let Some((base, _)) = path.rsplit_once('/') {
                        if verify_pack(v, base, a, &pack.summary_hash).is_ok() {
                            return Ok(PreparedNote {
                                vault_id: id.into(),
                                target: base.into(),
                                draft: None,
                            });
                        }
                    }
                }
            }
            let parent = format!("{}/{}", s.settings.root_folder, pack.folder);
            ensure_directory(v, &parent)?;
            let mut target = format!("{parent}/{}", pack.version);
            if v.list(&target, 1).is_ok() {
                if verify_pack(v, &target, a, &pack.summary_hash).is_ok() {
                    return Ok(PreparedNote {
                        vault_id: id.into(),
                        target,
                        draft: None,
                    });
                }
                target += &format!("-{}", crate::ffmpeg::runner::new_id());
            }
            let draft = format!("{parent}/Draft-{}", crate::ffmpeg::runner::new_id());
            v.create_dir(&draft)?;
            let result = (|| {
                for (name, text) in &pack.notes {
                    v.create_file(&format!("{draft}/{name}"), text)?;
                }
                for (name, bytes) in &pack.assets {
                    if let Some((dir, _)) = name.rsplit_once('/') {
                        ensure_directory(v, &format!("{draft}/{dir}"))?;
                    }
                    v.create_raster(&format!("{draft}/{name}"), bytes)?;
                }
                Ok(())
            })();
            if let Err(error) = result {
                let _ = v.trash(&draft, "");
                return Err(error);
            }
            Ok(PreparedNote {
                vault_id: id.into(),
                target,
                draft: Some(draft),
            })
        })
    }
    pub(crate) fn publish_prepared(
        &self,
        prepared: &super::generated::PreparedNote,
    ) -> AppResult<()> {
        self.with_vault(&prepared.vault_id, |_, v| {
            if let Some(draft) = &prepared.draft {
                v.move_entry(draft, &prepared.target)?;
            }
            Ok(())
        })
    }
    pub(crate) fn discard_prepared(&self, prepared: &super::generated::PreparedNote) {
        if let Some(draft) = &prepared.draft {
            let _ = self.with_vault(&prepared.vault_id, |_, v| v.trash(draft, "").map(|_| ()));
        }
    }
    pub(crate) fn index_generated(&self, id: &str, base: &str) -> AppResult<()> {
        self.with_vault(id, |s, v| {
            for name in super::generated::NAMES {
                let path = format!("{base}/{name}");
                if let Ok(doc) = v.read(&path) {
                    index::upsert(&mut s.db, &doc)
                        .map_err(|_| "笔记已保存，搜索索引更新失败，请重新扫描知识库".to_owned())?;
                }
            }
            Ok(())
        })
    }
    pub(crate) fn plan_generated_delete(
        &self,
        a: &crate::ai::models::VideoAnalysis,
        id: &str,
    ) -> AppResult<super::generated::DeletePlan> {
        self.with_vault(id, |_, v| super::generated::plan_files(v, a, id))
    }
    pub(crate) fn delete_generated_files(
        &self,
        a: &crate::ai::models::VideoAnalysis,
        id: &str,
        files: &[super::generated::GeneratedFile],
    ) -> AppResult<super::generated::BundleDeleted> {
        self.with_vault(id,|s,v|{let plan=super::generated::plan_files(v,a,id)?;if files!=plan.files||files.len()>9{return Err("笔记已变化或删除清单不一致，请重新预览并确认".into())}
   let mut result=super::generated::BundleDeleted{deleted_paths:vec![],record_deleted:false,warning:plan.warning};
   for file in files{let outcome=if file.path.to_lowercase().ends_with(".md"){v.trash(&file.path,&file.hash)}else{v.trash_raster(&file.path,&file.hash)};match outcome{Ok(_)=>{result.deleted_paths.push(file.path.clone());let _=index::remove(&mut s.db,&file.path);},Err(error)=>{result.warning=Some(format!("部分笔记已移入Vault回收站，分析记录和其他文件保留，请恢复后重试：{error}"));break}}}Ok(result)
  })
    }
    pub(crate) fn find_analysis_note(
        &self,
        id: &str,
        analysis: &str,
        summary_hash: &str,
    ) -> AppResult<Option<super::generated::NoteReference>> {
        self.with_vault(id, |s, v| {
            let mut q =
                s.db.prepare("SELECT path FROM knowledge_index WHERE analysis_id=?1 ORDER BY path")
                    .map_err(db::error)?;
            let paths = q
                .query_map([analysis], |r| r.get::<_, String>(0))
                .map_err(db::error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db::error)?;
            for path in paths {
                if path.ends_with("/视频总结.md") {
                    if let Ok(doc) = v.read(&path) {
                        if super::generated::metadata(&doc, analysis).is_ok_and(|(meta, _)| {
                            meta["summary_hash"].as_str() == Some(summary_hash)
                        }) {
                            return Ok(Some(super::generated::NoteReference {
                                vault_id: id.into(),
                                path,
                            }));
                        }
                    }
                }
            }
            Ok(None)
        })
    }
}

impl KnowledgeService {
    pub(crate) fn prepare_screenshot(
        &self,
        id: &str,
        note_id: &str,
        title: &str,
        item: &str,
        time: i64,
        content: &str,
        jpeg: &[u8],
    ) -> AppResult<super::generated::PreparedNote> {
        use super::generated::{course_folder, hash, time_link, PreparedNote};
        self.with_vault(id,|s,v|{let parent=format!("{}/{}",s.settings.root_folder,course_folder(title,item));ensure_directory(v,&parent)?;
   let target=format!("{parent}/截图-{}",&hash(note_id.as_bytes())[..20]);let draft=format!("{parent}/Draft-{}",crate::ffmpeg::runner::new_id());v.create_dir(&draft)?;
   let attachment=format!("{}/截图.jpg",s.settings.attachments_folder);let body=format!("# {title} · 截图笔记\n\n{}\n\n![[{attachment}]]\n\n{content}\n",time_link(Some(item),time));
   let meta=serde_json::json!({"source":"yingliu","generated_by":"yingliu","note_id":note_id,"item_id":item,"time_ms":time,"title":format!("{title} · 截图笔记"),"generated_body_hash":hash(body.as_bytes())});let text=format!("---\n{}---\n{body}",serde_yaml::to_string(&meta).map_err(|_|"笔记元数据无效")?);
   let result=(||{ensure_directory(v,&format!("{draft}/{}",s.settings.attachments_folder))?;v.create_raster(&format!("{draft}/{attachment}"),jpeg)?;v.create_file(&format!("{draft}/截图笔记.md"),&text)?;Ok(())})();if let Err(error)=result{let _=v.trash(&draft,"");return Err(error)}Ok(PreparedNote{vault_id:id.into(),target,draft:Some(draft)})
  })
    }
    pub(crate) fn index_screenshot(&self, id: &str, path: &str) -> AppResult<()> {
        self.with_vault(id, |s, v| {
            let doc = v.read(path)?;
            index::upsert(&mut s.db, &doc)?;
            Ok(())
        })
    }
}
