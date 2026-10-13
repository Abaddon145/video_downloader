use crate::domain::AppResult;
use rusqlite::Connection;
use std::{path::Path,time::Duration};
pub(crate) fn error(e:rusqlite::Error)->String{format!("学习库数据库错误：{e}")}
pub fn open(path:&Path)->AppResult<Connection>{
 let mut c=Connection::open(path).map_err(error)?;
 c.busy_timeout(Duration::from_secs(3)).map_err(error)?;
 c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;").map_err(error)?;
 let exists:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_version')",[],|r|r.get(0)).map_err(error)?;
 let version=if exists {c.query_row("SELECT version FROM schema_version",[],|r|r.get::<_,i64>(0)).map_err(error)?} else {0};
 if version>2 || version<0 {return Err("学习库版本高于本软件或版本记录无效，请保留数据库并使用对应版本".into());}
 let tx=c.transaction().map_err(error)?;
 if version==0{tx.execute_batch(include_str!("schema-v1.sql")).map_err(error)?;}
 if version<2 {tx.execute_batch("ALTER TABLE series ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0;
 ALTER TABLE series ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
 ALTER TABLE series_items ADD COLUMN file_key TEXT;
 ALTER TABLE series_items ADD COLUMN source_key TEXT;
 ALTER TABLE series_items ADD COLUMN download_request TEXT;
 ALTER TABLE series_items ADD COLUMN download_state TEXT NOT NULL DEFAULT 'waiting';
 CREATE UNIQUE INDEX idx_items_file_key ON series_items(series_id,file_key) WHERE file_key IS NOT NULL;
 CREATE UNIQUE INDEX idx_items_source_key ON series_items(series_id,source_key) WHERE source_key IS NOT NULL;
 CREATE INDEX idx_items_task ON series_items(download_task_id);
 CREATE TABLE learning_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
 UPDATE schema_version SET version=2;").map_err(error)?;}
 tx.execute_batch("PRAGMA user_version=2;").map_err(error)?;tx.commit().map_err(error)?;
 Ok(c)
}
