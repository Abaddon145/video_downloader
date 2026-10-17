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
