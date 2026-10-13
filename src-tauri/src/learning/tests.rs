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
