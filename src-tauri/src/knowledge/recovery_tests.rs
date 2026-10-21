use super::service_tests::setup;
use std::fs;

#[test]
fn guarded_flags_preserve_other_metadata_and_reject_stale_undo() {
    let (root, service, vault) = setup();
    let id = service
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    service
        .create(&id, "映流/flags.md", "# source", false)
        .unwrap();
    service
        .set_note_flag(&id, "映流/flags.md", "favorite", false, true)
        .unwrap();
    service
        .set_note_flag(&id, "映流/flags.md", "template", false, true)
        .unwrap();
    service
        .set_note_flag(&id, "映流/flags.md", "favorite", true, false)
        .unwrap();
    assert!(service
        .set_note_flag(&id, "映流/flags.md", "favorite", true, false)
        .is_err());
    let m = service.note_metadata(&id, "映流/flags.md").unwrap().data;
    assert!(!m.favorite && m.template);
    assert!(service
        .set_note_flag(
            &id,
            "映流/flags.md",
            "favorite=1;DROP TABLE series",
            false,
            true
        )
        .is_err());
    assert_eq!(
        fs::read_to_string(vault.join("映流/flags.md")).unwrap(),
        "# source"
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn trash_restore_preserves_exact_bytes_metadata_and_index() {
    let (root, service, vault) = setup();
    let id = service
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let path = "映流/recover.md";
    let bytes = b"\xef\xbb\xbf# recovery\r\nline\r\n";
    fs::write(vault.join(path), bytes).unwrap();
    service.reindex(&id).unwrap();
    service.set_note_metadata(&id, path, true, false).unwrap();
    let hash = service.read(&id, path).unwrap().data.hash;
    let receipt = service.trash_note(&id, path, &hash).unwrap();
    assert_eq!(receipt.hash, hash);
    assert!(service
        .restore_trashed_note(&id, &receipt.trash_path, "映流/other.md", &hash)
        .is_err());
    assert!(!vault.join(path).exists());
    assert_eq!(fs::read(vault.join(&receipt.trash_path)).unwrap(), bytes);
    assert_eq!(
        service.search(&id, "recovery", 1, None).unwrap().data.total,
        0
    );
    service
        .restore_trashed_note(&id, &receipt.trash_path, path, &hash)
        .unwrap();
    assert_eq!(fs::read(vault.join(path)).unwrap(), bytes);
    assert!(!vault.join(&receipt.trash_path).exists());
    assert_eq!(
        service.search(&id, "recovery", 1, None).unwrap().data.total,
        1
    );
    assert!(service.note_metadata(&id, path).unwrap().data.favorite);
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restore_rejects_conflicts_tampering_and_unsafe_paths_without_losing_trash() {
    let (root, service, vault) = setup();
    let id = service
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    let path = "映流/recover.md";
    let doc = service.create(&id, path, "original", false).unwrap();
    let hash = service.read(&id, &doc.path).unwrap().data.hash;
    let receipt = service.trash_note(&id, path, &hash).unwrap();
    assert!(service
        .restore_trashed_note("stale", &receipt.trash_path, path, &hash)
        .is_err());
    for unsafe_path in [
        "../recover.md",
        "C:/outside.md",
        "//server/share.md",
        ".obsidian/config.md",
        ".yingliu-trash/../recover.md",
        ".yingliu-trash/nested/file.md",
    ] {
        assert!(
            service
                .restore_trashed_note(&id, unsafe_path, path, &hash)
                .is_err(),
            "{unsafe_path}"
        );
    }
    assert!(service
        .restore_trashed_note(&id, &receipt.trash_path, "../outside.md", &hash)
        .is_err());
    assert!(service
        .restore_trashed_note(&id, &receipt.trash_path, path, "stale")
        .is_err());
    service.create(&id, path, "new occupant", false).unwrap();
    assert!(service
        .restore_trashed_note(&id, &receipt.trash_path, path, &hash)
        .is_err());
    assert_eq!(
        fs::read_to_string(vault.join(path)).unwrap(),
        "new occupant"
    );
    assert_eq!(
        fs::read_to_string(vault.join(&receipt.trash_path)).unwrap(),
        "original"
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn guarded_move_rejects_stale_hash_without_moving_file() {
    let (root, service, vault) = setup();
    let id = service
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    service.create(&id, "映流/from.md", "first", false).unwrap();
    assert!(service
        .move_note_guarded(&id, "映流/from.md", "映流/to.md", "stale")
        .is_err());
    assert_eq!(
        fs::read_to_string(vault.join("映流/from.md")).unwrap(),
        "first"
    );
    assert!(!vault.join("映流/to.md").exists());
    let hash = service.read(&id, "映流/from.md").unwrap().data.hash;
    service
        .move_note_guarded(&id, "映流/from.md", "映流/to.md", &hash)
        .unwrap();
    assert_eq!(
        fs::read_to_string(vault.join("映流/to.md")).unwrap(),
        "first"
    );
    drop(service);
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn note_trash_rejects_directories_before_mutation() {
    let (root, service, vault) = setup();
    let id = service
        .set_vault(vault.to_str().unwrap(), false)
        .unwrap()
        .vault_id
        .unwrap();
    service.create(&id, "映流/folder.md", "", true).unwrap();
    assert!(service.trash_note(&id, "映流/folder.md", "").is_err());
    assert!(vault.join("映流/folder.md").is_dir());
    drop(service);
    fs::remove_dir_all(root).unwrap();
}
