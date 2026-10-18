use super::vault::{Vault, WriteOutcome};
use std::{fs, path::PathBuf};
fn fixture() -> (PathBuf, Vault) {
    let root = std::env::temp_dir().join(format!(
        "映流 Vault #&（）한국어-{}",
        crate::ffmpeg::runner::new_id()
    ));
    fs::create_dir(&root).unwrap();
    let vault = Vault::open(&root).unwrap();
    (root, vault)
}
#[test]
fn vault_plain_markdown_conflicts_no_overwrite_rename_trash_and_settings_are_protected() {
    let (root, vault) = fixture();
    fs::create_dir(root.join(".obsidian")).unwrap();
    fs::write(root.join(".obsidian/config.json"), b"UNCHANGED").unwrap();
    vault.create_dir("课程 #&（一）").unwrap();
    let path = "课程 #&（一）/笔记.md";
    let first = vault
        .create_file(path, "---\ntitle: 中文\n---\n# 学习\n[[Other]]")
        .unwrap();
    assert_eq!(vault.read(path).unwrap().text, first.text);
    assert!(vault.create_file(path, "must not overwrite").is_err());
    fs::write(root.join(path), "# Obsidian外部编辑").unwrap();
    match vault.write(path, "# 映流编辑", &first.hash).unwrap() {
        WriteOutcome::Conflict(external) => assert_eq!(external.text, "# Obsidian外部编辑"),
        _ => panic!("must report conflict"),
    }
    assert_eq!(
        fs::read_to_string(root.join(path)).unwrap(),
        "# Obsidian外部编辑"
    );
    let external = vault.read(path).unwrap();
    let saved = match vault.write(path, "# 合并后的笔记", &external.hash).unwrap() {
        WriteOutcome::Saved(v) => v,
        _ => panic!("must save matching hash"),
    };
    vault.create_file("existing.md", "# Preserve").unwrap();
    assert!(vault.move_entry(path, "existing.md").is_err());
    vault.move_entry(path, "课程 #&（一）/新名称.md").unwrap();
    assert_eq!(
        vault.read("课程 #&（一）/新名称.md").unwrap().hash,
        saved.hash
    );
    let trash = vault.trash("课程 #&（一）/新名称.md", &saved.hash).unwrap();
    assert!(!root.join("课程 #&（一）/新名称.md").exists());
    assert_eq!(fs::read_to_string(root.join(trash)).unwrap(), saved.text);
    assert_eq!(
        fs::read(root.join(".obsidian/config.json")).unwrap(),
        b"UNCHANGED"
    );
    assert!(vault
        .list("", 1)
        .unwrap()
        .items
        .iter()
        .all(|i| !i.path.starts_with('.')));
    assert!(vault
        .create_file("large.md", &"x".repeat(5 * 1024 * 1024 + 1))
        .is_err());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn vault_blocks_traversal_absolute_unc_ads_device_and_protected_paths() {
    let (root, vault) = fixture();
    for path in [
        "../outside.md",
        r"..\..\Windows\system.ini",
        r"C:\Windows\win.ini",
        r"\\server\share\file.md",
        "/Windows/file.md",
        "a/../../x.md",
        "a//b.md",
        "a/./b.md",
        "note.md:secret",
        "CON.md",
        "PRN.txt",
        "AUX.md",
        "NUL.md",
        "COM1.md",
        "LPT9.md",
        "COM¹.md",
        "COM².md",
        "LPT³.md",
        "a./b.md",
        "a /b.md",
        ".obsidian/config.json",
        ".git/config",
        "node_modules/x.md",
        ".yingliu-trash/x.md",
    ] {
        assert!(vault.create_file(path, "test").is_err(), "allowed {path}");
        assert!(vault.read(path).is_err(), "read {path}");
    }
    assert!(vault.create_file("safe.md", "safe").is_ok());
    assert!(vault.move_entry("safe.md", "../escaped.md").is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn vault_bounds_external_markdown_reads_and_rejects_hardlinks_and_non_utf8() {
    let (root, vault) = fixture();
    fs::write(root.join("maximum.md"), vec![b'x'; 5 * 1024 * 1024]).unwrap();
    assert_eq!(
        vault.read("maximum.md").unwrap().text.len(),
        5 * 1024 * 1024
    );
    fs::write(root.join("maximum.md"), vec![b'x'; 5 * 1024 * 1024 + 1]).unwrap();
    assert!(vault.read("maximum.md").is_err());
    fs::write(root.join("invalid.md"), [0xff, 0xfe]).unwrap();
    assert!(vault.read("invalid.md").is_err());
    let outside = root.with_extension("private.md");
    fs::write(&outside, "PRIVATE ORIGINAL").unwrap();
    fs::hard_link(&outside, root.join("alias.md")).unwrap();
    assert!(vault.read("alias.md").is_err());
    assert!(vault.write("alias.md", "bad", "unknown").is_err());
    assert!(vault.move_entry("alias.md", "moved.md").is_err());
    assert!(vault.trash("alias.md", "unknown").is_err());
    assert!(vault
        .list("", 1)
        .unwrap()
        .items
        .iter()
        .all(|entry| entry.name != "alias.md"));
    assert_eq!(fs::read_to_string(&outside).unwrap(), "PRIVATE ORIGINAL");
    fs::remove_dir_all(root).unwrap();
    fs::remove_file(outside).unwrap();
}

#[test]
fn vault_rejects_hidden_files_and_svg_and_bounds_raster_attachments() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NORMAL,
    };
    let (root, vault) = fixture();
    let original = vault.create_file("hidden.md", "unchanged").unwrap();
    let name: Vec<u16> = root
        .join("hidden.md")
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    assert_ne!(
        unsafe { SetFileAttributesW(name.as_ptr(), FILE_ATTRIBUTE_HIDDEN) },
        0
    );
    assert!(vault.read("hidden.md").is_err());
    assert!(vault.write("hidden.md", "bad", &original.hash).is_err());
    assert!(vault.trash("hidden.md", &original.hash).is_err());
    assert_ne!(
        unsafe { SetFileAttributesW(name.as_ptr(), FILE_ATTRIBUTE_NORMAL) },
        0
    );
    assert_eq!(
        fs::read_to_string(root.join("hidden.md")).unwrap(),
        "unchanged"
    );
    fs::write(root.join("image.png"), b"raster attachment").unwrap();
    assert_eq!(
        vault.read_bytes("image.png", 100).unwrap(),
        b"raster attachment"
    );
    assert!(vault.read_bytes("image.png", 5).is_err());
    fs::write(root.join("unsafe.svg"), b"<svg onload='bad'/>").unwrap();
    assert!(vault.read_bytes("unsafe.svg", 100).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn vault_concurrent_creates_never_overwrite_the_winner() {
    use std::sync::{Arc, Barrier};
    let (root, vault) = fixture();
    drop(vault);
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|i| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let vault = Vault::open(&root).unwrap();
                barrier.wait();
                let text = format!("writer {i}");
                vault
                    .create_file("winner.md", &text)
                    .ok()
                    .map(|document| document.text)
            })
        })
        .collect();
    let winners: Vec<_> = workers
        .into_iter()
        .filter_map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(winners.len(), 1);
    assert_eq!(
        fs::read_to_string(root.join("winner.md")).unwrap(),
        winners[0]
    );
    assert!(fs::read_dir(&root).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".yingliu-new-")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn vault_preflights_directory_moves_and_internal_trash_cannot_follow_a_junction() {
    use std::os::windows::process::CommandExt;
    let (root, vault) = fixture();
    vault.create_dir("folder").unwrap();
    fs::create_dir(root.join("folder/.obsidian")).unwrap();
    fs::write(root.join("folder/.obsidian/config.json"), b"preserve").unwrap();
    assert!(vault.move_entry("folder", "moved").is_err());
    assert!(vault.trash("folder", "").is_err());
    fs::remove_dir_all(root.join("folder/.obsidian")).unwrap();
    fs::write(root.join("folder/source.mp4"), b"video").unwrap();
    assert!(vault.move_entry("folder", "moved").is_err());
    assert!(vault.trash("folder", "").is_err());
    assert_eq!(fs::read(root.join("folder/source.mp4")).unwrap(), b"video");
    let outside = root.with_extension("outside");
    fs::create_dir(&outside).unwrap();
    let status = std::process::Command::new("cmd.exe")
        .args(["/c", "mklink", "/J"])
        .arg(root.join(".yingliu-trash"))
        .arg(&outside)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(status.status.success());
    let note = vault.create_file("safe.md", "keep me").unwrap();
    assert!(vault.trash("safe.md", &note.hash).is_err());
    assert_eq!(vault.read("safe.md").unwrap().text, "keep me");
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    fs::remove_dir(root.join(".yingliu-trash")).unwrap();
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn vault_lists_pages_and_folder_trash_and_no_overwrite_moves() {
    let (root, vault) = fixture();
    vault.create_dir("folder").unwrap();
    vault.create_file("folder/note.md", "content").unwrap();
    for index in 0..51 {
        vault
            .create_file(&format!("note-{index:02}.md"), "page")
            .unwrap();
    }
    let first = vault.list("", 1).unwrap();
    let second = vault.list("", 2).unwrap();
    assert_eq!(first.total, 52);
    assert_eq!(first.items.len(), 50);
    assert_eq!(second.items.len(), 2);
    assert!(first.items[0].directory);
    assert!(vault.move_entry("note-00.md", "note-01.md").is_err());
    assert_eq!(vault.read("note-00.md").unwrap().text, "page");
    let trash = vault.trash("folder", "").unwrap();
    assert_eq!(
        fs::read_to_string(root.join(trash).join("note.md")).unwrap(),
        "content"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn vault_supports_paths_longer_than_260_utf16_units() {
    use std::os::windows::ffi::OsStrExt;
    let (root, vault) = fixture();
    let mut directory = String::new();
    for level in 0..4 {
        if !directory.is_empty() {
            directory.push('/');
        }
        directory.push_str(&format!("第{level}层{}", "中文目录한국어".repeat(10)));
        vault.create_dir(&directory).unwrap();
    }
    let path = format!("{directory}/长路径笔记.md");
    assert!(root.join(&path).as_os_str().encode_wide().count() > 260);
    let first = vault.create_file(&path, "# Long Unicode path").unwrap();
    assert_eq!(vault.read(&path).unwrap().hash, first.hash);
    let updated = match vault.write(&path, "updated", &first.hash).unwrap() {
        WriteOutcome::Saved(d) => d,
        _ => panic!("matching hash must save"),
    };
    let moved = format!("{directory}/新名称.md");
    vault.move_entry(&path, &moved).unwrap();
    let trash = vault.trash(&moved, &updated.hash).unwrap();
    assert_eq!(fs::read_to_string(root.join(trash)).unwrap(), "updated");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn vault_returns_actual_long_path_case_and_reports_unreadable_listing_entries() {
    use std::os::windows::fs::OpenOptionsExt;
    let (root, vault) = fixture();
    vault.create_file("note.md", "unchanged").unwrap();
    assert_eq!(vault.read("NOTE.MD").unwrap().path, "note.md");
    let locked = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(root.join("note.md"))
        .unwrap();
    assert!(root.join("note.md").exists());
    let page = vault.list("", 1).unwrap();
    assert!(
        page.skipped > 0,
        "a locked safe-name entry must prevent complete index pruning"
    );
    assert!(page.items.iter().all(|entry| entry.name != "note.md"));
    drop(locked);
    assert_eq!(vault.list("", 1).unwrap().skipped, 0);
    assert_eq!(vault.read("note.md").unwrap().text, "unchanged");
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires enabled volume 8.3 name generation; run explicitly after probing the volume"]
fn vault_rejects_protected_directory_opened_through_8dot3_alias() {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
    let (root, vault) = fixture();
    let protected = root.join(".obsidian");
    fs::create_dir(&protected).unwrap();
    fs::write(protected.join("private.md"), "CONFIGURATION").unwrap();
    let source: Vec<u16> = protected.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut destination = vec![0u16; 32768];
    let length = unsafe {
        GetShortPathNameW(
            source.as_ptr(),
            destination.as_mut_ptr(),
            destination.len() as u32,
        )
    };
    assert!(length > 0 && length < destination.len() as u32);
    let shortened = PathBuf::from(std::ffi::OsString::from_wide(
        &destination[..length as usize],
    ));
    let alias = shortened.file_name().unwrap().to_str().unwrap();
    assert!(
        !alias.eq_ignore_ascii_case(".obsidian"),
        "8.3 names are disabled; this test must remain explicitly ignored on that volume"
    );
    assert!(vault.read(&format!("{alias}/private.md")).is_err());
    assert!(vault
        .create_file(&format!("{alias}/new.md"), "bad")
        .is_err());
    assert!(vault.move_entry(alias, "renamed").is_err());
    assert_eq!(
        fs::read_to_string(protected.join("private.md")).unwrap(),
        "CONFIGURATION"
    );
    assert!(!protected.join("new.md").exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn vault_rejects_junction_escape_and_replaced_root_identity() {
    use std::os::windows::process::CommandExt;
    let (root, vault) = fixture();
    let outside = root.with_extension("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("private.md"), "DO NOT READ OR CHANGE").unwrap();
    let status = std::process::Command::new("cmd.exe")
        .args(["/c", "mklink", "/J"])
        .arg(root.join("escape"))
        .arg(&outside)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "junction setup failed: {:?}",
        status
    );
    assert!(vault.read("escape/private.md").is_err());
    assert!(vault.create_file("escape/new.md", "bad").is_err());
    assert!(vault.move_entry("escape", "moved").is_err());
    assert!(vault.trash("escape/private.md", "unknown").is_err());
    assert_eq!(
        fs::read_to_string(outside.join("private.md")).unwrap(),
        "DO NOT READ OR CHANGE"
    );
    fs::remove_dir(root.join("escape")).unwrap();
    let old_id = vault.id.clone();
    let retired = root.with_extension("retired");
    fs::rename(&root, &retired).unwrap();
    fs::create_dir(&root).unwrap();
    assert_ne!(Vault::open(&root).unwrap().id, old_id);
    assert!(vault.read("private.md").is_err());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(retired).unwrap();
    fs::remove_dir_all(outside).unwrap();
}
