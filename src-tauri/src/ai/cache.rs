use super::service::AiService;
use crate::{
    domain::AppResult,
    learning::{cache::owned, db},
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    pub bytes: u64,
    pub clearable_bytes: u64,
}
struct CachedFile {
    path: PathBuf,
    bytes: u64,
    clearable: bool,
}
fn files(root: &Path, referenced: &HashSet<PathBuf>) -> AppResult<Vec<CachedFile>> {
    let mut result = vec![];
    if !root.exists() {
        return Ok(result);
    }
    let root = root.canonicalize().map_err(|_| "无法读取AI缓存")?;
    let mut pending = vec![(root.clone(), 0)];
    let mut scanned = 0;
    while let Some((directory, depth)) = pending.pop() {
        if depth > 8 {
            return Err("AI缓存目录过深，请保留原目录后检查".into());
        }
        for entry in std::fs::read_dir(directory).map_err(|_| "无法读取AI缓存目录")? {
            scanned += 1;
            if scanned > 100000 {
                return Err("AI缓存文件过多，请分批删除分析记录".into());
            }
            let entry = entry.map_err(|_| "无法读取AI缓存文件")?;
            let path = entry.path();
            let Ok(path) = owned(&root, &path) else {
                continue;
            };
            let meta = std::fs::metadata(&path).map_err(|_| "无法读取AI缓存信息")?;
            if meta.is_dir() {
                pending.push((path, depth + 1));
                continue;
            }
            if !meta.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let temporary = ["audio.wav", "transcript.json", "model.bin"].contains(&name)
                || name.ends_with(".partial");
            let unused_frame = path
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|n| n == "keyframes")
                && path.extension().is_some_and(|e| e == "jpg")
                && !referenced.contains(&path);
            result.push(CachedFile {
                path,
                bytes: meta.len(),
                clearable: temporary || unused_frame,
            });
        }
    }
    Ok(result)
}
impl AiService {
    fn cache_files(&self) -> AppResult<Vec<CachedFile>> {
        let referenced = self.with(|r| {
            let mut stmt = r
                .conn
                .prepare("SELECT path FROM analysis_keyframes")
                .map_err(db::error)?;
            let paths = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(db::error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db::error)?;
            Ok(paths
                .into_iter()
                .filter_map(|p| Path::new(&p).canonicalize().ok())
                .collect::<HashSet<_>>())
        })?;
        files(&self.data.join("ai/cache"), &referenced)
    }
    pub fn cache_info(&self) -> AppResult<CacheInfo> {
        let files = self.cache_files()?;
        Ok(CacheInfo {
            bytes: files.iter().map(|f| f.bytes).sum(),
            clearable_bytes: files.iter().filter(|f| f.clearable).map(|f| f.bytes).sum(),
        })
    }
    pub fn clear_cache(&self) -> AppResult<CacheInfo> {
        let runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
        if runtime.active.is_some() || !runtime.pending.is_empty() {
            return Err("请先完成或取消分析，再清理临时文件".into());
        }
        let root = self.data.join("ai/cache");
        for file in self.cache_files()?.into_iter().filter(|f| f.clearable) {
            let path = owned(&root, &file.path)?;
            std::fs::remove_file(path).map_err(|_| "部分临时文件正被使用，请稍后重试")?;
        }
        self.cache_info()
    }
    pub fn delete_analysis(&self, id: &str) -> AppResult<Option<String>> {
        let runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
        if runtime
            .active
            .as_ref()
            .is_some_and(|(active, _)| active == id)
            || runtime.pending.iter().any(|p| p == id)
            || self.question_busy.load(Ordering::SeqCst)
        {
            return Err("请先结束该分析或当前问答，再删除记录".into());
        }
        if id.len() > 128
            || id.is_empty()
            || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err("分析记录标识无效".into());
        }
        self.with(|r| {
            r.get(id)?;
            let tx = r.conn.transaction().map_err(db::error)?;
            let fts = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='transcript_fts')",
                    [],
                    |r| r.get::<_, bool>(0),
                )
                .map_err(db::error)?;
            if fts {
                tx.execute("DELETE FROM transcript_fts WHERE analysis_id=?1", [id])
                    .map_err(db::error)?;
            }
            tx.execute("DELETE FROM video_analyses WHERE id=?1", [id])
                .map_err(db::error)?;
            tx.commit().map_err(db::error)
        })?;
        let root = self.data.join("ai/cache");
        let cache = root.join(id);
        let warning = if cache.exists() {
            match owned(&root, &cache)
                .and_then(|p| std::fs::remove_dir_all(p).map_err(|_| "缓存正在使用".into()))
            {
                Ok(()) => None,
                Err(_) => Some("记录已删除，部分临时缓存未能清理；源视频和知识笔记已保留".into()),
            }
        } else {
            None
        };
        Ok(warning)
    }
}
