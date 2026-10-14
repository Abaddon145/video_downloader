use super::{
    cache,
    compatibility::{self, BrowserPlaybackSupport, PlaybackMode, PlaybackPlan},
    db,
    importer::regular_file,
    models::*,
    repository::Repository,
    service::LearningService,
};
use crate::{
    domain::AppResult,
    ffmpeg::{command, models::*},
    native,
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::Emitter;
#[derive(Clone)]
pub(crate) struct SharedPlayback {
    task: String,
    finalize: Arc<Mutex<()>>,
}
#[derive(Default)]
pub(crate) struct PlaybackState {
    sessions: HashMap<String, Option<String>>,
    cancelled: HashSet<String>,
    jobs: HashMap<String, SharedPlayback>,
    subtitle_keys: HashMap<String, HashSet<String>>,
}
impl PlaybackState {
    pub(crate) fn begin(&mut self, session: &str) -> AppResult<()> {
        if self.cancelled.contains(session) {
            return Err("播放准备已取消".into());
        }
        self.sessions.entry(session.into()).or_insert(None);
        Ok(())
    }
    pub(crate) fn join(&mut self, session: &str, key: &str) -> AppResult<()> {
        self.begin(session)?;
        self.sessions.insert(session.into(), Some(key.into()));
        Ok(())
    }
    pub(crate) fn protected(&self) -> HashSet<String> {
        self.sessions
            .values()
            .filter_map(Clone::clone)
            .chain(self.subtitle_keys.values().flatten().cloned())
            .collect()
    }
    pub(crate) fn release(&mut self, session: &str) -> Option<String> {
        // ponytail: 4096 late-cancel tombstones, matching the bounded native preview cancellation policy; use generation IDs if sessions exceed this in one run.
        if self.cancelled.len() >= 4096 {
            self.cancelled.clear();
        }
        self.cancelled.insert(session.into());
        self.subtitle_keys.remove(session);
        let key = self.sessions.remove(session).flatten()?;
        (!self.sessions.values().any(|v| v.as_deref() == Some(&key))).then_some(key)
    }
}
impl Repository {
    pub fn playback_file(&self, id: &str) -> AppResult<PathBuf> {
        let item = self.item(id)?;
        let result = item
            .file_path
            .as_deref()
            .ok_or_else(|| "课程尚未下载".to_string())
            .and_then(regular_file);
        match result {
            Ok(path) => {
                self.conn
                    .execute("UPDATE series_items SET available=1 WHERE id=?1", [id])
                    .map_err(db::error)?;
                Ok(path)
            }
            _ => {
                self.conn
                    .execute("UPDATE series_items SET available=0 WHERE id=?1", [id])
                    .map_err(db::error)?;
                Err(if item.file_path.is_some() {
                    "文件已移动或不存在，请重新定位"
                } else {
                    "课程尚未下载，请先下载视频"
                }
                .into())
            }
        }
    }
    pub fn neighbors(&self, id: &str) -> AppResult<(Option<String>, Option<String>)> {
        let item = self.item(id)?;
        let ids = self.item_ids(&item.series_id)?;
        let n = ids.iter().position(|x| x == id).ok_or("课程不存在")?;
        Ok((
            n.checked_sub(1).and_then(|n| ids.get(n)).cloned(),
            ids.get(n + 1).cloned(),
        ))
    }
}

impl LearningService {
    fn playback_event(
        &self,
        session: &str,
        id: &str,
        plan: Option<&PlaybackPlan>,
        state: &str,
        percent: Option<f64>,
        eta: Option<f64>,
    ) {
        let _=self.app.emit("learning-playback-updated",serde_json::json!({"session":session,"itemId":id,"mode":plan.map(|p|p.mode),"state":state,"percent":percent,"eta":eta}));
    }
    pub fn cancel_player(&self, session: &str) -> AppResult<()> {
        crate::media::trim::timeline::validate_session(session)?;
        let mut state = self.playback.lock().map_err(|_| "播放缓存暂不可用")?;
        if let Some(key) = state.release(session) {
            if let Some(shared) = state.jobs.remove(&key) {
                let _ = self.media.cancel(&shared.task);
            }
        }
        drop(state);
        self.media.cancel_editor(session)
    }
    pub fn prepare_player(
        &self,
        id: &str,
        session: &str,
        proxy: bool,
        support: BrowserPlaybackSupport,
        fallback: u8,
        force: bool,
    ) -> AppResult<PlayerMedia> {
        crate::media::trim::timeline::validate_session(session)?;
        self.playback
            .lock()
            .map_err(|_| "播放缓存暂不可用")?
            .begin(session)?;
        self.playback_event(session, id, None, "解析课程", None, None);
        let source = match self.with(|r| r.playback_file(id)) {
            Ok(p) => p,
            Err(e) => {
                self.changed("missing", id);
                return Err(e);
            }
        };
        let (item, series, settings, previous, next) = self.with(|r| {
            let i = r.item(id)?;
            let (a, b) = r.neighbors(id)?;
            Ok((i.clone(), r.get_series(&i.series_id)?, r.settings()?, a, b))
        })?;
        let original_stamp = cache::stamp(&source)?;
        let info = self
            .media
            .probe_file(&source, &format!("{session}-probe"), false)?;
        if cache::stamp(&source)? != original_stamp {
            return Err("源视频在读取期间改变，请重新播放".into());
        }
        let preference = if proxy {
            PlaybackPreference::Compatible
        } else {
            settings.playback_preference
        };
        let mut plan = compatibility::plan(&info, support, preference, fallback)?;
        let path = if plan.mode == PlaybackMode::Direct {
            self.media.authorize_asset(session, &source)?;
            source.to_string_lossy().into_owned()
        } else {
            self.compatible_path(id, session, &source, &info, &mut plan, force)?
        };
        let mut subtitles = vec![];
        let mut warnings = vec![];
        if settings.auto_subtitles {
            let stem = source
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if let Some(parent) = source.parent() {
                if let Ok(entries) = fs::read_dir(parent) {
                    let mut candidates = entries
                        .take(10000)
                        .filter_map(Result::ok)
                        .map(|e| e.path())
                        .filter(|p| {
                            let name = p
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_lowercase();
                            (name == stem || name.starts_with(&format!("{stem}.")))
                                && ["srt", "ass", "ssa", "vtt"].contains(
                                    &p.extension()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_lowercase()
                                        .as_str(),
                                )
                        })
                        .collect::<Vec<_>>();
                    candidates.sort();
                    for p in candidates.into_iter().take(4) {
                        match self.subtitle(&p, session) {
                            Ok(s) => subtitles.push(s),
                            Err(e) => warnings.push(e),
                        }
                    }
                }
            }
            if subtitles.is_empty() {
                if let Some(track) = info.subtitles.iter().find(|s| {
                    ["subrip", "ass", "ssa", "webvtt", "mov_text"].contains(&s.codec.as_str())
                }) {
                    let mut selected = info.clone();
                    selected.subtitles = vec![track.clone()];
                    match self.convert_subtitle(&source, &selected, session, Some(track.index)) {
                        Ok(path) => subtitles.push(PlayerSubtitle {
                            label: "内嵌字幕".into(),
                            path,
                        }),
                        Err(e) => warnings.push(e),
                    }
                }
            }
        }
        self.playback
            .lock()
            .map_err(|_| "播放缓存暂不可用")?
            .begin(session)?;
        if cache::stamp(&source)? != original_stamp || self.with(|r| r.playback_file(id))? != source
        {
            return Err("课程文件已经重新定位，请重新播放".into());
        }
        self.playback_event(session, id, Some(&plan), "准备完成", Some(100.), None);
        Ok(PlayerMedia {
            item,
            series,
            settings,
            previous,
            next,
            path,
            subtitles,
            subtitle_warning: (!warnings.is_empty()).then(|| warnings.join("；")),
            plan,
            technical: format!(
                "{} · {} · {}",
                info.container,
                info.videos
                    .iter()
                    .find(|v| !v.attached_picture)
                    .map(|v| v.codec.as_str())
                    .unwrap_or("?"),
                info.audios
                    .first()
                    .map(|a| a.codec.as_str())
                    .unwrap_or("无音频")
            ),
        })
    }
    fn compatible_path(
        &self,
        id: &str,
        session: &str,
        source: &Path,
        info: &MediaInfo,
        plan: &mut PlaybackPlan,
        force: bool,
    ) -> AppResult<String> {
        let source_stamp = cache::stamp(source)?;
        let profile = format!("learning-v1-{:?}-{:?}-crf22-aac192", plan.mode, plan.format);
        let codecs = serde_json::to_string(&(info.container.clone(), &info.videos, &info.audios))
            .map_err(|_| "编码信息无效")?;
        let key = cache::key(&source_stamp, &codecs, &profile);
        let dir = cache::directory(&self.data, "playback", &key)?;
        let shared = {
            let mut state = self.playback.lock().map_err(|_| "播放缓存暂不可用")?;
            state.join(session, &key)?;
            if !force {
                if let Ok(mut e) = cache::read_entry(&dir, &source_stamp, &profile) {
                    e.last_used = crate::service::now();
                    native::save_atomic(
                        &dir.join("info.json"),
                        &serde_json::to_vec(&e).map_err(|_| "缓存信息无效")?,
                    )?;
                    self.media.authorize_asset(session, Path::new(&e.file))?;
                    plan.cached = true;
                    return Ok(e.file);
                }
            }
            let active = state
                .jobs
                .get(&key)
                .filter(|job| {
                    self.media.task(&job.task).is_ok_and(|t| {
                        matches!(
                            t.status,
                            MediaTaskStatus::Queued
                                | MediaTaskStatus::Probing
                                | MediaTaskStatus::Processing
                        )
                    })
                })
                .cloned();
            let shared = if let Some(job) = active {
                job
            } else {
                let task = self.media.create_playback(compatibility::request(
                    &source.to_string_lossy(),
                    &dir.to_string_lossy(),
                    plan,
                ))?;
                SharedPlayback {
                    task: task.id,
                    finalize: Arc::new(Mutex::new(())),
                }
            };
            state.jobs.insert(key.clone(), shared.clone());
            shared
        };
        let mut emitted = None;
        loop {
            self.playback
                .lock()
                .map_err(|_| "播放缓存暂不可用")?
                .begin(session)?;
            let t = self.media.task(&shared.task)?;
            let signature = (t.status, t.progress.map(|p| p as u16), t.phase.clone());
            if emitted.as_ref() != Some(&signature) {
                self.playback_event(session, id, Some(plan), &t.phase, t.progress, t.eta);
                emitted = Some(signature);
            }
            match t.status {
                MediaTaskStatus::Completed => {
                    let _finish = shared.finalize.lock().map_err(|_| "缓存发布失败")?;
                    let mut e = if let Some(e) = cache::read_entry(&dir, &source_stamp, &profile)
                        .ok()
                        .filter(|e| Some(e.file.as_str()) == t.output_path.as_deref())
                    {
                        e
                    } else {
                        if cache::stamp(source)? != source_stamp {
                            return Err("源视频在处理期间改变，请重新播放".into());
                        }
                        let path = cache::owned(
                            &dir,
                            Path::new(t.output_path.as_deref().ok_or("兼容视频未生成")?),
                        )?;
                        let verified =
                            self.media
                                .probe_file(&path, &format!("{session}-verify"), false)?;
                        if !verified.videos.iter().any(|v| !v.attached_picture)
                            || info
                                .duration
                                .zip(verified.duration)
                                .is_some_and(|(a, b)| b + 1. < a)
                        {
                            return Err("兼容视频不完整，请重新生成".into());
                        }
                        cache::PlaybackEntry {
                            source: source_stamp.clone(),
                            profile: profile.clone(),
                            output: cache::stamp(&path)?,
                            file: path.to_string_lossy().into(),
                            last_used: crate::service::now(),
                        }
                    };
                    let mut state = self.playback.lock().map_err(|_| "播放缓存暂不可用")?;
                    state.begin(session)?;
                    if cache::stamp(source)? != source_stamp {
                        return Err("源视频已改变，请重新准备".into());
                    }
                    e.last_used = crate::service::now();
                    native::save_atomic(
                        &dir.join("info.json"),
                        &serde_json::to_vec(&e).map_err(|_| "缓存记录无效")?,
                    )?;
                    self.media.authorize_asset(session, Path::new(&e.file))?;
                    return Ok(e.file);
                }
                MediaTaskStatus::Failed
                | MediaTaskStatus::Cancelled
                | MediaTaskStatus::Interrupted => {
                    return Err(format!(
                        "{}；{}",
                        t.error.unwrap_or(t.phase),
                        t.logs.join("\n")
                    ))
                }
                _ => std::thread::sleep(Duration::from_millis(200)),
            }
        }
    }
    fn convert_subtitle(
        &self,
        input: &Path,
        info: &MediaInfo,
        session: &str,
        index: Option<u32>,
    ) -> AppResult<String> {
        let _single = self.subtitle_guard.lock().map_err(|_| "字幕缓存暂不可用")?;
        let before = cache::stamp(input)?;
        let key = cache::key(&before, &format!("subtitle:{index:?}"), "webvtt-v1");
        let dir = cache::directory(&self.data, "subtitles", &key)?;
        let target = dir.join("subtitles.vtt");
        {
            let mut state = self.playback.lock().map_err(|_| "播放缓存暂不可用")?;
            state.begin(session)?;
            state
                .subtitle_keys
                .entry(session.into())
                .or_default()
                .insert(key);
        }
        if cache::owned(&dir, &target).is_err() {
            let temp = dir.join(format!("{}.vtt", super::repository::id()));
            let request = MediaRequest {
                input_path: input.to_string_lossy().into(),
                output_dir: dir.to_string_lossy().into(),
                operation: MediaOperation::SubtitleConvert,
                output_format: OutputFormat::Vtt,
                ..Default::default()
            };
            let mut c = command::build(&request, info, &temp)?;
            if let Some(i) = index {
                if let Some(n) = c.args.iter().position(|a| a == "-map") {
                    c.args[n + 1] = format!("0:{i}");
                }
            }
            let result: AppResult<()> = (|| {
                self.media.auxiliary(session, &c.args, 30)?;
                if cache::stamp(input)? != before {
                    return Err("字幕源已改变".into());
                }
                fs::rename(&temp, &target).map_err(|_| "无法保存字幕缓存")?;
                Ok(())
            })();
            let _ = fs::remove_file(&temp);
            result?;
        }
        native::save_atomic(
            &dir.join("info.json"),
            &serde_json::to_vec(&serde_json::json!({"lastUsed":crate::service::now()}))
                .map_err(|_| "字幕缓存无效")?,
        )?;
        self.media
            .authorize_asset(session, &cache::owned(&dir, &target)?)?;
        Ok(target.to_string_lossy().into())
    }
    pub fn subtitle(&self, path: &Path, session: &str) -> AppResult<PlayerSubtitle> {
        crate::media::trim::timeline::validate_session(session)?;
        let p = regular_file(&path.to_string_lossy())?;
        crate::media::subtitle::extension(&p.to_string_lossy())?;
        if fs::metadata(&p).map_err(|_| "无法读取字幕")?.len() > 8 * 1024 * 1024 {
            return Err("字幕超过8MiB，请选择较小文件".into());
        }
        let info = self
            .media
            .probe_file(&p, &format!("{session}-subtitle-probe"), false)?;
        Ok(PlayerSubtitle {
            label: p.file_name().unwrap_or_default().to_string_lossy().into(),
            path: self.convert_subtitle(&p, &info, session, None)?,
        })
    }
}
