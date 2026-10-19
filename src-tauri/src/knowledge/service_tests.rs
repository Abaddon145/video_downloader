use super::{service::KnowledgeService, vault::WriteOutcome};

#[test]
fn pending_edits_default_and_toggle_without_database_state() {
    let (root, service, _) = setup();
    assert!(!service.pending_edits());
    service.set_pending_edits(true);
    assert!(service.pending_edits());
    service.set_pending_edits(false);
    assert!(!service.pending_edits());
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn workspace_write_checkpoints_exact_bytes_before_replacement() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let original = "\u{feff}---\r\ntags: [中文]\r\n---\r\n[[Wiki]]\r\n- [ ] task\r\n```rust\r\nfn main() {}\r\n```\r\n";
    s.create(&id, "映流/history.md", original, false).unwrap();
    let old = s.read(&id, "映流/history.md").unwrap().data;
    s.write(&id, &old.path, "# New", &old.hash).unwrap();
    let db = crate::learning::db::open(&root.join("data/learning.db")).unwrap();
    let saved: (String, String) = db
        .query_row(
            "SELECT text,content_hash FROM knowledge_note_versions WHERE vault_id=?1 AND path=?2",
            rusqlite::params![id, old.path],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(saved, (original.into(), old.hash));
    let current = s.read(&id, &old.path).unwrap().data;
    s.write(&id, &old.path, &current.text, &current.hash)
        .unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM knowledge_note_versions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(db);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn workspace_history_failure_blocks_write_and_conflict_adds_no_version() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&id, "映流/history.md", "original", false).unwrap();
    let old = s.read(&id, "映流/history.md").unwrap().data;
    let db = crate::learning::db::open(&root.join("data/learning.db")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_history BEFORE INSERT ON knowledge_note_versions BEGIN SELECT RAISE(ABORT,'history unavailable'); END;").unwrap();
    assert!(s.write(&id, &old.path, "new", &old.hash).is_err());
    assert_eq!(fs::read(vault.join(&old.path)).unwrap(), b"original");
    db.execute_batch("DROP TRIGGER fail_history;").unwrap();
    assert!(matches!(
        s.write(&id, &old.path, "new", "stale").unwrap().outcome,
        WriteOutcome::Conflict(_)
    ));
    assert_eq!(
        db.query_row("SELECT count(*) FROM knowledge_note_versions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(db);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn workspace_metadata_and_history_survive_index_clear_and_restore_is_guarded() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(
        &id,
        "映流/template.md",
        "---\ntags: [保留]\n---\n[[Wiki]]",
        false,
    )
    .unwrap();
    s.set_note_metadata(&id, "映流/template.md", true, true)
        .unwrap();
    let first = s.read(&id, "映流/template.md").unwrap().data;
    s.write(&id, &first.path, "new", &first.hash).unwrap();
    let version = s
        .note_versions(&id, &first.path, 1)
        .unwrap()
        .data
        .items
        .remove(0);
    let other = root.join("other");
    fs::create_dir(&other).unwrap();
    let other_id = s
        .set_vault(other.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&other_id, &first.path, "other", false).unwrap();
    assert!(
        !s.note_metadata(&other_id, &first.path)
            .unwrap()
            .data
            .favorite
    );
    assert!(s
        .note_versions(&other_id, &first.path, 1)
        .unwrap()
        .data
        .items
        .is_empty());
    assert!(s.write(&id, &first.path, "bad", &first.hash).is_err());
    s.set_vault(vault.to_str().unwrap(), false).unwrap();
    let metadata = s.note_collection(&id, "templates", 1).unwrap().data;
    assert_eq!(metadata.total, 1);
    assert!(
        metadata.items[0].favorite && metadata.items[0].template && metadata.items[0].available
    );
    assert!(s.note_collection(&id, "DROP TABLE", 1).is_err());
    assert!(s.note_version(&id, "映流/else.md", &version.id).is_err());
    assert!(matches!(
        s.restore_note_version(&id, &first.path, &version.id, "stale")
            .unwrap()
            .outcome,
        WriteOutcome::Conflict(_)
    ));
    let current = s.read(&id, &first.path).unwrap().data;
    s.restore_note_version(&id, &first.path, &version.id, &current.hash)
        .unwrap();
    assert_eq!(s.read(&id, &first.path).unwrap().data.text, first.text);
    assert_eq!(s.note_versions(&id, &first.path, 1).unwrap().data.total, 2);
    s.create(&id, "映流/from-template.md", &first.text, false)
        .unwrap();
    assert_eq!(
        s.read(&id, "映流/from-template.md").unwrap().data.text,
        first.text
    );
    assert!(s
        .create(&id, "映流/from-template.md", "overwrite", false)
        .is_err());
    assert!(
        !s.note_metadata(&id, "映流/from-template.md")
            .unwrap()
            .data
            .template
    );
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn workspace_move_migrates_metadata_history_and_attachments_and_compensates_sql_failure() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&id, "映流/folder/n.md", "old", false).unwrap();
    s.set_note_metadata(&id, "映流/folder/n.md", true, true)
        .unwrap();
    let doc = s.read(&id, "映流/folder/n.md").unwrap().data;
    s.write(&id, &doc.path, "new", &doc.hash).unwrap();
    let db = crate::learning::db::open(&root.join("data/learning.db")).unwrap();
    db.execute(
        "INSERT INTO knowledge_attachments VALUES(?1,'映流/folder/a.pdf','a.pdf','hash',4,1)",
        [&id],
    )
    .unwrap();
    db.execute(
        "INSERT INTO knowledge_note_metadata VALUES(?1,'映流/folder-sibling/n.md',1,0)",
        [&id],
    )
    .unwrap();
    db.execute(
        "INSERT INTO knowledge_note_metadata VALUES('other-vault','映流/folder/n.md',1,0)",
        [],
    )
    .unwrap();
    db.execute_batch("CREATE TRIGGER fail_metadata BEFORE UPDATE OF path ON knowledge_note_versions BEGIN SELECT RAISE(ABORT,'history move failed'); END;").unwrap();
    assert!(s.move_note(&id, "映流/folder", "映流/renamed").is_err());
    assert!(vault.join(&doc.path).exists());
    assert!(!vault.join("映流/renamed").exists());
    assert_eq!(
        s.note_collection(&id, "favorites", 1)
            .unwrap()
            .data
            .items
            .iter()
            .filter(|x| x.path == doc.path)
            .count(),
        1
    );
    db.execute_batch("DROP TRIGGER fail_metadata;").unwrap();
    s.move_note(&id, "映流/folder", "映流/renamed").unwrap();
    assert!(
        s.note_metadata(&id, "映流/renamed/n.md")
            .unwrap()
            .data
            .favorite
    );
    assert_eq!(
        s.note_versions(&id, "映流/renamed/n.md", 1)
            .unwrap()
            .data
            .total,
        1
    );
    assert_eq!(
        db.query_row("SELECT path FROM knowledge_attachments", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "映流/renamed/a.pdf"
    );
    assert_eq!(db.query_row("SELECT count(*) FROM knowledge_note_metadata WHERE path='映流/folder-sibling/n.md' OR vault_id='other-vault'",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    drop(db);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}
use std::{fs, path::PathBuf};
pub(super) fn setup() -> (PathBuf, std::sync::Arc<KnowledgeService>, PathBuf) {
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
fn empty_notebook_delete_rejects_nonempty_and_keeps_obsidian_untouched() {
    let (root, service, path) = setup();
    let settings = service.set_vault(path.to_str().unwrap(), false).unwrap();
    let id = settings.vault_id.unwrap();
    service.create(&id, "映流/空笔记本", "", true).unwrap();
    service
        .create(&id, "映流/有内容/笔记.md", "# 不能删除", false)
        .unwrap();
    assert!(service.delete_empty_directory(&id, "映流/有内容").is_err());
    assert_eq!(
        service.read(&id, "映流/有内容/笔记.md").unwrap().data.text,
        "# 不能删除"
    );
    service
        .delete_empty_directory(&id, "映流/空笔记本")
        .unwrap();
    assert!(!path.join("映流/空笔记本").exists());
    assert!(service.delete_empty_directory(&id, ".obsidian").is_err());
    assert_eq!(
        fs::read_to_string(path.join(".obsidian/config.json")).unwrap(),
        "unchanged"
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn notebook_name_sort_is_whitelisted_and_applied_before_pagination() {
    let (root, service, path) = setup();
    let id = service
        .set_vault(path.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    for n in 0..51 {
        fs::write(path.join(format!("映流/笔记{n:02}.md")), "# 内容").unwrap();
    }
    let first = service
        .tree_sorted(&id, "映流", 1, "nameDesc")
        .unwrap()
        .data;
    assert_eq!(first.total, 51);
    assert_eq!(first.items.len(), 50);
    assert_eq!(first.items[0].name, "笔记50.md");
    assert_eq!(
        service
            .tree_sorted(&id, "映流", 2, "nameDesc")
            .unwrap()
            .data
            .items[0]
            .name,
        "笔记00.md"
    );
    assert!(service.tree_sorted(&id, "映流", 1, "anything").is_err());
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

fn linked_note(db: &rusqlite::Connection, vault_id: &str, path: &str) {
    db.execute_batch("INSERT INTO series(id,title,created_at,updated_at) VALUES('series','Series',1,1);
        INSERT INTO series_items(id,series_id,title,created_at,updated_at) VALUES('lesson','series','Lesson',1,1);").unwrap();
    db.execute("INSERT INTO video_analyses(id,item_id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,markdown_vault_id,markdown_path,created_at,updated_at) VALUES('analysis','lesson','source.mp4','fp','completed','standard','test','test',?1,?2,1,1)",rusqlite::params![vault_id,path]).unwrap();
    db.execute("INSERT INTO video_notes(id,item_id,time_ms,content,markdown_vault_id,markdown_path,created_at,updated_at) VALUES('note','lesson',0,'Note',?1,?2,1,1)",rusqlite::params![vault_id,path]).unwrap();
}

#[test]
fn obsidian_settings_are_optional_and_preserve_separate_vault_identity() {
    let old: super::models::KnowledgeSettings = serde_json::from_value(serde_json::json!({"vaultPath":null,"vaultId":null,"rootFolder":"映流","attachmentsFolder":"Attachments"})).unwrap();
    let mut next = serde_json::to_value(old).unwrap();
    next["obsidian"] = serde_json::json!({"yingliuVaultId":"local-root-identity","kind":"vaultId","obsidianVaultIdentifier":"ef6ca3e3b524d22f","registrationConfirmed":true,"confirmedByUser":false});
    let parsed: super::models::KnowledgeSettings = serde_json::from_value(next)
        .expect("new binding must load without replacing the local Vault identity");
    let serialized = serde_json::to_value(parsed).unwrap();
    assert_eq!(serialized["vaultId"], serde_json::Value::Null);
    assert_eq!(
        serialized["obsidian"]["yingliuVaultId"],
        "local-root-identity"
    );
    assert_eq!(
        serialized["obsidian"]["obsidianVaultIdentifier"],
        "ef6ca3e3b524d22f"
    );
}

#[test]
fn move_note_rolls_back_file_and_first_update_when_second_update_fails() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&id, "映流/source.md", "# Original\n[[Other]]\n#tag", false)
        .unwrap();
    let db = crate::learning::db::open(&root.join("data/learning.db")).unwrap();
    linked_note(&db, &id, "映流/source.md");
    db.execute_batch("CREATE TRIGGER fail_note_move BEFORE UPDATE OF markdown_path ON video_notes BEGIN SELECT RAISE(ABORT,'injected second association update failure'); END;").unwrap();
    assert!(
        s.move_note(&id, "映流/source.md", "映流/moved.md").is_err(),
        "association failure must fail the move"
    );
    assert_eq!(
        fs::read_to_string(vault.join("映流/source.md")).unwrap(),
        "# Original\n[[Other]]\n#tag"
    );
    assert!(!vault.join("映流/moved.md").exists());
    for table in ["video_analyses", "video_notes"] {
        let path: String = db
            .query_row(&format!("SELECT markdown_path FROM {table}"), [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(path, "映流/source.md");
    }
    assert_eq!(
        s.search(&id, "Original", 1, None).unwrap().data.items[0]
            .path
            .as_deref(),
        Some("映流/source.md")
    );
    drop(db);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn move_note_restores_file_when_commit_fails() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&id, "映流/source.md", "# Original", false)
        .unwrap();
    let db = crate::learning::db::open(&root.join("data/learning.db")).unwrap();
    linked_note(&db, &id, "映流/source.md");
    db.execute_batch("CREATE TABLE move_commit_fault(parent TEXT REFERENCES series_items(id) DEFERRABLE INITIALLY DEFERRED);
        CREATE TRIGGER fail_move_commit AFTER UPDATE OF markdown_path ON video_notes BEGIN INSERT INTO move_commit_fault VALUES('missing-parent'); END;").unwrap();
    assert!(s.move_note(&id, "映流/source.md", "映流/moved.md").is_err());
    assert!(vault.join("映流/source.md").exists());
    assert!(!vault.join("映流/moved.md").exists());
    for table in ["video_analyses", "video_notes"] {
        let path: String = db
            .query_row(&format!("SELECT markdown_path FROM {table}"), [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(path, "映流/source.md");
    }
    drop(db);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn obsidian_requires_explicit_registration_and_user_confirmation_without_touching_sources() {
    use super::models::{ObsidianKind, ObsidianLaunchStatus};
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let path = "映流/中文 空格 #&%25.md";
    s.create(
        &id,
        path,
        "---\ntags: [keep]\n---\n# Original\n[[Other]]",
        false,
    )
    .unwrap();
    let bytes = fs::read(vault.join(path)).unwrap();
    let config = fs::read(vault.join(".obsidian/config.json")).unwrap();
    assert!(s
        .open_in_obsidian_using(&id, path, |_| panic!("unconfigured must not launch"))
        .is_err());
    let pending = s
        .configure_obsidian(
            &id,
            ObsidianKind::VaultId,
            Some("ef6ca3e3b524d22f".into()),
            false,
        )
        .unwrap()
        .obsidian
        .unwrap();
    assert!(s
        .open_in_obsidian_using(&id, path, |_| panic!("unregistered must not launch"))
        .is_err());
    assert!(s.confirm_obsidian(&id, &pending, path).is_err());
    let binding = s
        .configure_obsidian(
            &id,
            ObsidianKind::VaultId,
            Some("ef6ca3e3b524d22f".into()),
            true,
        )
        .unwrap()
        .obsidian
        .unwrap();
    for rejected in ["missing.md", "../outside.md", ".obsidian/config.md"] {
        assert!(s
            .open_in_obsidian_using(&id, rejected, |_| panic!("invalid file must not launch"))
            .is_err());
    }
    let result = s
        .open_in_obsidian_using(&id, path, |uri| {
            let url = url::Url::parse(uri).unwrap();
            let pairs = url
                .query_pairs()
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(pairs["vault"], "ef6ca3e3b524d22f");
            assert_eq!(pairs["file"], path);
            assert!(!pairs.contains_key("path"));
            Ok(())
        })
        .unwrap();
    assert_eq!(result.status, ObsidianLaunchStatus::LaunchAccepted);
    assert!(!s.settings().unwrap().obsidian.unwrap().confirmed_by_user);
    let mut wrong = binding.clone();
    wrong.obsidian_vault_identifier = Some("1111111111111111".into());
    assert!(s.confirm_obsidian(&id, &wrong, path).is_err());
    assert!(
        s.confirm_obsidian(&id, &binding, path)
            .unwrap()
            .obsidian
            .unwrap()
            .confirmed_by_user
    );
    let failed = s
        .open_in_obsidian_using(&id, path, |_| Err("injected protocol error".into()))
        .unwrap();
    assert_eq!(failed.status, ObsidianLaunchStatus::LaunchFailed);
    assert!(failed.message.contains("injected protocol error"));
    assert!(s
        .confirm_obsidian(&id, &s.settings().unwrap().obsidian.unwrap(), path)
        .is_err());
    assert_eq!(fs::read(vault.join(path)).unwrap(), bytes);
    assert_eq!(
        fs::read(vault.join(".obsidian/config.json")).unwrap(),
        config
    );
    drop(s);
    let reopened = KnowledgeService::new(&root.join("data"));
    assert!(
        reopened
            .settings()
            .unwrap()
            .obsidian
            .unwrap()
            .confirmed_by_user
    );
    drop(reopened);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn obsidian_unreadable_note_is_rejected_before_launch() {
    use super::models::ObsidianKind;
    use std::os::windows::fs::OpenOptionsExt;
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&id, "映流/locked.md", "# Unchanged", false)
        .unwrap();
    s.configure_obsidian(
        &id,
        ObsidianKind::VaultId,
        Some("ef6ca3e3b524d22f".into()),
        true,
    )
    .unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(vault.join("映流/locked.md"))
        .unwrap();
    assert!(s
        .open_in_obsidian_using(&id, "映流/locked.md", |_| panic!(
            "unreadable note must not launch"
        ))
        .is_err());
    drop(held);
    assert_eq!(
        fs::read_to_string(vault.join("映流/locked.md")).unwrap(),
        "# Unchanged"
    );
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn obsidian_binding_becomes_stale_after_vault_switch_or_root_replacement() {
    use super::models::{ObsidianConnectionState, ObsidianKind};
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&id, "映流/note.md", "# Original", false).unwrap();
    s.configure_obsidian(
        &id,
        ObsidianKind::VaultName,
        Some("Explicit name".into()),
        true,
    )
    .unwrap();
    let other = root.join("Other");
    fs::create_dir(&other).unwrap();
    let next = s
        .set_vault(other.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    assert_eq!(
        s.obsidian_connection(&next).unwrap().state,
        ObsidianConnectionState::StaleVault
    );
    assert!(s
        .open_in_obsidian_using(&next, "映流/note.md", |_| panic!(
            "other Vault must not launch"
        ))
        .is_err());
    assert!(s
        .open_in_obsidian_using(&id, "映流/note.md", |_| panic!(
            "stale request must not launch"
        ))
        .is_err());
    s.set_vault(vault.to_str().unwrap(), false).unwrap();
    fs::rename(&vault, root.join("moved-vault")).unwrap();
    fs::create_dir(&vault).unwrap();
    assert_eq!(
        s.obsidian_connection(&id).unwrap().state,
        ObsidianConnectionState::StaleVault
    );
    assert!(s
        .open_in_obsidian_using(&id, "映流/note.md", |_| panic!(
            "replaced root must not launch"
        ))
        .is_err());
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn obsidian_compatibility_path_is_explicit_and_reconfiguration_invalidates_old_confirmation() {
    use super::models::ObsidianKind;
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&id, "映流/note.md", "# Original", false).unwrap();
    assert!(s
        .configure_obsidian(&id, ObsidianKind::VaultId, Some(id.clone()), true)
        .is_err());
    assert!(s
        .configure_obsidian(
            &id,
            ObsidianKind::AbsolutePath,
            Some("unexpected".into()),
            true
        )
        .is_err());
    let old = s
        .configure_obsidian(&id, ObsidianKind::AbsolutePath, None, true)
        .unwrap()
        .obsidian
        .unwrap();
    s.open_in_obsidian_using(&id, "映流/note.md", |uri| {
        let url = url::Url::parse(uri).unwrap();
        let pairs = url
            .query_pairs()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert!(pairs["path"].replace('\\', "/").ends_with("映流/note.md"));
        assert!(!pairs.contains_key("vault"));
        assert!(!pairs.contains_key("file"));
        Ok(())
    })
    .unwrap();
    s.configure_obsidian(
        &id,
        ObsidianKind::VaultId,
        Some("ef6ca3e3b524d22f".into()),
        true,
    )
    .unwrap();
    assert!(s.confirm_obsidian(&id, &old, "映流/note.md").is_err());
    assert!(!s.settings().unwrap().obsidian.unwrap().confirmed_by_user);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn move_directory_updates_descendants_but_preserves_other_vaults_and_prefix_siblings() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    s.create(&id, "映流/folder/note.md", "# Original", false)
        .unwrap();
    let db = crate::learning::db::open(&root.join("data/learning.db")).unwrap();
    linked_note(&db, &id, "映流/folder/note.md");
    db.execute("INSERT INTO video_notes(id,item_id,time_ms,content,markdown_vault_id,markdown_path,created_at,updated_at) VALUES('other','lesson',0,'Other','other-vault','映流/folder/note.md',1,1)",[]).unwrap();
    db.execute("INSERT INTO video_notes(id,item_id,time_ms,content,markdown_vault_id,markdown_path,created_at,updated_at) VALUES('sibling','lesson',0,'Sibling',?1,'映流/folder-sibling/note.md',1,1)",[&id]).unwrap();
    s.move_note(&id, "映流/folder", "映流/renamed").unwrap();
    assert!(vault.join("映流/renamed/note.md").exists());
    for (key, expected) in [
        ("note", "映流/renamed/note.md"),
        ("other", "映流/folder/note.md"),
        ("sibling", "映流/folder-sibling/note.md"),
    ] {
        let path: String = db
            .query_row(
                "SELECT markdown_path FROM video_notes WHERE id=?1",
                [key],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(path, expected);
    }
    let path: String = db
        .query_row(
            "SELECT markdown_path FROM video_analyses WHERE id='analysis'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(path, "映流/renamed/note.md");
    drop(db);
    drop(s);
    fs::remove_dir_all(root).unwrap();
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
