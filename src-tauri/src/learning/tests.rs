use super::db;
use rusqlite::Connection;
use std::path::Path;
#[test]
fn learning_database_creates_versioned_tables_and_enforces_relations() {
    let c = db::open(Path::new(":memory:")).unwrap();
    let version: i64 = c
        .query_row("SELECT version FROM schema_version", [], |r| r.get(0))
        .expect("schema must exist");
    assert_eq!(version, 2);
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
