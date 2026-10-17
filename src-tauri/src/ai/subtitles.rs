use super::{jobs::Control, models::*, service::AiService, transcription as t};
use crate::{
    domain::AppResult,
    ffmpeg::{command, models::*},
    learning::{cache, db},
};
use std::{fs, io::Read, path::Path, time::Duration};
const MAX_BYTES: usize = 8 * 1024 * 1024;
const PAGE: usize = 50;
fn read(path: &Path) -> AppResult<Vec<u8>> {
    let mut bytes = Vec::new();
    t::lock_source(path)?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取字幕")?;
    if bytes.len() > MAX_BYTES {
        return Err("字幕超过8MiB".into());
    }
    Ok(bytes)
}
fn time(value: &str) -> AppResult<i64> {
    let normalized = value.replace(',', ".");
    let parts = normalized.split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err("字幕时间码无效".into());
    }
    let (sec, ms) = parts
        .last()
        .unwrap()
        .split_once('.')
        .ok_or("字幕毫秒无效")?;
    if ms.len() != 3 || !ms.bytes().all(|b| b.is_ascii_digit()) {
        return Err("字幕毫秒无效".into());
    }
    let number = |s: &str| -> AppResult<i64> {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err("字幕时间码无效".into());
        }
        s.parse().map_err(|_| "字幕时间码无效".into())
    };
    let seconds = number(sec)?;
    let minutes = number(parts[parts.len() - 2])?;
    let hours = if parts.len() == 3 {
        number(parts[0])?
    } else {
        0
    };
    if minutes >= 60 || seconds >= 60 || hours > 168 {
        return Err("字幕时间码超出范围".into());
    }
    Ok(((hours * 60 + minutes) * 60 + seconds) * 1000 + number(ms)?)
}
pub fn parse(bytes: &[u8], id: &str, duration: i64) -> AppResult<Vec<TranscriptSegment>> {
    if bytes.len() > MAX_BYTES || duration <= 0 || duration > 604800000 {
        return Err("字幕大小或媒体时长无效".into());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "字幕不是有效UTF-8，请转换编码")?
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut rows: Vec<TranscriptSegment> = vec![];
    for block in text.split("\n\n").filter(|v| !v.trim().is_empty()) {
        let lines = block.lines().map(str::trim).collect::<Vec<_>>();
        if lines[0].starts_with("WEBVTT")
            || lines[0] == "STYLE"
            || lines[0] == "REGION"
            || lines[0] == "NOTE"
            || lines[0].starts_with("NOTE ")
        {
            continue;
        }
        let n = if lines[0].contains("-->") { 0 } else { 1 };
        let (from, to) = lines
            .get(n)
            .and_then(|v| v.split_once("-->"))
            .ok_or("字幕缺少时间码")?;
        let start = time(from.trim())?;
        let end = time(to.split_whitespace().next().ok_or("字幕结束时间无效")?)?;
        if start >= duration
            || end <= start
            || end > duration.saturating_add(1000)
            || rows.last().is_some_and(|r| r.start_ms > start)
        {
            return Err("字幕时间或顺序超出媒体范围".into());
        }
        let body = lines[n + 1..].join("\n");
        if body.is_empty() {
            continue;
        }
        if body.len() > 16000 || rows.len() >= 200000 {
            return Err("字幕文本过长或段落过多".into());
        }
        let index = rows.len() as u32;
        rows.push(TranscriptSegment {
            id: format!("{id}-segment-{index}"),
            analysis_id: id.into(),
            segment_index: index,
            start_ms: start,
            end_ms: end.min(duration),
            text: body,
        });
    }
    if rows.is_empty() {
        return Err("字幕没有有效时间段".into());
    }
    Ok(rows)
}
fn convert(
    resources: &Path,
    data: &Path,
    input: &Path,
    info: &MediaInfo,
    index: Option<u32>,
    id: &str,
    duration: i64,
    control: &Control,
) -> AppResult<Vec<TranscriptSegment>> {
    let _cache = cache::SUBTITLE_GUARD
        .lock()
        .map_err(|_| "字幕缓存暂不可用")?;
    control.check()?;
    let _lock = t::lock_source(input)?;
    let before = t::fingerprint(input)?;
    let stamp = cache::stamp(input)?;
    let key = cache::key(
        &stamp,
        &format!("ai-subtitle:{index:?}:{before}"),
        "utf8-vtt-v1",
    );
    let dir = cache::directory(data, "subtitles", &key)?;
    let output = dir.join("ai-subtitles.vtt");
    let bytes = if let Ok(path) = cache::owned(&dir, &output) {
        read(&path)?
    } else {
        let request = MediaRequest {
            input_path: input.to_string_lossy().into(),
            output_dir: dir.to_string_lossy().into(),
            operation: MediaOperation::SubtitleConvert,
            output_format: OutputFormat::Vtt,
            ..Default::default()
        };
        let mut args = command::build(&request, info, &output)?.args;
        if let Some(n) = args.iter().position(|s| s == "-progress") {
            args.drain(n..n + 2);
        }
        if let Some(index) = index {
            let n = args
                .iter()
                .position(|s| s == "-map")
                .ok_or("字幕轨道无效")?;
            args[n + 1] = format!("0:{index}");
        }
        *args.last_mut().ok_or("字幕转换参数无效")? = "pipe:1".into();
        let bytes = control.capture(
            &t::verified_tool(resources, "ffmpeg.exe")?,
            &args,
            None,
            Duration::from_secs(30),
        )?;
        parse(&bytes, id, duration)?;
        control.check()?;
        if t::fingerprint(input)? != before {
            return Err("字幕源已改变".into());
        }
        crate::native::save_atomic(&output, &bytes)?;
        bytes
    };
    control.check()?;
    if t::fingerprint(input)? != before {
        return Err("字幕源已改变".into());
    }
    parse(&bytes, id, duration)
}
fn cached_probe(
    resources: &Path,
    data: &Path,
    source: &Path,
    control: &Control,
) -> AppResult<MediaInfo> {
    let _cache = cache::SUBTITLE_GUARD
        .lock()
        .map_err(|_| "字幕缓存暂不可用")?;
    control.check()?;
    let before = t::fingerprint(source)?;
    let stamp = cache::stamp(source)?;
    let key = cache::key(&stamp, &before, "ai-media-probe-v1");
    let dir = cache::directory(data, "subtitles", &key)?;
    let path = dir.join("ai-media-info.json");
    if let Ok(path) = cache::owned(&dir, &path) {
        if let Ok(bytes) = read(&path) {
            if let Ok(info) = serde_json::from_slice::<MediaInfo>(&bytes) {
                return Ok(info);
            }
        }
    }
    let info = t::probe_media(resources, source, control)?;
    control.check()?;
    if t::fingerprint(source)? != before {
        return Err("课程媒体已改变".into());
    }
    crate::native::save_atomic(
        &path,
        &serde_json::to_vec(&info).map_err(|_| "无法保存媒体信息")?,
    )?;
    Ok(info)
}
pub fn existing(
    resources: &Path,
    data: &Path,
    source: &Path,
    info: &MediaInfo,
    id: &str,
    duration: i64,
    control: &Control,
) -> AppResult<(Vec<TranscriptSegment>, Option<String>)> {
    control.check()?;
    let mut warnings = vec![];
    let stem = source
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let mut paths = fs::read_dir(source.parent().ok_or("媒体目录无效")?)
        .into_iter()
        .flatten()
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
                && crate::media::subtitle::extension(&p.to_string_lossy()).is_ok()
        })
        .collect::<Vec<_>>();
    for ext in ["srt", "vtt", "ass", "ssa"] {
        let path = source.with_extension(ext);
        if path.exists() {
            paths.push(path)
        }
    }
    // Prefer exact basename, then stable lexical language suffix; never combine different languages.
    // ponytail: scan 10,000 siblings and try 16 tracks; add an explicit selector for larger collections.
    paths.sort_by_key(|p| {
        (
            p.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                != stem,
            p.to_string_lossy().to_lowercase(),
        )
    });
    paths.dedup_by(|a, b| {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    });
    for path in paths.into_iter().take(16) {
        control.check()?;
        let result = (|| {
            let path = crate::learning::importer::regular_file(&path.to_string_lossy())?;
            let bytes = read(&path)?;
            if ["srt", "vtt"].contains(
                &path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
                    .as_str(),
            ) {
                parse(&bytes, id, duration)
            } else {
                std::str::from_utf8(&bytes).map_err(|_| "字幕不是有效UTF-8")?;
                let info = cached_probe(resources, data, &path, control)?;
                convert(resources, data, &path, &info, None, id, duration, control)
            }
        })();
        match result {
            Ok(rows) => return Ok((rows, (!warnings.is_empty()).then(|| warnings.join("；")))),
            Err(e) => {
                control.check()?;
                warnings.push(format!("外挂字幕不可用：{e}"));
            }
        }
    }
    let mut tracks = info
        .subtitles
        .iter()
        .filter(|s| ["subrip", "ass", "ssa", "webvtt", "mov_text"].contains(&s.codec.as_str()))
        .collect::<Vec<_>>();
    tracks.sort_by_key(|s| s.index);
    for track in tracks.into_iter().take(16) {
        let mut selected = info.clone();
        selected.subtitles = vec![track.clone()];
        match convert(
            resources,
            data,
            source,
            &selected,
            Some(track.index),
            id,
            duration,
            control,
        ) {
            Ok(rows) => return Ok((rows, (!warnings.is_empty()).then(|| warnings.join("；")))),
            Err(e) => {
                control.check()?;
                warnings.push(format!("内嵌字幕不可用：{e}"));
            }
        }
    }
    if !info.subtitles.is_empty() && warnings.is_empty() {
        warnings.push("内嵌字幕为位图或不支持的格式".into())
    }
    Ok((
        vec![],
        Some(if warnings.is_empty() {
            "未找到已有文本字幕，可使用本地语音转录".into()
        } else {
            format!("{}；可回退到本地语音转录", warnings.join("；"))
        }),
    ))
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseTranscriptPage {
    pub items: Vec<TranscriptSegment>,
    pub total: u64,
    pub page: u32,
    pub source: String,
    pub analysis_id: Option<String>,
    pub warning: Option<String>,
    pub next_start_ms: Option<i64>,
}
fn paged(
    mut rows: Vec<TranscriptSegment>,
    page: u32,
    current: Option<i64>,
    source: &str,
    analysis_id: Option<String>,
    warning: Option<String>,
) -> CourseTranscriptPage {
    let total = rows.len();
    let page = current
        .map(|ms| rows.iter().rposition(|r| r.start_ms <= ms).unwrap_or(0) / PAGE + 1)
        .unwrap_or(page as usize)
        .max(1)
        .min(total.div_ceil(PAGE).max(1));
    let next_start_ms = rows.get(page * PAGE).map(|r| r.start_ms);
    let items = rows
        .drain(((page - 1) * PAGE).min(total)..(page * PAGE).min(total))
        .collect();
    CourseTranscriptPage {
        items,
        total: total as u64,
        page: page as u32,
        source: source.into(),
        analysis_id,
        warning,
        next_start_ms,
    }
}
impl AiService {
    pub fn course_transcript(
        &self,
        item: &str,
        page: u32,
        current: Option<i64>,
    ) -> AppResult<CourseTranscriptPage> {
        use rusqlite::OptionalExtension;
        if item.is_empty()
            || item.len() > 128
            || current.is_some_and(|v| !(0..=604800000).contains(&v))
        {
            return Err("课程或字幕定位时间无效".into());
        }
        let path = self
            .with(|r| {
                r.conn
                    .query_row(
                        "SELECT file_path FROM series_items WHERE id=?1",
                        [item],
                        |row| row.get::<_, Option<String>>(0),
                    )
                    .optional()
                    .map_err(db::error)
            })?
            .flatten();
        let Some(path) = path else {
            return Ok(paged(
                vec![],
                1,
                None,
                "none",
                None,
                Some("课程没有本地媒体".into()),
            ));
        };
        let source = crate::learning::importer::regular_file(&path)?;
        let _lock = t::lock_source(&source)?;
        let fingerprint = t::fingerprint(&source)?;
        let control = Control::default();
        let info = cached_probe(&self.resources, &self.data, &source, &control)?;
        let duration = info
            .duration
            .filter(|d| d.is_finite() && *d > 0. && *d <= 604800.)
            .ok_or("无法确定课程媒体时长")?;
        let duration = (duration * 1000.).round() as i64;
        let (rows, warning) = existing(
            &self.resources,
            &self.data,
            &source,
            &info,
            item,
            duration,
            &control,
        )?;
        if !rows.is_empty() {
            return Ok(paged(rows, page, current, "subtitle", None, warning));
        }
        // A completed transcript can belong to an older analysis than the latest queued job.
        let analysis=self.with(|r|{let id:Option<String>=r.conn.query_row("SELECT id FROM video_analyses WHERE item_id=?1 AND source_fingerprint=?2 AND transcript_done=1 AND EXISTS(SELECT 1 FROM transcript_segments WHERE analysis_id=video_analyses.id) ORDER BY created_at DESC,id DESC LIMIT 1",rusqlite::params![item,fingerprint],|row|row.get(0)).optional().map_err(db::error)?;id.map(|id|r.all_transcript(&id).map(|rows|(id,rows))).transpose()})?;
        if let Some((id, rows)) = analysis {
            let marker = self
                .data
                .join("ai/cache")
                .join(&id)
                .join("transcript-source.txt");
            let kind = if read(&marker).ok().as_deref() == Some(b"subtitle".as_slice()) {
                "subtitle"
            } else {
                "whisper"
            };
            return Ok(paged(rows, page, current, kind, Some(id), warning));
        }
        Ok(paged(vec![], 1, None, "none", None, warning))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_subtitle_probe_waits_for_cache_cleaner_guard() {
        let dir = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
        fs::create_dir(&dir).unwrap();
        let source = dir.join("course.mkv");
        fs::write(&source, b"source").unwrap();
        let key = cache::key(
            &cache::stamp(&source).unwrap(),
            &t::fingerprint(&source).unwrap(),
            "ai-media-probe-v1",
        );
        let cached = cache::directory(&dir, "subtitles", &key).unwrap();
        fs::write(
            cached.join("ai-media-info.json"),
            serde_json::to_vec(&MediaInfo::default()).unwrap(),
        )
        .unwrap();
        let cleaner = cache::SUBTITLE_GUARD.lock().unwrap();
        let (started, ready) = std::sync::mpsc::channel();
        let (sent, received) = std::sync::mpsc::channel();
        let data = dir.clone();
        let worker = std::thread::spawn(move || {
            started.send(()).unwrap();
            sent.send(cached_probe(&data, &data, &source, &Control::default()))
                .unwrap();
        });
        ready.recv().unwrap();
        let early = received.recv_timeout(Duration::from_millis(100));
        drop(cleaner);
        let blocked = early.is_err();
        let result =
            early.unwrap_or_else(|_| received.recv_timeout(Duration::from_secs(3)).unwrap());
        worker.join().unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(result.is_ok());
        assert!(blocked, "AI cache read bypassed the cleaner guard");
    }
    const SRT:&str="1\r\n00:00:00,500 --> 00:00:01,500\r\n课程第一行\r\nsecond line\r\n\r\n2\r\n00:00:02,000 --> 00:00:03,000\r\n结束\r\n";
    #[test]
    fn subtitle_cues_preserve_millisecond_times_and_multiline_utf8() {
        let rows = parse(SRT.as_bytes(), "a", 4000).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].start_ms, rows[0].end_ms), (500, 1500));
        assert_eq!(rows[0].text, "课程第一行\nsecond line");
        let vtt = b"WEBVTT\n\ncue-1\n00:00.200 --> 00:01.800 align:start\nHello\n\n";
        let rows = parse(vtt, "b", 2000).unwrap();
        assert_eq!(rows[0].start_ms, 200);
    }
    #[test]
    fn subtitle_cues_reject_invalid_encoding_timestamps_and_empty_files() {
        for bytes in [
            b"\xff".as_slice(),
            b"WEBVTT\n\n",
            b"1\n00:00:03,000 --> 00:00:02,000\nbad\n",
            b"1\n00:00:00,000 --> 00:00:99,000\nbad\n",
            b"1\n00:00:10,000 --> 00:00:11,000\nbad\n",
        ] {
            assert!(parse(bytes, "a", 4000).is_err());
        }
    }
    #[test]
    fn local_subtitle_priority_and_invalid_sibling_fallback_are_read_only() {
        let dir = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
        std::fs::create_dir(&dir).unwrap();
        let source = dir.join("课程.mp4");
        std::fs::write(&source, b"unchanged").unwrap();
        std::fs::write(source.with_extension("srt"), b"malformed").unwrap();
        std::fs::write(dir.join("课程.zh.srt"), SRT).unwrap();
        let (rows, warning) = existing(
            &dir,
            &dir,
            &source,
            &MediaInfo::default(),
            "a",
            4000,
            &Control::default(),
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert!(warning.is_some());
        assert_eq!(std::fs::read(&source).unwrap(), b"unchanged");
        let c = Control::default();
        c.cancel();
        assert!(existing(&dir, &dir, &source, &MediaInfo::default(), "a", 4000, &c).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn subtitle_paging_locates_seek_on_later_pages() {
        let rows = (0..125)
            .map(|i| TranscriptSegment {
                id: format!("s{i}"),
                analysis_id: "a".into(),
                segment_index: i,
                start_ms: i as i64 * 1000,
                end_ms: i as i64 * 1000 + 900,
                text: "cue".into(),
            })
            .collect::<Vec<_>>();
        let page = paged(rows.clone(), 1, Some(110000), "subtitle", None, None);
        assert_eq!((page.page, page.total, page.items.len()), (3, 125, 25));
        assert_eq!(page.items[0].start_ms, 100000);
        let first = paged(rows.clone(), 1, None, "subtitle", None, None);
        assert_eq!(first.next_start_ms, Some(50000));
        assert_eq!(page.next_start_ms, None);
        let page = paged(rows, 99, None, "subtitle", None, None);
        assert_eq!(page.page, 3);
        assert_eq!(page.items.last().unwrap().end_ms, 124900);
    }
    #[test]
    #[ignore = "Requires verified FFmpeg resources in AI_QA_RESOURCES"]
    fn real_embedded_subtitles_pipeline_without_whisper_and_persistent_probe_cache() {
        use std::sync::Arc;
        let resources = std::path::PathBuf::from(std::env::var("AI_QA_RESOURCES").unwrap());
        let control = Control::default();
        let exe = t::verified_tool(&resources, "ffmpeg.exe").unwrap();
        let dir =
            std::env::temp_dir().join(format!("字幕测试 # & {}", crate::ffmpeg::runner::new_id()));
        fs::create_dir(&dir).unwrap();
        let srt = dir.join("source.srt");
        fs::write(&srt, SRT).unwrap();
        let source = dir.join("课程.mkv");
        let args = [
            "-hide_banner",
            "-nostdin",
            "-loglevel",
            "error",
            "-n",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=4",
            "-i",
            srt.to_str().unwrap(),
            "-map",
            "0:a",
            "-map",
            "1:s",
            "-c:a",
            "pcm_s16le",
            "-c:s",
            "srt",
            source.to_str().unwrap(),
        ]
        .map(str::to_owned);
        control
            .capture(&exe, &args, None, Duration::from_secs(15))
            .unwrap();
        let before = t::file_hash(&source).unwrap();
        let info = t::probe_media(&resources, &source, &control).unwrap();
        let (rows, warning) =
            existing(&resources, &dir, &source, &info, "embedded", 4000, &control).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(warning.is_none());
        assert_eq!(rows[0].text, "课程第一行\nsecond line");
        let (rows, _) = existing(
            &dir.join("missing-tools"),
            &dir,
            &source,
            &info,
            "cached",
            4000,
            &control,
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        let service = AiService::create(None, &dir, &resources);
        let c = Arc::new(Control::default());
        service.runtime.lock().unwrap().active = Some(("a".into(), c.clone()));
        service.with(|r|{r.conn.execute("INSERT INTO video_analyses(id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,options_json,created_at,updated_at) VALUES('a',?1,?2,'queued','standard','deepseek-flash','test',?3,1,1)",rusqlite::params![source.to_string_lossy(),t::fingerprint(&source)?,serde_json::to_string(&AnalysisOptions::default()).unwrap()]).unwrap();Ok(())}).unwrap();
        service.run_analysis("a", &c, None).unwrap();
        let a = service.analysis("a").unwrap();
        assert!(a.transcript_done);
        assert_eq!(a.status, "completed");
        assert_eq!(service.with(|r| r.all_transcript("a")).unwrap().len(), 2);
        assert!(!dir.join("ai/cache/a/audio.wav").exists());
        assert!(!dir.join("ai/models").exists());
        // Resolve a real course, then make the tools unavailable: seeks must reuse media metadata and extraction.
        service.with(|r|{r.conn.execute("INSERT INTO series(id,title,source_type,created_at,updated_at) VALUES('series','课程','local',1,1)",[]).unwrap();r.conn.execute("INSERT INTO series_items(id,series_id,title,sort_order,file_path,created_at,updated_at) VALUES('lesson','series','课',0,?1,1,1)",[source.to_string_lossy().as_ref()]).unwrap();Ok(())}).unwrap();
        let first = service.course_transcript("lesson", 1, Some(2000)).unwrap();
        assert_eq!(first.source, "subtitle");
        assert_eq!(first.total, 2);
        drop(service);
        let service = AiService::create(None, &dir, &dir.join("missing-tools"));
        let cached = service.course_transcript("lesson", 1, Some(500)).unwrap();
        assert_eq!(cached.total, 2);
        assert_eq!(cached.source, "subtitle");
        assert_eq!(t::file_hash(&source).unwrap(), before);
        drop(service);
        fs::remove_dir_all(dir).unwrap();
    }
}
