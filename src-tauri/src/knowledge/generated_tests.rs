use super::service::KnowledgeService;
use crate::ai::{models::*, service::AiService, transcription};
use std::{fs, path::PathBuf, sync::Arc};
fn fixture() -> (PathBuf, Arc<AiService>, Arc<KnowledgeService>, String) {
    let root = std::env::temp_dir().join(crate::ffmpeg::runner::new_id());
    fs::create_dir(&root).unwrap();
    let vault = root.join("Vault 中文");
    fs::create_dir(&vault).unwrap();
    let source = root.join("课程.mp4");
    fs::write(&source, b"source-read-only").unwrap();
    let source = fs::canonicalize(source).unwrap();
    let ai = AiService::create(None, &root, &root);
    let k = KnowledgeService::new(&root);
    let id = k
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let summary = serde_json::json!({"contentType":"tutorial","oneSentenceSummary":"课程总览","overview":"概述","chapters":[{"startMs":0,"endMs":400000,"title":"章节","summary":"总结","keywords":[],"keyframeIds":[]}],"keyPoints":[{"timeMs":180000,"title":"重点","content":"知识解释","importance":3,"sourceSegmentIds":["s1"]}],"tutorialSteps":[],"terms":[{"name":"GPU","definition":"图形处理器","timeMs":180000}],"actionItems":["练习"],"tags":["课程","../../escape"]});
    ai.with(|r|{r.conn.execute("INSERT INTO series(id,title,created_at,updated_at) VALUES('series','系列',1,1)",[]).unwrap();r.conn.execute("INSERT INTO series_items(id,series_id,title,file_path,duration_ms,created_at,updated_at) VALUES('lesson','series','课程:测试/安全',?1,400000,1,1)",[source.to_string_lossy().as_ref()]).unwrap();r.conn.execute("INSERT INTO video_analyses(id,item_id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,duration_ms,summary_json,options_json,created_at,updated_at) VALUES('a','lesson',?1,?2,'completed','standard','deepseek-flash','test',400000,?3,?4,1,1)",rusqlite::params![source.to_string_lossy(),transcription::fingerprint(&source)?,summary.to_string(),serde_json::to_string(&AnalysisOptions{local_only:false,..Default::default()}).unwrap()]).unwrap();
 let rows=(0..3).map(|i|TranscriptSegment{id:format!("s{i}"),analysis_id:"a".into(),segment_index:i,start_ms:i as i64*180000,end_ms:i as i64*180000+1000,text:format!("真实字幕 {i}")}).collect::<Vec<_>>();r.save_transcript("a",Some("zh"),&rows)}).unwrap();
    (root, ai, k, id)
}
#[test]
fn generated_pack_is_plain_safe_timestamped_reusable_and_preserves_edits() {
    let (root, ai, k, id) = fixture();
    let before = fs::read(root.join("课程.mp4")).unwrap();
    let first = ai.generate_note(&k, "a", &id).unwrap();
    assert!(first.path.ends_with("/视频总结.md"));
    assert!(first.transcript_path.ends_with("/完整转录.md"));
    assert!(first.knowledge_path.ends_with("/关键知识.md"));
    let summary = k.read(&id, &first.path).unwrap().data;
    assert!(summary.text.contains("yingliu://video/lesson?t=180000"));
    assert!(summary.text.contains("[[完整转录]]"));
    let parsed = super::markdown::parse_note(&first.path, &summary.text);
    assert_eq!(parsed.analysis_id.as_deref(), Some("a"));
    assert_eq!(parsed.item_id.as_deref(), Some("lesson"));
    let transcript = k.read(&id, &first.transcript_path).unwrap().data;
    assert!(transcript.text.contains("00:03:00"));
    assert!(transcript.text.contains("真实字幕 2"));
    let edited = summary.text + "\n用户编辑保留";
    k.write(&id, &first.path, &edited, &summary.hash).unwrap();
    let repeated = ai.generate_note(&k, "a", &id).unwrap();
    assert_eq!(first.path, repeated.path);
    assert!(k
        .read(&id, &first.path)
        .unwrap()
        .data
        .text
        .ends_with("用户编辑保留"));
    assert_eq!(fs::read(root.join("课程.mp4")).unwrap(), before);
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn export_vault_switch_source_change_and_cancel_never_publish() {
    let (root, ai, k, id) = fixture();
    let other = root.join("Other");
    fs::create_dir(&other).unwrap();
    k.set_vault(other.to_str().unwrap(), false).unwrap();
    assert!(ai.generate_note(&k, "a", &id).is_err());
    k.set_vault(root.join("Vault 中文").to_str().unwrap(), false)
        .unwrap();
    fs::write(root.join("课程.mp4"), b"replaced").unwrap();
    assert!(ai.generate_note(&k, "a", &id).is_err());
    assert!(ai.analysis("a").unwrap().markdown_path.is_none());
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn joint_delete_requires_current_hashes_and_keeps_source_and_unknown_files() {
    let (root, ai, k, id) = fixture();
    let generated = ai.generate_note(&k, "a", &id).unwrap();
    let plan = ai.plan_note_delete(&k, "a", &id).unwrap();
    assert_eq!(plan.files.len(), 3);
    let source = root.join("课程.mp4");
    let doc = k.read(&id, &generated.path).unwrap().data;
    k.write(&id, &generated.path, &(doc.text + "\nuser edit"), &doc.hash)
        .unwrap();
    assert!(ai.delete_note_bundle(&k, "a", &id, &plan.files).is_err());
    assert!(ai.analysis("a").is_ok());
    assert!(source.exists());
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn generation_schema_is_idempotent_and_note_mapping_survives_plain_note_edits() {
    let (root, ai, k, _) = fixture();
    ai.with(|r|{let count:i64=r.conn.query_row("SELECT count(*) FROM pragma_table_info('video_analyses') WHERE name IN ('note_target_vault_id','note_warning')",[],|r|r.get(0)).unwrap();assert_eq!(count,2);Ok(())}).unwrap();
    let db = crate::learning::repository::Repository::open(&root.join("learning.db")).unwrap();
    let note = db
        .save_note(&crate::learning::models::NoteInput {
            id: None,
            item_id: "lesson".into(),
            time_ms: 1000,
            content: "普通笔记".into(),
        })
        .unwrap();
    let row = db.notes("lesson", 1).unwrap().notes.pop().unwrap();
    assert_eq!(row.id, note);
    assert!(row.markdown_path.is_none());
    drop(db);
    drop(ai);
    drop(k);
    let db = crate::learning::db::open(&root.join("learning.db")).unwrap();
    drop(db);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn generated_frontmatter_has_required_fields_and_handles_yaml_delimiter_in_title() {
    let (root, ai, k, id) = fixture();
    ai.with(|r| {
        r.conn
            .execute(
                "UPDATE series_items SET title='多行\n---\n课程' WHERE id='lesson'",
                [],
            )
            .unwrap();
        Ok(())
    })
    .unwrap();
    let generated = ai.generate_note(&k, "a", &id).unwrap();
    let doc = k.read(&id, &generated.path).unwrap().data;
    let (meta, _) = super::generated::metadata(&doc, "a").unwrap();
    assert_eq!(meta["source"].as_str(), Some("yingliu"));
    assert!(meta["duration"].as_i64().is_some());
    assert_eq!(meta["ai_model"].as_str(), Some("deepseek-flash"));
    assert_eq!(meta["analysis_mode"].as_str(), Some("standard"));
    assert!(meta["analyzed_at"].as_i64().is_some());
    let mut crlf = doc.clone();
    crlf.text = format!("\u{feff}{}", doc.text.replace('\n', "\r\n"));
    super::generated::metadata(&crlf, "a").unwrap();
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn linked_screenshot_notes_cannot_be_edited_through_plain_note_save() {
    let (root, ai, k, id) = fixture();
    let db = crate::learning::repository::Repository::open(&root.join("learning.db")).unwrap();
    let note = db
        .save_note(&crate::learning::models::NoteInput {
            id: None,
            item_id: "lesson".into(),
            time_ms: 1000,
            content: "摘要".into(),
        })
        .unwrap();
    db.conn
        .execute(
            "UPDATE video_notes SET markdown_vault_id=?2,markdown_path='映流/截图.md' WHERE id=?1",
            rusqlite::params![note, id],
        )
        .unwrap();
    assert!(db
        .save_note(&crate::learning::models::NoteInput {
            id: Some(note),
            item_id: "lesson".into(),
            time_ms: 1000,
            content: "不应覆盖MD".into()
        })
        .is_err());
    drop(db);
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn metadata_edits_are_preserved_by_joint_delete_preview() {
    let (root, ai, k, id) = fixture();
    let generated = ai.generate_note(&k, "a", &id).unwrap();
    let doc = k.read(&id, &generated.path).unwrap().data;
    let changed = doc.text.replacen("tags:", "edited_by: user\ntags:", 1);
    k.write(&id, &generated.path, &changed, &doc.hash).unwrap();
    let plan = ai.plan_note_delete(&k, "a", &id).unwrap();
    assert!(!plan.files.iter().any(|f| f.path == generated.path));
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn auto_note_uses_captured_vault_and_export_failure_keeps_completed_summary() {
    use crate::ai::jobs::Control;
    let (root, ai, k, id) = fixture();
    ai.with(|r| {
        r.conn
            .execute(
                "UPDATE video_analyses SET note_target_vault_id=?1 WHERE id='a'",
                [&id],
            )
            .unwrap();
        Ok(())
    })
    .unwrap();
    let c = Arc::new(Control::default());
    ai.runtime.lock().unwrap().active = Some(("a".into(), c.clone()));
    let other = root.join("Other");
    fs::create_dir(&other).unwrap();
    k.set_vault(other.to_str().unwrap(), false).unwrap();
    ai.export_auto_note_with(&k, "a", &c).unwrap();
    let a = ai.analysis("a").unwrap();
    assert_eq!(a.status, "completed");
    assert!(a.summary.is_some());
    assert!(a.note_warning.is_some());
    assert!(a.markdown_path.is_none());
    k.set_vault(root.join("Vault 中文").to_str().unwrap(), false)
        .unwrap();
    c.cancel();
    assert!(ai.export_auto_note_with(&k, "a", &c).is_err());
    assert!(ai.analysis("a").unwrap().markdown_path.is_none());
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn joint_delete_only_trashes_verified_pack_and_keeps_user_file_and_media() {
    let (root, ai, k, id) = fixture();
    let generated = ai.generate_note(&k, "a", &id).unwrap();
    let base = generated.path.rsplit_once('/').unwrap().0;
    let extra = format!("{base}/用户.md");
    k.create(&id, &extra, "用户文件", false).unwrap();
    let plan = ai.plan_note_delete(&k, "a", &id).unwrap();
    let result = ai.delete_note_bundle(&k, "a", &id, &plan.files).unwrap();
    assert!(result.record_deleted);
    assert_eq!(result.deleted_paths.len(), 3);
    assert!(ai.analysis("a").is_err());
    assert_eq!(k.read(&id, &extra).unwrap().data.text, "用户文件");
    assert_eq!(
        fs::read(root.join("课程.mp4")).unwrap(),
        b"source-read-only"
    );
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn current_summary_version_recovers_after_external_folder_move_and_keeps_edits() {
    let (root, ai, k, id) = fixture();
    let old = ai.generate_note(&k, "a", &id).unwrap();
    ai.with(|r|{r.conn.execute("UPDATE video_analyses SET summary_json=json_set(summary_json,'$.overview','新版概述') WHERE id='a'",[]).unwrap();Ok(())}).unwrap();
    let current = ai.generate_note(&k, "a", &id).unwrap();
    assert_ne!(old.path, current.path);
    let doc = k.read(&id, &current.path).unwrap().data;
    k.write(&id, &current.path, &(doc.text + "\n用户改稿"), &doc.hash)
        .unwrap();
    let base = current.path.rsplit_once('/').unwrap().0;
    let moved = format!("{base}-移动后");
    fs::rename(
        root.join("Vault 中文").join(base),
        root.join("Vault 中文").join(&moved),
    )
    .unwrap();
    let old_base = old.path.rsplit_once('/').unwrap().0;
    let oldest = format!("{}/000-old", old_base.rsplit_once('/').unwrap().0);
    fs::rename(
        root.join("Vault 中文").join(old_base),
        root.join("Vault 中文").join(oldest),
    )
    .unwrap();
    k.reindex(&id).unwrap();
    let reference = ai.analysis_note(&k, "a").unwrap().unwrap();
    assert_eq!(reference.path, format!("{moved}/视频总结.md"));
    assert_eq!(
        ai.analysis("a").unwrap().markdown_path.as_deref(),
        Some(reference.path.as_str())
    );
    ai.with(|r| {
        r.conn
            .execute(
                "UPDATE video_analyses SET markdown_path=?1 WHERE id='a'",
                [&current.path],
            )
            .unwrap();
        Ok(())
    })
    .unwrap();
    let repeated = ai.generate_note(&k, "a", &id).unwrap();
    assert_eq!(repeated.path, reference.path);
    assert!(k
        .read(&id, &repeated.path)
        .unwrap()
        .data
        .text
        .ends_with("用户改稿"));
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn toggling_only_auto_note_reuses_paid_analysis_without_another_queue_entry() {
    let (root, ai, k, _) = fixture();
    crate::ai::settings::store_key(&root, b"sk-fake-cache-test").unwrap();
    ai.with(|r| {
        r.conn
            .execute(
                "UPDATE video_analyses SET prompt_version=?1,template_id='general',template_version=1,template_snapshot=?2 WHERE id='a'",
                rusqlite::params![crate::ai::prompts::TEMPLATE_PROMPT_VERSION,serde_json::to_string(&crate::ai::templates::builtins().remove(0)).unwrap()],
            )
            .unwrap();
        Ok(())
    })
    .unwrap();
    let mut settings = ai.settings().unwrap().settings;
    settings.auto_generate_notes = false;
    ai.save_settings(settings).unwrap();
    ai.runtime.lock().unwrap().active = Some((
        "unrelated".into(),
        Arc::new(crate::ai::jobs::Control::default()),
    ));
    let got = ai
        .start_analysis(AnalysisRequest {
            template_id: None,
            template_version: None,
            reuse_analysis_id: None,
            auto_generate_note: None,
            force: false,
            path: root.join("课程.mp4").to_string_lossy().into(),
            item_id: Some("lesson".into()),
            mode: AnalysisMode::Standard,
            transcribe_audio: true,
            allow_transcript: true,
            allow_images: true,
        })
        .unwrap();
    assert_eq!(got.id, "a");
    assert!(ai.runtime.lock().unwrap().pending.is_empty());
    let restricted = ai
        .start_analysis(AnalysisRequest {
            template_id: None,
            template_version: None,
            reuse_analysis_id: None,
            auto_generate_note: None,
            force: false,
            path: root.join("课程.mp4").to_string_lossy().into(),
            item_id: Some("lesson".into()),
            mode: AnalysisMode::Standard,
            transcribe_audio: true,
            allow_transcript: true,
            allow_images: false,
        })
        .unwrap();
    assert_ne!(restricted.id, "a");
    assert!(!restricted.options.allow_images);
    assert_eq!(ai.runtime.lock().unwrap().pending.len(), 1);
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn auto_note_uses_frozen_single_choice_instead_of_new_task_default() {
    for (choice, default) in [(true, false), (false, true)] {
        let (root, ai, k, id) = fixture();
        let mut settings = ai.settings().unwrap().settings;
        settings.auto_generate_notes = default;
        ai.save_settings(settings).unwrap();
        ai.with(|r|{r.conn.execute("UPDATE video_analyses SET note_target_vault_id=?1,options_json=json_set(options_json,'$.autoGenerateNote',json(?2)) WHERE id='a'",rusqlite::params![id,if choice {"true"}else{"false"}]).unwrap();Ok(())}).unwrap();
        let control = Arc::new(crate::ai::jobs::Control::default());
        ai.runtime.lock().unwrap().active = Some(("a".into(), control.clone()));
        ai.export_auto_note_with(&k, "a", &control).unwrap();
        assert_eq!(ai.analysis("a").unwrap().markdown_path.is_some(), choice);
        ai.shutdown();
        drop(ai);
        drop(k);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn cached_note_export_uses_current_vault_without_rewriting_paid_history_or_usage() {
    let (root, ai, k, id) = fixture();
    ai.with(|r|{r.conn.execute("UPDATE video_analyses SET note_target_vault_id=?1,options_json=json_set(options_json,'$.autoGenerateNote',json('false')) WHERE id='a'",[id]).unwrap();Ok(())}).unwrap();
    let before = ai.analysis("a").unwrap();
    let other = root.join("CurrentVault");
    fs::create_dir(&other).unwrap();
    let current = k
        .set_vault(other.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    ai.export_cached_note_with(&k, "a").unwrap();
    let after = ai.analysis("a").unwrap();
    assert_eq!(after.markdown_vault_id, Some(current));
    assert!(after.markdown_path.is_some());
    assert_eq!(
        serde_json::to_value(before.summary).unwrap(),
        serde_json::to_value(after.summary).unwrap()
    );
    assert_eq!(
        serde_json::to_value(before.options).unwrap(),
        serde_json::to_value(after.options).unwrap()
    );
    assert_eq!(
        ai.with(|r| r
            .conn
            .query_row("SELECT count(*) FROM ai_usage", [], |r| r.get::<_, i64>(0))
            .map_err(crate::learning::db::error))
            .unwrap(),
        0
    );
    let empty_data = root.join("NoVaultSettings");
    fs::create_dir(&empty_data).unwrap();
    let no_vault = KnowledgeService::new(&empty_data);
    ai.export_cached_note_with(&no_vault, "a").unwrap();
    let warning = ai.analysis("a").unwrap();
    assert!(warning.note_warning.unwrap().contains("Vault"));
    assert_eq!(warning.markdown_path, after.markdown_path);
    drop(no_vault);
    assert_eq!(
        fs::read(root.join("课程.mp4")).unwrap(),
        b"source-read-only"
    );
    drop(ai);
    drop(k);
    fs::remove_dir_all(root).unwrap();
}
