use super::repository::Repository;
use std::path::Path;
#[test]
fn analysis_recovery_paging_and_invalid_filter() {
    let r = Repository::open(Path::new(":memory:")).unwrap();
    for i in 0..51 {
        r.conn.execute("INSERT INTO video_analyses(id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,created_at,updated_at) VALUES(?1,'keep.wmv','fp','transcribing','standard','deepseek-flash','v0.5.0-1',1,1)",[format!("a{i:03}")]).unwrap();
    }
    assert_eq!(r.list(None, 1).unwrap().items.len(), 50);
    assert_eq!(r.list(None, 2).unwrap().items.len(), 1);
    assert_eq!(r.recover().unwrap(), 51);
    assert_eq!(r.get("a000").unwrap().status, "interrupted");
    assert_eq!(r.recover().unwrap(), 0);
    assert!(r.list(Some("' OR 1=1"), 1).is_err());
    assert!(r.get("missing").is_err());
    assert_eq!(r.list(Some("completed"), 1).unwrap().total, 0);
}

#[test]
fn deepseek_key_roundtrip_replace_delete_and_settings_never_expose_plaintext() {
    use super::settings::*;
    let data = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
    std::fs::create_dir(&data).unwrap();
    assert!(load_key(&data).unwrap_err().contains("密钥"));
    let first = b"sk-test-private-first-value-123456";
    store_key(&data, first).unwrap();
    assert_eq!(load_key(&data).unwrap().as_bytes(), first);
    let encrypted = std::fs::read(data.join("ai/deepseek.key")).unwrap();
    assert!(!encrypted.windows(first.len()).any(|v| v == first));
    let next = b"sk-test-private-second-value-654321";
    store_key(&data, next).unwrap();
    assert_eq!(load_key(&data).unwrap().as_bytes(), next);
    assert!(!data.join("ai/deepseek.bak").exists());
    assert!(store_key(&data, b"bad\r\nAuthorization: leaked").is_err());
    let settings = AiSettings::default();
    save_settings(&data, &settings).unwrap();
    let view = serde_json::to_string(&AiSettingsView {
        settings,
        has_api_key: true,
    })
    .unwrap();
    assert!(!view.contains("sk-test"));
    assert!(!std::fs::read_to_string(data.join("ai/settings.json"))
        .unwrap()
        .contains("sk-test"));
    clear_key(&data).unwrap();
    assert!(!data.join("ai/deepseek.key").exists());
    clear_key(&data).unwrap();
    std::fs::remove_dir_all(data).unwrap();
}
#[test]
fn ai_settings_enforce_endpoint_model_and_privacy_boundaries() {
    use super::settings::*;
    let mut s = AiSettings::default();
    assert_eq!(s.model, "deepseek-flash");
    assert!(!s.auto_analyze_downloads);
    assert!(s.auto_generate_notes && s.clean_temporary_files);
    s.chunk_concurrency = 3;
    assert!(s.validate().is_err());
    s.chunk_concurrency = 1;
    s.model = "https://evil.invalid".into();
    assert!(s.validate().is_err());
    s.model = "deepseek-v4-pro".into();
    assert!(s.validate().is_err());
    s.allow_images = false;
    assert!(s.validate().is_ok());
    s.allow_transcript = false;
    assert!(s.validate().is_ok()); // local transcription remains available
    let unknown = serde_json::json!({"apiBaseUrl":"https://evil.invalid"});
    assert!(serde_json::from_value::<AiSettings>(unknown).is_err());
}

#[test]
fn whisper_timestamps_validate_and_preserve_real_segments() {
    use super::transcription::parse_transcript;
    let valid=br#"{"result":{"language":"en"},"transcription":[{"offsets":{"from":0,"to":10500},"text":" real speech "}]}"#;
    let (language, segments) = parse_transcript(valid, "a", 11000).unwrap();
    assert_eq!(language.as_deref(), Some("en"));
    assert_eq!(segments[0].start_ms, 0);
    assert_eq!(segments[0].end_ms, 10500);
    assert_eq!(segments[0].text, "real speech");
    for bad in [
        br#"{"transcription":[{"offsets":{"from":-1,"to":100},"text":"bad"}]}"#.as_slice(),
        br#"{"transcription":[{"offsets":{"from":20000,"to":30000},"text":"bad"}]}"#,
        br#"{"transcription":[{"offsets":{"from":200,"to":10},"text":"bad"}]}"#,
        b"{truncated",
    ] {
        assert!(parse_transcript(bad, "a", 11000).is_err());
    }
}
#[test]
fn whisper_manifest_and_preflight_reject_missing_or_corrupt_files() {
    use super::transcription::*;
    let models = model_specs().unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].bytes, 147951465);
    assert!(!models[0].url.contains("latest"));
    assert_eq!(models[0].sha256.len(), 64);
    let data = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
    std::fs::create_dir(&data).unwrap();
    assert!(verify_model(&data, "base").unwrap_err().contains("模型"));
    std::fs::create_dir_all(data.join("ai/models")).unwrap();
    std::fs::write(data.join("ai/models/ggml-base.bin"), b"broken").unwrap();
    assert!(verify_model(&data, "base").is_err());
    assert!(verify_model(&data, "../../Windows").is_err());
    std::fs::remove_dir_all(data).unwrap();
}

#[test]
#[ignore = "Requires verified Whisper models/FFmpeg fixtures; run scripts/qa instructions with AI_QA_FIXTURES and AI_QA_RESOURCES"]
fn whisper_real_audio_unicode_pipeline_and_source_protection() {
    use super::{jobs::Control, transcription as t};
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };
    let fixtures = std::path::PathBuf::from(std::env::var("AI_QA_FIXTURES").unwrap());
    let resources = std::path::PathBuf::from(std::env::var("AI_QA_RESOURCES").unwrap());
    let data = std::env::temp_dir().join(format!(
        "映流 AI #&（）韩文한국어-{}",
        crate::ffmpeg::runner::new_id()
    ));
    std::fs::create_dir_all(data.join("ai/models")).unwrap();
    for model in t::model_specs().unwrap() {
        std::fs::hard_link(
            fixtures.join(&model.filename),
            data.join("ai/models").join(model.filename),
        )
        .unwrap();
    }
    let source = data.join("第01课 #Niagara & English（最终版）.wav");
    std::fs::copy(fixtures.join("jfk.wav"), &source).unwrap();
    let before = t::file_hash(&source).unwrap();
    let control = Control::default();
    let info = t::probe_media(&resources, &source, &control).unwrap();
    assert!(info.videos.is_empty());
    assert!(!info.audios.is_empty());
    let duration = (info.duration.unwrap() * 1000.).round() as i64;
    for model in ["base", "small"] {
        let cache = data.join(format!("ai/cache/中文括号（）#{model}"));
        t::extract_audio(&resources, &source, &cache, &info, &control).unwrap();
        let start = Instant::now();
        let (lang, segments) =
            t::transcribe(&resources, &data, &cache, model, "real", duration, &control).unwrap();
        assert_eq!(lang.as_deref(), Some("en"));
        assert!(segments
            .iter()
            .any(|s| s.text.to_lowercase().contains("country")));
        assert!(segments.iter().all(|s| s.end_ms <= duration));
        println!(
            "REAL_WHISPER model={model} duration_ms={duration} segments={} elapsed_ms={}",
            segments.len(),
            start.elapsed().as_millis()
        );
    }
    assert_eq!(t::file_hash(&source).unwrap(), before);
    let lock = t::lock_source(&source).unwrap();
    assert!(std::fs::OpenOptions::new()
        .write(true)
        .open(&source)
        .is_err());
    drop(lock);
    let empty = crate::ffmpeg::models::MediaInfo::default();
    assert!(t::extract_audio(
        Path::new("missing"),
        &source,
        &data.join("none"),
        &empty,
        &control
    )
    .unwrap()
    .is_none());
    assert!(t::whisper_binary(&data).is_err());
    let cache = data.join("cancel");
    std::fs::create_dir(&cache).unwrap();
    let exe = t::verified_tool(&resources, "ffmpeg.exe").unwrap();
    control
        .capture(
            &exe,
            &[
                "-hide_banner",
                "-nostdin",
                "-loglevel",
                "error",
                "-n",
                "-stream_loop",
                "60",
                "-i",
            ]
            .map(str::to_owned)
            .into_iter()
            .chain([
                source.to_string_lossy().into_owned(),
                "-t".into(),
                "600".into(),
                "-ar".into(),
                "16000".into(),
                "-ac".into(),
                "1".into(),
                cache.join("audio.wav").to_string_lossy().into_owned(),
            ])
            .collect::<Vec<_>>(),
            None,
            Duration::from_secs(30),
        )
        .unwrap();
    let c = Arc::new(Control::default());
    let worker = c.clone();
    let res = resources.clone();
    let d = data.clone();
    let dir = cache.clone();
    let thread = std::thread::spawn(move || {
        t::transcribe(&res, &d, &dir, "base", "cancel", 600000, &worker)
    });
    let start = Instant::now();
    while !c.has_job() && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        c.has_job(),
        "Whisper process must actually start before cancellation"
    );
    c.cancel();
    let error = thread.join().unwrap().unwrap_err();
    assert!(error.contains("取消"));
    assert!(!cache.join("model.bin").exists());
    println!("REAL_WHISPER cancelled_process=true source_sha256={before}");
    std::fs::remove_dir_all(data).unwrap();
}
#[test]
fn ai_transcript_database_paging_and_fts_are_atomic() {
    let mut r = Repository::open(Path::new(":memory:")).unwrap();
    r.conn.execute("INSERT INTO video_analyses(id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,created_at,updated_at) VALUES('a','a.wav','fp','transcribing','standard','deepseek-flash','v0.5.0-1',1,1)",[]).unwrap();
    let items = (0..51)
        .map(|i| super::models::TranscriptSegment {
            id: format!("t{i}"),
            analysis_id: "a".into(),
            segment_index: i,
            start_ms: i as i64 * 1000,
            end_ms: i as i64 * 1000 + 900,
            text: format!("真实字幕 {i}"),
        })
        .collect::<Vec<_>>();
    r.save_transcript("a", Some("zh"), &items).unwrap();
    assert_eq!(r.transcript("a", 1).unwrap().items.len(), 50);
    assert_eq!(r.transcript("a", 2).unwrap().items.len(), 1);
    let mut bad = items[0].clone();
    bad.end_ms = -1;
    assert!(r.save_transcript("a", None, &[bad]).is_err());
    assert_eq!(r.transcript("a", 1).unwrap().total, 51);
    r.save_transcript("a", Some("zh"), &items[..1]).unwrap();
    assert_eq!(r.transcript("a", 1).unwrap().total, 1);
}

#[test]
fn ai_keyframe_sampling_is_bounded_distributed_and_rejects_fake_scene_times() {
    use super::{keyframes::*, models::AnalysisMode};
    for (mode, cap) in [
        (AnalysisMode::Quick, 12),
        (AnalysisMode::Standard, 24),
        (AnalysisMode::Deep, 60),
    ] {
        let policy = policy(mode);
        assert_eq!(policy.max_frames, cap);
        let times =
            select_times(7200000, mode, &[(1000, 0.9), (2000, 0.8), (5000000, 0.95)]).unwrap();
        assert!(times.len() <= cap);
        assert!(times.iter().any(|t| t.0 > 6000000));
        assert!(times.windows(2).all(|w| w[0].0 < w[1].0));
    }
    assert!(select_times(0, AnalysisMode::Standard, &[]).is_err());
    let scenes=parse_scenes("frame:0 pts:9000 pts_time:0.1\nlavfi.scene_score=0.7\nframe:1 pts:200 pts_time:NaN\nlavfi.scene_score=0.8\nframe:2 pts:9 pts_time:999\nlavfi.scene_score=0.9\n",10000);
    assert_eq!(scenes, vec![(100, 0.7)]);
    let short = select_times(100, AnalysisMode::Standard, &[]).unwrap();
    assert!(!short.is_empty());
    assert!(short.iter().all(|s| s.0 >= 0 && s.0 < 100));
}

#[test]
#[ignore = "Requires real media matrix; AI_QA_MATRIX and AI_QA_RESOURCES"]
fn keyframes_real_format_matrix_scene_4k_and_readonly_sources() {
    use super::{jobs::Control, keyframes as k, models::AnalysisMode, transcription as t};
    use std::time::Duration;
    let resources = std::path::PathBuf::from(std::env::var("AI_QA_RESOURCES").unwrap());
    let matrix: serde_json::Value =
        serde_json::from_slice(&std::fs::read(std::env::var("AI_QA_MATRIX").unwrap()).unwrap())
            .unwrap();
    let directory = std::env::temp_dir().join(format!(
        "映流画面 #&（）-{}",
        crate::ffmpeg::runner::new_id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let control = Control::default();
    let mut total = 0;
    for (index, record) in matrix["records"].as_array().unwrap().iter().enumerate() {
        let source = Path::new(record["path"].as_str().unwrap());
        let before = t::file_hash(source).unwrap();
        let info = t::probe_media(&resources, source, &control).unwrap();
        let frames = k::extract(
            &resources,
            source,
            &directory.join(format!("matrix-{index}")),
            "matrix",
            AnalysisMode::Standard,
            &info,
            &control,
        )
        .unwrap();
        assert!(!frames.is_empty(), "{}", source.display());
        assert!(frames.len() <= 24);
        for frame in &frames {
            let (w, h) = k::jpeg_dimensions(&std::fs::read(&frame.path).unwrap()).unwrap();
            assert!(w <= 1280 && h <= 1280);
            assert!(frame.time_ms >= 0 && frame.time_ms <= (info.duration.unwrap() * 1000.) as i64);
            assert_eq!(t::file_hash(Path::new(&frame.path)).unwrap(), frame.sha256);
        }
        assert_eq!(t::file_hash(source).unwrap(), before);
        total += 1;
        println!(
            "REAL_FRAMES source={} codec={} frames={} source_unchanged=true",
            source.file_name().unwrap().to_string_lossy(),
            info.videos[0].codec,
            frames.len()
        );
    }
    let ffmpeg = t::verified_tool(&resources, "ffmpeg.exe").unwrap();
    let scene = directory.join("场景颜色.mp4");
    let graph="color=c=red:s=320x180:r=5:d=3[r];color=c=blue:s=320x180:r=5:d=3[b];color=c=white:s=320x180:r=5:d=3[w];[r][b][w]concat=n=3:v=1:a=0";
    control
        .capture(
            &ffmpeg,
            &[
                "-hide_banner",
                "-nostdin",
                "-loglevel",
                "error",
                "-n",
                "-f",
                "lavfi",
                "-i",
                graph,
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p",
            ]
            .map(str::to_owned)
            .into_iter()
            .chain([scene.to_string_lossy().into_owned()])
            .collect::<Vec<_>>(),
            None,
            Duration::from_secs(30),
        )
        .unwrap();
    let info = t::probe_media(&resources, &scene, &control).unwrap();
    assert!(info.audios.is_empty());
    let frames = k::extract(
        &resources,
        &scene,
        &directory.join("scene"),
        "scene",
        AnalysisMode::Deep,
        &info,
        &control,
    )
    .unwrap();
    assert!(
        frames.iter().any(|f| f.scene_score.is_some()),
        "scene changes must actually be detected"
    );
    println!(
        "REAL_FRAMES scene_detected={} frames={}",
        frames.iter().filter(|f| f.scene_score.is_some()).count(),
        frames.len()
    );
    let fourk = directory.join("4K 无音轨.mp4");
    control
        .capture(
            &ffmpeg,
            &[
                "-hide_banner",
                "-nostdin",
                "-loglevel",
                "error",
                "-n",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=3840x2160:rate=1",
                "-t",
                "10",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-crf",
                "35",
                "-pix_fmt",
                "yuv420p",
            ]
            .map(str::to_owned)
            .into_iter()
            .chain([fourk.to_string_lossy().into_owned()])
            .collect::<Vec<_>>(),
            None,
            Duration::from_secs(60),
        )
        .unwrap();
    let info = t::probe_media(&resources, &fourk, &control).unwrap();
    let before = t::file_hash(&fourk).unwrap();
    let frames = k::extract(
        &resources,
        &fourk,
        &directory.join("4k"),
        "fourk",
        AnalysisMode::Standard,
        &info,
        &control,
    )
    .unwrap();
    assert!(!frames.is_empty());
    assert_eq!(
        k::jpeg_dimensions(&std::fs::read(&frames[0].path).unwrap()).unwrap(),
        (1280, 720)
    );
    assert_eq!(t::file_hash(&fourk).unwrap(), before);
    let cancelled = Control::default();
    cancelled.cancel();
    assert!(k::extract(
        &resources,
        &fourk,
        &directory.join("cancel"),
        "cancel",
        AnalysisMode::Standard,
        &info,
        &cancelled
    )
    .is_err());
    assert!(k::extract(
        &resources,
        &fourk,
        &directory.join("audio"),
        "audio",
        AnalysisMode::Standard,
        &crate::ffmpeg::models::MediaInfo::default(),
        &control
    )
    .unwrap()
    .is_empty());
    println!("REAL_FRAMES matrix_formats={total} fourk=1280x720 no_audio=true pre_cancel=true");
    std::fs::remove_dir_all(directory).unwrap();
}
