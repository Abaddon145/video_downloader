use super::db;
use rusqlite::Connection;
use std::path::Path;
#[test]
fn learning_database_creates_versioned_tables_and_enforces_relations() {
    let c = db::open(Path::new(":memory:")).unwrap();
    let version: i64 = c
        .query_row("SELECT version FROM schema_version", [], |r| r.get(0))
        .expect("schema must exist");
    assert_eq!(version, 4);
    for table in [
        "categories",
        "series",
        "series_items",
        "playback_progress",
        "video_notes",
        "learning_settings",
    ] {
        assert_eq!(
            c.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }
    assert!(c.execute("INSERT INTO series_items(id,series_id,title,created_at,updated_at) VALUES('a','missing','A',0,0)",[]).is_err());
}
#[test]
fn learning_database_migrates_v1_preserving_data_and_rejects_future_schema() {
    let dir = std::env::temp_dir().join(format!(
        "learning-migration-{}",
        crate::ffmpeg::runner::new_id()
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("learning.db");
    {
        let c = Connection::open(&path).unwrap();
        c.execute_batch(include_str!("schema-v1.sql")).unwrap();
        c.execute(
            "INSERT INTO series(id,title,created_at,updated_at) VALUES('s','保留',0,0)",
            [],
        )
        .unwrap();
    }
    {
        let c = db::open(&path).unwrap();
        assert_eq!(
            c.query_row("SELECT title FROM series WHERE id='s'", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "保留"
        );
        assert!(c
            .execute("UPDATE series SET favorite=1,sort_order=2", [])
            .is_ok());
        c.execute("UPDATE schema_version SET version=999", [])
            .unwrap();
    }
    assert!(db::open(&path).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

use super::{models::*, repository::Repository};
fn repository() -> Repository {
    Repository::open(Path::new(":memory:")).unwrap()
}
fn category(name: &str) -> CategoryInput {
    CategoryInput {
        name: name.into(),
        ..Default::default()
    }
}
fn course(title: &str, category_id: Option<String>) -> SeriesInput {
    SeriesInput {
        title: title.into(),
        category_id,
        ..Default::default()
    }
}
#[test]
fn learning_catalog_edits_and_deletes_categories_without_losing_series() {
    let r = repository();
    let c = r.save_category(&category("UE5")).unwrap();
    let s = r.save_series(&course("教程", Some(c.clone()))).unwrap();
    let mut edit = category("Unreal");
    edit.id = Some(c.clone());
    r.save_category(&edit).unwrap();
    assert_eq!(r.categories().unwrap()[0].name, "Unreal");
    assert!(r.delete_category(&c, false).is_err());
    assert_eq!(r.series(&s, 1).unwrap().series.category_id, Some(c.clone()));
    r.delete_category(&c, true).unwrap();
    assert_eq!(r.series(&s, 1).unwrap().series.category_id, None);
    assert!(r.save_category(&category("")).is_err());
    assert!(r
        .save_category(&CategoryInput {
            color: Some("url(file)".into()),
            ..category("A")
        })
        .is_err());
}
#[test]
fn learning_catalog_searches_all_content_and_counts_live_progress_with_paging() {
    let r = repository();
    let c = r.save_category(&category("程序设计")).unwrap();
    let s = r.save_series(&course("课程", Some(c.clone()))).unwrap();
    r.conn.execute("INSERT INTO series_items(id,series_id,title,created_at,updated_at) VALUES('i',?1,'材质',0,0)",[&s]).unwrap();
    r.conn
        .execute(
            "INSERT INTO playback_progress(item_id,completed,updated_at) VALUES('i',1,0)",
            [],
        )
        .unwrap();
    r.conn.execute("INSERT INTO video_notes(id,item_id,time_ms,content,created_at,updated_at) VALUES('n','i',750000,'Collision',0,0)",[]).unwrap();
    for q in ["程序设计", "课程", "材质", "Collision"] {
        assert_eq!(r.list_series(q, None, 1, false).unwrap().1, 1);
    }
    let detail = r.series(&s, 1).unwrap();
    assert_eq!(detail.series.total_items, 1);
    assert_eq!(detail.series.completed_items, 1);
    assert_eq!(detail.items[0].id, "i");
    for n in 0..50 {
        r.save_series(&course(&format!("S{n}"), None)).unwrap();
    }
    assert_eq!(r.list_series("", None, 1, false).unwrap().0.len(), 50);
    assert_eq!(r.list_series("", None, 2, false).unwrap().0.len(), 1);
    assert_eq!(r.list_series("%", None, 1, false).unwrap().1, 0);
}
#[test]
fn learning_catalog_reorders_atomically_and_series_removal_only_cascades_records() {
    let mut r = repository();
    let a = r.save_category(&category("A")).unwrap();
    let b = r.save_category(&category("B")).unwrap();
    let c = r.save_category(&category("C")).unwrap();
    r.move_entry("category", &c, Some(&a), None).unwrap();
    assert_eq!(
        r.categories()
            .unwrap()
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>(),
        vec![c.clone(), a, b]
    );
    let before = r
        .categories()
        .unwrap()
        .iter()
        .map(|c| c.id.clone())
        .collect::<Vec<_>>();
    assert!(r.move_entry("category", "missing", Some(&c), None).is_err());
    assert_eq!(
        r.categories()
            .unwrap()
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>(),
        before
    );
    let s = r.save_series(&course("移除", None)).unwrap();
    r.conn.execute("INSERT INTO series_items(id,series_id,title,file_path,created_at,updated_at) VALUES('i',?1,'A','C:/keep.mp4',0,0)",[&s]).unwrap();
    r.conn.execute("INSERT INTO video_notes(id,item_id,time_ms,content,created_at,updated_at) VALUES('n','i',0,'保留文件',0,0)",[]).unwrap();
    r.delete_series_records(&s).unwrap();
    assert_eq!(
        r.conn
            .query_row("SELECT COUNT(*) FROM video_notes", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn learning_import_scans_in_natural_lesson_order_and_ignores_nonvideo() {
    let dir = std::env::temp_dir().join(format!(
        "learning-files-{}",
        crate::ffmpeg::runner::new_id()
    ));
    std::fs::create_dir(&dir).unwrap();
    for name in ["Lesson 10.mp4", "第2课.mp4", "EP01.mp4", "说明.txt"] {
        std::fs::write(dir.join(name), b"source").unwrap();
    }
    let paths = super::importer::scan_paths(&[
        dir.to_string_lossy().into(),
        dir.join("EP01.mp4").to_string_lossy().into(),
    ])
    .unwrap();
    let names = paths
        .iter()
        .map(|p| {
            Path::new(p)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["EP01.mp4", "第2课.mp4", "Lesson 10.mp4"]);
    assert!(
        super::importer::scan_paths(&[dir.join("absent.mp4").to_string_lossy().into()]).is_err()
    );
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn learning_import_deduplicates_paths_and_keeps_original_files() {
    let mut r = repository();
    let dir = std::env::temp_dir().join(format!(
        "learning-source-{}",
        crate::ffmpeg::runner::new_id()
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("1.mp4");
    std::fs::write(&path, b"unchanged").unwrap();
    let path = std::fs::canonicalize(path)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let file = PreparedLocal {
        key: path.to_lowercase(),
        path: path.clone(),
        title: "第一课".into(),
        duration_ms: Some(900000),
    };
    let input = LocalImportInput {
        paths: vec![path.clone()],
        title: "系列".into(),
        category_id: None,
        series_id: None,
    };
    let a = r.import_local(&input, &[file.clone()]).unwrap();
    assert_eq!(a.added, 1);
    let b = r.import_local(&input, &[file]).unwrap();
    assert_eq!(b.added, 0);
    assert_eq!(b.duplicates, 1);
    assert_eq!(a.series_id, b.series_id);
    assert_eq!(
        r.series(&a.series_id, 1).unwrap().items[0].duration_ms,
        Some(900000)
    );
    r.delete_series_records(&a.series_id).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"unchanged");
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn learning_file_deletion_requires_confirmation_and_rejects_shared_references() {
    let mut r = repository();
    let a = r.save_series(&course("A", None)).unwrap();
    let b = r.save_series(&course("B", None)).unwrap();
    let dir = std::env::temp_dir().join(format!(
        "learning-delete-{}",
        crate::ffmpeg::runner::new_id()
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("keep.mp4");
    std::fs::write(&path, b"keep").unwrap();
    let path = std::fs::canonicalize(path)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let key = path.to_lowercase();
    for (id, s) in [("i1", a), ("i2", b)] {
        r.conn.execute("INSERT INTO series_items(id,series_id,title,file_path,file_key,available,created_at,updated_at) VALUES(?1,?2,'A',?3,?4,1,0,0)",rusqlite::params![id,s,path,key]).unwrap();
    }
    assert!(r.remove_library_files(&["i1".into()], false).is_err());
    assert!(r.remove_library_files(&["i1".into()], true).is_err());
    assert!(Path::new(&path).exists());
    r.delete_item_records("i2").unwrap();
    assert_eq!(r.remove_library_files(&["i1".into()], true).unwrap(), 1);
    assert!(!Path::new(&path).exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn learning_player_only_reads_registered_video_and_marks_missing_file() {
    let r = repository();
    let s = r.save_series(&course("播放", None)).unwrap();
    r.conn.execute("INSERT INTO series_items(id,series_id,title,file_path,available,created_at,updated_at) VALUES('i',?1,'课时','C:/absent-lesson.mp4',1,0,0)",[s]).unwrap();
    assert!(r.playback_file("unregistered").is_err());
    assert!(r.playback_file("i").is_err());
    assert!(!r.item("i").unwrap().available);
}
#[test]
fn learning_player_neighbors_cross_page_boundary_in_user_order() {
    let r = repository();
    let s = r.save_series(&course("播放", None)).unwrap();
    for n in 0..52 {
        r.conn.execute("INSERT INTO series_items(id,series_id,title,sort_order,created_at,updated_at) VALUES(?1,?2,'课时',?3,0,0)",rusqlite::params![format!("i{n}"),s,n]).unwrap();
    }
    assert_eq!(
        r.neighbors("i49").unwrap(),
        (Some("i48".into()), Some("i50".into()))
    );
    assert_eq!(r.neighbors("i0").unwrap().0, None);
}

#[test]
fn learning_progress_persists_half_resume_rejects_stale_and_short_snapshots() {
    let r = repository();
    let s = r.save_series(&course("断点", None)).unwrap();
    r.conn.execute("INSERT INTO series_items(id,series_id,title,duration_ms,created_at,updated_at) VALUES('i',?1,'课时',900000,0,0)",[&s]).unwrap();
    let mut p = ProgressInput {
        item_id: "i".into(),
        position_ms: 450000,
        duration_ms: 900000,
        playback_rate: 1.5,
        ended: false,
        sequence: 10,
    };
    r.save_progress(&p).unwrap();
    assert_eq!(r.item("i").unwrap().progress.position_ms, 450000);
    assert_eq!(r.continue_learning().unwrap()[0].item.id, "i");
    p.position_ms = 1000;
    p.sequence = 11;
    r.save_progress(&p).unwrap();
    assert_eq!(r.item("i").unwrap().progress.position_ms, 450000);
    p.sequence = 9;
    p.position_ms = 90000;
    r.save_progress(&p).unwrap();
    assert_eq!(r.item("i").unwrap().progress.position_ms, 450000);
    p.sequence = 12;
    p.position_ms = 810000;
    r.save_progress(&p).unwrap();
    assert!(r.item("i").unwrap().progress.completed);
    assert!(r.continue_learning().unwrap().is_empty());
    assert_eq!(
        r.conn
            .query_row(
                "SELECT total_items,completed_items FROM series WHERE id=?1",
                [&s],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            )
            .unwrap(),
        (1, 1)
    );
    r.mark_completed("i", false).unwrap();
    assert!(!r.item("i").unwrap().progress.completed);
    assert_eq!(r.item("i").unwrap().progress.position_ms, 0);
    assert_eq!(r.get_series(&s).unwrap().completed_items, 0);
    p.position_ms = -1;
    assert!(r.save_progress(&p).is_err());
}
#[test]
fn learning_progress_reopens_database_and_keeps_rate_and_completion() {
    let dir = std::env::temp_dir().join(format!(
        "learning-resume-{}",
        crate::ffmpeg::runner::new_id()
    ));
    std::fs::create_dir(&dir).unwrap();
    let file = dir.join("learning.db");
    {
        let r = Repository::open(&file).unwrap();
        let s = r.save_series(&course("重启", None)).unwrap();
        r.conn.execute("INSERT INTO series_items(id,series_id,title,duration_ms,created_at,updated_at) VALUES('i',?1,'第三课',900000,0,0)",[s]).unwrap();
        r.save_progress(&ProgressInput {
            item_id: "i".into(),
            position_ms: 450000,
            duration_ms: 900000,
            playback_rate: 1.5,
            ended: false,
            sequence: 10,
        })
        .unwrap();
    }
    {
        let r = Repository::open(&file).unwrap();
        let p = r.item("i").unwrap().progress;
        assert_eq!(p.position_ms, 450000);
        assert_eq!(p.playback_rate, 1.5);
        assert!(!p.completed);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn learning_notes_edit_timestamp_search_and_cascade() {
    let r = repository();
    let s = r.save_series(&course("笔记", None)).unwrap();
    r.conn.execute("INSERT INTO series_items(id,series_id,title,duration_ms,created_at,updated_at) VALUES('i',?1,'课程',900000,0,0)",[&s]).unwrap();
    let mut input = NoteInput {
        id: None,
        item_id: "i".into(),
        time_ms: 750000,
        content: "GPU Collision".into(),
    };
    let id = r.save_note(&input).unwrap();
    input.id = Some(id.clone());
    input.content = "Collision 修正".into();
    r.save_note(&input).unwrap();
    assert_eq!(r.notes("i", 1).unwrap().notes[0].time_ms, 750000);
    assert_eq!(r.list_series("Collision", None, 1, false).unwrap().1, 1);
    input.id = None;
    input.time_ms = 30000;
    r.save_note(&input).unwrap();
    assert_eq!(r.notes("i", 1).unwrap().notes[0].time_ms, 30000);
    input.time_ms = 900001;
    assert!(r.save_note(&input).is_err());
    r.delete_note(&id).unwrap();
    assert_eq!(r.notes("i", 1).unwrap().total, 1);
    r.delete_series_records(&s).unwrap();
    assert_eq!(
        r.conn
            .query_row("SELECT COUNT(*) FROM video_notes", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn learning_notes_survive_reopen_with_original_item_and_timestamp() {
    let dir = std::env::temp_dir().join(format!(
        "learning-notes-{}",
        crate::ffmpeg::runner::new_id()
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("learning.db");
    {
        let r = Repository::open(&path).unwrap();
        let s = r.save_series(&course("笔记", None)).unwrap();
        r.conn.execute("INSERT INTO series_items(id,series_id,title,duration_ms,created_at,updated_at) VALUES('i',?1,'课程',900000,0,0)",[s]).unwrap();
        r.save_note(&NoteInput {
            id: None,
            item_id: "i".into(),
            time_ms: 750000,
            content: "持久笔记".into(),
        })
        .unwrap();
    }
    {
        let r = Repository::open(&path).unwrap();
        let notes = r.notes("i", 1).unwrap();
        assert_eq!(notes.notes[0].content, "持久笔记");
        assert_eq!(notes.notes[0].time_ms, 750000);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

fn playlist_input() -> PlaylistImport {
    let raw = serde_json::json!({"_type":"playlist","id":"pl-test","title":"离线播放列表","extractor_key":"Generic","entries":[{"id":"v10","url":"https://example.com/10.mp4","title":"第10课","playlist_index":10},{"id":"v2","url":"https://example.com/2.mp4","title":"第2课","playlist_index":2},{"id":"v1","url":"https://example.com/1.mp4","title":"第1课","playlist_index":1}]});
    PlaylistImport {
        preview: crate::engine::media_preview("https://example.com/playlist", &raw).unwrap(),
        series_id: None,
        category_id: None,
        title: "系列".into(),
        keep_order: true,
        template: crate::models::DownloadRequest {
            kind: crate::models::MediaKind::Video,
            max_height: 720,
            filename_template: "{播放列表序号} - {标题} [{视频ID}]".into(),
            ..Default::default()
        },
    }
}
#[test]
fn learning_playlist_keeps_source_order_deduplicates_ids_and_freezes_requests() {
    let mut r = repository();
    let mut input = playlist_input();
    let first = r.stage_playlists(&[input.clone()]).unwrap().remove(0);
    assert_eq!(first.added, 3);
    let detail = r.series(&first.series_id, 1).unwrap();
    assert_eq!(
        detail
            .items
            .iter()
            .map(|i| i.source_playlist_index.unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 10]
    );
    let id = &detail.items[0].id;
    assert!(!detail.items[0].available);
    assert_eq!(r.download_request(id).unwrap().max_height, 720);
    input.template.max_height = 1080;
    input.preview.site = "CoreAlias".into();
    input.preview.entries[0].title = "改名".into();
    input.preview.entries[0].url = "https://example.com/changed.mp4".into();
    let again = r.stage_playlists(&[input]).unwrap().remove(0);
    assert_eq!(again.added, 0);
    assert_eq!(again.duplicates, 3);
    assert_eq!(again.series_id, first.series_id);
    assert_eq!(r.download_request(id).unwrap().max_height, 720);
}
#[test]
fn learning_download_callback_before_binding_and_failure_recovery_are_idempotent() {
    let mut r = repository();
    let input = playlist_input();
    let s = r.stage_playlists(&[input]).unwrap().remove(0).series_id;
    let item = r.series(&s, 1).unwrap().items.remove(0);
    let request = r.download_request(&item.id).unwrap();
    let mut task = crate::models::DownloadTask {
        id: "download1".into(),
        request,
        status: crate::models::TaskStatus::Failed,
        created_at: super::repository::now() as u64,
        ..Default::default()
    };
    assert_eq!(r.apply_download(&task, None).unwrap().len(), 1);
    let failed = r.item(&item.id).unwrap();
    assert_eq!(failed.download_task_id.as_deref(), Some("download1"));
    assert_eq!(failed.download_state, "failed");
    assert!(!failed.available);
    task.status = crate::models::TaskStatus::Completed;
    let file = PreparedLocal {
        path: "C:/actual/final.mp4".into(),
        key: "c:/actual/final.mp4".into(),
        title: "final".into(),
        duration_ms: Some(900000),
    };
    r.apply_download(&task, Some(&file)).unwrap();
    r.apply_download(&task, Some(&file)).unwrap();
    assert_eq!(r.item(&item.id).unwrap().file_path, Some(file.path));
    assert!(r.item(&item.id).unwrap().available);
    assert_eq!(r.series(&s, 1).unwrap().total, 3);
    assert_eq!(
        r.download_request(&item.id).unwrap().playlist_index,
        Some(1)
    );
}

#[test]
fn learning_updates_only_add_new_source_ids_and_preserve_favorites() {
    let mut r = repository();
    let mut input = playlist_input();
    let s = r
        .stage_playlists(&[input.clone()])
        .unwrap()
        .remove(0)
        .series_id;
    input.preview.entries[0].title = "标题改变".into();
    input.preview.entries[0].url = "https://example.com/new-title".into();
    assert!(r.new_entries(&s, &input.preview).unwrap().is_empty());
    input.preview.entries.push(crate::models::PlaylistEntry {
        id: "new-video".into(),
        url: "https://example.com/11.mp4".into(),
        title: "新课".into(),
        duration: Some(900.),
        thumbnail: None,
        playlist_index: 11,
    });
    let delta = r.new_entries(&s, &input.preview).unwrap();
    assert_eq!(delta.len(), 1);
    assert_eq!(delta[0].id, "new-video");
    let series = r.get_series(&s).unwrap();
    let mut edit = course(&series.title, None);
    edit.id = Some(s.clone());
    edit.source_url = series.source_url;
    edit.favorite = true;
    r.save_series(&edit).unwrap();
    assert_eq!(r.list_series("", None, 1, true).unwrap().1, 1);
    input.series_id = Some(s.clone());
    input.preview.entries = delta;
    r.stage_playlists(&[input]).unwrap();
    assert_eq!(r.get_series(&s).unwrap().total_items, 4);
    assert!(r.get_series(&s).unwrap().favorite);
}

#[test]
fn learning_large_library_is_bounded_at_50_and_searches_10000_lessons() {
    let r = repository();
    let start = std::time::Instant::now();
    let tx = r.conn.unchecked_transaction().unwrap();
    for c in 0..100 {
        tx.execute(
            "INSERT INTO categories(id,name,created_at,updated_at) VALUES(?1,?2,0,0)",
            rusqlite::params![format!("c{c}"), format!("分类{c}")],
        )
        .unwrap();
    }
    for s in 0..1000 {
        tx.execute(
            "INSERT INTO series(id,category_id,title,created_at,updated_at) VALUES(?1,?2,?3,0,0)",
            rusqlite::params![format!("s{s}"), format!("c{}", s % 100), format!("系列{s}")],
        )
        .unwrap();
        for i in 0..10 {
            tx.execute("INSERT INTO series_items(id,series_id,title,duration_ms,sort_order,created_at,updated_at) VALUES(?1,?2,?3,900000,?4,0,0)",rusqlite::params![format!("i{s}-{i}"),format!("s{s}"),format!("课程{s}-{i}"),i]).unwrap();
        }
    }
    tx.commit().unwrap();
    let seed = start.elapsed();
    let start = std::time::Instant::now();
    let page = r.list_series("", None, 1, false).unwrap();
    assert_eq!(page.0.len(), 50);
    assert_eq!(page.1, 1000);
    assert_eq!(r.categories().unwrap().len(), 100);
    assert_eq!(r.list_series("课程999-9", None, 1, false).unwrap().1, 1);
    assert_eq!(r.series("s999", 1).unwrap().total, 10);
    assert_eq!(r.list_series("", None, 20, false).unwrap().0.len(), 50);
    eprintln!(
        "LEARNING_BENCH categories=100 series=1000 lessons=10000 seed_ms={} query_ms={}",
        seed.as_millis(),
        start.elapsed().as_millis()
    );
}
#[test]
fn learning_corrupt_database_is_reported_without_overwriting_original() {
    let dir = std::env::temp_dir().join(format!(
        "learning-corrupt-{}",
        crate::ffmpeg::runner::new_id()
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("learning.db");
    let bytes = b"not a SQLite database - keep evidence";
    std::fs::write(&path, bytes).unwrap();
    assert!(db::open(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn learning_reuses_earlier_download_without_matching_old_history() {
    let mut r = repository();
    let series = r
        .stage_playlists(&[playlist_input()])
        .unwrap()
        .remove(0)
        .series_id;
    let item = r.series(&series, 1).unwrap().items.remove(0);
    r.conn
        .execute(
            "UPDATE series_items SET created_at=2000 WHERE id=?1",
            [&item.id],
        )
        .unwrap();
    let mut task = crate::models::DownloadTask {
        id: "earlier-task".into(),
        request: r.download_request(&item.id).unwrap(),
        status: crate::models::TaskStatus::Paused,
        created_at: 1000,
        ..Default::default()
    };
    r.link_task(&task, &item.id).unwrap();
    assert_eq!(
        r.item(&item.id).unwrap().download_task_id.as_deref(),
        Some("earlier-task")
    );
    r.conn
        .execute(
            "UPDATE series_items SET download_task_id=NULL WHERE id=?1",
            [&item.id],
        )
        .unwrap();
    assert_eq!(r.matching_items(&task).unwrap(), vec![item.id.clone()]);
    task.status = crate::models::TaskStatus::Completed;
    task.finished_at = Some(1500);
    assert!(
        r.matching_items(&task).unwrap().is_empty(),
        "old history must not bind a newly staged lesson"
    );
    task.finished_at = Some(3000);
    assert_eq!(
        r.matching_items(&task).unwrap(),
        vec![item.id.clone()],
        "completion before explicit binding still recovers"
    );
    r.apply_download(&task, None).unwrap();
    assert_eq!(
        r.item(&item.id).unwrap().download_task_id.as_deref(),
        Some("earlier-task")
    );
    let mut later = playlist_input();
    later.preview.url = "https://example.com/another-playlist".into();
    let later_series = r.stage_playlists(&[later]).unwrap().remove(0).series_id;
    let later_item = r.series(&later_series, 1).unwrap().items.remove(0);
    r.link_task(&task, &item.id).unwrap();
    assert!(
        r.item(&later_item.id).unwrap().download_task_id.is_none(),
        "an explicit link must not attach unrelated later imports to old history"
    );
}
