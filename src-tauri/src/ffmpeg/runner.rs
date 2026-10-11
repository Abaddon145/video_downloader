use super::{command, models::*, output, probe, progress};
use crate::{
    domain::{redact, AppResult},
    native::{self, Job},
    service::{now, verify_hash},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub fn new_id() -> String {
    format!(
        "media-{}-{}",
        now(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn recovered_status(s: MediaTaskStatus) -> MediaTaskStatus {
    if matches!(
        s,
        MediaTaskStatus::Queued | MediaTaskStatus::Probing | MediaTaskStatus::Processing
    ) {
        MediaTaskStatus::Interrupted
    } else {
        s
    }
}
pub fn can_retry(s: MediaTaskStatus) -> bool {
    matches!(
        s,
        MediaTaskStatus::Failed | MediaTaskStatus::Cancelled | MediaTaskStatus::Interrupted
    )
}
pub fn friendly_error(raw: &str) -> String {
    let l = raw.to_ascii_lowercase();
    if l.contains("no space left") || l.contains("disk full") {
        "磁盘剩余空间不足".into()
    } else if l.contains("permission denied") || l.contains("access is denied") {
        "当前目录不可写，请更换保存位置".into()
    } else if l.contains("unknown encoder") || l.contains("encoder not found") {
        "当前 FFmpeg 不支持所选编码器，请更换编码格式".into()
    } else if l.contains("invalid data") || l.contains("moov atom") {
        "无法识别此媒体文件，文件可能损坏或格式不受支持".into()
    } else if l.contains("could not find tag") || l.contains("not supported in container") {
        "当前编码无法封装到目标容器，请选择兼容转换".into()
    } else if raw.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)) {
        raw.chars().take(240).collect()
    } else {
        "媒体处理失败，原始文件已保留；可展开技术详情".into()
    }
}
pub(crate) struct Runtime {
    pub(crate) cancelled_previews: HashSet<String>,
    tasks: Vec<MediaTask>,
    jobs: HashMap<String, Arc<Job>>,
    running: Option<String>,
    ready: bool,
    error: Option<String>,
    pub(crate) exiting: bool,
}
pub struct MediaService {
    pub(crate) app: tauri::AppHandle,
    pub(crate) state_file: PathBuf,
    pub(crate) resources: PathBuf,
    pub(crate) state: Mutex<Runtime>,
    pub(crate) pro: Mutex<crate::media::preset::MediaProSettings>,
    pub(crate) gpu_cache: Mutex<Option<Vec<String>>>,
}
impl MediaService {
    pub fn new(app: tauri::AppHandle, data: &Path, resources: &Path) -> AppResult<Arc<Self>> {
        let state_file = data.join("media-state.json");
        let loaded: AppResult<Vec<MediaTask>> = if state_file.exists()
            || state_file.with_extension("bak").exists()
        {
            native::load_with_backup(&state_file)
                .and_then(|v| serde_json::from_value(v).map_err(|_| "媒体任务记录无法读取".into()))
        } else {
            Ok(Vec::new())
        };
        let (mut tasks, mut startup_error) = match loaded {
            Ok(tasks) => (tasks, None),
            Err(_) => (
                Vec::new(),
                Some(
                    "媒体记录损坏，请保留 media-state.json 及备份后处理；下载功能仍可使用"
                        .to_string(),
                ),
            ),
        };
        for t in &mut tasks {
            let recovered = recovered_status(t.status);
            if recovered != t.status {
                t.status = recovered;
                t.phase = "上次处理已中断，请重新开始".into();
                t.progress = None;
                t.speed = None;
                t.eta = None;
            }
        }
        if startup_error.is_none() {
            if native::save_atomic(
                &state_file,
                &serde_json::to_vec(&tasks).map_err(|e| e.to_string())?,
            )
            .is_err()
            {
                startup_error = Some("无法保存媒体任务记录，请检查数据目录权限".into());
            }
        }
        let settings_file = data.join("media-settings.json");
        let pro = if settings_file.exists() {
            match native::load_with_backup(&settings_file)
                .and_then(|v| {
                    serde_json::from_value::<crate::media::preset::MediaProSettings>(v)
                        .map_err(|_| "媒体设置损坏".into())
                })
                .and_then(|s| {
                    crate::media::preset::validate(&s)?;
                    Ok(s)
                }) {
                Ok(s) => s,
                Err(e) => {
                    startup_error = Some(e);
                    Default::default()
                }
            }
        } else {
            Default::default()
        };
        Ok(Arc::new(Self {
            pro: Mutex::new(pro),
            gpu_cache: Mutex::new(None),
            app,
            state_file,
            resources: resources.into(),
            state: Mutex::new(Runtime {
                cancelled_previews: HashSet::new(),
                tasks,
                jobs: HashMap::new(),
                running: None,
                ready: false,
                error: startup_error,
                exiting: false,
            }),
        }))
    }
    fn commit(&self, state: &mut Runtime, tasks: Vec<MediaTask>) -> AppResult<()> {
        native::save_atomic(
            &self.state_file,
            &serde_json::to_vec(&tasks).map_err(|_| "媒体记录无法序列化")?,
        )
        .map_err(|_| "无法保存媒体任务记录，请检查磁盘空间和数据目录权限")?;
        state.tasks = tasks;
        Ok(())
    }
    pub fn snapshot(&self) -> MediaSnapshot {
        let s = self.state.lock().unwrap();
        MediaSnapshot {
            tasks: s.tasks.clone(),
            ready: s.ready,
            error: s.error.clone(),
        }
    }
    fn emit(&self) {
        let _ = self.app.emit("media-snapshot-updated", self.snapshot());
    }
    pub fn start(self: &Arc<Self>) {
        let service = self.clone();
        std::thread::spawn(move || {
            if service.state.lock().unwrap().error.is_some() {
                service.emit();
                return;
            }
            let checked = service.check_tools();
            {
                let mut s = service.state.lock().unwrap();
                match checked {
                    Ok(()) => s.ready = true,
                    Err(e) => s.error = Some(e),
                }
            }
            service.emit();
            if let Some(download) = service.app.try_state::<Arc<crate::service::Service>>() {
                for task in download.snapshot().tasks {
                    if let Err(e) = service.after_download(&task) {
                        let _ = service.app.emit("media-automation-error", e);
                    }
                }
            }
            loop {
                if service.state.lock().unwrap().exiting {
                    break;
                }
                service.schedule();
                std::thread::sleep(Duration::from_millis(250));
            }
        });
    }
    fn check_tools(&self) -> AppResult<()> {
        let path = self.resources.join("tools");
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(path.join("binaries.json"))
                .map_err(|_| "FFmpeg 组件校验失败，请重新安装映流")?,
        )
        .map_err(|_| "FFmpeg 组件校验失败，请重新安装映流")?;
        for binary in ["ffmpeg.exe", "ffprobe.exe"] {
            verify_hash(
                &path.join(binary),
                manifest[binary]["sha256"]
                    .as_str()
                    .ok_or("FFmpeg 校验清单不完整")?,
            )
            .map_err(|_| "FFmpeg 组件校验失败，请重新安装映流")?;
        }
        Ok(())
    }
    pub(crate) fn ensure_ready(&self) -> AppResult<()> {
        let s = self.state.lock().unwrap();
        if s.exiting {
            return Err("程序正在退出".into());
        }
        if !s.ready {
            return Err(s
                .error
                .clone()
                .unwrap_or_else(|| "正在校验媒体组件，请稍后重试".into()));
        }
        Ok(())
    }
    fn register(&self, id: &str, job: Arc<Job>, task: bool) -> AppResult<()> {
        let mut s = self.state.lock().unwrap();
        if s.exiting
            || s.tasks.iter().any(|t| {
                id.starts_with(&format!("{}-", t.id))
                    && matches!(
                        t.status,
                        MediaTaskStatus::Cancelled | MediaTaskStatus::Interrupted
                    )
            })
            || s.cancelled_previews
                .iter()
                .any(|session| id.starts_with(&format!("{session}-")))
            || task
                && !s.tasks.iter().any(|t| {
                    t.id == id
                        && matches!(
                            t.status,
                            MediaTaskStatus::Probing | MediaTaskStatus::Processing
                        )
                })
        {
            let _ = job.terminate();
            return Err("媒体任务已取消或中断".into());
        }
        s.jobs.insert(id.into(), job);
        Ok(())
    }
    pub(crate) fn probe_file(&self, path: &Path, id: &str, task: bool) -> AppResult<MediaInfo> {
        let (done_tx, done_rx) = mpsc::channel();
        let mut registration = Ok(());
        let mut watcher = None;
        let result = native::capture_with(
            &self.resources.join("tools/ffprobe.exe"),
            &probe::arguments(path),
            |job| {
                registration = self.register(id, job.clone(), task);
                watcher = Some(std::thread::spawn(move || {
                    if done_rx.recv_timeout(Duration::from_secs(30)).is_err() {
                        let _ = job.terminate();
                    }
                }));
            },
        );
        let _ = done_tx.send(());
        if let Some(w) = watcher {
            let _ = w.join();
        }
        self.state.lock().unwrap().jobs.remove(id);
        registration?;
        let text = result.map_err(|e| friendly_error(&e))?;
        probe::parse(&text, path)
    }
    pub fn probe(&self, path: &str) -> AppResult<MediaInfo> {
        self.ensure_ready()?;
        let path = output::input(path)?;
        let id = new_id();
        self.probe_file(&path, &id, false)
    }
    pub fn create(&self, mut request: MediaRequest) -> AppResult<MediaTask> {
        self.ensure_ready()?;
        crate::media::metadata::validate(&request)?;
        if request
            .subtitle_path
            .as_ref()
            .is_some_and(|p| p.len() > 30000)
            || request.cover_path.as_ref().is_some_and(|p| p.len() > 30000)
        {
            return Err("辅助文件路径过长".into());
        }
        let input = output::input(&request.input_path)?;
        let directory = output::directory(&request.output_dir)?;
        request.input_path = input.to_string_lossy().into_owned();
        request.output_dir = directory.to_string_lossy().into_owned();
        let task = MediaTask {
            id: new_id(),
            kind: request.operation,
            input_path: request.input_path.clone(),
            output_path: None,
            request,
            status: MediaTaskStatus::Queued,
            phase: "等待处理".into(),
            progress: None,
            speed: None,
            processed_time: None,
            total_duration: None,
            eta: None,
            error: None,
            logs: Vec::new(),
            created_at: now(),
            finished_at: None,
        };
        {
            let mut s = self.state.lock().unwrap();
            if s.exiting {
                return Err("程序正在退出".into());
            }
            if task
                .request
                .source_download_id
                .as_ref()
                .is_some_and(|id| id.len() > 140 || id.chars().any(char::is_control))
            {
                return Err("来源下载标识无效".into());
            }
            if let Some(existing) = s.tasks.iter().find(|t| {
                task.request.source_download_id.is_some()
                    && t.request.source_download_id == task.request.source_download_id
                    && t.input_path == task.input_path
            }) {
                return Ok(existing.clone());
            }
            let mut next = s.tasks.clone();
            next.push(task.clone());
            self.commit(&mut s, next)?;
        }
        self.emit();
        Ok(task)
    }
    pub fn cancel(&self, id: &str) -> AppResult<()> {
        let job = {
            let mut s = self.state.lock().unwrap();
            let mut next = s.tasks.clone();
            let t = next
                .iter_mut()
                .find(|t| t.id == id)
                .ok_or("媒体任务不存在")?;
            if !matches!(
                t.status,
                MediaTaskStatus::Queued | MediaTaskStatus::Probing | MediaTaskStatus::Processing
            ) {
                return Err("此任务当前不能取消".into());
            }
            t.status = MediaTaskStatus::Cancelled;
            t.phase = "已取消，原始文件已保留".into();
            t.finished_at = Some(now());
            t.speed = None;
            t.eta = None;
            for (job_id, job) in &s.jobs {
                if job_id == id || job_id.starts_with(&format!("{id}-")) {
                    job.terminate().map_err(|_| "无法终止媒体进程，请重试")?;
                }
            }
            self.commit(&mut s, next)?;
            None::<Arc<Job>>
        };
        if let Some(job) = job {
            job.terminate().map_err(|_| "无法终止媒体进程，请重试")?;
        }
        self.emit();
        Ok(())
    }
    pub fn retry(&self, id: &str) -> AppResult<()> {
        {
            let mut s = self.state.lock().unwrap();
            if s.exiting {
                return Err("程序正在退出".into());
            }
            if s.running.as_deref() == Some(id) {
                return Err("任务进程正在退出，请稍后重试".into());
            }
            let mut next = s.tasks.clone();
            let t = next
                .iter_mut()
                .find(|t| t.id == id)
                .ok_or("媒体任务不存在")?;
            if !can_retry(t.status) {
                return Err("只有失败、取消或中断任务可以重新开始".into());
            }
            t.status = MediaTaskStatus::Queued;
            t.phase = "等待重新开始".into();
            t.error = None;
            t.logs.clear();
            t.progress = None;
            t.processed_time = None;
            t.speed = None;
            t.eta = None;
            t.finished_at = None;
            t.output_path = None;
            self.commit(&mut s, next)?;
        }
        self.emit();
        Ok(())
    }
    fn schedule(self: &Arc<Self>) {
        let task = {
            let mut s = self.state.lock().unwrap();
            if !s.ready || s.exiting || s.running.is_some() {
                return;
            }
            let mut next = s.tasks.clone();
            let Some(t) = next
                .iter_mut()
                .find(|t| t.status == MediaTaskStatus::Queued)
            else {
                return;
            };
            t.status = MediaTaskStatus::Probing;
            t.phase = "分析媒体信息".into();
            let task = t.clone();
            if let Err(e) = self.commit(&mut s, next) {
                s.error = Some(e);
                s.ready = false;
                drop(s);
                self.emit();
                return;
            }
            s.running = Some(task.id.clone());
            task
        };
        self.emit();
        let service = self.clone();
        std::thread::spawn(move || service.worker(task));
    }
    fn worker(&self, task: MediaTask) {
        let result = self.process(&task);
        {
            let mut s = self.state.lock().unwrap();
            s.jobs.remove(&task.id);
            s.running = None;
            if let Err(error) = result {
                let mut next = s.tasks.clone();
                if let Some(t) = next.iter_mut().find(|t| t.id == task.id) {
                    if matches!(
                        t.status,
                        MediaTaskStatus::Probing | MediaTaskStatus::Processing
                    ) {
                        t.status = MediaTaskStatus::Failed;
                        t.phase = "处理失败".into();
                        t.error = Some(friendly_error(&error));
                        t.logs.extend(
                            redact(&error)
                                .lines()
                                .take(40)
                                .map(|l| l.chars().take(2048).collect()),
                        );
                        t.finished_at = Some(now());
                        t.speed = None;
                        t.eta = None;
                    }
                }
                if let Err(e) = self.commit(&mut s, next) {
                    s.error = Some(e);
                    s.ready = false;
                }
            }
        }
        self.emit();
    }
    fn process(&self, task: &MediaTask) -> AppResult<()> {
        let input = output::input(&task.request.input_path)?;
        let directory = output::directory(&task.request.output_dir)?;
        let info = self.probe_file(&input, &task.id, true)?;
        let temp_dir = output::temporary(&directory, &task.id)?;
        if input.starts_with(&temp_dir) {
            return Err("原始文件不能位于此任务的临时目录中".into());
        }
        let result = (|| {
            if let Some(path) = &task.request.subtitle_path {
                crate::media::subtitle::stage(path, &temp_dir)?;
            }
            let mut temp = temp_dir.join(format!(
                "output-{}.processing.{}",
                new_id(),
                task.request.output_format.extension()
            ));
            if task.request.operation == MediaOperation::ExtractFrames {
                fs::create_dir(&temp).map_err(|_| "无法创建图片输出目录")?;
            }
            if task.request.operation == MediaOperation::ContactSheet {
                fs::write(
                    temp_dir.join("sheet-title.txt"),
                    crate::media::contact_sheet::title(&info),
                )
                .map_err(|_| "无法创建联系表标题")?;
            }
            if let Some(path) = &task.request.cover_path {
                crate::media::metadata::stage_cover(path, &temp_dir)?;
            }
            let mut built = command::build(&task.request, &info, &temp)?;
            let cpu_args = built.args.clone();
            let mode = if task.request.hardware_acceleration
                == crate::media::preset::HardwareAcceleration::Auto
            {
                self.pro_settings().hardware_acceleration
            } else {
                task.request.hardware_acceleration
            };
            let encoding = built.args.iter().any(|a| a == "libx264" || a == "libx265");
            let selected = if encoding && mode != crate::media::preset::HardwareAcceleration::Cpu {
                crate::media::gpu::choose(
                    mode,
                    task.request.video_codec,
                    &self.capabilities(&task.id)?,
                )
            } else {
                None
            };
            if let Some(encoder) = &selected {
                crate::media::gpu::apply(
                    &mut built.args,
                    encoder,
                    task.request
                        .crf
                        .unwrap_or(crate::media::compressor::quality_value(
                            task.request.video_codec,
                            task.request.quality,
                        )),
                );
            }
            {
                let mut s = self.state.lock().unwrap();
                let mut next = s.tasks.clone();
                let t = next
                    .iter_mut()
                    .find(|t| t.id == task.id)
                    .ok_or("媒体任务不存在")?;
                if t.status != MediaTaskStatus::Probing || s.exiting {
                    return Err("媒体任务已取消或中断".into());
                }
                t.status = MediaTaskStatus::Processing;
                t.phase = selected
                    .as_ref()
                    .map(|e| format!("硬件编码 {e}"))
                    .unwrap_or_else(|| {
                        if encoding && mode != crate::media::preset::HardwareAcceleration::Cpu {
                            "GPU 不可用，使用 CPU 编码".into()
                        } else {
                            "正在处理".into()
                        }
                    });
                t.total_duration = built.duration;
                self.commit(&mut s, next)?;
            }
            self.emit();
            let (mut code, mut logs) =
                self.run_ffmpeg(&task.id, &built.args, built.duration, &temp_dir)?;
            if code != 0 && selected.is_some() {
                {
                    let mut s = self.state.lock().unwrap();
                    let Some(t) = s.tasks.iter_mut().find(|t| t.id == task.id) else {
                        return Err("任务不存在".into());
                    };
                    if t.status != MediaTaskStatus::Processing {
                        return Ok(());
                    }
                    t.phase = "硬件编码失败，正在回退 CPU".into();
                    t.progress = None;
                }
                self.emit();
                temp = temp_dir.join(format!(
                    "cpu-{}.{}",
                    new_id(),
                    task.request.output_format.extension()
                ));
                let mut args = cpu_args;
                *args.last_mut().ok_or("编码参数为空")? = temp.to_string_lossy().into_owned();
                let retried = self.run_ffmpeg(&task.id, &args, built.duration, &temp_dir)?;
                code = retried.0;
                logs.push("硬件编码失败，已自动回退 CPU".into());
                logs.extend(retried.1);
            }
            let mut s = self.state.lock().unwrap();
            let mut next = s.tasks.clone();
            let t = next
                .iter_mut()
                .find(|t| t.id == task.id)
                .ok_or("媒体任务不存在")?;
            t.logs = logs;
            if t.status != MediaTaskStatus::Processing || s.exiting {
                return Ok(());
            }
            if code != 0 {
                let detail = t.logs.join("\n");
                return Err(if detail.is_empty() {
                    format!("FFmpeg 退出码 {code}")
                } else {
                    detail
                });
            }
            // Optional preset thumbnails are a separate queue task, retaining the original result.
            // Completion and cancellation cannot cross the publication boundary.
            let destination = output::publish(
                &temp,
                &directory,
                &output::stem(&task.request),
                task.request.output_format,
            )?;
            t.output_path = Some(destination.to_string_lossy().into_owned());
            t.status = MediaTaskStatus::Completed;
            t.phase = "处理完成".into();
            t.progress = Some(100.);
            t.processed_time = built.duration;
            t.speed = None;
            t.eta = Some(0.);
            t.finished_at = Some(now());
            self.commit(&mut s, next)?;
            Ok(())
        })();
        output::cleanup(&directory, &task.id);
        if result.is_ok()
            && task.request.generate_thumbnails
            && self
                .snapshot()
                .tasks
                .iter()
                .any(|t| t.id == task.id && t.status == MediaTaskStatus::Completed)
        {
            let mut request = task.request.clone();
            request.operation = MediaOperation::ExtractFrames;
            request.output_format = OutputFormat::Jpg;
            request.frame_mode = FrameMode::Count;
            request.frame_count = 20;
            request.generate_thumbnails = false;
            request.source_download_id = None;
            request.subtitle_path = None;
            request.cover_path = None;
            self.create(request)?;
        }
        result
    }
    fn run_ffmpeg(
        &self,
        id: &str,
        args: &[String],
        duration: Option<f64>,
        directory: &Path,
    ) -> AppResult<(u32, Vec<String>)> {
        let (process, stdout, stderr) = native::spawn_in(
            &self.resources.join("tools/ffmpeg.exe"),
            args,
            Some(directory),
        )?;
        self.register(id, process.job.clone(), true)?;
        let error_reader = std::thread::spawn(move || native::read_bounded(stderr, 64 * 1024));
        let mut block = String::new();
        for line in BufReader::new(stdout).lines() {
            let line = line.map_err(|_| "媒体进度读取失败")?;
            if block.len() < 4096 {
                block.push_str(&line);
                block.push('\n');
            }
            if line.starts_with("progress=") {
                let p = progress::parse(&block, duration);
                block.clear();
                {
                    let mut s = self.state.lock().unwrap();
                    if let Some(t) = s.tasks.iter_mut().find(|t| t.id == id) {
                        if t.status == MediaTaskStatus::Processing {
                            t.progress = p.percent;
                            t.speed = p.speed;
                            t.processed_time = p.processed_time;
                            t.eta = p.eta;
                        }
                    }
                }
                self.emit();
            }
        }
        let code = process.wait()?;
        let logs = error_reader.join().map_err(|_| "读取媒体日志失败")??;
        let logs = redact(&String::from_utf8_lossy(&logs))
            .lines()
            .take(120)
            .map(|s| s.chars().take(2048).collect::<String>())
            .collect::<Vec<_>>();
        Ok((code, logs))
    }
    pub(crate) fn preview_directory(&self, session: &str) -> AppResult<PathBuf> {
        crate::media::trim::timeline::validate_session(session)?;
        let data = fs::canonicalize(self.state_file.parent().ok_or("数据目录无效")?)
            .map_err(|_| "数据目录不可访问")?;
        let root = data.join("media-preview").join(session);
        fs::create_dir_all(&root).map_err(|_| "无法创建预览缓存")?;
        let actual = fs::canonicalize(&root).map_err(|_| "预览缓存不可访问")?;
        if !actual.starts_with(&data) {
            return Err("预览缓存目录无效".into());
        }
        Ok(actual)
    }
    pub(crate) fn auxiliary(
        &self,
        session: &str,
        args: &[String],
        seconds: u64,
    ) -> AppResult<String> {
        let id = format!("{session}-{}", new_id());
        let (tx, rx) = mpsc::channel();
        let mut registration = Ok(());
        let mut watcher = None;
        let result = native::capture_with(&self.resources.join("tools/ffmpeg.exe"), args, |job| {
            registration = self.register(&id, job.clone(), false);
            watcher = Some(std::thread::spawn(move || {
                if rx.recv_timeout(Duration::from_secs(seconds)).is_err() {
                    let _ = job.terminate();
                }
            }));
        });
        let _ = tx.send(());
        if let Some(w) = watcher {
            let _ = w.join();
        }
        self.state.lock().unwrap().jobs.remove(&id);
        registration?;
        result
    }
    pub fn cancel_editor(self: &Arc<Self>, session: &str) -> AppResult<()> {
        crate::media::trim::timeline::validate_session(session)?;
        let mut s = self.state.lock().unwrap();
        if s.cancelled_previews.len() > 1024 {
            return Err("预览会话过多，请重启程序".into());
        }
        s.cancelled_previews.insert(session.into());
        for (id, job) in &s.jobs {
            if id.starts_with(&format!("{session}-")) {
                job.terminate()?;
            }
        }
        drop(s);
        let service = self.clone();
        let session = session.to_string();
        std::thread::spawn(move || {
            let end = Instant::now() + Duration::from_secs(5);
            loop {
                let busy = service
                    .state
                    .lock()
                    .unwrap()
                    .jobs
                    .keys()
                    .any(|id| id.starts_with(&format!("{session}-")));
                if !busy {
                    break;
                }
                if Instant::now() > end {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            if let Ok(data) = fs::canonicalize(service.state_file.parent().unwrap()) {
                let root = data.join("media-preview").join(&session);
                if let Ok(actual) = fs::canonicalize(&root) {
                    if actual == root && actual.starts_with(&data) {
                        let _ = fs::remove_dir_all(actual);
                    }
                }
            }
        });
        Ok(())
    }
    pub fn open_output(&self, id: &str, folder: bool) -> AppResult<()> {
        let s = self.state.lock().unwrap();
        let t = s
            .tasks
            .iter()
            .find(|t| t.id == id)
            .ok_or("媒体任务不存在")?;
        let directory = fs::canonicalize(&t.request.output_dir).map_err(|_| "保存目录不存在")?;
        let target = if folder {
            directory.clone()
        } else {
            fs::canonicalize(t.output_path.as_ref().ok_or("尚未生成输出文件")?)
                .map_err(|_| "输出文件不存在")?
        };
        if !target.starts_with(&directory) {
            return Err("文件不在此媒体任务的保存目录中".into());
        }
        drop(s);
        crate::service::shell_open(&target.to_string_lossy())
    }
    pub fn shutdown(&self) -> AppResult<()> {
        let jobs = {
            let mut s = self.state.lock().unwrap();
            s.exiting = true;
            let mut next = s.tasks.clone();
            for t in &mut next {
                if matches!(
                    t.status,
                    MediaTaskStatus::Queued
                        | MediaTaskStatus::Probing
                        | MediaTaskStatus::Processing
                ) {
                    t.status = MediaTaskStatus::Interrupted;
                    t.phase = "已中断，下次启动可重新开始".into();
                    t.speed = None;
                    t.eta = None;
                }
            }
            let saved = if s.error.as_ref().is_some_and(|e| e.contains("记录损坏")) {
                Ok(())
            } else {
                self.commit(&mut s, next)
            };
            if let Err(e) = saved {
                s.error = Some(e);
            }
            s.jobs.values().cloned().collect::<Vec<_>>()
        };
        for job in jobs {
            job.terminate().map_err(|_| "无法终止媒体子进程")?;
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let s = self.state.lock().unwrap();
            if s.running.is_none() && s.jobs.is_empty() {
                if let Some(e) = s.error.as_ref().filter(|e| e.contains("无法保存")) {
                    return Err(e.clone());
                }
                return Ok(());
            }
            drop(s);
            if Instant::now() > deadline {
                return Err("媒体进程仍在退出，请稍后重试".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
