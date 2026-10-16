use super::{models::*, native_window::NativeWindow, service::LearningService};
use crate::{domain::AppResult, native};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeViewport {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub visible: bool,
}
#[derive(Debug, Clone, Copy)]
pub struct PhysicalViewport {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub visible: bool,
}
impl NativeViewport {
    pub fn physical(
        self,
        scale: f64,
        client_width: i32,
        client_height: i32,
    ) -> AppResult<PhysicalViewport> {
        if ![self.left, self.top, self.width, self.height, scale]
            .iter()
            .all(|x| x.is_finite() && x.abs() <= 100000.)
            || self.width < 0.
            || self.height < 0.
            || scale <= 0.
            || scale > 16.
        {
            return Err("播放区域无效".into());
        }
        let (x, y, w, h) = (
            (self.left * scale).round() as i32,
            (self.top * scale).round() as i32,
            (self.width * scale).round() as i32,
            (self.height * scale).round() as i32,
        );
        Ok(PhysicalViewport {
            x,
            y,
            width: w,
            height: h,
            visible: self.visible
                && x >= 0
                && y >= 0
                && w > 0
                && h > 0
                && x + w <= client_width + 1
                && y + h <= client_height + 1,
        })
    }
}
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum NativeAction {
    Pause { paused: bool },
    Seek { position: f64 },
    Rate { rate: f64 },
    Volume { volume: f64 },
    Mute { muted: bool },
    Subtitle { id: Option<i64> },
    LoadSubtitle { path: String },
    Fullscreen { enabled: bool },
}
impl NativeAction {
    fn command(&self) -> AppResult<Vec<Value>> {
        let set = |p: &str, v: Value| vec![json!("set_property"), json!(p), v];
        Ok(match self {
            Self::Pause { paused } => set("pause", json!(paused)),
            Self::Seek { position }
                if position.is_finite() && (0. ..=604800.).contains(position) =>
            {
                vec![json!("seek"), json!(position), json!("absolute+exact")]
            }
            Self::Rate { rate } if [0.5, 0.75, 1., 1.25, 1.5, 1.75, 2.].contains(rate) => {
                set("speed", json!(rate))
            }
            Self::Volume { volume } if volume.is_finite() && (0. ..=1.).contains(volume) => {
                set("volume", json!(volume * 100.))
            }
            Self::Mute { muted } => set("mute", json!(muted)),
            Self::Subtitle { id } if id.is_none_or(|x| x >= 0) => {
                set("sid", id.map_or(json!("no"), |v| json!(v)))
            }
            Self::LoadSubtitle { path } => {
                let p = super::importer::regular_file(path)?;
                if !p.extension().is_some_and(|e| {
                    ["srt", "ass", "ssa", "vtt"]
                        .contains(&e.to_string_lossy().to_lowercase().as_str())
                }) {
                    return Err("请选择 SRT、ASS、SSA 或 VTT 字幕".into());
                }
                vec![
                    json!("sub-add"),
                    json!(p.to_string_lossy()),
                    json!("select"),
                ]
            }
            _ => return Err("播放操作参数无效".into()),
        })
    }
}
fn parse_reply(line: &str, expected: u64) -> AppResult<Option<Value>> {
    let v: Value = serde_json::from_str(line).map_err(|_| "播放内核返回无效数据")?;
    if v.get("request_id").and_then(Value::as_u64) != Some(expected) {
        return Ok(None);
    }
    if v["error"] != "success" {
        return Err(format!(
            "播放操作失败：{}",
            v["error"].as_str().unwrap_or("未知错误")
        ));
    }
    Ok(Some(v.get("data").cloned().unwrap_or(Value::Null)))
}
fn line(reader: &mut BufReader<File>) -> AppResult<String> {
    let mut s = String::new();
    let count = reader
        .take(1024 * 1024 + 1)
        .read_line(&mut s)
        .map_err(|e| e.to_string())?;
    if count == 0 || count > 1024 * 1024 || !s.ends_with('\n') {
        return Err("播放内核连接已断开或响应过大".into());
    }
    Ok(s)
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSubtitle {
    pub id: i64,
    pub label: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSnapshot {
    pub session: String,
    pub item_id: String,
    pub ready: bool,
    pub position: f64,
    pub duration: f64,
    pub rate: f64,
    pub paused: bool,
    pub volume: f64,
    pub muted: bool,
    pub ended: bool,
    pub subtitles: Vec<NativeSubtitle>,
    pub subtitle_id: Option<i64>,
    pub error: Option<String>,
}
type Request = (Vec<Value>, mpsc::Sender<AppResult<Value>>);
pub(crate) struct NativeSession {
    window: NativeWindow,
    job: Arc<native::Job>,
    commands: mpsc::Sender<Request>,
    snapshot: Mutex<NativeSnapshot>,
    closed: AtomicBool,
    path: PathBuf,
    operations: Mutex<()>,
    fullscreen: AtomicBool,
}
impl Drop for NativeSession {
    fn drop(&mut self) {
        let _ = self.job.terminate();
    }
}
impl NativeSession {
    fn request(&self, command: Vec<Value>) -> AppResult<Value> {
        if self.closed.load(Ordering::Acquire) {
            return Err("播放会话已关闭".into());
        }
        let (tx, rx) = mpsc::channel();
        self.commands
            .send((command, tx))
            .map_err(|_| "播放内核已退出")?;
        match rx.recv_timeout(Duration::from_secs(3)) {
            Ok(result) => result,
            Err(_) => {
                self.snapshot.lock().unwrap().error = Some("播放内核响应超时，请重新尝试".into());
                let _ = self.job.terminate();
                Err("播放内核响应超时，请重新尝试".into())
            }
        }
    }
    fn snapshot(&self) -> NativeSnapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
    fn refresh(&self) -> AppResult<NativeSnapshot> {
        for property in [
            "time-pos",
            "duration",
            "speed",
            "pause",
            "volume",
            "mute",
            "eof-reached",
        ] {
            let value = self.request(vec![json!("get_property"), json!(property)])?;
            update(
                &mut *self.snapshot.lock().map_err(|_| "播放状态不可用")?,
                property,
                &value,
            );
        }
        let s = self.snapshot();
        if s.error.is_some() {
            return Err(s.error.unwrap());
        }
        Ok(s)
    }
}
#[derive(Default)]
pub(crate) struct NativeRuntime {
    starting: Option<String>,
    active: Option<Arc<NativeSession>>,
}
fn update(s: &mut NativeSnapshot, name: &str, v: &Value) {
    match name {
        "time-pos" => {
            if let Some(n) = v.as_f64() {
                s.position = n.max(0.)
            }
        }
        "duration" => {
            if let Some(n) = v.as_f64() {
                s.duration = n.max(0.)
            }
        }
        "speed" => {
            if let Some(n) = v.as_f64() {
                s.rate = n
            }
        }
        "pause" => {
            if let Some(n) = v.as_bool() {
                s.paused = n
            }
        }
        "volume" => {
            if let Some(n) = v.as_f64() {
                s.volume = n / 100.
            }
        }
        "mute" => {
            if let Some(n) = v.as_bool() {
                s.muted = n
            }
        }
        "eof-reached" => {
            if let Some(n) = v.as_bool() {
                s.ended = n
            }
        }
        "sid" => s.subtitle_id = v.as_i64(),
        "track-list" => {
            s.subtitles = v
                .as_array()
                .into_iter()
                .flatten()
                .filter(|t| t["type"] == "sub")
                .filter_map(|t| {
                    Some(NativeSubtitle {
                        id: t["id"].as_i64()?,
                        label: t["title"]
                            .as_str()
                            .or(t["lang"].as_str())
                            .unwrap_or("内嵌字幕")
                            .into(),
                    })
                })
                .collect()
        }
        _ => {}
    }
}
fn connect(pipe: &str, job: &native::Job) -> AppResult<File> {
    let start = Instant::now();
    loop {
        match OpenOptions::new().read(true).write(true).open(pipe) {
            Ok(f) => return Ok(f),
            Err(_) if start.elapsed() < Duration::from_secs(3) => {
                thread::sleep(Duration::from_millis(20))
            }
            Err(_) => {
                let _ = job.terminate();
                return Err("无法连接内置播放内核".into());
            }
        }
    }
}
impl LearningService {
    fn native_session(&self, session: &str) -> AppResult<Arc<NativeSession>> {
        self.native_player
            .lock()
            .map_err(|_| "播放状态不可用")?
            .active
            .as_ref()
            .filter(|s| s.snapshot().session == session)
            .cloned()
            .ok_or_else(|| "播放会话已关闭".into())
    }
    pub fn native_start(
        self: &Arc<Self>,
        id: &str,
        session: &str,
        viewport: NativeViewport,
        autoplay: bool,
    ) -> AppResult<NativeSnapshot> {
        crate::media::trim::timeline::validate_session(session)?;
        self.playback
            .lock()
            .map_err(|_| "播放状态不可用")?
            .begin(session)?;
        let old = {
            let mut r = self.native_player.lock().map_err(|_| "播放状态不可用")?;
            r.starting = Some(session.into());
            r.active.take()
        };
        if let Some(old) = old {
            self.finish_native(&old);
        }
        let result = (|| {
            let (path, item, settings) =
                self.with(|r| Ok((r.playback_file(id)?, r.item(id)?, r.settings()?)))?;
            let exe = self.media.resources.join("tools/mpv.exe");
            let manifest: Value = serde_json::from_slice(
                &std::fs::read(self.media.resources.join("tools/binaries.json"))
                    .map_err(|_| "播放内核校验清单缺失，请重新安装")?,
            )
            .map_err(|_| "播放内核清单损坏")?;
            crate::service::verify_hash(
                &exe,
                manifest["mpv.exe"]["sha256"]
                    .as_str()
                    .ok_or("播放内核未安装")?,
            )?;
            let window = NativeWindow::create(&self.app)?;
            window.update(viewport)?;
            let pipe = format!(r"\\.\pipe\yingliu-{}-{}", std::process::id(), session);
            let mut args = vec![
                "--no-config",
                "--load-scripts=no",
                "--load-auto-profiles=no",
                "--load-console=no",
                "--load-stats-overlay=no",
                "--load-select=no",
                "--demuxer-lavf-o=protocol_whitelist=file",
                "--osc=no",
                "--osd-level=0",
                "--input-default-bindings=no",
                "--input-terminal=no",
                "--idle=yes",
                "--force-window=yes",
                "--keep-open=yes",
                "--access-references=no",
                "--sub-auto=no",
                "--ytdl=no",
                "--hwdec=auto-safe",
                "--vo=gpu",
                "--gpu-api=d3d11",
                "--gpu-context=d3d11",
                "--terminal=no",
                "--input-media-keys=no",
            ]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
            args.extend([
                format!("--wid={}", window.id()),
                format!("--input-ipc-server={pipe}"),
                format!(
                    "--input-conf={}",
                    self.media.resources.join("mpv-input.conf").display()
                ),
            ]);
            let (process, out, err) = native::spawn(&exe, &args)?;
            let job = process.job.clone();
            thread::spawn(move || {
                let _ = native::read_bounded(out, 8192);
            });
            thread::spawn(move || {
                let _ = native::read_bounded(err, 8192);
            });
            thread::spawn(move || {
                let _ = process.wait();
            });
            let mut events = connect(&pipe, &job)?;
            let commands = connect(&pipe, &job)?;
            for (i, p) in [
                "time-pos",
                "duration",
                "speed",
                "pause",
                "volume",
                "mute",
                "sid",
                "track-list",
                "eof-reached",
            ]
            .iter()
            .enumerate()
            {
                writeln!(events, "{}", json!({"command":["observe_property",i,p]})).map_err(
                    |e| {
                        let _ = job.terminate();
                        e.to_string()
                    },
                )?;
            }
            let (tx, rx) = mpsc::channel::<Request>();
            thread::spawn(move || {
                let mut reader = BufReader::new(commands);
                for (i, (command, reply)) in rx.into_iter().enumerate() {
                    let result = (|| {
                        writeln!(
                            reader.get_mut(),
                            "{}",
                            json!({"command":command,"request_id":i as u64})
                        )
                        .map_err(|e| e.to_string())?;
                        loop {
                            if let Some(v) = parse_reply(&line(&mut reader)?, i as u64)? {
                                return Ok(v);
                            }
                        }
                    })();
                    let _ = reply.send(result);
                }
            });
            let rate = if item.progress.last_played_at.is_some() {
                item.progress.playback_rate
            } else {
                settings.default_playback_rate
            };
            let engine = Arc::new(NativeSession {
                window,
                job,
                commands: tx,
                snapshot: Mutex::new(NativeSnapshot {
                    session: session.into(),
                    item_id: id.into(),
                    ready: false,
                    position: 0.,
                    duration: 0.,
                    rate,
                    paused: !autoplay,
                    volume: 0.8,
                    muted: false,
                    ended: false,
                    subtitles: vec![],
                    subtitle_id: None,
                    error: None,
                }),
                closed: AtomicBool::new(false),
                path: path.clone(),
                operations: Mutex::new(()),
                fullscreen: AtomicBool::new(false),
            });
            let weak = Arc::downgrade(&engine);
            let app = self.app.clone();
            thread::spawn(move || {
                let mut reader = BufReader::new(events);
                loop {
                    let event = line(&mut reader)
                        .and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string()));
                    let Some(engine) = weak.upgrade() else { break };
                    if engine.closed.load(Ordering::Acquire) {
                        break;
                    }
                    let Ok(v) = event else {
                        engine.snapshot.lock().unwrap().error =
                            Some("播放内核已退出，请重新尝试".into());
                        break;
                    };
                    let mut s = engine.snapshot.lock().unwrap();
                    match v["event"].as_str() {
                        Some("property-change") => {
                            update(&mut s, v["name"].as_str().unwrap_or(""), &v["data"])
                        }
                        Some("playback-restart") => s.ready = true,
                        Some("end-file") if v["reason"] == "error" => {
                            s.error = Some(format!(
                                "原文件无法播放：{}",
                                v["file_error"].as_str().unwrap_or("解码失败")
                            ))
                        }
                        Some("client-message") => {
                            let _=app.emit("learning-native-shortcut",json!({"session":s.session,"itemId":s.item_id,"action":v["args"][0]}));
                        }
                        _ => {}
                    }
                }
            });
            {
                let mut r = self.native_player.lock().map_err(|_| "播放状态不可用")?;
                if r.starting.as_deref() != Some(session) {
                    return Err("播放准备已取消".into());
                }
                self.playback
                    .lock()
                    .map_err(|_| "播放状态不可用")?
                    .begin(session)?;
                r.active = Some(engine.clone());
                r.starting = None;
            }
            engine.request(vec![
                json!("set_property"),
                json!("pause"),
                json!(!autoplay),
            ])?;
            engine.request(NativeAction::Rate { rate }.command()?)?;
            engine.request(NativeAction::Volume { volume: 0.8 }.command()?)?;
            if !settings.auto_subtitles {
                engine.request(NativeAction::Subtitle { id: None }.command()?)?;
            }
            let position = if settings.restore_position && !item.progress.completed {
                item.progress.position_ms.max(0) as f64 / 1000.
            } else {
                0.
            };
            if self.with(|r| r.playback_file(id))? != path {
                return Err("课程文件已改变，请重新播放".into());
            }
            engine.request(vec![
                json!("loadfile"),
                json!(path.to_string_lossy()),
                json!("replace"),
                json!(-1),
                json!({"start":position.to_string()}),
            ])?;
            let start = Instant::now();
            loop {
                let s = engine.snapshot();
                if let Some(error) = s.error {
                    return Err(error);
                }
                if s.ready {
                    break;
                }
                if start.elapsed() > Duration::from_secs(5) {
                    return Err("播放内核未能及时出画，请重试或手动转换兼容视频".into());
                }
                if self.native_session(session).is_err() {
                    return Err("播放准备已取消".into());
                }
                thread::sleep(Duration::from_millis(20));
            }
            if settings.auto_subtitles {
                if let (Some(parent), Some(stem)) = (path.parent(), path.file_stem()) {
                    let stem = stem.to_string_lossy().to_lowercase();
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        let mut candidates = entries
                            .take(10000)
                            .filter_map(Result::ok)
                            .map(|e| e.path())
                            .filter(|p| {
                                p.file_stem().is_some_and(|s| {
                                    let s = s.to_string_lossy().to_lowercase();
                                    s == stem || s.starts_with(&format!("{stem}."))
                                })
                            })
                            .filter(|p| {
                                p.extension().is_some_and(|e| {
                                    ["srt", "ass", "ssa", "vtt"]
                                        .contains(&e.to_string_lossy().to_lowercase().as_str())
                                })
                            })
                            .collect::<Vec<_>>();
                        candidates.sort();
                        for p in candidates.into_iter().take(4) {
                            if let Ok(command) = (NativeAction::LoadSubtitle {
                                path: p.to_string_lossy().into_owned(),
                            })
                            .command()
                            {
                                let _ = engine.request(command);
                            }
                        }
                    }
                }
            }
            let weak = Arc::downgrade(self);
            let session = session.to_string();
            thread::spawn(move || {
                let mut last = Instant::now();
                loop {
                    thread::sleep(Duration::from_millis(250));
                    let Some(service) = weak.upgrade() else { break };
                    let Ok(engine) = service.native_session(&session) else {
                        break;
                    };
                    let Ok(_guard) = engine.operations.lock() else {
                        break;
                    };
                    if engine.closed.load(Ordering::Acquire)
                        || service.native_session(&session).is_err()
                    {
                        break;
                    }
                    let s = engine.snapshot();
                    let _ = service.app.emit("learning-native-updated", &s);
                    if s.error.is_some() {
                        break;
                    }
                    let persist = last.elapsed() >= Duration::from_secs(5) || s.ended;
                    let _ = service.persist_native(&engine, &s, persist, false);
                    if persist {
                        last = Instant::now();
                    }
                }
            });
            Ok(engine.snapshot())
        })();
        if result.is_err() {
            self.native_stop(session);
        }
        result
    }
    fn persist_native(
        &self,
        engine: &NativeSession,
        s: &NativeSnapshot,
        persist: bool,
        ended: bool,
    ) -> AppResult<PlaybackProgress> {
        if self.with(|r| r.playback_file(&s.item_id))? != engine.path {
            return Err("课程文件已改变，请重新播放".into());
        }
        if s.ready && s.duration > 0. {
            self.live_progress(
                ProgressInput {
                    item_id: s.item_id.clone(),
                    position_ms: (s.position.min(s.duration) * 1000.).round() as i64,
                    duration_ms: (s.duration * 1000.).round() as i64,
                    playback_rate: s.rate,
                    ended: ended || s.ended,
                    sequence: super::progress::sequence(),
                },
                persist,
            )?;
        }
        self.with(|r| Ok(r.item(&s.item_id)?.progress))
    }
    pub fn native_control(&self, session: &str, action: NativeAction) -> AppResult<NativeSnapshot> {
        let engine = self.native_session(session)?;
        let _guard = engine.operations.lock().map_err(|_| "播放操作不可用")?;
        if let NativeAction::Fullscreen { enabled } = action {
            self.app
                .get_webview_window("main")
                .ok_or("主窗口不可用")?
                .set_fullscreen(enabled)
                .map_err(|e| e.to_string())?;
            engine.fullscreen.store(enabled, Ordering::Release);
            return Ok(engine.snapshot());
        }
        if let NativeAction::Subtitle { id: Some(id) } = &action {
            if !engine.snapshot().subtitles.iter().any(|t| t.id == *id) {
                return Err("字幕轨道不存在".into());
            }
        }
        engine.request(action.command()?)?;
        let s = engine.refresh()?;
        if matches!(action, NativeAction::Pause { paused: true }) {
            self.persist_native(&engine, &s, true, false)?;
        }
        let _ = self.app.emit("learning-native-updated", &s);
        Ok(s)
    }
    pub fn native_viewport(&self, session: &str, viewport: NativeViewport) -> AppResult<()> {
        self.native_session(session)?.window.update(viewport)
    }
    pub fn native_save(&self, session: &str, ended: bool) -> AppResult<PlaybackProgress> {
        let engine = self.native_session(session)?;
        let _guard = engine.operations.lock().map_err(|_| "播放操作不可用")?;
        let s = engine.refresh()?;
        self.persist_native(&engine, &s, true, ended)
    }
    fn finish_native(&self, engine: &NativeSession) {
        if let Ok(_guard) = engine.operations.lock() {
            let _ = engine.request(NativeAction::Pause { paused: true }.command().unwrap());
            let s = engine.refresh().unwrap_or_else(|_| engine.snapshot());
            let _ = self.persist_native(engine, &s, true, false);
            engine.closed.store(true, Ordering::Release);
            let _ = engine.job.terminate();
            if engine.fullscreen.swap(false, Ordering::AcqRel) {
                let app = self.app.clone();
                let _ = self.app.run_on_main_thread(move || {
                    if let Some(main) = app.get_webview_window("main") {
                        let _ = main.set_fullscreen(false);
                    }
                });
            }
        }
    }
    pub fn native_stop(&self, session: &str) {
        let old = if let Ok(mut r) = self.native_player.lock() {
            if r.starting.as_deref() == Some(session) {
                r.starting = None;
            }
            if r.active
                .as_ref()
                .is_some_and(|s| s.snapshot().session == session)
            {
                r.active.take()
            } else {
                None
            }
        } else {
            None
        };
        if let Some(old) = old {
            self.finish_native(&old);
        }
    }
    pub fn native_pause(&self) {
        let id = self
            .native_player
            .lock()
            .ok()
            .and_then(|r| r.active.as_ref().map(|s| s.snapshot().session));
        if let Some(id) = id {
            let _ = self.native_control(&id, NativeAction::Pause { paused: true });
        }
    }
    pub fn native_shutdown(&self) {
        let old = self.native_player.lock().ok().and_then(|mut r| {
            r.starting = None;
            r.active.take()
        });
        if let Some(old) = old {
            self.finish_native(&old);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_rejects_unsafe_controls() {
        for action in [
            NativeAction::Seek { position: f64::NAN },
            NativeAction::Seek { position: -1. },
            NativeAction::Rate { rate: 3. },
            NativeAction::Volume {
                volume: f64::INFINITY,
            },
            NativeAction::LoadSubtitle {
                path: "https://example.com/a.srt".into(),
            },
            NativeAction::Subtitle { id: Some(-2) },
        ] {
            assert!(
                action.command().is_err(),
                "unsafe control accepted: {action:?}"
            );
        }
        assert_eq!(
            NativeAction::Rate { rate: 1.5 }.command().unwrap(),
            vec![json!("set_property"), json!("speed"), json!(1.5)]
        );
    }
    #[test]
    fn native_viewport_is_finite_and_bounded() {
        let rect = NativeViewport {
            left: 10.,
            top: 20.,
            width: 300.,
            height: 200.,
            visible: true,
        };
        let scaled = rect.physical(1.5, 900, 700).unwrap();
        assert_eq!(
            (scaled.x, scaled.y, scaled.width, scaled.height),
            (15, 30, 450, 300)
        );
        assert!(NativeViewport {
            left: f64::NAN,
            ..rect
        }
        .physical(1., 900, 700)
        .is_err());
        assert!(NativeViewport { width: -1., ..rect }
            .physical(1., 900, 700)
            .is_err());
        assert!(
            !NativeViewport { top: -20., ..rect }
                .physical(1., 900, 700)
                .unwrap()
                .visible
        );
    }
    #[test]
    fn native_ipc_matches_response_and_ignores_events() {
        assert!(parse_reply(r#"{"event":"property-change","data":3}"#, 7)
            .unwrap()
            .is_none());
        assert!(
            parse_reply(r#"{"request_id":8,"data":3,"error":"success"}"#, 7)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_reply(r#"{"request_id":7,"data":3,"error":"success"}"#, 7).unwrap(),
            Some(json!(3))
        );
        assert!(parse_reply(r#"{"request_id":7,"error":"failure"}"#, 7).is_err());
    }
}
