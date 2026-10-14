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
