use crate::{domain::AppResult, models::*};

#[derive(Debug, PartialEq)]
pub enum TemplatePart {
    Literal(String),
    Title,
    Author,
    Date,
    PlaylistIndex,
    VideoId,
}

pub fn parse_template(raw: &str) -> AppResult<Vec<TemplatePart>> {
    use TemplatePart::*;
    if raw.trim().is_empty() {
        return Ok(vec![
            Title,
            Literal(" [".into()),
            VideoId,
            Literal("]".into()),
        ]);
    }
    if raw.chars().count() > 4096 {
        return Err("文件名模板过长，请控制在 4096 个字符以内".into());
    }
    if raw.starts_with(['/', '\\']) || raw.as_bytes().get(1) == Some(&b':') || raw.contains("..") {
        return Err("文件名模板不允许绝对路径或 ..；子目录请使用“按作者分目录”".into());
    }
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut chars = raw.chars();
    let flush = |parts: &mut Vec<TemplatePart>, literal: &mut String| {
        if !literal.is_empty() {
            parts.push(Literal(std::mem::take(literal).chars().take(120).collect()));
        }
    };
    while let Some(ch) = chars.next() {
        if ch == '{' {
            flush(&mut parts, &mut literal);
            let mut variable = String::new();
            let mut closed = false;
            for ch in chars.by_ref() {
                if ch == '}' {
                    closed = true;
                    break;
                }
                variable.push(ch);
            }
            if !closed {
                return Err("文件名变量缺少右花括号".into());
            }
            parts.push(match variable.as_str() {
                "标题" => Title,
                "作者" => Author,
                "日期" => Date,
                "播放列表序号" => PlaylistIndex,
                "视频ID" => VideoId,
                _ => {
                    return Err(
                        "未知文件名变量；可使用：标题、作者、日期、播放列表序号、视频ID".into(),
                    )
                }
            });
        } else if ch == '}' {
            return Err("文件名模板有多余的右花括号".into());
        } else {
            literal.push(if ch.is_control() || "<>:\"/\\|?*".contains(ch) {
                '_'
            } else {
                ch
            });
        }
    }
    flush(&mut parts, &mut literal);
    Ok(parts)
}
pub fn output_template(request: &DownloadRequest) -> AppResult<String> {
    use TemplatePart::*;
    let parts = parse_template(&request.filename_template)?;
    let legacy = request.filename_template.trim().is_empty();
    let mut output = if request.by_author {
        "作者_%(uploader,channel|未知作者).60B/".to_string()
    } else {
        String::new()
    };
    for part in parts {
        output.push_str(&match part {
            Literal(text) => text.replace('%', "%%"),
            Title => "%(title).120B".into(),
            Author => "%(uploader,channel|未知作者).60B".into(),
            Date => "%(upload_date|日期未知)s".into(),
            PlaylistIndex => request.playlist_index.map_or_else(
                || "%(playlist_index)03d".into(),
                |index| format!("{index:03}"),
            ),
            VideoId => {
                if legacy {
                    "%(id)s".into()
                } else {
                    "%(id).60B".into()
                }
            }
        });
    }
    output.push_str(".%(ext)s");
    Ok(output)
}
pub fn validate_presets(presets: &[DownloadPreset]) -> AppResult<()> {
    if presets.len() > 20 {
        return Err("最多保存 20 个下载预设".into());
    }
    let mut names = std::collections::HashSet::new();
    for preset in presets {
        if preset.name.trim().is_empty()
            || preset.name.chars().count() > 40
            || preset.name.chars().any(char::is_control)
        {
            return Err("预设名称须为 1–40 个字符，不能包含控制字符".into());
        }
        if !names.insert(preset.name.trim().to_lowercase()) {
            return Err("预设名称不能重复".into());
        }
        crate::engine::video_args(&preset.video_mode, preset.max_height)?;
        crate::engine::download_args(
            &DownloadRequest {
                url: "https://example.com/preset-validation".into(),
                kind: preset.kind.clone(),
                video_mode: preset.video_mode.clone(),
                max_height: preset.max_height,
                subtitle_languages: preset.subtitle_languages.clone(),
                filename_template: preset.filename_template.clone(),
                by_author: preset.by_author,
                ..Default::default()
            },
            "C:/downloads",
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filename_templates_accept_only_controlled_variables_and_filter_literals() {
        for (input, ok) in [
            ("{标题} [{视频ID}]", true),
            ("{作者}_{日期}_{播放列表序号}_{标题}", true),
            ("目录/{标题}:?*", true),
            ("", true),
            ("{未知变量}", false),
            ("../{标题}", false),
            ("C:\\{标题}", false),
            ("/{标题}", false),
            ("{标题", false),
            ("标题}", false),
        ] {
            assert_eq!(parse_template(input).is_ok(), ok, "{input}");
        }
        let request = DownloadRequest {
            filename_template: "目录/{标题}:?*".into(),
            ..Default::default()
        };
        let output = output_template(&request).unwrap();
        assert!(
            !output.contains('/')
                && !output.contains(':')
                && !output.contains('?')
                && !output.contains('*')
        );
        assert_eq!(
            output_template(&DownloadRequest::default()).unwrap(),
            "%(title).120B [%(id)s].%(ext)s"
        );
        assert!(parse_template(&"长".repeat(1000)).is_ok());
    }
    #[test]
    fn arguments_compile_a_structure_escape_percent_and_preserve_file_protection() {
        let request = DownloadRequest {
            url: "https://example.com/movie".into(),
            filename_template: "%(danger)s_{标题}_{播放列表序号}".into(),
            by_author: true,
            playlist_index: Some(7),
            ..Default::default()
        };
        let args = crate::engine::download_args(&request, "C:/downloads").unwrap();
        let template = &args[args.iter().position(|arg| arg == "-o").unwrap() + 1];
        assert!(template.starts_with("作者_%(uploader,channel|未知作者).60B/"));
        assert!(template.contains("%%(danger)s_") && template.contains("_007"));
        assert!(!args.contains(&request.filename_template));
        assert_eq!(
            args.iter()
                .filter(|arg| *arg == "--windows-filenames")
                .count(),
            1
        );
        assert!(
            args.contains(&"--no-overwrites".into()) && args.contains(&"--trim-filenames".into())
        );
        assert!(crate::engine::download_args(
            &DownloadRequest {
                filename_template: "{exec}".into(),
                ..request
            },
            "C:/downloads"
        )
        .is_err());
    }
    #[test]
    fn author_directory_prefix_prevents_metadata_dot_and_reserved_paths() {
        let request = DownloadRequest {
            by_author: true,
            ..Default::default()
        };
        assert!(output_template(&request)
            .unwrap()
            .starts_with("作者_%(uploader,channel|未知作者).60B/"));
    }
    #[test]
    fn presets_limit_validate_existing_options_and_freeze_into_queued_requests() {
        let preset = DownloadPreset {
            name: "音频与字幕".into(),
            kind: MediaKind::Audio,
            subtitle_languages: vec!["en".into()],
            filename_template: "{标题}_{视频ID}".into(),
            by_author: true,
            ..Default::default()
        };
        assert!(validate_presets(&vec![preset.clone(); 21]).is_err());
        assert!(validate_presets(
            &(0..20)
                .map(|index| DownloadPreset {
                    name: format!("预设 {index}"),
                    ..preset.clone()
                })
                .collect::<Vec<_>>()
        )
        .is_ok());
        assert!(validate_presets(&[preset.clone()]).is_ok());
        assert!(validate_presets(&[DownloadPreset {
            name: String::new(),
            ..preset.clone()
        }])
        .is_err());
        assert!(validate_presets(&[DownloadPreset {
            subtitle_languages: vec!["--exec".into()],
            ..preset.clone()
        }])
        .is_err());
        let mut settings = AppSettings {
            download_presets: vec![preset],
            ..Default::default()
        };
        let p = &settings.download_presets[0];
        let task = DownloadTask {
            request: DownloadRequest {
                kind: p.kind.clone(),
                video_mode: p.video_mode.clone(),
                max_height: p.max_height,
                subtitle_languages: p.subtitle_languages.clone(),
                filename_template: p.filename_template.clone(),
                by_author: p.by_author,
                ..Default::default()
            },
            ..Default::default()
        };
        settings.download_presets[0].kind = MediaKind::Video;
        settings.download_presets[0].filename_template = "{日期}".into();
        assert_eq!(task.request.kind, MediaKind::Audio);
        assert_eq!(task.request.filename_template, "{标题}_{视频ID}");
        let old: AppSettings = serde_json::from_str(r#"{"downloadDir":"C:/saved","concurrency":2,"cookieMode":"none","browser":"edge","browserProfile":"","hasCookieFile":false,"proxyEnabled":false,"proxyUrl":""}"#).unwrap();
        assert!(old.download_presets.is_empty());
        let task: DownloadTask = serde_json::from_str(
            r#"{"id":"old","request":{"url":"https://example.com/movie"},"status":"paused"}"#,
        )
        .unwrap();
        assert!(!task.request.by_author);
        assert!(task.request.filename_template.is_empty());
        assert!(task.request.playlist_index.is_none());
    }
    #[test]
    fn author_subdirectory_subtitles_resolve_to_the_final_output_directory() {
        let task = DownloadTask {
            id: "test".into(),
            output_dir: "C:/downloads".into(),
            ..Default::default()
        };
        assert_eq!(
            crate::engine::final_output_path(
                &task,
                "C:/downloads/.video-downloader/test/作者/movie.en.srt"
            ),
            std::path::PathBuf::from("C:/downloads/作者/movie.en.srt")
        );
    }
}
