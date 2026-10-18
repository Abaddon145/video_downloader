// Explicit opt-in only. Never runs in the normal suite or in the application.
// Credential enters only this Rust test process through private stdin or a transient test-process environment; reports contain booleans and usage.
#[test]
#[ignore = "Requires explicit paid API consent and credential on private stdin; see QA documentation"]
fn live_deepseek_video_acceptance() {
    use super::{models::*, service::AiService, settings};
    use crate::knowledge::service::KnowledgeService;
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        path::PathBuf,
        time::{Duration, Instant},
    };
    let data =
        PathBuf::from(std::env::var("AI_QA_LIVE_STATE").expect("Set isolated QA state directory"));
    let resources = PathBuf::from(std::env::var("AI_QA_RESOURCES").unwrap());
    let source = fs::canonicalize(std::env::var("AI_QA_VIDEO").unwrap()).unwrap();
    let model = PathBuf::from(std::env::var("AI_QA_BASE_MODEL").unwrap());
    fs::create_dir_all(data.join("ai/models")).unwrap();
    fs::copy(model, data.join("ai/models/ggml-base.bin")).unwrap();
    if !data.join("ai/deepseek.key").exists() {
        let mut input = std::env::var("YINGLIU_LIVE_API_KEY").unwrap_or_default();
        std::env::remove_var("YINGLIU_LIVE_API_KEY");
        if input.is_empty() {
            std::io::stdin()
                .read_line(&mut input)
                .expect("Read credential from private stdin");
        }
        let mut key = input.into_bytes();
        while matches!(key.last(), Some(b'\r' | b'\n')) {
            key.pop();
        }
        let stored = settings::store_key(&data, &key);
        settings::wipe(&mut key);
        stored.unwrap();
    }
    // Optional offline import separates credential entry from the network-capable test process.
    if std::env::var("AI_QA_IMPORT_ONLY").as_deref() == Ok("1") {
        return;
    }
    struct RemoveKey(PathBuf);
    impl Drop for RemoveKey {
        fn drop(&mut self) {
            let _ = settings::clear_key(&self.0);
        }
    }
    let _remove_key = RemoveKey(data.clone());
    let ai = AiService::create(None, &data, &resources);
    let mut config = ai.settings().unwrap().settings;
    config.auto_generate_notes = false;
    config.auto_analyze_downloads = false;
    ai.save_settings(config).unwrap();
    tauri::async_runtime::block_on(ai.test_connection())
        .expect("Live model-list connection failed");
    let digest = |p: &std::path::Path| format!("{:x}", Sha256::digest(fs::read(p).unwrap()));
    let before = digest(&source);
    let analysis = ai
        .start_analysis(AnalysisRequest {
            force: false,
            path: source.to_string_lossy().into_owned(),
            item_id: None,
            mode: AnalysisMode::Quick,
            transcribe_audio: true,
            allow_transcript: true,
            allow_images: true,
        })
        .unwrap();
    let started = Instant::now();
    let done = loop {
        let a = ai.analysis(&analysis.id).unwrap();
        if ["completed", "failed", "cancelled", "interrupted"].contains(&a.status.as_str()) {
            break a;
        }
        assert!(
            started.elapsed() < Duration::from_secs(420),
            "Live analysis exceeded QA deadline"
        );
        std::thread::sleep(Duration::from_millis(200));
    };
    assert_eq!(
        done.status,
        "completed",
        "{}",
        done.error.as_deref().unwrap_or("Analysis did not complete")
    );
    let transcript = ai.with(|r| r.transcript(&analysis.id, 1)).unwrap();
    assert!(transcript.total > 0);
    assert!(done
        .summary
        .as_ref()
        .is_some_and(|s| !s.overview.is_empty() && !s.chapters.is_empty()));
    let frame_count = ai
        .with(|r| {
            r.conn
                .query_row(
                    "SELECT count(*) FROM analysis_keyframes WHERE analysis_id=?1",
                    [&analysis.id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(crate::learning::db::error)
        })
        .unwrap();
    assert!(frame_count > 0);
    let answer = tauri::async_runtime::block_on(ai.ask_video(
        &analysis.id,
        "What does the speaker say about country? Cite the video time.",
        None,
    ))
    .unwrap();
    assert!(!answer.result.answer.is_empty());
    assert!(!answer.result.references.is_empty());
    let vault = data.join("Vault");
    fs::create_dir_all(&vault).unwrap();
    let knowledge = KnowledgeService::new(&data);
    let vault_id = knowledge
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let note = ai
        .generate_note(&knowledge, &analysis.id, &vault_id)
        .unwrap();
    assert!(knowledge
        .read(&vault_id, &note.path)
        .unwrap()
        .data
        .text
        .starts_with("---\n"));
    assert_eq!(before, digest(&source));
    let usage = ai.usage(0, crate::service::now() as i64 + 1000).unwrap();
    assert!(!usage.models.is_empty());
    assert!(usage
        .models
        .iter()
        .any(|m| m.input_tokens > 0 && m.output_tokens > 0));
    settings::clear_key(&data).unwrap();
    assert!(!data.join("ai/deepseek.key").exists());
    let report = serde_json::json!({"liveApi":true,"mocked":false,"endpoint":super::client::API_BASE,"model":done.model,"analysisId":done.id,"status":done.status,"transcriptSegments":transcript.total,"keyframes":frame_count,"chapters":done.summary.as_ref().unwrap().chapters.len(),"questionReferences":answer.result.references,"usage":usage,"markdown":note,"sourceSha256Before":before,"sourceSha256After":digest(&source),"credentialRemoved":true,"elapsedSeconds":started.elapsed().as_secs()});
    let report_path = PathBuf::from(std::env::var("AI_QA_LIVE_REPORT").unwrap());
    fs::create_dir_all(report_path.parent().unwrap()).unwrap();
    fs::write(report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("LIVE_API_ACCEPTANCE passed; report includes usage, no credential or request body");
}

#[test]
#[ignore = "Requires verified FFmpeg resources and AI_QA_LOCAL_STATE; no API requests"]
fn real_duration_unicode_subtitle_matrix() {
    use super::{
        jobs::Control, keyframes, models::AnalysisMode, service::AiService, transcription as t,
    };
    use std::{
        fs,
        path::PathBuf,
        time::{Duration, Instant},
    };
    let root = PathBuf::from(std::env::var("AI_QA_LOCAL_STATE").unwrap());
    let resources = PathBuf::from(std::env::var("AI_QA_RESOURCES").unwrap());
    let mut folder = root.clone();
    for n in 0..4 {
        folder = folder.join(format!(
            "中文 课程 学习资料（最终版） # & Unicode Ω 한국어 {n}"
        ));
    }
    fs::create_dir_all(&folder).unwrap();
    let folder = fs::canonicalize(folder).unwrap();
    assert!(folder.to_string_lossy().len() > 260);
    let control = Control::default();
    let ffmpeg = t::verified_tool(&resources, "ffmpeg.exe").unwrap();
    let texts = [
        "中文课程 GPU 碰撞",
        "English course collision",
        "中英混合 Niagara GPU",
        "한국어 강의 충돌",
    ];
    let mut records = Vec::new();
    for seconds in [10, 600, 3660] {
        let source = folder.join(format!("第 01 课 {seconds}秒.mp4"));
        if !source.exists() {
            let args = [
                "-hide_banner",
                "-nostdin",
                "-loglevel",
                "error",
                "-n",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=320x180:r=1",
                "-t",
                &seconds.to_string(),
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p",
                source.to_str().unwrap(),
            ]
            .map(str::to_owned);
            control
                .capture(&ffmpeg, &args, None, Duration::from_secs(90))
                .unwrap();
        }
        let subtitle = source.with_extension("srt");
        let body = texts
            .iter()
            .enumerate()
            .map(|(i, text)| {
                format!(
                    "{}\n00:00:{:02},000 --> 00:00:{:02},900\n{}\n\n",
                    i + 1,
                    i * 2,
                    i * 2 + 1,
                    text
                )
            })
            .collect::<String>();
        fs::write(&subtitle, body).unwrap();
        let before = t::file_hash(&source).unwrap();
        let state = root.join(format!("state-{seconds}"));
        fs::create_dir_all(&state).unwrap();
        let ai = AiService::create(None, &state, &resources);
        let a = ai
            .start_transcription(source.to_str().unwrap(), None)
            .unwrap();
        let now = Instant::now();
        let done = loop {
            let a = ai.analysis(&a.id).unwrap();
            if ["completed", "failed", "cancelled", "interrupted"].contains(&a.status.as_str()) {
                break a;
            }
            assert!(now.elapsed() < Duration::from_secs(60));
            std::thread::sleep(Duration::from_millis(100));
        };
        assert_eq!(done.status, "completed", "{:?}", done.error);
        let rows = ai.with(|r| r.all_transcript(&a.id)).unwrap();
        assert_eq!(rows.len(), texts.len());
        for (row, text) in rows.iter().zip(texts) {
            assert_eq!(row.text, text);
        }
        let info = t::probe_media(&resources, &source, &control).unwrap();
        assert!(info.audios.is_empty());
        assert!((info.duration.unwrap() - seconds as f64).abs() < 1.);
        let frames = keyframes::extract(
            &resources,
            &source,
            &state.join("frames"),
            &a.id,
            AnalysisMode::Quick,
            &info,
            &control,
        )
        .unwrap();
        assert!(!frames.is_empty() && frames.len() <= 12);
        assert_eq!(before, t::file_hash(&source).unwrap());
        records.push(serde_json::json!({"seconds":seconds,"pathCharacters":source.to_string_lossy().len(),"subtitleLanguages":["Chinese","English","mixed","Korean"],"transcriptSegments":rows.len(),"keyframes":frames.len(),"noAudio":true,"apiRequests":0,"sourceSha256Before":before,"sourceSha256After":t::file_hash(&source).unwrap()}));
    }
    let report = serde_json::json!({"realLocalMedia":true,"speechQualityTest":false,"mockedApi":false,"apiRequests":0,"records":records});
    fs::write(
        root.join("local-duration-language-matrix.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("REAL_LOCAL_MATRIX 10s/600s/3660s, >260 character Unicode paths, four subtitle languages, source hashes unchanged; no API calls");
}
