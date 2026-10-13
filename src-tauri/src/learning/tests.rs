use super::db;
use rusqlite::Connection;
use std::path::Path;
#[test]
fn learning_database_creates_versioned_tables_and_enforces_relations(){
 let c=db::open(Path::new(":memory:")).unwrap();
 let version:i64=c.query_row("SELECT version FROM schema_version",[],|r|r.get(0)).expect("schema must exist");assert_eq!(version,2);
 for table in ["categories","series","series_items","playback_progress","video_notes","learning_settings"] {assert_eq!(c.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",[table],|r|r.get::<_,i64>(0)).unwrap(),1);}
 assert!(c.execute("INSERT INTO series_items(id,series_id,title,created_at,updated_at) VALUES('a','missing','A',0,0)",[]).is_err());
}
#[test]
fn learning_database_migrates_v1_preserving_data_and_rejects_future_schema(){
 let dir=std::env::temp_dir().join(format!("learning-migration-{}",crate::ffmpeg::runner::new_id()));std::fs::create_dir(&dir).unwrap();let path=dir.join("learning.db");
 {let c=Connection::open(&path).unwrap();c.execute_batch(include_str!("schema-v1.sql")).unwrap();c.execute("INSERT INTO series(id,title,created_at,updated_at) VALUES('s','保留',0,0)",[]).unwrap();}
 {let c=db::open(&path).unwrap();assert_eq!(c.query_row("SELECT title FROM series WHERE id='s'",[],|r|r.get::<_,String>(0)).unwrap(),"保留");assert!(c.execute("UPDATE series SET favorite=1,sort_order=2",[]).is_ok());c.execute("UPDATE schema_version SET version=999",[]).unwrap();}
 assert!(db::open(&path).is_err());std::fs::remove_dir_all(dir).unwrap();
}
