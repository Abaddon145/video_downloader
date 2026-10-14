use super::{
    db,
    importer::{prepare, regular_file, video_path},
    models::*,
    repository::{id, now, optional, text, Repository},
    service::LearningService,
};
use crate::{
    domain::{validate_url, AppResult},
    models::{DownloadRequest, DownloadTask, MediaKind, TaskStatus},
    service::Service,
};
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;
pub fn request_key(r: &DownloadRequest) -> String {
    serde_json::json!([
        r.url,
        r.kind,
        r.video_mode,
        r.max_height,
        r.subtitle_languages,
        r.filename_template,
        r.by_author,
        r.playlist_index
    ])
    .to_string()
}
fn request(
    template: &DownloadRequest,
    e: &crate::models::PlaylistEntry,
) -> AppResult<DownloadRequest> {
    let mut r = template.clone();
    r.url = validate_url(&e.url)?;
    r.title = text(&e.title, 500, false)?;
    if r.title.is_empty() {
        r.title = "未命名课程".into();
    }
    r.thumbnail = e.thumbnail.clone();
    r.playlist_index = Some(e.playlist_index);
    crate::engine::download_args(&r, "C:/validation")?;
    Ok(r)
}
impl Repository {
    pub fn stage_playlists(&mut self, imports: &[PlaylistImport]) -> AppResult<Vec<ImportResult>> {
        if imports.is_empty()
            || imports.len() > 20
            || imports
                .iter()
                .map(|i| i.preview.entries.len())
                .sum::<usize>()
                > 10000
        {
            return Err("一次最多20个播放列表、10000课".into());
        }
        for input in imports {
            if !input.preview.is_playlist
                || input.preview.entries.is_empty()
                || input.preview.entries.len() > 1000
                || input.template.kind != MediaKind::Video
            {
                return Err("学习库需要1至1000课的视频播放列表".into());
            }
            if let Some(s) = &input.series_id {
                self.get_series(s)?;
            }
            for e in &input.preview.entries {
                request(&input.template, e)?;
            }
        }
        let tx = self.conn.transaction().map_err(db::error)?;
        let mut results = vec![];
        for input in imports {
            let url = validate_url(&input.preview.url)?;
            let title = text(&input.title, 240, true)?;
            let site = text(&input.preview.site, 80, false)?;
            let author = text(&input.preview.uploader, 500, false)?;
            let category = optional(&input.category_id, 100)?;
            let existing: Option<String> = tx
                .query_row(
                    "SELECT id FROM series WHERE source_url=?1 ORDER BY created_at,id LIMIT 1",
                    [&url],
                    |r| r.get(0),
                )
                .optional()
                .map_err(db::error)?;
            let series = input.series_id.clone().or(existing).unwrap_or_else(id);
            let cover = input
                .preview
                .thumbnail
                .as_ref()
                .and_then(|p| validate_url(p).ok());
            let source_playlist_id = optional(&input.preview.source_playlist_id, 500)?;
            tx.execute("INSERT OR IGNORE INTO series(id,category_id,title,cover_path,source_type,source_url,source_playlist_id,author,sort_order,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,(SELECT COALESCE(MAX(sort_order)+1,0) FROM series),?9,?9)",params![series,category,title,cover,site,url,source_playlist_id,author,now()]).map_err(db::error)?;
            let mut entries = input.preview.entries.clone();
            if input.keep_order {
                entries.sort_by_key(|e| e.playlist_index);
            }
            let mut added = 0;
            let mut duplicates = 0;
            let next: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(sort_order)+1,0) FROM series_items WHERE series_id=?1",
                    [&series],
                    |r| r.get(0),
                )
                .map_err(db::error)?;
            for entry in entries {
                let request = request(&input.template, &entry)?;
                let video_id = optional(&Some(entry.id.clone()), 500)?;
                let key = video_id
                    .as_ref()
                    .map(|id| format!("id:{site}:{id}"))
                    .unwrap_or_else(|| format!("url:{}", request.url));
                let existing:Option<String>=tx.query_row("SELECT id FROM series_items WHERE series_id=?1 AND (source_key=?2 OR source_url=?3 OR (source_video_id IS NOT NULL AND source_video_id=?4))",params![series,key,request.url,video_id],|r|r.get(0)).optional().map_err(db::error)?;
                if existing.is_some() {
                    duplicates += 1;
                    continue;
                }
                let duration = entry
                    .duration
                    .filter(|d| d.is_finite() && *d >= 0. && *d <= 604800.)
                    .map(|d| (d * 1000.).round() as i64);
                let frozen = serde_json::to_string(&request).map_err(|_| "下载参数无法保存")?;
                tx.execute("INSERT INTO series_items(id,series_id,title,source_url,source_video_id,source_playlist_index,duration_ms,sort_order,thumbnail_path,available,source_key,download_request,request_key,download_state,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,0,?10,?11,?12,'waiting',?13,?13)",params![id(),series,request.title,request.url,video_id,entry.playlist_index,duration,next+added as i64,request.thumbnail,key,frozen,request_key(&request),now()]).map_err(db::error)?;
                added += 1;
            }
            tx.execute("UPDATE series SET updated_at=?1,source_url=COALESCE(source_url,?3),source_type=COALESCE(source_type,?4),source_playlist_id=COALESCE(source_playlist_id,?5),author=COALESCE(author,?6),cover_path=COALESCE(cover_path,?7) WHERE id=?2",params![now(),series,url,site,source_playlist_id,author,cover]).map_err(db::error)?;
            results.push(ImportResult {
                series_id: series,
                added,
                duplicates,
                errors: vec![],
            });
        }
        tx.commit().map_err(db::error)?;
        Ok(results)
    }
    pub fn download_request(&self, id: &str) -> AppResult<DownloadRequest> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT download_request FROM series_items WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db::error)?
            .flatten();
        serde_json::from_str(
            json.as_deref()
                .ok_or("此课程没有来源下载参数，请重新定位本地文件")?,
        )
        .map_err(|_| "课程下载参数损坏，请保留数据库".into())
    }
    pub fn source_item(
        &self,
        series: &str,
        entry: &crate::models::PlaylistEntry,
    ) -> AppResult<LearningItem> {
        let id:String=self.conn.query_row("SELECT id FROM series_items WHERE series_id=?1 AND ((source_video_id IS NOT NULL AND source_video_id=?2) OR source_url=?3) LIMIT 1",params![series,entry.id,validate_url(&entry.url)?],|r|r.get(0)).map_err(db::error)?;
        self.item(&id)
    }
    pub fn matching_items(&self, task: &DownloadTask) -> AppResult<Vec<String>> {
        self.rows("SELECT id FROM series_items WHERE download_task_id=?1 OR (download_task_id IS NULL AND available=0 AND request_key=?2 AND created_at<=?3)",params![task.id,request_key(&task.request),task.created_at as i64],|r|r.get(0))
    }
    pub fn link_task(&self, task: &DownloadTask) -> AppResult<()> {
        self.conn.execute("UPDATE series_items SET download_task_id=?1 WHERE available=0 AND request_key=?2 AND created_at<=?3",params![task.id,request_key(&task.request),task.created_at as i64]).map_err(db::error)?;
        Ok(())
    }
    pub fn apply_download(
        &self,
        task: &DownloadTask,
        file: Option<&PreparedLocal>,
    ) -> AppResult<Vec<String>> {
        let ids = self.matching_items(task)?;
        let mut series = vec![];
        let state = match task.status {
            TaskStatus::Completed if file.is_none() => "missing",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Paused => "paused",
            TaskStatus::Queued => "queued",
            _ => "downloading",
        };
        let tx = self.conn.unchecked_transaction().map_err(db::error)?;
        for id in ids {
            let item = self.item(&id)?;
            if !series.contains(&item.series_id) {
                series.push(item.series_id.clone());
            }
            tx.execute("UPDATE series_items SET download_task_id=?1,download_state=?2,updated_at=?3 WHERE id=?4",params![task.id,state,now(),id]).map_err(db::error)?;
            if let Some(f) = file {
                tx.execute("UPDATE series_items SET file_path=?1,file_key=?2,duration_ms=COALESCE(?3,duration_ms),available=1 WHERE id=?4",params![f.path,f.key,f.duration_ms,id]).map_err(db::error)?;
            }
        }
        tx.commit().map_err(db::error)?;
        Ok(series)
    }
    pub fn pending_downloads(
        &self,
    ) -> AppResult<Vec<(String, Option<String>, Option<String>, i64)>> {
        self.rows("SELECT id,download_task_id,request_key,created_at FROM series_items WHERE available=0 AND download_request IS NOT NULL",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))
    }
}
impl LearningService {
    pub fn on_download_task(&self, task: &DownloadTask) -> AppResult<()> {
        if task.request.kind != MediaKind::Video {
            return Ok(());
        }
        let ids = self.with(|r| r.matching_items(task))?;
        if ids.is_empty() {
            return Ok(());
        }
        let mut file = None;
        if task.status == TaskStatus::Completed {
            let root = std::fs::canonicalize(&task.output_dir).ok();
            for path in task.files.iter().filter(|p| video_path(p)) {
                if let Ok(p) = regular_file(path) {
                    if root.as_ref().is_some_and(|root| p.starts_with(root)) {
                        if let Ok(info) = self.media.probe(&p.to_string_lossy()) {
                            if let Ok(prepared) = prepare(&p.to_string_lossy(), &info) {
                                file = Some(prepared);
                                break;
                            }
                        }
                    }
                }
            }
        }
        let result = self.with(|r| r.apply_download(task, file.as_ref()));
        let series = match result {
            Ok(s) => s,
            Err(e) => {
                if file.is_some() {
                    let _ = self.with(|r| r.apply_download(task, None));
                    self.changed("download", &task.id);
                }
                return Err(e);
            }
        };
        if file.is_some() {
            for id in &series {
                self.try_cover(id);
            }
        }
        self.changed("download", &task.id);
        Ok(())
    }
    pub fn enqueue_playlists(
        &self,
        service: &Arc<Service>,
        requests: Vec<DownloadRequest>,
        imports: Vec<PlaylistImport>,
    ) -> AppResult<LearningEnqueueResult> {
        if requests.len() > 1000 {
            return Err("一次最多1000个下载任务".into());
        }
        if imports.is_empty() {
            return Err("请选择要加入学习库的播放列表".into());
        }
        let staged = self.with(|r| r.stage_playlists(&imports))?;
        let mut queue = vec![];
        for request in requests {
            let mut associated = None;
            for (input, result) in imports.iter().zip(&staged) {
                if let Some(e) = input.preview.entries.iter().find(|e| e.url == request.url) {
                    associated = Some(self.with(|r| r.source_item(&result.series_id, e))?);
                    break;
                }
            }
            if let Some(item) = associated {
                if self.with(|r| r.playback_file(&item.id)).is_ok() {
                    continue;
                }
                queue.push(self.with(|r| r.download_request(&item.id))?);
            } else {
                queue.push(request);
            }
        }
        let tasks = if queue.is_empty() {
            vec![]
        } else {
            service.enqueue(queue.clone())?
        };
        let snapshot = service.snapshot();
        for request in &queue {
            let task = snapshot
                .tasks
                .iter()
                .filter(|t| {
                    request_key(&t.request) == request_key(request)
                        && (tasks.iter().any(|n| n.id == t.id)
                            || t.status.active()
                            || matches!(t.status, TaskStatus::Queued | TaskStatus::Paused))
                })
                .max_by_key(|t| t.created_at);
            if let Some(task) = task {
                self.with(|r| r.link_task(task))?;
                self.on_download_task(task)?;
            }
        }
        self.changed("import", "");
        Ok(LearningEnqueueResult {
            tasks,
            series_ids: staged.iter().map(|s| s.series_id.clone()).collect(),
            added: staged.iter().map(|s| s.added).sum(),
            duplicates: staged.iter().map(|s| s.duplicates).sum(),
        })
    }
    pub fn download_item(&self, service: &Arc<Service>, id: &str) -> AppResult<()> {
        let item = self.with(|r| r.item(id))?;
        if item.available && self.with(|r| r.playback_file(id)).is_ok() {
            return Ok(());
        }
        let snapshot = service.snapshot();
        if let Some(task) = item
            .download_task_id
            .as_ref()
            .and_then(|id| snapshot.tasks.iter().find(|t| &t.id == id))
        {
            match task.status {
                TaskStatus::Failed | TaskStatus::Cancelled => {
                    return service.control(&task.id, "retry")
                }
                TaskStatus::Paused => return service.control(&task.id, "resume"),
                TaskStatus::Queued
                | TaskStatus::Resolving
                | TaskStatus::Downloading
                | TaskStatus::Processing => return Ok(()),
                TaskStatus::Completed => {
                    self.on_download_task(task)?;
                    if self.with(|r| r.item(id))?.available {
                        return Ok(());
                    }
                }
            }
        }
        let request = self.with(|r| r.download_request(id))?;
        let tasks = service.enqueue(vec![request.clone()])?;
        let snapshot = service.snapshot();
        if let Some(task) = snapshot
            .tasks
            .iter()
            .filter(|t| {
                request_key(&t.request) == request_key(&request)
                    && (tasks.iter().any(|n| n.id == t.id)
                        || t.status.active()
                        || matches!(t.status, TaskStatus::Queued | TaskStatus::Paused))
            })
            .max_by_key(|t| t.created_at)
        {
            self.with(|r| r.link_task(task))?;
            self.on_download_task(task)?;
        }
        Ok(())
    }
    pub fn start(self: &Arc<Self>, service: Arc<Service>) {
        let this = self.clone();
        std::thread::spawn(move || {
            for _ in 0..120 {
                if this.media.snapshot().ready {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            let Ok(pending) = this.with(|r| r.pending_downloads()) else {
                return;
            };
            let tasks = service.snapshot().tasks;
            let mut selected = std::collections::HashSet::new();
            for (id, task_id, key, created) in pending {
                let task = task_id
                    .as_ref()
                    .and_then(|id| tasks.iter().find(|t| &t.id == id))
                    .or_else(|| {
                        tasks
                            .iter()
                            .filter(|t| {
                                key.as_ref() == Some(&request_key(&t.request))
                                    && t.created_at as i64 >= created
                            })
                            .max_by_key(|t| t.created_at)
                    });
                if let Some(task) = task {
                    if selected.insert(task.id.clone()) {
                        let _ = this.with(|r| r.link_task(task));
                        if this.on_download_task(task).is_err() {
                            eprintln!("学习库下载关联恢复失败，记录已保留");
                        }
                    }
                } else {
                    let _=this.with(|r|{r.conn.execute("UPDATE series_items SET download_task_id=NULL,download_state='waiting' WHERE id=?1",[id]).map_err(db::error)?;Ok(())});
                }
            }
        });
    }
}

impl Repository {
    pub fn new_entries(
        &self,
        series: &str,
        preview: &crate::models::MediaPreview,
    ) -> AppResult<Vec<crate::models::PlaylistEntry>> {
        self.get_series(series)?;
        let old = self.rows(
            "SELECT source_video_id,source_url FROM series_items WHERE series_id=?1",
            [series],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                ))
            },
        )?;
        let mut ids = std::collections::HashSet::new();
        let mut urls = std::collections::HashSet::new();
        for (id, url) in old {
            if let Some(id) = id {
                ids.insert(id);
            }
            if let Some(url) = url {
                urls.insert(url);
            }
        }
        let mut entries = vec![];
        for e in &preview.entries {
            let url = validate_url(&e.url)?;
            if (!e.id.is_empty() && ids.contains(&e.id)) || urls.contains(&url) {
                continue;
            }
            ids.insert(e.id.clone());
            urls.insert(url);
            entries.push(e.clone());
        }
        entries.sort_by_key(|e| e.playlist_index);
        Ok(entries)
    }
}
impl LearningService {
    pub fn check_updates(&self, service: &Arc<Service>, id: &str) -> AppResult<SeriesUpdate> {
        let (series,template)=self.with(|r|{let series=r.get_series(id)?;let first=r.rows("SELECT id FROM series_items WHERE series_id=?1 AND download_request IS NOT NULL ORDER BY sort_order,created_at LIMIT 1",[id],|r|r.get::<_,String>(0))?;let template=first.first().map(|id|r.download_request(id)).transpose()?.unwrap_or_default();Ok((series,template))})?;
        let url = series
            .source_url
            .as_ref()
            .ok_or("此系列没有来源播放列表，可先编辑系列来源")?;
        let result = service
            .preview(vec![url.clone()])?
            .into_iter()
            .next()
            .ok_or("来源未返回解析结果")?;
        let preview = result.preview.ok_or_else(|| {
            result
                .error
                .unwrap_or_else(|| "来源解析失败，请检查登录与代理设置".into())
        })?;
        if !preview.is_playlist {
            return Err("此来源不是播放列表，请编辑系列中的来源地址".into());
        }
        let new_entries = self.with(|r| r.new_entries(id, &preview))?;
        Ok(SeriesUpdate {
            series,
            preview,
            new_entries,
            template,
        })
    }
}
