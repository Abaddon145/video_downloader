use super::{
    db,
    models::*,
    repository::{id, now, text, Repository},
    service::LearningService,
};
use crate::{
    domain::AppResult,
    ffmpeg::{models::*, output},
};
use rusqlite::{params, OptionalExtension};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};
use tauri::Manager;
pub(crate) fn video_path(p: &str) -> bool {
    Path::new(p).extension().is_some_and(|e| {
        [
            "mp4", "m4v", "mov", "mkv", "avi", "wmv", "asf", "webm", "flv", "mpg", "mpeg", "ts",
            "m2ts", "mts", "3gp", "3g2", "ogv",
        ]
        .contains(&e.to_string_lossy().to_lowercase().as_str())
    })
}
fn prefix(s: &str) -> Option<u64> {
    let s = s.trim().to_lowercase();
    let s = s
        .strip_prefix("lesson")
        .or_else(|| s.strip_prefix("ep"))
        .or_else(|| s.strip_prefix('第'))
        .unwrap_or(&s)
        .trim_start_matches([' ', '.', '_', '-']);
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}
fn natural(a: &str, b: &str) -> Ordering {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let order = if x.is_ascii_digit() && y.is_ascii_digit() {
                    let x: String =
                        std::iter::from_fn(|| a.next_if(|c| c.is_ascii_digit())).collect();
                    let y: String =
                        std::iter::from_fn(|| b.next_if(|c| c.is_ascii_digit())).collect();
                    let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                    x.len().cmp(&y.len()).then(x.cmp(y))
                } else {
                    a.next().cmp(&b.next())
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}
pub fn scan_paths(paths: &[String]) -> AppResult<Vec<String>> {
    let mut files = crate::media::batch::discover_filtered(paths, |p, explicit| {
        explicit || video_path(&p.to_string_lossy())
    })?;
    files.sort_by(|a, b| {
        let a_name = Path::new(a)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let b_name = Path::new(b)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let order = match (prefix(&a_name), prefix(&b_name)) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            _ => Ordering::Equal,
        };
        order.then_with(|| natural(&a_name, &b_name)).then(a.cmp(b))
    });
    Ok(files)
}
pub(crate) fn regular_file(p: &str) -> AppResult<PathBuf> {
    crate::ffmpeg::command::valid_path(p)?;
    let meta = fs::symlink_metadata(p).map_err(|_| "文件已移动或不存在")?;
    if !meta.is_file() || meta.file_attributes() & 0x400 != 0 {
        return Err("只支持本地普通文件，不读取链接或设备".into());
    }
    output::input(p)
}
pub(crate) fn prepare(path: &str, info: &MediaInfo) -> AppResult<PreparedLocal> {
    let path = regular_file(path)?;
    if !info.videos.iter().any(|v| !v.attached_picture) {
        return Err("文件没有可播放的视频流".into());
    }
    let title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let duration = info
        .duration
        .filter(|d| d.is_finite() && *d >= 0. && *d <= 604800.)
        .map(|d| (d * 1000.).round() as i64);
    let path = path.to_string_lossy().into_owned();
    Ok(PreparedLocal {
        key: path.to_lowercase(),
        path,
        title,
        duration_ms: duration,
    })
}
impl Repository {
    pub fn local_duplicates(&self, paths: &[String]) -> AppResult<(usize, Option<String>)> {
        let mut count = 0;
        let mut series = None;
        for p in paths {
            let key = regular_file(p)?.to_string_lossy().to_lowercase();
            let s: Option<String> = self
                .conn
                .query_row(
                    "SELECT series_id FROM series_items WHERE file_key=?1 LIMIT 1",
                    [key],
                    |r| r.get(0),
                )
                .optional()
                .map_err(db::error)?;
            if s.is_some() {
                count += 1;
                if series.is_none() {
                    series = s;
                }
            }
        }
        Ok((count, series))
    }
    pub fn import_local(
        &mut self,
        input: &LocalImportInput,
        files: &[PreparedLocal],
    ) -> AppResult<ImportResult> {
        if files.is_empty() || files.len() > 500 {
            return Err("请选择1至500个可读取的本地视频".into());
        }
        if let Some(id) = input.series_id.as_ref() {
            self.get_series(id)?;
        }
        let title = text(&input.title, 240, input.series_id.is_none())?;
        let mut added = 0;
        let mut duplicates = 0;
        let mut existing_series = None;
        let mut pending = vec![];
        let mut seen = HashSet::new();
        let tx = self.conn.transaction().map_err(db::error)?;
        for f in files {
            if !seen.insert(f.key.clone()) {
                duplicates += 1;
                continue;
            }
            let existing: Option<(String, String)> = tx
                .query_row(
                    "SELECT id,series_id FROM series_items WHERE file_key=?1 LIMIT 1",
                    [&f.key],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(db::error)?;
            if let Some((id, s)) = existing {
                duplicates += 1;
                existing_series.get_or_insert(s);
                tx.execute("UPDATE series_items SET file_path=?1,duration_ms=?2,available=1,download_state='completed',updated_at=?3 WHERE id=?4",params![f.path,f.duration_ms,now(),id]).map_err(db::error)?;
            } else {
                pending.push(f);
            }
        }
        if pending.is_empty() {
            tx.commit().map_err(db::error)?;
            return Ok(ImportResult {
                series_id: input
                    .series_id
                    .clone()
                    .or(existing_series)
                    .unwrap_or_default(),
                added: 0,
                duplicates,
                errors: vec![],
            });
        }
        let series = input.series_id.clone().unwrap_or_else(id);
        if input.series_id.is_none() {
            tx.execute("INSERT INTO series(id,category_id,title,source_type,sort_order,created_at,updated_at) VALUES(?1,?2,?3,'local',(SELECT COALESCE(MAX(sort_order)+1,0) FROM series),?4,?4)",params![series,input.category_id,title,now()]).map_err(db::error)?;
        }
        let next: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(sort_order)+1,0) FROM series_items WHERE series_id=?1",
                [&series],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        for (n, f) in pending.iter().enumerate() {
            tx.execute("INSERT INTO series_items(id,series_id,title,file_path,file_key,duration_ms,sort_order,available,download_state,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,1,'completed',?8,?8)",params![id(),series,f.title,f.path,f.key,f.duration_ms,next+n as i64,now()]).map_err(db::error)?;
            added += 1;
        }
        tx.execute(
            "UPDATE series SET updated_at=?1 WHERE id=?2",
            params![now(), series],
        )
        .map_err(db::error)?;
        tx.commit().map_err(db::error)?;
        Ok(ImportResult {
            series_id: series,
            added,
            duplicates,
            errors: vec![],
        })
    }
    pub fn item_ids(&self, series: &str) -> AppResult<Vec<String>> {
        self.get_series(series)?;
        self.rows(
            "SELECT id FROM series_items WHERE series_id=?1 ORDER BY sort_order,created_at,id",
            [series],
            |r| r.get(0),
        )
    }
    pub fn remove_library_files(&mut self, ids: &[String], confirmed: bool) -> AppResult<usize> {
        if !confirmed {
            return Err("删除本地文件需要再次确认".into());
        }
        if ids.len() > 10000 {
            return Err("删除数量超过10000，请分批操作".into());
        }
        let targets: HashSet<_> = ids.iter().collect();
        let mut files = vec![];
        let mut seen = HashSet::new();
        for id in ids {
            if let Some(p) = self.item(id)?.file_path {
                if !Path::new(&p).exists() {
                    continue;
                }
                let actual = regular_file(&p)?;
                let key = actual.to_string_lossy().to_lowercase();
                let stored: Option<String> = self
                    .conn
                    .query_row("SELECT file_key FROM series_items WHERE id=?1", [id], |r| {
                        r.get(0)
                    })
                    .map_err(db::error)?;
                if stored.as_ref().is_some_and(|k| k != &key) {
                    return Err("文件路径指向已改变，请重新定位后再操作".into());
                }
                let legacy=self.rows("SELECT id,file_path FROM series_items WHERE file_key IS NULL AND file_path IS NOT NULL",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?;
                for (legacy_id, legacy_path) in legacy {
                    if !targets.contains(&legacy_id)
                        && Path::new(&legacy_path).exists()
                        && regular_file(&legacy_path)?.to_string_lossy().to_lowercase() == key
                    {
                        return Err("其他课时仍引用此文件，请仅移除学习库记录".into());
                    }
                }
                let refs = self.rows(
                    "SELECT id FROM series_items WHERE file_key=?1",
                    [&key],
                    |r| r.get::<_, String>(0),
                )?;
                if refs.iter().any(|id| !targets.contains(id)) {
                    return Err("其他课时仍引用此文件，请仅移除学习库记录".into());
                }
                if seen.insert(key) {
                    files.push(actual);
                }
            }
        }
        let mut deleted = 0;
        for file in files {
            if let Err(e) = fs::remove_file(&file) {
                return Err(format!(
                    "已删除{deleted}个文件，但{}删除失败：{e}；记录保留，请检查后重试",
                    file.file_name().unwrap_or_default().to_string_lossy()
                ));
            }
            deleted += 1;
        }
        Ok(deleted)
    }
    pub fn relink(&self, id: &str, file: &PreparedLocal) -> AppResult<()> {
        self.item(id)?;
        let duplicates: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM series_items WHERE file_key=?1 AND id<>?2",
                params![file.key, id],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        if duplicates > 0 {
            return Err("此文件已经属于另一课时，请选择对应视频".into());
        }
        let tx = self.conn.unchecked_transaction().map_err(db::error)?;
        tx.execute("UPDATE series_items SET file_path=?1,file_key=?2,duration_ms=?3,available=1,download_state='completed',updated_at=?4 WHERE id=?5",params![file.path,file.key,file.duration_ms,now(),id]).map_err(db::error)?;
        if let Some(duration) = file.duration_ms {
            tx.execute("UPDATE playback_progress SET duration_ms=?1,position_ms=min(position_ms,?1) WHERE item_id=?2",params![duration,id]).map_err(db::error)?;
        }
        tx.commit().map_err(db::error)
    }
}
impl LearningService {
    pub fn scan(&self, paths: &[String]) -> AppResult<LocalScan> {
        let files = scan_paths(paths)?;
        let suggested = if paths.len() == 1 && Path::new(&paths[0]).is_dir() {
            Path::new(&paths[0])
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        } else {
            files
                .first()
                .map(|p| {
                    Path::new(p)
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                })
                .unwrap_or_else(|| "本地教程".into())
        };
        let (duplicates, existing_series_id) = self.with(|r| r.local_duplicates(&files))?;
        Ok(LocalScan {
            paths: files,
            suggested_title: suggested.chars().take(240).collect(),
            duplicates,
            existing_series_id,
        })
    }
    pub fn import_files(&self, input: LocalImportInput) -> AppResult<ImportResult> {
        let paths = scan_paths(&input.paths)?;
        let mut files = vec![];
        let mut errors = vec![];
        for path in paths {
            match self.media.probe(&path).and_then(|i| prepare(&path, &i)) {
                Ok(f) => files.push(f),
                Err(e) => errors.push(ImportFileError { path, error: e }),
            }
        }
        if files.is_empty() {
            return Ok(ImportResult {
                series_id: input.series_id.unwrap_or_default(),
                added: 0,
                duplicates: 0,
                errors,
            });
        }
        let mut result = self.with(|r| r.import_local(&input, &files))?;
        result.errors = errors;
        self.try_cover(&result.series_id);
        self.changed("import", &result.series_id);
        Ok(result)
    }
    pub fn relink_file(&self, id: &str, path: &str) -> AppResult<LearningItem> {
        let info = self.media.probe(path)?;
        let f = prepare(path, &info)?;
        self.with(|r| r.relink(id, &f))?;
        self.schedule_thumbnails(&[id.into()]);
        self.changed("relink", id);
        self.with(|r| r.item(id))
    }
    pub fn delete_preview(&self, kind: &str, id: &str) -> AppResult<Vec<String>> {
        self.with(|r| {
            let ids = match kind {
                "series" => r.item_ids(id)?,
                "item" => vec![r.item(id)?.id],
                _ => return Err("移除类型无效".into()),
            };
            let mut paths = vec![];
            for id in ids {
                if let Some(p) = r.item(&id)?.file_path {
                    if !paths.contains(&p) {
                        paths.push(p);
                    }
                }
            }
            Ok(paths)
        })
    }
    pub fn open_file(&self, id: &str) -> AppResult<()> {
        let item = self.with(|r| r.item(id))?;
        let file = regular_file(item.file_path.as_deref().ok_or("课程尚未下载")?)?;
        crate::service::shell_open(&file.parent().ok_or("文件目录无效")?.to_string_lossy())
    }
    fn cover_directory(&self) -> AppResult<PathBuf> {
        let root = fs::canonicalize(&self.data).map_err(|_| "数据目录不可访问")?;
        let dir = root.join("learning-covers");
        fs::create_dir_all(&dir).map_err(|_| "无法创建封面缓存")?;
        let dir = fs::canonicalize(dir).map_err(|_| "封面目录不可访问")?;
        if !dir.starts_with(root) {
            return Err("封面目录无效".into());
        }
        Ok(dir)
    }
    pub(crate) fn authorize_cover(&self, path: &str) {
        if !path.starts_with("http://") && !path.starts_with("https://") {
            if let Ok(p) = regular_file(path) {
                let _ = self.app.asset_protocol_scope().allow_file(p);
            }
        }
    }
    pub(crate) fn copy_cover(&self, input: &mut SeriesInput) -> AppResult<()> {
        let Some(path) = input.cover_path.as_ref() else {
            return Ok(());
        };
        if let Some(id) = input.id.as_ref() {
            if self.with(|r| r.get_series(id))?.cover_path.as_deref() == Some(path) {
                return Ok(());
            }
        }
        let file = regular_file(path)?;
        let ext = file
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if !["jpg", "jpeg", "png", "webp"].contains(&ext.as_str())
            || fs::metadata(&file).map_err(|_| "无法读取封面")?.len() > 20 * 1024 * 1024
        {
            return Err("封面仅支持20MiB以内的JPG/PNG/WebP".into());
        }
        let target = self.cover_directory()?.join(format!("{}.{}", id(), ext));
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|_| "无法保存封面")?;
        let mut source = fs::File::open(&file).map_err(|_| "无法打开封面")?;
        std::io::copy(&mut source, &mut output).map_err(|_| "无法复制封面")?;
        self.authorize_cover(&target.to_string_lossy());
        input.cover_path = Some(target.to_string_lossy().into_owned());
        Ok(())
    }
    pub(crate) fn try_cover(&self, series: &str) {
        self.schedule_series_thumbnails(series);
    }
}
