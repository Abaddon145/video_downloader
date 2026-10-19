use super::{service_tests::setup, vault::Vault};
use std::fs;

#[test]
fn configured_attachment_directory_cannot_move_and_imports_stay_manageable() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let attachment = s
        .import_attachment(&id, "image.png", b"image")
        .unwrap()
        .data;
    let failure = s
        .move_note(&id, "Attachments", "new-parent/Archive")
        .err()
        .unwrap();
    assert!(failure.contains("附件目录"), "{failure}");
    assert!(!vault.join("new-parent").exists());
    assert_eq!(s.attachment_bytes(&id, &attachment.path).unwrap(), b"image");
    assert_eq!(s.attachments(&id, 1).unwrap().data.items[0].state, "ready");
    s.create(&id, "映流/n.md", "note", false).unwrap();
    s.move_note(&id, "映流/n.md", "映流/renamed.md").unwrap();
    assert_eq!(s.read(&id, "映流/renamed.md").unwrap().data.text, "note");
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_attachment_source_is_a_bounded_regular_file() {
    let (root, s, _) = setup();
    let source = root.join("中文 空格.pdf");
    fs::write(&source, b"").unwrap();
    assert_eq!(
        super::attachments::read_selected_attachment(&source).unwrap(),
        ("中文 空格.pdf".into(), Vec::new())
    );
    assert!(super::attachments::read_selected_attachment(&root).is_err());
    fs::write(&source, vec![0; 20 * 1024 * 1024 + 1]).unwrap();
    assert!(super::attachments::read_selected_attachment(&source).is_err());
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn attachment_import_is_bounded_create_new_and_vault_scoped() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let first = s
        .import_attachment(&id, "中文 空格 #.pdf", b"%PDF-fixture")
        .unwrap()
        .data;
    let second = s
        .import_attachment(&id, "中文 空格 #.pdf", b"other")
        .unwrap()
        .data;
    assert_ne!(first.path, second.path);
    assert!(first.path.starts_with("Attachments/"));
    assert_eq!(first.name, "中文 空格 #.pdf");
    assert_eq!(
        s.attachment_bytes(&id, &first.path).unwrap(),
        b"%PDF-fixture"
    );
    assert_eq!(s.attachments(&id, 1).unwrap().data.total, 2);
    assert!(s.attachment_bytes(&id, ".obsidian/config.json").is_err());
    assert!(s.attachment_bytes(&id, "Attachments/../映流/n.md").is_err());
    assert!(s
        .import_attachment(&id, "large.bin", &vec![0; 20 * 1024 * 1024 + 1])
        .is_err());
    assert!(s
        .import_attachment(&id, "large.png", &vec![0; 8 * 1024 * 1024 + 1])
        .is_err());
    assert!(s.import_attachment(&id, "empty.png", b"").is_err());
    let empty = s.import_attachment(&id, "empty.txt", b"").unwrap().data;
    assert_eq!(empty.bytes, 0);
    let other = root.join("other");
    fs::create_dir(&other).unwrap();
    s.set_vault(other.to_str().unwrap(), false).unwrap();
    assert!(s.attachment_bytes(&id, &first.path).is_err());
    assert_eq!(fs::read(vault.join(&first.path)).unwrap(), b"%PDF-fixture");
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn attachment_database_failure_compensates_and_delete_checks_actual_hash() {
    let (root, s, vault) = setup();
    let id = s
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let db = crate::learning::db::open(&root.join("data/learning.db")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_attachment BEFORE INSERT ON knowledge_attachments BEGIN SELECT RAISE(ABORT,'injected attachment failure'); END;").unwrap();
    assert!(s.import_attachment(&id, "a.pdf", b"content").is_err());
    assert_eq!(fs::read_dir(vault.join("Attachments")).unwrap().count(), 0);
    db.execute_batch("DROP TRIGGER fail_attachment;").unwrap();
    let file = s.import_attachment(&id, "a.pdf", b"content").unwrap().data;
    fs::write(vault.join(&file.path), b"changed").unwrap();
    assert!(s.delete_attachment(&id, &file.path, &file.hash).is_err());
    let changed = s.attachments(&id, 1).unwrap().data.items.remove(0);
    assert_eq!(changed.state, "changed");
    assert_ne!(changed.hash, file.hash);
    s.delete_attachment(&id, &changed.path, &changed.hash)
        .unwrap();
    assert!(!vault.join(&changed.path).exists());
    assert_eq!(s.attachments(&id, 1).unwrap().data.total, 0);
    s.create(&id, "映流/n.md", "note", false).unwrap();
    let note = s.read(&id, "映流/n.md").unwrap().data;
    assert!(s.delete_attachment(&id, &note.path, &note.hash).is_err());
    db.execute(
        "INSERT INTO knowledge_attachments VALUES(?1,?2,'n.md',?3,4,1)",
        rusqlite::params![id, note.path, note.hash],
    )
    .unwrap();
    assert!(s.attachment_bytes(&id, &note.path).is_err());
    assert!(s.delete_attachment(&id, &note.path, &note.hash).is_err());
    assert_eq!(fs::read_to_string(vault.join(&note.path)).unwrap(), "note");
    drop(db);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn attachment_vault_rejects_escape_overwrite_and_exact_size_limits() {
    let (root, s, path) = setup();
    let v = Vault::open(&path).unwrap();
    v.create_dir("Attachments").unwrap();
    assert!(v
        .create_attachment("Attachments/../../outside.pdf", b"x")
        .is_err());
    let image = vec![1; 8 * 1024 * 1024];
    v.create_attachment("Attachments/boundary.png", &image)
        .unwrap();
    assert!(v
        .create_attachment("Attachments/boundary.png", b"overwrite")
        .is_err());
    assert_eq!(
        v.read_attachment("Attachments/boundary.png").unwrap(),
        image
    );
    let binary = vec![2; 20 * 1024 * 1024];
    v.create_attachment("Attachments/boundary.bin", &binary)
        .unwrap();
    assert_eq!(
        v.read_attachment("Attachments/boundary.bin").unwrap(),
        binary
    );
    assert!(v.trash_attachment("Attachments", "").is_err());
    drop(v);
    drop(s);
    fs::remove_dir_all(root).unwrap();
}
