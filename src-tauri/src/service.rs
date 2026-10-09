use crate::{
    domain::{self, AppResult},
    engine,
    models::*,
    native,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};

static IDS: AtomicU64 = AtomicU64::new(0);
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn unique() -> String {
    format!(
        "{}-{}-{}",
        now(),
        std::process::id(),
        IDS.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn hash_file(path: &Path) -> AppResult<String> {
    let mut input = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn verify_hash(path: &Path, expected: &str) -> AppResult<()> {
    if expected.len() != 64
        || !expected.chars().all(|c| c.is_ascii_hexdigit())
        || hash_file(path)? != expected.to_ascii_lowercase()
    {
        return Err("组件 SHA256 校验失败，已保留原版本".into());
    }
    Ok(())
}
fn valid_directory(raw: &str) -> AppResult<PathBuf> {
    if raw.chars().any(char::is_control) || !Path::new(raw).is_absolute() {
        return Err("请选择有效的绝对下载目录".into());
    }
    let path = PathBuf::from(raw);
    fs::create_dir_all(&path).map_err(|e| format!("无法创建下载目录：{e}"))?;
    let probe = path.join(format!(".video-downloader-write-{}.tmp", unique()));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|e| format!("下载目录不可写：{e}"))?;
    drop(file);
    fs::remove_file(&probe).map_err(|e| e.to_string())?;
    Ok(path)
}

pub struct Runtime {
    pub disk: DiskState,
    pub running: HashMap<String, Option<Arc<native::Job>>>,
    pub previewing: bool,
    pub preview_cancelled: bool,
    pub preview_jobs: Vec<Arc<native::Job>>,
    pub updating: bool,
    pub exiting: bool,
    pub engine: EngineInfo,
    pub notice: Option<String>,
}
pub struct Service {
    pub app: AppHandle,
    pub state: Mutex<Runtime>,
    pub data: PathBuf,
    pub resources: PathBuf,
    pub yt: PathBuf,
    batch_gate: Mutex<()>,
}
struct Context {
    args: Vec<String>,
    temporary_cookie: Option<PathBuf>,
}
impl Drop for Context {
    fn drop(&mut self) {
        if let Some(path) = &self.temporary_cookie {
            let _ = fs::remove_file(path);
        }
    }
}

impl Service {
    pub fn new(app: AppHandle) -> AppResult<Arc<Self>> {
        let data = std::env::var_os("VIDEO_DOWNLOADER_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or(app.path().app_local_data_dir().map_err(|e| e.to_string())?);
        fs::create_dir_all(data.join("engine")).map_err(|e| e.to_string())?;
        let cookie_directory = data.join("auth-temp");
        fs::create_dir_all(&cookie_directory).map_err(|e| e.to_string())?;
        // Only app-owned temporary Cookie files are removed after an interrupted run.
        for entry in fs::read_dir(&cookie_directory)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            if entry.file_type().is_ok_and(|kind| kind.is_file()) {
                let _ = fs::remove_file(entry.path());
            }
        }
        let state_path = data.join("state.json");
        let mut disk: DiskState =
            if state_path.exists() || state_path.with_extension("bak").exists() {
                serde_json::from_value(native::load_with_backup(&state_path)?)
                    .map_err(|e| format!("设置数据无法解析：{e}"))?
            } else {
                DiskState::default()
            };
        if disk.schema_version > 1 {
            return Err("此数据由更新版本的软件创建，请升级软件".into());
        }
        disk.schema_version = 1;
        engine::recover_tasks(&mut disk.tasks);
        if disk.settings.download_dir.is_empty() {
            disk.settings.download_dir = app
                .path()
                .download_dir()
                .map_err(|e| e.to_string())?
                .join("视频下载")
                .to_string_lossy()
                .into_owned();
        }
        let mut resources = app.path().resource_dir().map_err(|e| e.to_string())?;
        if cfg!(debug_assertions) && !resources.join("tools/yt-dlp.exe").is_file() {
            resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
        }
        let yt = data.join("engine/yt-dlp.exe");
        let result = Arc::new(Self {
            app,
            data,
            resources,
            yt,
            batch_gate: Mutex::new(()),
            state: Mutex::new(Runtime {
                disk,
                running: HashMap::new(),
                previewing: false,
                preview_cancelled: false,
                preview_jobs: Vec::new(),
                updating: false,
                exiting: false,
                engine: EngineInfo::default(),
                notice: None,
            }),
        });
        result.persist(&result.state.lock().unwrap())?;
        Ok(result)
    }
    fn persist(&self, state: &Runtime) -> AppResult<()> {
        // ponytail: one JSON snapshot suits a personal queue; use SQLite when large histories make measured saves slow.
        native::save_atomic(
            &self.data.join("state.json"),
            &serde_json::to_vec(&state.disk).map_err(|e| e.to_string())?,
        )
    }
    fn commit_disk(&self, state: &mut Runtime, next: DiskState) -> AppResult<()> {
        native::save_atomic(
            &self.data.join("state.json"),
            &serde_json::to_vec(&next).map_err(|e| e.to_string())?,
        )?;
        state.disk = next;
        Ok(())
    }
    pub fn snapshot(&self) -> AppSnapshot {
        let state = self.state.lock().unwrap();
        AppSnapshot {
            settings: state.disk.settings.clone(),
            tasks: state.disk.tasks.clone(),
            engine: state.engine.clone(),
            updating: state.updating,
            previewing: state.previewing,
            notice: state.notice.clone(),
        }
    }
    fn publish(&self) {
        let _ = self.app.emit("snapshot-updated", self.snapshot());
    }
    fn changed(&self, task: &DownloadTask) {
        let _ = self.app.emit("task-updated", task);
    }
    pub fn start(self: &Arc<Self>) {
        let service = self.clone();
        std::thread::spawn(move || {
            let checked = service.initialize_tools();
            {
                let mut state = service.state.lock().unwrap();
                match checked {
                    Ok(info) => state.engine = info,
                    Err(error) => state.engine.error = Some(domain::redact(&error)),
                }
            }
            service.publish();
            loop {
                if service.state.lock().unwrap().exiting {
                    break;
                }
                service.schedule();
                std::thread::sleep(Duration::from_millis(250));
            }
        });
    }
    fn initialize_tools(&self) -> AppResult<EngineInfo> {
        let tools = self.resources.join("tools");
        let manifest: Value = serde_json::from_slice(
            &fs::read(tools.join("binaries.json"))
                .map_err(|_| "下载组件尚未准备，请运行 npm run prepare:tools 后重新构建")?,
        )
        .map_err(|e| e.to_string())?;
        for binary in ["yt-dlp.exe", "ffmpeg.exe", "ffprobe.exe", "deno.exe"] {
            verify_hash(
                &tools.join(binary),
                manifest[binary]["sha256"]
                    .as_str()
                    .ok_or("组件校验清单不完整")?,
            )
            .map_err(|error| format!("{binary} 无法通过组件检查：{error}。请重新安装软件。"))?;
        }
        if !self.yt.is_file() {
            fs::copy(tools.join("yt-dlp.exe"), &self.yt).map_err(|e| e.to_string())?;
        }
        let version =
            match native::capture(&self.yt, &["--ignore-config".into(), "--version".into()]) {
                Ok(version) => version,
                Err(error) => {
                    let backup = self.yt.with_extension("backup.exe");
                    if !backup.is_file() {
                        return Err(format!("下载内核无法启动：{error}"));
                    }
                    let version =
                        native::capture(&backup, &["--ignore-config".into(), "--version".into()])?;
                    let restored = self.yt.with_extension("restore.exe");
                    fs::copy(&backup, &restored).map_err(|e| e.to_string())?;
                    native::replace_file(&restored, &self.yt)?;
                    version
                }
            };
        let ffmpeg_version = native::capture(&tools.join("ffmpeg.exe"), &["-version".into()])?
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        let deno_version = native::capture(&tools.join("deno.exe"), &["--version".into()])?
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        Ok(EngineInfo {
            version,
            ready: true,
            ffmpeg_version,
            deno_version,
            error: None,
        })
    }
    fn context(&self, disk: &DiskState) -> AppResult<Context> {
        let mut args = vec![
            "--ffmpeg-location".into(),
            self.resources.join("tools").to_string_lossy().into_owned(),
            "--js-runtimes".into(),
            format!("deno:{}", self.resources.join("tools/deno.exe").display()),
        ];
        let proxy = if disk.settings.proxy_enabled && !disk.proxy_secret.is_empty() {
            String::from_utf8(native::unprotect(&disk.proxy_secret)?)
                .map_err(|_| "代理设置无法解密")?
        } else {
            String::new()
        };
        args.extend(["--proxy".into(), proxy]);
        let mut temporary_cookie = None;
        match disk.settings.cookie_mode.as_str() {
            "browser" => {
                let browser = if disk.settings.browser_profile.is_empty() {
                    disk.settings.browser.clone()
                } else {
                    format!(
                        "{}:{}",
                        disk.settings.browser, disk.settings.browser_profile
                    )
                };
                args.extend(["--cookies-from-browser".into(), browser]);
            }
            "file" => {
                if disk.cookie_secret.is_empty() {
                    return Err("请先在设置中导入 Cookie 文件".into());
                }
                let path = self
                    .data
                    .join("auth-temp")
                    .join(format!("{}.txt", unique()));
                fs::write(&path, native::unprotect(&disk.cookie_secret)?)
                    .map_err(|e| e.to_string())?;
                args.extend(["--cookies".into(), path.to_string_lossy().into_owned()]);
                temporary_cookie = Some(path);
            }
            _ => {}
        }
        Ok(Context {
            args,
            temporary_cookie,
        })
    }
    pub fn preview(&self, urls: Vec<String>) -> AppResult<Vec<PreviewResult>> {
        if urls.is_empty() || urls.len() > 100 {
            return Err("请提供 1–100 个视频链接，播放列表作为一个链接解析".into());
        }
        let urls: Vec<_> = urls
            .iter()
            .map(|url| domain::validate_url(url))
            .collect::<AppResult<_>>()?;
        let disk = {
            let mut state = self.state.lock().unwrap();
            if state.updating || state.previewing || state.exiting {
                return Err("已有解析或更新正在运行".into());
            }
            if !state.engine.ready {
                return Err(state
                    .engine
                    .error
                    .clone()
                    .unwrap_or("正在检查下载组件，请稍候".into()));
            }
            state.previewing = true;
            state.preview_cancelled = false;
            state.disk.clone()
        };
        self.publish();
        let results = (|| {
            let context = self.context(&disk)?;
            let mut results = Vec::new();
            for url in urls {
                if self.state.lock().unwrap().preview_cancelled {
                    break;
                }
                let mut args = engine::base_args();
                args.extend(context.args.clone());
                args.extend(
                    [
                        "-t",
                        "mp4",
                        "--flat-playlist",
                        "--dump-single-json",
                        "--skip-download",
                        "--",
                        &url,
                    ]
                    .map(str::to_string),
                );
                let result = native::capture_with(&self.yt, &args, |job| {
                    let mut state = self.state.lock().unwrap();
                    if state.preview_cancelled || state.exiting {
                        let _ = job.terminate();
                    }
                    state.preview_jobs.push(job);
                })
                .and_then(|text| {
                    serde_json::from_str::<Value>(&text)
                        .map_err(|e| format!("内核信息无法解析：{e}"))
                })
                .and_then(|json| engine::media_preview(&url, &json));
                self.state.lock().unwrap().preview_jobs.clear();
                let result = match result {
                    Ok(preview) => PreviewResult {
                        url,
                        preview: Some(preview),
                        error: None,
                    },
                    Err(error) => PreviewResult {
                        url,
                        preview: None,
                        error: Some(domain::redact(&error)),
                    },
                };
                let _ = self.app.emit("preview-result", &result);
                results.push(result);
            }
            Ok(results)
        })();
        {
            let mut state = self.state.lock().unwrap();
            state.previewing = false;
            state.preview_jobs.clear();
        }
        self.publish();
        results
    }
    pub fn cancel_preview(&self) {
        let jobs = {
            let mut state = self.state.lock().unwrap();
            state.preview_cancelled = true;
            state.preview_jobs.clone()
        };
        for job in jobs {
            let _ = job.terminate();
        }
    }
    pub fn enqueue(&self, requests: Vec<DownloadRequest>) -> AppResult<Vec<DownloadTask>> {
        if requests.is_empty() || requests.len() > 1000 {
            return Err("一次请选择 1–1000 个下载条目".into());
        }
        let mut state = self.state.lock().unwrap();
        if state.exiting || state.updating {
            return Err("软件正在退出或更新，请稍候".into());
        }
        if !state.engine.ready {
            return Err("下载组件尚未就绪".into());
        }
        let output = valid_directory(&state.disk.settings.download_dir)?
            .to_string_lossy()
            .into_owned();
        let mut tasks = Vec::new();
        for mut request in requests {
            request.url = domain::validate_url(&request.url)?;
            engine::download_args(&request, &output)?;
            if state
                .disk
                .tasks
                .iter()
                .chain(tasks.iter())
                .any(|task: &DownloadTask| {
                    task.request.url == request.url
                        && task.request.kind == request.kind
                        && task.request.video_mode == request.video_mode
                        && task.request.max_height == request.max_height
                        && task.request.subtitle_languages == request.subtitle_languages
                        && (task.status.active()
                            || matches!(task.status, TaskStatus::Queued | TaskStatus::Paused))
                })
            {
                continue;
            }
            if request.title.is_empty() {
                request.title = request.url.clone();
            }
            tasks.push(DownloadTask {
                id: unique(),
                request,
                output_dir: output.clone(),
                phase: "等待下载".into(),
                created_at: now(),
                ..Default::default()
            });
        }
        let mut next = state.disk.clone();
        next.tasks.extend(tasks.clone());
        self.commit_disk(&mut state, next)?;
        drop(state);
        self.publish();
        Ok(tasks)
    }
    pub fn control(&self, id: &str, action: &str) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        let mut next = state.disk.clone();
        let task = next
            .tasks
            .iter_mut()
            .find(|task| task.id == id)
            .ok_or("任务不存在")?;
        engine::transition(task, action)?;
        let job = if matches!(action, "pause" | "cancel") {
            state.running.get(id).cloned().flatten()
        } else {
            None
        };
        self.commit_disk(&mut state, next)?;
        drop(state);
        if let Some(job) = job {
            job.terminate()?;
        }
        self.publish();
        Ok(())
    }
    pub fn remove_task(&self, id: &str) -> AppResult<()> {
        let mut state = self.state.lock().unwrap();
        let mut next = state.disk.clone();
        crate::batch::remove_record(
            &mut next.tasks,
            id,
            &state.running.keys().cloned().collect::<Vec<_>>(),
        )?;
        self.commit_disk(&mut state, next)?;
        drop(state);
        self.publish();
        Ok(())
    }
    pub fn batch_task_action(
        self: &Arc<Self>,
        ids: Vec<String>,
        action: crate::batch::BatchAction,
    ) -> AppResult<crate::batch::BatchResult> {
        use crate::batch::{self, BatchAction};
        let gate = self.batch_gate.lock().unwrap();
        let result = if matches!(action, BatchAction::Remove | BatchAction::Pin) {
            let mut state = self.state.lock().unwrap();
            let running = state.running.keys().cloned().collect::<Vec<_>>();
            batch::records(&mut state.disk, &ids, action, &running, |next| {
                native::save_atomic(
                    &self.data.join("state.json"),
                    &serde_json::to_vec(next).map_err(|e| e.to_string())?,
                )
            })?
        } else {
            let plan = {
                let state = self.state.lock().unwrap();
                batch::prepare(&state.disk.tasks, &ids, action, &[])?
            };
            batch::execute(
                plan,
                |id| {
                    if action == BatchAction::Copy {
                        return Ok(());
                    }
                    {
                        let state = self.state.lock().unwrap();
                        let task = state
                            .disk
                            .tasks
                            .iter()
                            .find(|task| task.id == id)
                            .ok_or("任务不存在")?;
                        if let Some(reason) = batch::skip_reason(&task.status, action) {
                            return Err(reason.into());
                        }
                    }
                    self.control(id, action.command())
                },
                || {},
            )
        };
        // Scheduler ticks cannot claim intermediate states while this gate is held.
        Ok(batch::complete(result, || {
            drop(gate);
            self.publish();
            self.schedule();
        }))
    }
    pub fn save_settings(&self, input: SettingsInput) -> AppResult<AppSettings> {
        if !(1..=4).contains(&input.concurrency) {
            return Err("并发数必须在 1–4 之间".into());
        }
        if !matches!(input.cookie_mode.as_str(), "none" | "browser" | "file")
            || !matches!(input.browser.as_str(), "chrome" | "edge" | "firefox")
            || input.browser_profile.chars().any(char::is_control)
        {
            return Err("登录设置无效".into());
        }
        let directory = valid_directory(&input.download_dir)?
            .to_string_lossy()
            .into_owned();
        let new_proxy = input
            .proxy_url
            .as_ref()
            .map(|raw| domain::validate_proxy(raw))
            .transpose()?;
        let mut state = self.state.lock().unwrap();
        let mut next = state.disk.clone();
        if input.cookie_mode == "file" && next.cookie_secret.is_empty() {
            return Err("请先导入 Cookie 文件".into());
        }
        if let Some(proxy) = new_proxy {
            next.proxy_secret = if proxy.is_empty() {
                Vec::new()
            } else {
                native::protect(proxy.as_bytes())?
            };
            next.settings.proxy_url = domain::redact(&proxy);
        }
        if input.proxy_enabled && next.proxy_secret.is_empty() {
            return Err("请填写代理地址后再启用代理".into());
        }
        next.settings.download_dir = directory;
        next.settings.concurrency = input.concurrency;
        next.settings.cookie_mode = input.cookie_mode;
        next.settings.browser = input.browser;
        next.settings.browser_profile = input.browser_profile.trim().into();
        next.settings.proxy_enabled = input.proxy_enabled;
        let settings = next.settings.clone();
        self.commit_disk(&mut state, next)?;
        drop(state);
        self.publish();
        Ok(settings)
    }
    pub fn import_cookies(&self, path: &str) -> AppResult<()> {
        let metadata = fs::metadata(path).map_err(|e| format!("Cookie 文件无法读取：{e}"))?;
        if !metadata.is_file() || metadata.len() > 2 * 1024 * 1024 {
            return Err("请选择小于 2 MiB 的 Netscape 格式 Cookie 文件".into());
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        let text = std::str::from_utf8(&bytes).map_err(|_| "Cookie 文件应使用 UTF-8 编码")?;
        if !text.lines().any(|line| {
            line.contains("Netscape HTTP Cookie File") || line.contains("HTTP Cookie File")
        }) || !text.lines().any(|line| {
            (!line.starts_with('#') || line.starts_with("#HttpOnly_"))
                && line.split('\t').count() == 7
        }) {
            return Err("这不是有效的 Netscape Cookie 文件，请重新导出".into());
        }
        let sealed = native::protect(&bytes)?;
        let mut state = self.state.lock().unwrap();
        let mut next = state.disk.clone();
        next.cookie_secret = sealed;
        next.settings.has_cookie_file = true;
        next.settings.cookie_mode = "file".into();
        self.commit_disk(&mut state, next)?;
        drop(state);
        self.publish();
        Ok(())
    }
    fn schedule(self: &Arc<Self>) {
        let Ok(_gate) = self.batch_gate.try_lock() else {
            return;
        };
        let claims = {
            let mut state = self.state.lock().unwrap();
            if state.exiting || state.updating || !state.engine.ready {
                return;
            }
            let ids = engine::queued_ids(
                &state.disk.tasks,
                state.disk.settings.concurrency as usize,
                &state.running.keys().cloned().collect::<Vec<_>>(),
            );
            let mut claims = Vec::new();
            for id in ids {
                let task = state
                    .disk
                    .tasks
                    .iter_mut()
                    .find(|task| task.id == id)
                    .unwrap();
                task.status = TaskStatus::Resolving;
                task.phase = "正在获取下载信息".into();
                claims.push(task.clone());
                state.running.insert(id, None);
            }
            if !claims.is_empty() {
                if let Err(error) = self.persist(&state) {
                    for task in &claims {
                        state.running.remove(&task.id);
                        if let Some(task) = state
                            .disk
                            .tasks
                            .iter_mut()
                            .find(|entry| entry.id == task.id)
                        {
                            task.status = TaskStatus::Paused;
                            task.phase = "任务无法保存，请检查磁盘后恢复".into();
                        }
                    }
                    state.notice = Some(format!("任务保存失败：{error}"));
                    claims.clear();
                }
            }
            claims
        };
        for task in claims {
            self.changed(&task);
            let service = self.clone();
            std::thread::spawn(move || {
                let result = service.run_task(&task);
                service.finish_task(&task.id, result);
            });
        }
    }
    fn register(&self, id: &str, job: Arc<native::Job>) {
        let mut state = self.state.lock().unwrap();
        if state.exiting
            || !state
                .disk
                .tasks
                .iter()
                .any(|task| task.id == id && task.status.active())
        {
            let _ = job.terminate();
        }
        if state.running.contains_key(id) {
            state.running.insert(id.to_string(), Some(job));
        }
    }
    fn mutate_task(&self, id: &str, update: impl FnOnce(&mut DownloadTask)) {
        let changed = {
            let mut state = self.state.lock().unwrap();
            state
                .disk
                .tasks
                .iter_mut()
                .find(|task| task.id == id && task.status.active())
                .map(|task| {
                    update(task);
                    task.clone()
                })
        };
        if let Some(task) = changed {
            self.changed(&task);
        }
    }
    fn run_task(self: &Arc<Self>, task: &DownloadTask) -> AppResult<()> {
        let disk = self.state.lock().unwrap().disk.clone();
        let context = self.context(&disk)?;
        let mut preview_args = engine::base_args();
        preview_args.extend(context.args.clone());
        if task.request.kind == MediaKind::Video {
            preview_args.extend(engine::video_args(
                &task.request.video_mode,
                task.request.max_height,
            )?);
        }
        if task.request.kind == MediaKind::Audio {
            preview_args.extend(["-f".into(), "ba/b".into()]);
        }
        if task.request.kind == MediaKind::Subtitles {
            preview_args.push("--ignore-no-formats-error".into());
        }
        preview_args.extend(
            [
                "--no-playlist",
                "--dump-single-json",
                "--skip-download",
                "--",
                &task.request.url,
            ]
            .map(str::to_string),
        );
        let information =
            native::capture_with(&self.yt, &preview_args, |job| self.register(&task.id, job))?;
        let preview = engine::media_preview(
            &task.request.url,
            &serde_json::from_str::<Value>(&information).map_err(|e| e.to_string())?,
        )?;
        if task.request.kind == MediaKind::Video
            && task.request.video_mode == VideoMode::Compatible
            && !preview.compatible
        {
            return Err("源站没有 H.264/AAC 兼容组合。请重新解析，选择“源站最高画质”下载；软件不会自动执行耗时的视频转码。".into());
        }
        self.mutate_task(&task.id, |task| {
            task.request.title = preview.title;
            task.request.thumbnail = preview.thumbnail;
            task.status = TaskStatus::Downloading;
            task.phase = if task.request.kind == MediaKind::Subtitles {
                "正在获取字幕"
            } else {
                "正在下载"
            }
            .into();
        });
        if !self
            .state
            .lock()
            .unwrap()
            .disk
            .tasks
            .iter()
            .any(|entry| entry.id == task.id && entry.status.active())
        {
            return Ok(());
        }
        let mut args = engine::task_args(task)?;
        args.splice(0..0, context.args.clone());
        let (process, stdout, stderr) = native::spawn(&self.yt, &args)?;
        self.register(&task.id, process.job.clone());
        let service = self.clone();
        let task_id = task.id.clone();
        let errors = std::thread::spawn(move || -> AppResult<Vec<u8>> {
            let mut errors = Vec::new();
            for line in BufReader::new(stderr).lines() {
                let line = line.map_err(|error| error.to_string())?;
                if line.starts_with("VD_PROGRESS") || line.starts_with("VD_PROCESS") {
                    service.process_line(&task_id, &line);
                } else {
                    let bytes = line.as_bytes();
                    errors.extend_from_slice(
                        &bytes[..bytes
                            .len()
                            .min((64 * 1024usize).saturating_sub(errors.len()))],
                    );
                    if errors.len() < 64 * 1024 {
                        errors.push(b'\n');
                    }
                }
            }
            Ok(errors)
        });
        let mut read_error = None;
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => self.process_line(&task.id, &line),
                Err(error) => {
                    let _ = process.job.terminate();
                    read_error = Some(error.to_string());
                    break;
                }
            }
        }
        let code = process.wait()?;
        let errors = errors.join().map_err(|_| "读取下载日志失败")??;
        let errors = domain::redact(&String::from_utf8_lossy(&errors));
        self.mutate_task(&task.id, |task| {
            task.logs.extend(
                errors
                    .lines()
                    .rev()
                    .take(12)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(str::to_string),
            );
            if task.logs.len() > 60 {
                task.logs.drain(..task.logs.len() - 60);
            }
        });
        if let Some(error) = read_error {
            return Err(error);
        }
        if code != 0 {
            return Err(if errors.trim().is_empty() {
                "下载内核已停止，可点击重试".into()
            } else {
                errors
            });
        }
        let exists = self
            .state
            .lock()
            .unwrap()
            .disk
            .tasks
            .iter()
            .find(|entry| entry.id == task.id)
            .is_some_and(|task| {
                !task.files.is_empty() && task.files.iter().all(|file| Path::new(file).is_file())
            });
        if !exists {
            return Err(
                "内核结束但没有发现输出文件；可能没有所选字幕或源站不提供此格式，请重新解析".into(),
            );
        }
        Ok(())
    }
    fn process_line(&self, id: &str, line: &str) {
        if let Some(raw) = line.strip_prefix("VD_PROGRESS") {
            if let Ok(value) = serde_json::from_str::<Value>(raw) {
                self.mutate_task(id, |task| {
                    task.progress = domain::parse_progress(&value);
                    task.status = if value["status"] == "finished" {
                        TaskStatus::Processing
                    } else {
                        TaskStatus::Downloading
                    };
                    task.phase = if task.status == TaskStatus::Processing {
                        "下载片段完成，正在处理"
                    } else {
                        "正在下载"
                    }
                    .into();
                });
            }
        } else if line.starts_with("VD_PROCESS") {
            self.mutate_task(id, |task| {
                task.status = TaskStatus::Processing;
                task.phase = match task.request.kind {
                    MediaKind::Audio => "正在转换 MP3",
                    MediaKind::Subtitles => "正在转换字幕",
                    _ => "正在合并音视频",
                }
                .into();
            });
        } else if let Some(raw) = line
            .strip_prefix("VD_FILE")
            .or_else(|| line.strip_prefix("VD_SUBS"))
        {
            if let Ok(value) = serde_json::from_str::<Value>(raw) {
                let paths: Vec<&str> = if let Some(path) = value.as_str() {
                    vec![path]
                } else {
                    value
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .collect()
                };
                self.mutate_task(id, |task| {
                    for path in paths {
                        let path = engine::final_output_path(task, path)
                            .to_string_lossy()
                            .into_owned();
                        if !task.files.contains(&path) {
                            task.files.push(path);
                        }
                    }
                });
            }
        } else if !line.trim().is_empty() {
            self.mutate_task(id, |task| {
                task.logs.push(domain::redact(line));
                if task.logs.len() > 60 {
                    task.logs.remove(0);
                }
            });
        }
    }
    fn finish_task(&self, id: &str, result: AppResult<()>) {
        let mut state = self.state.lock().unwrap();
        state.running.remove(id);
        if let Some(task) = state
            .disk
            .tasks
            .iter_mut()
            .find(|task| task.id == id && task.status.active())
        {
            task.finished_at = Some(now());
            match result {
                Ok(()) => {
                    task.status = TaskStatus::Completed;
                    task.phase = "下载完成".into();
                    task.progress.percent = Some(100.0);
                    task.progress.speed = None;
                    task.progress.eta = None;
                }
                Err(error) => {
                    task.status = TaskStatus::Failed;
                    task.phase = "下载失败，可重试".into();
                    task.error = Some(domain::redact(&error));
                }
            }
        }
        if let Err(error) = self.persist(&state) {
            state.notice = Some(format!("任务记录保存失败：{error}"));
        }
        drop(state);
        self.publish();
    }
    fn curl(&self, url: &str, destination: Option<&Path>) -> AppResult<String> {
        let mut args = [
            "-sS",
            "--fail",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "20",
            "--max-time",
            "300",
            "--user-agent",
            "Windows-Video-Downloader/0.1",
        ]
        .map(str::to_string)
        .to_vec();
        {
            let state = self.state.lock().unwrap();
            if state.disk.settings.proxy_enabled {
                args.extend([
                    "--proxy".into(),
                    String::from_utf8(native::unprotect(&state.disk.proxy_secret)?)
                        .map_err(|_| "代理解密失败")?,
                ]);
            } else {
                args.extend(["--proxy".into(), String::new()]);
            }
        }
        if let Some(path) = destination {
            args.extend(["--output".into(), path.to_string_lossy().into_owned()]);
        }
        args.push(url.to_string());
        let executable =
            PathBuf::from(std::env::var_os("SystemRoot").unwrap_or("C:/Windows".into()))
                .join("System32/curl.exe");
        native::capture(&executable, &args)
    }
    pub fn check_update(&self) -> AppResult<EngineUpdate> {
        let json: Value = serde_json::from_str(&self.curl(
            "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest",
            None,
        )?)
        .map_err(|e| e.to_string())?;
        let version = json["tag_name"]
            .as_str()
            .ok_or("官方版本信息不完整")?
            .to_string();
        if version.is_empty() || !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Err("官方版本号无法验证".into());
        }
        let current_version = self.state.lock().unwrap().engine.version.clone();
        Ok(EngineUpdate {
            available: current_version != version,
            current_version,
            version,
            published_at: json["published_at"].as_str().unwrap_or_default().into(),
        })
    }
    pub fn update_engine(&self) -> AppResult<EngineInfo> {
        {
            let mut state = self.state.lock().unwrap();
            if state.updating
                || state.previewing
                || !state.running.is_empty()
                || state
                    .disk
                    .tasks
                    .iter()
                    .any(|task| task.status == TaskStatus::Queued)
                || state.exiting
            {
                return Err("请先暂停或完成所有解析与下载，再更新内核".into());
            }
            state.updating = true;
        }
        self.publish();
        let result = (|| {
            let update = self.check_update()?;
            if !update.available {
                return Ok(self.state.lock().unwrap().engine.clone());
            }
            let candidate = self.data.join("engine/yt-dlp-candidate.exe");
            fs::copy(&self.yt, &candidate).map_err(|e| e.to_string())?;
            let disk = self.state.lock().unwrap().disk.clone();
            let context = self.context(&disk)?;
            let mut args = vec![
                "--ignore-config".into(),
                "--update-to".into(),
                format!("stable@{}", update.version),
            ];
            args.extend(context.args.clone());
            native::capture(&candidate, &args)?;
            let sums = self.curl(
                &format!(
                    "https://github.com/yt-dlp/yt-dlp/releases/download/{}/SHA2-256SUMS",
                    update.version
                ),
                None,
            )?;
            verify_hash(&candidate, &engine::checksum_entry(&sums, "yt-dlp.exe")?)?;
            let version =
                native::capture(&candidate, &["--ignore-config".into(), "--version".into()])?;
            if version.trim() != update.version {
                return Err("更新版本与官方校验信息不一致，已保留原版本".into());
            }
            native::activate_checked(&candidate, &self.yt, |path| {
                let actual =
                    native::capture(path, &["--ignore-config".into(), "--version".into()])?;
                if actual.trim() == update.version {
                    Ok(())
                } else {
                    Err("内核版本与官方版本不符".into())
                }
            })?;
            let mut info = self.state.lock().unwrap().engine.clone();
            info.version = version;
            info.ready = true;
            info.error = None;
            Ok(info)
        })();
        {
            let mut state = self.state.lock().unwrap();
            state.updating = false;
            if let Ok(info) = &result {
                state.engine = info.clone();
            }
        }
        self.publish();
        result
    }
    pub fn shutdown(&self) {
        let jobs = {
            let mut state = self.state.lock().unwrap();
            state.exiting = true;
            state.preview_cancelled = true;
            engine::recover_tasks(&mut state.disk.tasks);
            let mut jobs: Vec<_> = state.running.values().flatten().cloned().collect();
            jobs.extend(state.preview_jobs.clone());
            let _ = self.persist(&state);
            jobs
        };
        for job in jobs {
            let _ = job.terminate();
        }
    }
}

pub fn open_target(service: &Service, id: &str, folder: bool) -> AppResult<()> {
    let state = service.state.lock().unwrap();
    let task = state
        .disk
        .tasks
        .iter()
        .find(|task| task.id == id)
        .ok_or("任务不存在")?;
    let directory = fs::canonicalize(&task.output_dir).map_err(|e| e.to_string())?;
    let target = if folder {
        directory.clone()
    } else {
        fs::canonicalize(task.files.first().ok_or("尚未生成输出文件")?)
            .map_err(|e| e.to_string())?
    };
    if !target.starts_with(directory) {
        return Err("文件不在此任务的下载目录中".into());
    }
    drop(state);
    shell_open(target.to_str().ok_or("文件路径无法读取")?)
}
pub fn shell_open(target: &str) -> AppResult<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    let wide: Vec<u16> = std::ffi::OsStr::new(target)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let operation: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result <= 32 {
        Err(format!("无法打开文件或目录（Windows 错误 {result}）"))
    } else {
        Ok(())
    }
}
