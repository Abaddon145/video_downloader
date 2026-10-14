use super::{models::*, repository::Repository};
use crate::{domain::AppResult, ffmpeg::runner::MediaService};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};
pub struct LearningService {
    pub(crate) db: Mutex<AppResult<Repository>>,
    pub(crate) live: Mutex<std::collections::HashMap<String, ProgressInput>>,
    pub(crate) app: tauri::AppHandle,
    pub(crate) media: Arc<MediaService>,
    pub(crate) data: PathBuf,
}
impl LearningService {
    pub fn new(app: tauri::AppHandle, data: &Path, media: Arc<MediaService>) -> Arc<Self> {
        Arc::new(Self {
            db: Mutex::new(Repository::open(&data.join("learning.db"))),
            live: Mutex::new(Default::default()),
            app,
            media,
            data: data.into(),
        })
    }
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut Repository) -> AppResult<R>) -> AppResult<R> {
        let mut db = self.db.lock().map_err(|_| "学习库暂时不可用")?;
        if db.is_err() {
            *db = Repository::open(&self.data.join("learning.db"));
        }
        let r = db.as_mut().map_err(|e| format!("学习库初始化失败：{e}"))?;
        f(r)
    }
    pub(crate) fn changed(&self, kind: &str, id: &str) {
        let _ = self.app.emit(
            "learning-library-updated",
            serde_json::json!({"kind":kind,"id":id}),
        );
    }
    pub fn home(
        &self,
        query: &str,
        category: Option<&str>,
        page: u32,
        favorite: bool,
    ) -> AppResult<LibraryHome> {
        let home = self.with(|r| {
            let (series, total) = r.list_series(query, category, page, favorite)?;
            Ok(LibraryHome {
                categories: r.categories()?,
                series,
                total,
                page: page.max(1).min(((total + 49) / 50).max(1) as u32),
                continue_learning: r.continue_learning()?,
                settings: r.settings()?,
            })
        })?;
        for series in home
            .series
            .iter()
            .chain(home.continue_learning.iter().map(|c| &c.series))
        {
            if let Some(p) = &series.cover_path {
                self.authorize_cover(p);
            }
        }
        Ok(home)
    }
    pub fn series(&self, id: &str, page: u32) -> AppResult<SeriesDetail> {
        let mut d = self.with(|r| r.series(id, page))?;
        if let Some(p) = d.series.cover_path.as_ref() {
            if !p.starts_with("https://") && !p.starts_with("http://") {
                if let Ok(path) = crate::ffmpeg::output::input(p) {
                    let _ = self.app.asset_protocol_scope().allow_file(path);
                } else {
                    d.series.cover_path = None;
                }
            }
        }
        Ok(d)
    }
    pub fn change(&self, change: LearningChange) -> AppResult<String> {
        let mut change = change;
        if let LearningChange::SaveSeries { series } = &mut change {
            self.copy_cover(series)?;
        }
        let id = self.with(|r| match change {
            LearningChange::SaveCategory { category } => r.save_category(&category),
            LearningChange::DeleteCategory {
                id,
                move_to_uncategorized,
            } => {
                r.delete_category(&id, move_to_uncategorized)?;
                Ok(id)
            }
            LearningChange::SaveSeries { series } => r.save_series(&series),
            LearningChange::DeleteSeries {
                id,
                delete_files,
                confirmed,
            } => {
                if delete_files {
                    let ids = r.item_ids(&id)?;
                    r.remove_library_files(&ids, confirmed)?;
                }
                r.delete_series_records(&id)?;
                Ok(id)
            }
            LearningChange::RenameItem { id, title } => {
                r.rename_item(&id, &title)?;
                Ok(id)
            }
            LearningChange::DeleteItem {
                id,
                delete_files,
                confirmed,
            } => {
                if delete_files {
                    r.remove_library_files(&[id.clone()], confirmed)?;
                }
                r.delete_item_records(&id)?;
                Ok(id)
            }
            LearningChange::Move {
                scope,
                id,
                before_id,
                direction,
            } => {
                r.move_entry(&scope, &id, before_id.as_deref(), direction.as_deref())?;
                Ok(id)
            }
            LearningChange::SaveSettings { settings } => {
                r.save_settings(&settings)?;
                Ok("settings".into())
            }
            LearningChange::MarkCompleted { id, completed } => {
                r.mark_completed(&id, completed)?;
                self.live
                    .lock()
                    .map_err(|_| "进度保存暂不可用")?
                    .remove(&id);
                Ok(id)
            }
            LearningChange::DeleteNote { id } => {
                r.delete_note(&id)?;
                Ok(id)
            }
        })?;
        self.changed("structure", &id);
        Ok(id)
    }
}
