use super::{
    analysis::{parse_result, validate_result, Scope},
    models::*,
};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

fn reference_service() -> (
    Arc<super::service::AiService>,
    Arc<super::jobs::Control>,
    PathBuf,
) {
    let dir = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("never-upload.mp4");
    std::fs::write(&source, b"SOURCE_MEDIA_NOT_FOR_UPLOAD").unwrap();
    let service = super::service::AiService::create(None, &dir, &dir);
    super::settings::store_key(&dir, b"sk-fake-private-reference-tests").unwrap();
    service.with(|r| {
        r.conn.execute("INSERT INTO video_analyses(id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,duration_ms,options_json,created_at,updated_at) VALUES('a',?1,?2,'analyzing_chunks','standard','deepseek-flash','v0.5.0-1',100000,?3,1,1)",
            rusqlite::params![source.to_string_lossy(), super::transcription::fingerprint(&source)?, serde_json::to_string(&AnalysisOptions { local_only:false, allow_images:false, ..Default::default() }).unwrap()]).unwrap();
        let mut cues = segments();
        cues.push(TranscriptSegment { id:"unrelated".into(), analysis_id:"a".into(), segment_index:1, start_ms:70000, end_ms:71000, text:"UNRELATED_PRIVATE_SUBTITLE".into() });
        r.save_transcript("a", Some("zh"), &cues)
    }).unwrap();
    let control = Arc::new(super::jobs::Control::default());
    service.runtime.lock().unwrap().active = Some(("a".into(), control.clone()));
    (service, control, dir)
}

pub(super) fn reference_http(
    replies: Vec<serde_json::Value>,
    mut first_hook: Option<Box<dyn FnOnce() + Send>>,
) -> (
    super::client::DeepSeekClient,
    Arc<Mutex<Vec<serde_json::Value>>>,
    std::thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = super::client::DeepSeekClient::fixture(format!(
        "http://{}",
        listener.local_addr().unwrap()
    ));
    listener.set_nonblocking(true).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let thread = std::thread::spawn(move || {
        for reply in replies {
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    _ => return,
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 8192];
            let (offset, length) = loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let length = String::from_utf8_lossy(&bytes[..end])
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while bytes.len() < offset + length {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
            }
            captured
                .lock()
                .unwrap()
                .push(serde_json::from_slice(&bytes[offset..offset + length]).unwrap());
            if let Some(hook) = first_hook.take() {
                hook();
            }
            let payload = json!({"model":"deepseek-flash","choices":[{"message":{"content":reply.to_string()},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":20,"prompt_cache_hit_tokens":10}}).to_string();
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                )
                .as_bytes(),
            );
        }
    });
    (client, requests, thread)
}

fn invalid_live_result() -> VideoAnalysisResult {
    let mut original = result(50000, &["s0"]);
    let mut valid = result(11000, &["s0"]).key_points.remove(0);
    valid.title = "原有效重点保持".into();
    original.key_points.push(valid);
    original
}

#[test]
fn reference_live_repair_is_one_bounded_http_request_and_completed_chunk_is_reused() {
    let (service, control, dir) = reference_service();
    let (client, requests, thread) = reference_http(
        vec![
            serde_json::to_value(invalid_live_result()).unwrap(),
            json!({"repairs":[{"index":0,"timeMs":11000,"sourceSegmentIds":["s0"]}]}),
        ],
        None,
    );
    let results =
        tauri::async_runtime::block_on(service.analyze_chunks("a", &control, &client)).unwrap();
    thread.join().unwrap();
    assert_eq!(
        results[0].key_points.len(),
        2,
        "the failed point should be repaired rather than silently omitted"
    );
    assert_eq!(results[0].key_points[0].time_ms, 11000);
    assert_eq!(results[0].key_points[1].title, "原有效重点保持");
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    let repair = requests[1].to_string();
    assert!(repair.contains("实际课程内容"));
    assert!(!repair.contains("UNRELATED_PRIVATE_SUBTITLE"));
    assert!(!repair.contains("原有效重点保持"));
    assert!(!repair.contains("SOURCE_MEDIA_NOT_FOR_UPLOAD"));
    assert!(!repair.contains("image_url"));
    assert!(repair.len() <= 24000);
    assert_eq!(requests[1]["max_tokens"], 2048);
    drop(requests);
    service
        .with(|r| {
            let operations = r
                .conn
                .prepare("SELECT operation FROM ai_usage ORDER BY operation")
                .unwrap()
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(operations, vec!["chunk", "reference_repair"]);
            Ok(())
        })
        .unwrap();
    let unreachable = super::client::DeepSeekClient::fixture("http://127.0.0.1:1".into());
    assert!(
        tauri::async_runtime::block_on(service.analyze_chunks("a", &control, &unreachable)).is_ok()
    );
    drop(service);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn reference_live_repair_invalid_output_degrades_only_failed_point_once() {
    for repair in [
        json!({"repairs":[{"index":0,"timeMs":11000,"sourceSegmentIds":["forged"]}]}),
        json!({"repairs":[{"index":0,"timeMs":70000,"sourceSegmentIds":["unrelated"]}]}),
        json!({"repairs":[{"index":0,"timeMs":100001,"sourceSegmentIds":["s0"]}]}),
        json!({"repairs":[{"index":1,"timeMs":11000,"sourceSegmentIds":["s0"]}]}),
        json!({"repairs":[{"index":0,"timeMs":11000,"sourceSegmentIds":["s0"]},{"index":0,"timeMs":11000,"sourceSegmentIds":["s0"]}]}),
        json!({"repairs":[],"unexpected":"forbidden"}),
    ] {
        let (service, control, dir) = reference_service();
        let (client, requests, thread) = reference_http(
            vec![serde_json::to_value(invalid_live_result()).unwrap(), repair],
            None,
        );
        let results =
            tauri::async_runtime::block_on(service.analyze_chunks("a", &control, &client)).unwrap();
        thread.join().unwrap();
        assert_eq!(
            requests.lock().unwrap().len(),
            2,
            "at most one repair attempt, including a failed one"
        );
        assert_eq!(results[0].key_points.len(), 1);
        assert_eq!(results[0].key_points[0].title, "原有效重点保持");
        assert!(serde_json::to_value(&results[0]).unwrap()["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w.as_str().unwrap().contains("部分重点无法核对已略过")));
        drop(service);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn reference_live_repair_checks_cancel_privacy_and_source_before_followup() {
    for cause in ["cancel", "privacy", "source"] {
        let (service, control, dir) = reference_service();
        let s = service.clone();
        let c = control.clone();
        let source = dir.join("never-upload.mp4");
        let hook = Box::new(move || match cause {
            "cancel" => c.cancel(),
            "privacy" => {
                s.settings
                    .lock()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .allow_transcript = false;
            }
            _ => std::fs::write(source, b"CHANGED_SOURCE").unwrap(),
        });
        let (client, requests, thread) = reference_http(
            vec![
                serde_json::to_value(invalid_live_result()).unwrap(),
                json!({"repairs":[]}),
            ],
            Some(hook),
        );
        let error = tauri::async_runtime::block_on(service.analyze_chunks("a", &control, &client))
            .expect_err("lifecycle change must stop reference repair");
        thread.join().unwrap();
        assert_eq!(
            requests.lock().unwrap().len(),
            1,
            "{cause}: no followup request allowed"
        );
        assert!(
            error.contains("取消") || error.contains("隐私") || error.contains("变化"),
            "{error}"
        );
        assert_eq!(service.with(|r| r.completed_chunks("a")).unwrap(), 0);
        drop(service);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

fn segments() -> Vec<TranscriptSegment> {
    vec![TranscriptSegment {
        id: "s0".into(),
        analysis_id: "a".into(),
        segment_index: 0,
        start_ms: 10500,
        end_ms: 12950,
        text: "实际课程内容".into(),
    }]
}

fn result(time: i64, ids: &[&str]) -> VideoAnalysisResult {
    serde_json::from_value(json!({"contentType":"lecture","oneSentenceSummary":"概要","overview":"课程内容",
        "chapters":[],"keyPoints":[{"timeMs":time,"title":"重点","content":"实际课程内容","importance":3,"sourceSegmentIds":ids}],
        "tutorialSteps":[],"terms":[],"actionItems":[],"tags":[]})).unwrap()
}

#[test]
fn reference_small_offsets_are_corrected_to_the_nearest_real_cue_boundary() {
    let segments = segments();
    let scope = Scope {
        start_ms: 0,
        end_ms: 100000,
        segments: &segments,
        frames: &[],
    };
    for (time, expected) in [
        (11000, 11000),
        (10000, 10500),
        (7500, 10500),
        (13000, 12950),
        (15950, 12950),
    ] {
        let original = result(time, &["s0"]);
        let parsed = parse_result(&serde_json::to_string(&original).unwrap(), &scope)
            .expect("small grounded offset should recover");
        assert_eq!(parsed.key_points[0].time_ms, expected);
        assert_eq!(
            parsed.key_points[0].source_segment_ids,
            original.key_points[0].source_segment_ids
        );
        validate_result(&parsed, &scope).unwrap();
    }
}

#[test]
fn reference_cache_degrades_only_unverified_points_and_keeps_durable_notice() {
    let segments = segments();
    let scope = Scope {
        start_ms: 0,
        end_ms: 100000,
        segments: &segments,
        frames: &[],
    };
    for (time, ids) in [
        (50000, vec!["s0"]),
        (7499, vec!["s0"]),
        (11000, vec!["s0", "forged"]),
        (-1, vec!["s0"]),
        (100001, vec!["s0"]),
    ] {
        let mut original = result(time, &ids);
        original
            .key_points
            .push(result(11000, &["s0"]).key_points.remove(0));
        let parsed = parse_result(&serde_json::to_string(&original).unwrap(), &scope)
            .expect("unverified point should not abort valid analysis");
        assert_eq!(parsed.key_points.len(), 1);
        assert_eq!(parsed.key_points[0].time_ms, 11000);
        let saved = serde_json::to_value(&parsed).unwrap();
        assert!(saved["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s.as_str().unwrap().contains("部分重点无法核对已略过")));
        let reloaded = parse_result(&saved.to_string(), &scope).unwrap();
        assert_eq!(serde_json::to_value(reloaded).unwrap(), saved);
    }
}

#[test]
fn reference_recovery_keeps_schema_limits_and_image_only_validation() {
    let segments = segments();
    let frame = AnalysisKeyframe {
        id: "f0".into(),
        analysis_id: "a".into(),
        time_ms: 50000,
        path: "never-read.jpg".into(),
        sha256: "0".repeat(64),
        scene_score: None,
    };
    let scope = Scope {
        start_ms: 0,
        end_ms: 100000,
        segments: &segments,
        frames: &[frame],
    };
    let parsed =
        parse_result(&serde_json::to_string(&result(50000, &[])).unwrap(), &scope).unwrap();
    assert_eq!(parsed.key_points[0].time_ms, 50000);
    let mut invalid = result(50000, &["s0"]);
    invalid.key_points[0].importance = 0;
    assert!(parse_result(&serde_json::to_string(&invalid).unwrap(), &scope).is_err());
    invalid = result(50000, &["s0"]);
    invalid.key_points[0].source_segment_ids = vec!["s0".into(); 33];
    assert!(parse_result(&serde_json::to_string(&invalid).unwrap(), &scope).is_err());
    assert!(parse_result("{truncated", &scope).is_err());
}

#[test]
fn reference_normalization_respects_scope_intersection_and_bounded_local_warnings() {
    let segments = segments();
    let scope = Scope {
        start_ms: 11000,
        end_ms: 20000,
        segments: &segments,
        frames: &[],
    };
    let parsed = parse_result(
        &serde_json::to_string(&result(14000, &["s0"])).unwrap(),
        &scope,
    )
    .unwrap();
    assert_eq!(parsed.key_points[0].time_ms, 12950);
    let no_overlap = Scope {
        start_ms: 20000,
        end_ms: 40000,
        segments: &segments,
        frames: &[],
    };
    let mut original = serde_json::to_value(result(30000, &["s0"])).unwrap();
    original["warnings"] = json!(["1", "2", "3", "4", "5", "6", "7", "8"]);
    let parsed = parse_result(&original.to_string(), &no_overlap).unwrap();
    assert!(parsed.key_points.is_empty());
    assert_eq!(
        serde_json::to_value(parsed).unwrap()["warnings"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    original["warnings"] = json!(["X".repeat(201)]);
    assert!(parse_result(&original.to_string(), &no_overlap).is_err());
}
