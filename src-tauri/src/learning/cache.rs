use crate::domain::AppResult;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileStamp {
    pub path: String,
    pub size: u64,
    pub modified: String,
}
pub fn stamp(path: &Path) -> AppResult<FileStamp> {
    let p = super::importer::regular_file(&path.to_string_lossy())?;
    let m = fs::metadata(&p).map_err(|_| "无法读取文件信息")?;
    Ok(FileStamp {
        path: p.to_string_lossy().to_lowercase(),
        size: m.len(),
        modified: m
            .modified()
            .map_err(|_| "无法读取文件日期")?
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "文件日期无效")?
            .as_nanos()
            .to_string(),
    })
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn key(s: &FileStamp, codecs: &str, profile: &str) -> String {
    hash(&serde_json::to_vec(&(s, codecs, profile)).unwrap())
}
fn plain(path: &Path) -> AppResult<()> {
    let m = fs::symlink_metadata(path).map_err(|_| "缓存路径无法访问")?;
    if m.file_attributes() & 0x400 != 0 {
        return Err("缓存不允许使用链接目录或文件".into());
    }
    Ok(())
}
pub fn owned(root: &Path, path: &Path) -> AppResult<PathBuf> {
    plain(root)?;
    let root = fs::canonicalize(root).map_err(|_| "缓存目录不存在")?;
    let actual = fs::canonicalize(path).map_err(|_| "缓存文件不存在")?;
    if !actual.starts_with(&root) {
        return Err("文件不属于应用缓存".into());
    }
    // Compare canonical parents so regular and verbatim Windows drive paths both work.
    let mut p = path.to_path_buf();
    loop {
        plain(&p)?;
        if fs::canonicalize(&p).map_err(|_| "缓存无法访问")? == root {
            break;
        }
        if !p.pop() {
            return Err("文件不属于应用缓存".into());
        }
    }
    let p = actual;
    Ok(p)
}
pub fn directory(data: &Path, kind: &str, id: &str) -> AppResult<PathBuf> {
    if !["playback", "thumbnails", "subtitles"].contains(&kind)
        || !(id.is_empty() || id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return Err("缓存标识无效".into());
    }
    let mut p = fs::canonicalize(data).map_err(|_| "数据目录不存在")?;
    for name in ["learning-cache", kind, id]
        .into_iter()
        .filter(|s| !s.is_empty())
    {
        p.push(name);
        if !p.exists() {
            fs::create_dir(&p).map_err(|_| "无法创建缓存目录，请检查空间与权限")?;
        }
        plain(&p)?;
        if !p.is_dir() {
            return Err("缓存目录被文件占用".into());
        }
    }
    Ok(p)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackEntry {
    pub source: FileStamp,
    pub profile: String,
    pub file: String,
    pub output: FileStamp,
    pub last_used: u64,
}
pub fn read_entry(dir: &Path, source: &FileStamp, profile: &str) -> AppResult<PlaybackEntry> {
    let meta = owned(dir, &dir.join("info.json"))?;
    if fs::metadata(&meta).map_err(|_| "缓存不可读")?.len() > 16384 {
        return Err("缓存记录过大".into());
    }
    let e: PlaybackEntry = serde_json::from_slice(&fs::read(meta).map_err(|_| "缓存不可读")?)
        .map_err(|_| "缓存记录无效")?;
    if &e.source != source || e.profile != profile {
        return Err("视频已经改变，需重新准备".into());
    }
    let p = owned(dir, Path::new(&e.file))?;
    if !p.is_file() || stamp(&p)? != e.output {
        return Err("缓存视频已改变".into());
    }
    Ok(e)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    pub playback_bytes: u64,
    pub thumbnail_bytes: u64,
    pub subtitle_bytes: u64,
    pub removed_bytes: u64,
}
pub(crate) fn files(root: &Path) -> AppResult<Vec<(PathBuf, u64)>> {
    let root = owned(root, root)?;
    let mut pending = vec![root.clone()];
    let mut out = vec![];
    let mut visited = 0;
    while let Some(p) = pending.pop() {
        visited += 1;
        if visited > 100000 {
            return Err("缓存文件过多，请分批清理".into());
        }
        if owned(&root, &p).is_err() {
            continue;
        }
        let m = fs::symlink_metadata(&p).map_err(|_| "缓存无法读取")?;
        if m.is_file() {
            out.push((p, m.len()));
        } else if m.is_dir() {
            if let Ok(entries) = fs::read_dir(p) {
                pending.extend(entries.filter_map(Result::ok).map(|e| e.path()));
            }
        }
    }
    Ok(out)
}
impl super::service::LearningService {
    pub fn cache_info(&self) -> AppResult<CacheInfo> {
        let bytes = |kind| -> AppResult<u64> {
            let p = directory(&self.data, kind, "")?;
            Ok(files(&p)?.iter().map(|(_, n)| n).sum())
        };
        Ok(CacheInfo {
            playback_bytes: bytes("playback")?,
            thumbnail_bytes: bytes("thumbnails")?,
            subtitle_bytes: bytes("subtitles")?,
            removed_bytes: 0,
        })
    }
    pub fn clean_cache(&self, kind: &str, manual: bool) -> AppResult<CacheInfo> {
        if !["playback", "thumbnails", "all"].contains(&kind) {
            return Err("缓存清理类型无效".into());
        }
        let settings = self.with(|r| r.settings())?;
        let source_files = self
            .with(|r| {
                r.rows(
                    "SELECT file_path FROM series_items WHERE file_path IS NOT NULL",
                    [],
                    |r| r.get::<_, String>(0),
                )
            })?
            .into_iter()
            .filter_map(|p| fs::canonicalize(p).ok())
            .collect::<Vec<_>>();
        let _sub = self.subtitle_guard.lock().map_err(|_| "字幕缓存暂不可用")?;
        let state = self.playback.lock().map_err(|_| "播放缓存暂不可用")?;
        let protected = state.protected();
        let mut removed = 0;
        if kind != "thumbnails" {
            for name in ["playback", "subtitles"] {
                let root = directory(&self.data, name, "")?;
                let mut entries = vec![];
                for dir in fs::read_dir(&root)
                    .map_err(|_| "缓存无法读取")?
                    .filter_map(Result::ok)
                {
                    let p = dir.path();
                    let key = dir.file_name().to_string_lossy().into_owned();
                    if protected.contains(&key)
                        || key.len() != 64
                        || !key.chars().all(|c| c.is_ascii_hexdigit())
                        || owned(&root, &p).is_err()
                        || !p.is_dir()
                        || source_files.iter().any(|source| source.starts_with(&p))
                    {
                        continue;
                    }
                    let selected = files(&p)?;
                    let size: u64 = selected.iter().map(|(_, n)| n).sum();
                    let used = fs::read(p.join("info.json"))
                        .ok()
                        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                        .and_then(|e| e["lastUsed"].as_u64())
                        .unwrap_or_else(|| {
                            fs::metadata(&p)
                                .and_then(|m| m.modified())
                                .ok()
                                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                                .map(|t| t.as_secs())
                                .unwrap_or(crate::service::now())
                        });
                    entries.push((used, p, selected, size));
                }
                entries.sort_by_key(|e| e.0);
                let mut total = files(&root)?.iter().map(|(_, n)| n).sum::<u64>();
                for (used, p, selected, size) in entries {
                    let expired = settings.playback_cache_days > 0
                        && crate::service::now().saturating_sub(used)
                            > settings.playback_cache_days as u64 * 86400;
                    let excess = total > settings.playback_cache_gb as u64 * 1024 * 1024 * 1024;
                    if manual || expired || excess {
                        for (file, n) in selected {
                            if owned(&root, &file).is_ok() && fs::remove_file(&file).is_ok() {
                                removed += n;
                            }
                        }
                        let _ = fs::remove_dir(&p);
                        total = total.saturating_sub(size);
                    }
                }
            }
        }
        drop(state);
        drop(_sub);
        // Thumbnail reference-aware cleanup is integrated with its worker, independently of playback leases.
        let mut info = self.cache_info()?;
        info.removed_bytes = removed;
        Ok(info)
    }
}
