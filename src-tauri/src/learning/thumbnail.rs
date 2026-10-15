use super::{cache, db, models::*, repository::Repository, service::LearningService};
use crate::{
    domain::AppResult,
    ffmpeg::{command, models::*},
};
use rusqlite::params;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::Path,
    sync::Arc,
    time::Duration,
};
use tauri::Manager;
pub fn sample(duration: f64) -> f64 {
    let t = if !duration.is_finite() || duration <= 0. {
        0.1
    } else if duration >= 30. {
        duration * 0.12
    } else if duration >= 5. {
        duration * 0.25
    } else {
        duration * 0.5
    };
    (t * 1000.).round() / 1000.
}
pub(crate) fn accurate_seek(args: &mut Vec<String>) -> bool {
    let Some(n) = args.iter().position(|a| a == "-ss") else {
        return false;
    };
    if n + 1 >= args.len() {
        return false;
    }
    let pair = args.drain(n..n + 2).collect::<Vec<_>>();
    let Some(i) = args.iter().position(|a| a == "-i") else {
        return false;
    };
    args.splice(i + 2..i + 2, pair);
    true
}
#[derive(Default)]
pub(crate) struct ThumbnailQueue {
    queue: VecDeque<String>,
    pending: HashSet<String>,
}
impl ThumbnailQueue {
    pub(crate) fn add(&mut self, id: &str) {
        if self.pending.len() < 10000 && self.pending.insert(id.into()) {
            self.queue.push_back(id.into());
        }
    }
    pub(crate) fn take(&mut self) -> Option<String> {
        self.queue.pop_front()
    }
    pub(crate) fn finish(&mut self, id: &str) {
        if !self.queue.iter().any(|queued| queued == id) {
            self.pending.remove(id);
        }
    }
}
#[derive(Default)]
pub(crate) struct ThumbnailState {
    queue: ThumbnailQueue,
    memo: HashMap<String, String>,
    forced: HashSet<String>,
}
impl Repository {
    pub(crate) fn set_thumbnail_cas(
        &self,
        old: &LearningItem,
        path: &str,
        source: &str,
        key: &str,
    ) -> AppResult<bool> {
        if !["manual", "remote", "generated"].contains(&source) {
            return Err("缩略图来源无效".into());
        }
        let n=self.conn.execute("UPDATE series_items SET thumbnail_path=?1,thumbnail_source=?2,thumbnail_key=?3 WHERE id=?4 AND file_path IS ?5 AND thumbnail_path IS ?6 AND thumbnail_source IS ?7 AND source_thumbnail_url IS ?8 AND thumbnail_key IS ?9",params![path,source,key,old.id,old.file_path,old.thumbnail_path,old.thumbnail_source,old.source_thumbnail_url,old.thumbnail_key]).map_err(db::error)?;
        Ok(n == 1)
    }
    pub(crate) fn thumbnail_references(&self) -> AppResult<HashSet<String>> {
        Ok(self.rows("SELECT thumbnail_path FROM series_items WHERE thumbnail_path IS NOT NULL UNION SELECT cover_path FROM series WHERE cover_path IS NOT NULL UNION SELECT file_path FROM series_items WHERE file_path IS NOT NULL",[],|r|r.get::<_,String>(0))?.into_iter().filter_map(|p|fs::canonicalize(p).ok()).map(|p|p.to_string_lossy().to_lowercase()).collect())
    }
}
impl LearningService {
    pub(crate) fn schedule_thumbnails(&self, ids: &[String]) {
        if let Ok(mut state) = self.thumbnails.0.lock() {
            for id in ids {
                state.queue.add(id);
            }
            self.thumbnails.1.notify_one();
        }
    }
    pub(crate) fn schedule_series_thumbnails(&self, series: &str) {
        if let Ok(ids) = self.with(|r| r.item_ids(series)) {
            self.schedule_thumbnails(&ids);
        }
    }
    fn thumbnail_signature(&self, item: &LearningItem) -> String {
        cache::hash(
            &serde_json::to_vec(&(
                item.file_path
                    .as_ref()
                    .and_then(|p| cache::stamp(Path::new(p)).ok()),
                &item.source_thumbnail_url,
                &item.thumbnail_source,
                &item.thumbnail_path,
                &item.thumbnail_key,
            ))
            .unwrap_or_default(),
        )
    }
    pub(crate) fn start_thumbnails(self: &Arc<Self>) {
        let this = self.clone();
        std::thread::spawn(move || {
            let mut cleaned = 0;
            loop {
                if this.media.state.lock().unwrap().exiting {
                    break;
                }
                if this.media.ensure_ready().is_err() {
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                }
                if crate::service::now().saturating_sub(cleaned) > 3600 {
                    let _ = this.clean_cache("playback", false);
                    cleaned = crate::service::now();
                }
                let id = {
                    let mut state = this.thumbnails.0.lock().unwrap();
                    if let Some(id) = state.queue.take() {
                        Some(id)
                    } else {
                        let (state, _) = this
                            .thumbnails
                            .1
                            .wait_timeout(state, Duration::from_secs(1))
                            .unwrap();
                        drop(state);
                        None
                    }
                };
                let Some(id) = id else {
                    continue;
                };
                if let Ok(item) = this.with(|r| r.item(&id)) {
                    let signature = this.thumbnail_signature(&item);
                    let forced = this.thumbnails.0.lock().unwrap().forced.remove(&id);
                    let memo = this.thumbnails.0.lock().unwrap().memo.get(&id).cloned();
                    if forced || memo.as_deref() != Some(&signature) {
                        let _ = this.resolve_thumbnail(&item, forced);
                        if let Ok(latest) = this.with(|r| r.item(&id)) {
                            let signature = this.thumbnail_signature(&latest);
                            let mut state = this.thumbnails.0.lock().unwrap();
                            if state.memo.len() >= 10000 {
                                state.memo.clear();
                            }
                            state.memo.insert(id.clone(), signature);
                        }
                    }
                }
                this.thumbnails.0.lock().unwrap().queue.finish(&id);
            }
        });
    }
    fn thumbnail_file(&self, path: &str) -> bool {
        cache::directory(&self.data, "thumbnails", "")
            .ok()
            .is_some_and(|root| cache::owned(&root, Path::new(path)).is_ok_and(|p| p.is_file()))
    }
    pub(crate) fn authorize_thumbnail(&self, item: &mut LearningItem) {
        if let Some(path) = item.thumbnail_path.as_ref() {
            if self.thumbnail_file(path) {
                let _ = self.app.asset_protocol_scope().allow_file(path);
            } else {
                item.thumbnail_path = None;
            }
        }
    }
    fn thumbnail_key(&self, item: &LearningItem, kind: &str) -> AppResult<String> {
        match kind {
            "remote" => Ok(cache::hash(
                &serde_json::to_vec(&(
                    &item.source_url,
                    &item.source_video_id,
                    &item.source_thumbnail_url,
                    "remote-320x180-v1",
                ))
                .map_err(|_| "图片来源无效")?,
            )),
            "generated" => Ok(cache::key(
                &cache::stamp(Path::new(item.file_path.as_deref().ok_or("课程未下载")?))?,
                "source-frame",
                "320x180-v1",
            )),
            _ => Err("图片来源无效".into()),
        }
    }
    fn image(&self, input: &Path, target: &Path, seek: Option<f64>) -> AppResult<()> {
        let input = super::importer::regular_file(&input.to_string_lossy())?;
        let info = self.media.probe(&input.to_string_lossy())?;
        let request = MediaRequest {
            input_path: input.to_string_lossy().into(),
            output_dir: target
                .parent()
                .ok_or("图片目录无效")?
                .to_string_lossy()
                .into(),
            operation: MediaOperation::Screenshot,
            output_format: OutputFormat::Jpg,
            start: seek.unwrap_or(0.),
            ..Default::default()
        };
        let temp = target.with_file_name(format!("{}.jpg", super::repository::id()));
        let mut c = command::build(&request, &info, &temp)?;
        c.args.splice(c.args.len()-1..c.args.len()-1,["-vf".into(),"scale=320:180:force_original_aspect_ratio=decrease,pad=320:180:(ow-iw)/2:(oh-ih)/2,setsar=1".into()]);
        if let Some(n) = c.args.iter().position(|a| a == "-q:v") {
            c.args[n + 1] = "3".into();
        }
        let session = format!("thumb-{}", super::repository::id());
        let result: AppResult<()> = (|| {
            if let Err(first) = self.media.auxiliary(&session, &c.args, 30) {
                // Sparse TS/M2TS keyframes can leave input seeking with no decodable frame.
                if seek.is_none() || !accurate_seek(&mut c.args) {
                    return Err(first);
                }
                if temp.exists() {
                    cache::owned(temp.parent().unwrap(), &temp)?;
                    fs::remove_file(&temp).map_err(|_| "无法重试缩略图")?;
                }
                self.media.auxiliary(&session, &c.args, 30)?;
            }
            let generated = self.media.probe(&temp.to_string_lossy())?;
            if !generated
                .videos
                .iter()
                .any(|v| v.width == Some(320) && v.height == Some(180))
            {
                return Err("缩略图尺寸无效".into());
            }
            if target.exists() {
                cache::owned(target.parent().unwrap(), target)?;
                fs::remove_file(target).map_err(|_| "无法更新缩略图")?;
            }
            fs::rename(&temp, target).map_err(|_| "无法保存缩略图")?;
            Ok(())
        })();
        let _ = fs::remove_file(temp);
        result
    }
    fn resolve_thumbnail(&self, old: &LearningItem, force: bool) -> AppResult<()> {
        let _single = self.thumbnail_guard.lock().map_err(|_| "缩略图暂不可用")?;
        let current = self.with(|r| r.item(&old.id))?;
        if current.thumbnail_source.as_deref() == Some("manual")
            && current
                .thumbnail_path
                .as_ref()
                .is_some_and(|p| self.thumbnail_file(p))
        {
            return Ok(());
        }
        // Keep the existing image when the video goes missing; relinking changes its fingerprint.
        if current
            .file_path
            .as_ref()
            .is_some_and(|p| !Path::new(p).exists())
            && current
                .thumbnail_path
                .as_ref()
                .is_some_and(|p| self.thumbnail_file(p))
        {
            return Ok(());
        }
        for kind in ["remote", "generated"] {
            if kind == "remote" && current.source_thumbnail_url.is_none()
                || kind == "generated" && current.file_path.is_none()
            {
                continue;
            }
            let Ok(key) = self.thumbnail_key(&current, kind) else {
                continue;
            };
            if current.thumbnail_source.as_deref() == Some(kind)
                && current.thumbnail_key.as_deref() == Some(&key)
                && current
                    .thumbnail_path
                    .as_ref()
                    .is_some_and(|p| self.thumbnail_file(p))
            {
                return Ok(());
            }
            let dir = cache::directory(&self.data, "thumbnails", &key)?;
            let target = dir.join("image.jpg");
            let result: AppResult<()> = (|| {
                if force || cache::owned(&dir, &target).is_err() {
                    if kind == "remote" {
                        let service = self
                            .app
                            .try_state::<Arc<crate::service::Service>>()
                            .ok_or("网络组件未准备好")?;
                        let bytes = service
                            .download_thumbnail(current.source_thumbnail_url.as_deref().unwrap())?;
                        let input = dir.join(format!("remote-{}.image", super::repository::id()));
                        fs::write(&input, bytes).map_err(|_| "无法保存远程缩略图")?;
                        let result = self.image(&input, &target, None);
                        let _ = fs::remove_file(input);
                        result?;
                    } else {
                        let source = Path::new(current.file_path.as_deref().unwrap());
                        let before = cache::stamp(source)?;
                        let info = self.media.probe(&source.to_string_lossy())?;
                        self.image(source, &target, Some(sample(info.duration.unwrap_or(0.))))?;
                        if cache::stamp(source)? != before
                            || self.thumbnail_key(&current, kind)? != key
                        {
                            return Err("源视频已经改变，缩略图已丢弃".into());
                        }
                    }
                }
                if self.with(|r| {
                    r.set_thumbnail_cas(&current, &target.to_string_lossy(), kind, &key)
                })? {
                    self.authorize_cover(&target.to_string_lossy());
                    let _=self.with(|r|{r.conn.execute("UPDATE series SET cover_path=?1 WHERE id=?2 AND cover_path IS NULL",params![target.to_string_lossy(),current.series_id]).map_err(db::error)?;Ok(())});
                    self.changed("thumbnail", &current.id);
                }
                Ok(())
            })();
            if result.is_ok() {
                return Ok(());
            }
        }
        Ok(())
    }
    pub fn set_thumbnail(&self, id: &str, path: &str) -> AppResult<LearningItem> {
        let _single = self.thumbnail_guard.lock().map_err(|_| "缩略图暂不可用")?;
        let old = self.with(|r| r.item(id))?;
        let input = super::importer::regular_file(path)?;
        if !input.extension().is_some_and(|e| {
            ["jpg", "jpeg", "png", "webp"]
                .contains(&e.to_string_lossy().to_ascii_lowercase().as_str())
        }) || fs::metadata(&input).map_err(|_| "图片无法读取")?.len() > 20 * 1024 * 1024
        {
            return Err("请选择20MiB以内的JPEG、PNG或WebP图片".into());
        }
        let before = cache::stamp(&input)?;
        let key = cache::key(&before, "manual", "320x180-v1");
        let dir = cache::directory(&self.data, "thumbnails", &key)?;
        let target = dir.join("image.jpg");
        if cache::owned(&dir, &target).is_err() {
            self.image(&input, &target, None)?;
        }
        if cache::stamp(&input)? != before {
            return Err("原图片已经改变，请重新选择".into());
        }
        if !self.with(|r| r.set_thumbnail_cas(&old, &target.to_string_lossy(), "manual", &key))? {
            return Err("课程图片已经改变，请重新选择".into());
        }
        self.changed("thumbnail", id);
        self.with(|r| r.item(id))
    }
    pub fn regenerate_thumbnail(&self, id: &str, confirmed: bool) -> AppResult<()> {
        let _single = self.thumbnail_guard.lock().map_err(|_| "缩略图暂不可用")?;
        let old = self.with(|r| r.item(id))?;
        if old.thumbnail_source.as_deref() == Some("manual") && !confirmed {
            return Err("重新生成将替换手动图片，请先确认".into());
        }
        // Clearing the binding allows the same source-frame cache to be reused without modifying the user's image.
        self.with(|r|{r.conn.execute("UPDATE series_items SET thumbnail_path=NULL,thumbnail_source=NULL,thumbnail_key=NULL WHERE id=?1",[id]).map_err(db::error)?;Ok(())})?;
        let mut state = self.thumbnails.0.lock().map_err(|_| "缩略图暂不可用")?;
        state.memo.remove(id);
        state.forced.insert(id.into());
        state.queue.queue.retain(|queued| queued != id);
        state.queue.pending.remove(id);
        state.queue.add(id);
        self.thumbnails.1.notify_one();
        drop(state);
        self.changed("thumbnail", id);
        Ok(())
    }
    pub(crate) fn clean_thumbnails(&self) -> AppResult<u64> {
        let _single = self.thumbnail_guard.lock().map_err(|_| "缩略图暂不可用")?;
        let references = self.with(|r| r.thumbnail_references())?;
        let root = cache::directory(&self.data, "thumbnails", "")?;
        let mut removed = 0;
        for (file, n) in cache::files(&root)? {
            if !references.contains(&file.to_string_lossy().to_lowercase())
                && cache::owned(&root, &file).is_ok()
            {
                fs::remove_file(&file).map_err(|_| "缓存图片仍被使用，请稍后重试")?;
                removed += n;
            }
        }
        Ok(removed)
    }
}
