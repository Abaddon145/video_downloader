use super::{service::KnowledgeService, vault::WriteOutcome};
use std::{fs, path::PathBuf};
fn setup() -> (PathBuf, std::sync::Arc<KnowledgeService>, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "knowledge-service-{}",
        crate::ffmpeg::runner::new_id()
    ));
    fs::create_dir(&root).unwrap();
    let vault = root.join("用户 中文 Vault");
    fs::create_dir(&vault).unwrap();
    fs::create_dir(vault.join(".obsidian")).unwrap();
    fs::write(vault.join(".obsidian/config.json"), "unchanged").unwrap();
    let data = root.join("data");
    fs::create_dir(&data).unwrap();
    let service = KnowledgeService::new(&data);
    (root, service, vault)
}
#[test]
fn knowledge_service_keeps_files_as_truth_handles_conflicts_and_isolates_vault_switch() {
    let (root, s, vault) = setup();
    let settings = s.set_vault(vault.to_str().unwrap(), false).unwrap();
    let id = settings.vault_id.unwrap();
    let created = s
        .create(
            &id,
            "映流/学习.md",
            "---\ntags: [UE5]\n---\n# 学习\n[[Other]]",
            false,
        )
        .unwrap();
    assert_eq!(created.path, "映流/学习.md");
    let first = s.read(&id, &created.path).unwrap().data;
    fs::write(vault.join(&created.path), "# 外部版本").unwrap();
    let conflict = s
        .write(&id, &created.path, "# 我的未保存文字", &first.hash)
        .unwrap();
    assert!(matches!(conflict.outcome, WriteOutcome::Conflict(_)));
    assert_eq!(
        fs::read_to_string(vault.join(&created.path)).unwrap(),
        "# 外部版本"
    );
    let external = s.read(&id, &created.path).unwrap().data;
    let saved = s
        .write(&id, &created.path, "# 合并\n#Niagara", &external.hash)
        .unwrap();
    assert!(matches!(saved.outcome, WriteOutcome::Saved(_)));
    assert!(s.tags(&id).unwrap().data.contains(&"Niagara".into()));
    s.move_note(&id, &created.path, "映流/新名称.md").unwrap();
    assert_eq!(s.search(&id, "合并", 1, None).unwrap().data.total, 1);
    let doc = s.read(&id, "映流/新名称.md").unwrap().data;
    s.delete(&id, "映流/新名称.md", &doc.hash).unwrap();
    assert_eq!(s.search(&id, "合并", 1, None).unwrap().data.total, 0);
    assert!(!vault.join("映流/新名称.md").exists());
    s.create(&id, "映流/保留.md", "# 保留", false).unwrap();
    let other = root.join("second");
    fs::create_dir(&other).unwrap();
    let changed = s.set_vault(other.to_str().unwrap(), false).unwrap();
    let next = changed.vault_id.unwrap();
    assert_ne!(next, id);
    assert!(s.read(&id, "映流/保留.md").is_err());
    assert!(s.write(&id, "映流/保留.md", "bad", "x").is_err());
    assert_eq!(s.search(&next, "保留", 1, None).unwrap().data.total, 0);
    assert_eq!(
        fs::read_to_string(vault.join("映流/保留.md")).unwrap(),
        "# 保留"
    );
    assert_eq!(
        fs::read_to_string(vault.join(".obsidian/config.json")).unwrap(),
        "unchanged"
    );
    drop(s);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn knowledge_save_reports_index_failure_after_file_was_safely_saved() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let first = s.create(&id, "映流/note.md", "# First", false).unwrap();
    let d = s.read(&id, &first.path).unwrap().data;
    let db = crate::learning::db::open(&root.join("data/learning.db")).unwrap();
    db.execute_batch("DROP TABLE knowledge_links").unwrap();
    let saved = s
        .write(&id, &first.path, "# Actually saved", &d.hash)
        .unwrap();
    assert!(matches!(saved.outcome, WriteOutcome::Saved(_)));
    assert!(saved.index_warning.is_some());
    assert_eq!(
        fs::read_to_string(vault.join(&first.path)).unwrap(),
        "# Actually saved"
    );
    drop(db);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn knowledge_external_scan_prunes_only_after_complete_success() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    fs::write(vault.join("external.md"), "# 外部笔记\n#tag").unwrap();
    assert!(s.reindex(&id).unwrap().data.complete);
    assert_eq!(s.search(&id, "外部笔记", 1, None).unwrap().data.total, 1);
    fs::remove_file(vault.join("external.md")).unwrap();
    assert!(s.reindex(&id).unwrap().data.complete);
    assert_eq!(s.search(&id, "外部笔记", 1, None).unwrap().data.total, 0);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn incomplete_scan_does_not_prune_previous_notes_when_a_file_is_temporarily_unreadable() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&id, "映流/keep.md", "# Keep this cached note", false)
        .unwrap();
    fs::write(vault.join("映流/keep.md"), vec![0xff; 5 * 1024 * 1024 + 1]).unwrap();
    let scan = s.reindex(&id).unwrap().data;
    assert!(!scan.complete);
    assert!(scan.skipped > 0);
    assert!(!scan.warnings.is_empty());
    assert_eq!(
        s.search(&id, "Keep this cached", 1, None)
            .unwrap()
            .data
            .total,
        1
    );
    assert_eq!(
        fs::metadata(vault.join("映流/keep.md")).unwrap().len(),
        5 * 1024 * 1024 + 1
    );
    drop(s);
    fs::remove_dir_all(root).unwrap();
}
