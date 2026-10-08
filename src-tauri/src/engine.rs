use crate::{
    domain::{validate_url, AppResult},
    models::*,
};
use serde_json::Value;

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_string()
}
fn thumbnail(value: &Value) -> Option<String> {
    value["thumbnail"]
        .as_str()
        .or_else(|| value["thumbnails"].as_array()?.last()?["url"].as_str())
        .and_then(|url| validate_url(url).ok())
}
pub fn media_preview(url: &str, json: &Value) -> AppResult<MediaPreview> {
    if !json.is_object() {
        return Err("内核没有返回有效的视频信息".into());
    }
    let is_playlist = json["entries"].is_array();
    let entries = json["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            if !entry.is_object() {
                return None;
            }
            let mut item_url = text(entry, "webpage_url");
            if item_url.is_empty() {
                item_url = text(entry, "url");
            }
            if !item_url.starts_with("http")
                && text(entry, "ie_key")
                    .to_ascii_lowercase()
                    .starts_with("youtube")
            {
                item_url = format!("https://www.youtube.com/watch?v={}", text(entry, "id"));
            }
            let item_url = validate_url(&item_url).ok()?;
            Some(PlaylistEntry {
                id: text(entry, "id"),
                url: item_url,
                title: entry["title"].as_str().unwrap_or("未命名视频").into(),
                duration: entry["duration"].as_f64(),
                thumbnail: thumbnail(entry),
            })
        })
        .collect();
    let formats: Vec<_> = json["formats"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|format| format["format_id"].is_string())
        .map(|format| MediaFormat {
            id: text(format, "format_id"),
            extension: text(format, "ext"),
            height: format["height"].as_u64().map(|n| n as u32),
            video_codec: text(format, "vcodec"),
            audio_codec: text(format, "acodec"),
            filesize: format["filesize"]
                .as_u64()
                .or_else(|| format["filesize_approx"].as_u64()),
        })
        .collect();
    let selected = json["requested_formats"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![json.clone()]);
    let codec = |key: &str| {
        selected
            .iter()
            .filter_map(|format| format[key].as_str())
            .find(|codec| !codec.is_empty() && *codec != "none")
            .map(str::to_string)
    };
    let video_codec = codec("vcodec");
    let audio_codec = codec("acodec");
    let compatible = video_codec
        .as_ref()
        .is_some_and(|codec| codec.starts_with("avc") || codec.contains("h264"))
        && audio_codec
            .as_ref()
            .is_some_and(|codec| codec.starts_with("mp4a") || codec.contains("aac"));
    let sizes: Option<Vec<u64>> = selected
        .iter()
        .map(|format| {
            format["filesize"]
                .as_u64()
                .or_else(|| format["filesize_approx"].as_u64())
        })
        .collect();
    let filesize = sizes.map(|sizes| sizes.into_iter().sum());
    let mut subtitles = Vec::new();
    for (key, automatic) in [("subtitles", false), ("automatic_captions", true)] {
        if let Some(languages) = json[key].as_object() {
            for language in languages.keys() {
                if valid_language(language)
                    && !subtitles
                        .iter()
                        .any(|track: &SubtitleTrack| track.language == *language)
                {
                    subtitles.push(SubtitleTrack {
                        language: language.clone(),
                        automatic,
                    });
                }
            }
        }
    }
    Ok(MediaPreview {
        url: validate_url(url)?,
        title: json["title"].as_str().unwrap_or("未命名视频").into(),
        thumbnail: thumbnail(json),
        duration: json["duration"].as_f64(),
        uploader: json["uploader"]
            .as_str()
            .or_else(|| json["channel"].as_str())
            .unwrap_or_default()
            .into(),
        site: json["extractor_key"]
            .as_str()
            .or_else(|| json["extractor"].as_str())
            .unwrap_or_default()
            .into(),
        is_playlist,
        entries,
        formats,
        subtitles,
        video_codec,
        audio_codec,
        compatible,
        filesize,
    })
}
fn valid_language(language: &str) -> bool {
    !language.is_empty()
        && language.len() <= 80
        && language
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
}
pub fn base_args() -> Vec<String> {
    [
        "--ignore-config",
        "--no-colors",
        "--encoding",
        "utf-8",
        "--socket-timeout",
        "20",
        "--retries",
        "3",
        "--fragment-retries",
        "3",
        "--extractor-retries",
        "2",
    ]
    .map(str::to_string)
    .to_vec()
}
pub fn video_args(mode: &VideoMode, height: u32) -> AppResult<Vec<String>> {
    if height > 8640 {
        return Err("画质上限超出支持范围".into());
    }
    let mut args = if *mode == VideoMode::Compatible {
        vec!["-t".into(), "mp4".into()]
    } else {
        vec!["-S".into(), "res,fps".into()]
    };
    if height > 0 {
        args.extend([
            "-f".into(),
            format!("bv*[height<=?{height}]+ba/b[height<=?{height}]"),
        ]);
    }
    Ok(args)
}
pub fn download_args(request: &DownloadRequest, output: &str) -> AppResult<Vec<String>> {
    let url = validate_url(&request.url)?;
    if request
        .subtitle_languages
        .iter()
        .any(|language| !valid_language(language))
    {
        return Err("字幕语言无效".into());
    }
    if request.kind == MediaKind::Subtitles && request.subtitle_languages.is_empty() {
        return Err("请至少选择一种字幕语言".into());
    }
    let mut args = base_args();
    args.extend(
        [
            "--no-playlist",
            "--no-overwrites",
            "--windows-filenames",
            "--trim-filenames",
            "180",
            "--continue",
            "--newline",
            "--progress",
            "--progress-delta",
            "0.3",
            "--no-simulate",
            "-P",
            output,
            "-o",
            "%(title).120B [%(id)s].%(ext)s",
            "--progress-template",
            "download:VD_PROGRESS%(progress)j",
            "--progress-template",
            "postprocess:VD_PROCESS%(progress)j",
            "--print",
            "after_move:VD_FILE%(filepath)j",
            "--print",
            "after_video:VD_SUBS%(requested_subtitles.:.filepath)j",
        ]
        .map(str::to_string),
    );
    match request.kind {
        MediaKind::Video => args.extend(video_args(&request.video_mode, request.max_height)?),
        MediaKind::Audio => {
            args.extend(["-f", "ba/b", "-x", "--audio-format", "mp3"].map(str::to_string))
        }
        MediaKind::Subtitles => args.push("--skip-download".into()),
    }
    if !request.subtitle_languages.is_empty() {
        args.extend([
            "--write-subs".into(),
            "--write-auto-subs".into(),
            "--sub-langs".into(),
            request.subtitle_languages.join(","),
            "--convert-subs".into(),
            "srt".into(),
        ]);
    }
    args.extend(["--".into(), url]);
    Ok(args)
}
pub fn task_args(task: &DownloadTask) -> AppResult<Vec<String>> {
    if task.id.is_empty()
        || task.id.len() > 128
        || !task
            .id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err("任务标识无效".into());
    }
    let mut args = download_args(&task.request, &task.output_dir)?;
    let temporary = std::path::Path::new(&task.output_dir)
        .join(".video-downloader")
        .join(&task.id);
    // Isolate all conversion outputs; the final move enforces --no-overwrites, including existing SRT files.
    args.splice(0..0, ["-P".into(), format!("temp:{}", temporary.display())]);
    Ok(args)
}
pub fn checksum_entry(sums: &str, filename: &str) -> AppResult<String> {
    for line in sums.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() == 2
            && fields[1].trim_start_matches('*') == filename
            && fields[0].len() == 64
            && fields[0].chars().all(|ch| ch.is_ascii_hexdigit())
        {
            return Ok(fields[0].to_ascii_lowercase());
        }
    }
    Err("官方 SHA256 清单缺少有效的 yt-dlp.exe 校验项，已拒绝更新".into())
}
pub fn recover_tasks(tasks: &mut [DownloadTask]) {
    for task in tasks {
        if task.status.active() || task.status == TaskStatus::Queued {
            task.status = TaskStatus::Paused;
            task.phase = "上次未完成，点击继续".into();
        }
    }
}
pub fn final_output_path(task: &DownloadTask, raw: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(raw);
    let output = std::path::Path::new(&task.output_dir);
    let temporary = output.join(".video-downloader").join(&task.id);
    // yt-dlp moves subtitle files but leaves requested_subtitles.filepath unchanged.
    if path.parent() == Some(temporary.as_path()) {
        if let Some(name) = path.file_name() {
            return output.join(name);
        }
    }
    path.to_path_buf()
}
pub fn queued_ids(tasks: &[DownloadTask], limit: usize, running: &[String]) -> Vec<String> {
    tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Queued && !running.contains(&task.id))
        .take(limit.saturating_sub(running.len()))
        .map(|task| task.id.clone())
        .collect()
}
pub fn transition(task: &mut DownloadTask, action: &str) -> AppResult<()> {
    match action {
        "pause"
            if matches!(
                task.status,
                TaskStatus::Queued | TaskStatus::Resolving | TaskStatus::Downloading
            ) =>
        {
            task.status = TaskStatus::Paused;
            task.phase = "已暂停，保留下载进度".into();
        }
        "resume" if task.status == TaskStatus::Paused => {
            task.status = TaskStatus::Queued;
            task.phase = "等待下载".into();
            task.error = None;
        }
        "retry"
            if matches!(
                task.status,
                TaskStatus::Failed | TaskStatus::Cancelled | TaskStatus::Paused
            ) =>
        {
            task.status = TaskStatus::Queued;
            task.phase = "等待重试".into();
            task.error = None;
            task.progress = Default::default();
            task.finished_at = None;
        }
        "cancel"
            if task.status != TaskStatus::Completed && task.status != TaskStatus::Cancelled =>
        {
            task.status = TaskStatus::Cancelled;
            task.phase = "已取消，保留已有文件".into();
        }
        _ => return Err("当前任务状态不支持此操作；合并/转换期间可以取消或等待完成".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preview_distinguishes_codec_compatibility_and_playlist_urls() {
        let video = media_preview("https://youtu.be/abc", &json!({"id":"abc", "title":"测试", "requested_formats":[{"vcodec":"avc1.640028", "acodec":"none", "filesize":100}, {"vcodec":"none", "acodec":"mp4a.40.2", "filesize":25}], "formats":[], "subtitles":{"zh-Hans":[{}]}, "automatic_captions":{"en":[{}]}})).unwrap();
        assert!(video.compatible);
        assert_eq!(video.filesize, Some(125));
        assert_eq!(video.subtitles.len(), 2);
        let incompatible = media_preview(
            "https://youtu.be/a",
            &json!({"title":"AV1", "vcodec":"av01", "acodec":"opus"}),
        )
        .unwrap();
        assert!(!incompatible.compatible);
        let list = media_preview("https://www.youtube.com/playlist?list=a", &json!({"_type":"playlist", "title":"清单", "entries":[{"id":"abc", "url":"abc", "ie_key":"Youtube", "title":"条目"},null]})).unwrap();
        assert_eq!(list.entries[0].url, "https://www.youtube.com/watch?v=abc");
        assert!(list.is_playlist);
    }

    #[test]
    fn download_preserves_url_as_literal_and_protects_existing_files() {
        let request = DownloadRequest {
            url: "https://example.com/watch?a=1&b=2".into(),
            ..Default::default()
        };
        let args = download_args(&request, "C:/测试 下载").unwrap();
        assert!(args.contains(&"--no-overwrites".into()));
        assert!(args.contains(&"--ignore-config".into()));
        assert!(args.contains(&"--windows-filenames".into()));
        assert_eq!(args[args.len() - 2], "--");
        assert_eq!(args.last().unwrap(), "https://example.com/watch?a=1&b=2");
        let subtitles = DownloadRequest {
            kind: MediaKind::Subtitles,
            ..request
        };
        assert!(download_args(&subtitles, "C:/测试").is_err());
    }

    #[test]
    fn simultaneous_video_and_audio_have_separate_resumable_temporary_paths() {
        let video = DownloadTask {
            id: "task-video".into(),
            request: DownloadRequest {
                url: "https://example.com/movie.mp4".into(),
                ..Default::default()
            },
            output_dir: "C:/downloads".into(),
            ..Default::default()
        };
        let audio = DownloadTask {
            id: "task-audio".into(),
            request: DownloadRequest {
                kind: MediaKind::Audio,
                ..video.request.clone()
            },
            ..video.clone()
        };
        let temp = |args: Vec<String>| {
            args.into_iter()
                .find(|arg| arg.starts_with("temp:"))
                .unwrap()
        };
        let video_path = temp(task_args(&video).unwrap());
        let audio_path = temp(task_args(&audio).unwrap());
        assert_ne!(video_path, audio_path);
        assert_eq!(video_path, temp(task_args(&video).unwrap()));
        assert!(video_path.contains("task-video"));
        let subtitles = DownloadTask {
            request: DownloadRequest {
                kind: MediaKind::Subtitles,
                subtitle_languages: vec!["en".into()],
                ..video.request.clone()
            },
            ..video.clone()
        };
        assert_eq!(temp(task_args(&subtitles).unwrap()), video_path);
        let invalid = DownloadTask {
            id: "../../outside".into(),
            ..video
        };
        assert!(task_args(&invalid).is_err());
    }

    #[test]
    fn checksums_require_exact_filename_and_valid_digest() {
        let hash = "a".repeat(64);
        assert_eq!(
            checksum_entry(
                &format!("{hash}  yt-dlp.exe\n{}  other.exe", "b".repeat(64)),
                "yt-dlp.exe"
            )
            .unwrap(),
            hash
        );
        assert!(checksum_entry("bad  yt-dlp.exe", "yt-dlp.exe").is_err());
        assert!(checksum_entry(&format!("{hash}  yt-dlp.exe.sig"), "yt-dlp.exe").is_err());
    }

    #[test]
    fn scheduler_respects_capacity_and_restart_requires_manual_resume() {
        let mut tasks: Vec<_> = (0..4)
            .map(|i| DownloadTask {
                id: i.to_string(),
                ..Default::default()
            })
            .collect();
        tasks[0].status = TaskStatus::Downloading;
        assert_eq!(queued_ids(&tasks, 2, &["0".into()]), vec!["1"]);
        tasks[0].status = TaskStatus::Queued;
        assert_eq!(queued_ids(&tasks, 2, &["0".into()]), vec!["1"]);
        recover_tasks(&mut tasks);
        assert!(tasks.iter().all(|task| task.status == TaskStatus::Paused));
        transition(&mut tasks[1], "resume").unwrap();
        assert_eq!(tasks[1].status, TaskStatus::Queued);
        tasks[1].status = TaskStatus::Processing;
        assert!(transition(&mut tasks[1], "pause").is_err());
        transition(&mut tasks[1], "cancel").unwrap();
        assert_eq!(tasks[1].status, TaskStatus::Cancelled);
    }

    #[test]
    fn moved_subtitles_use_the_final_directory() {
        let task = DownloadTask {
            id: "one".into(),
            output_dir: "C:/downloads".into(),
            ..Default::default()
        };
        assert_eq!(
            final_output_path(&task, "C:/downloads/.video-downloader/one/movie.en.srt"),
            std::path::PathBuf::from("C:/downloads/movie.en.srt")
        );
        assert_eq!(
            final_output_path(&task, "C:/downloads/movie.mp4"),
            std::path::PathBuf::from("C:/downloads/movie.mp4")
        );
    }
}
