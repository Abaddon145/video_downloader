use super::db;use crate::domain::AppResult;use rusqlite::Connection;use std::path::Path;
pub struct Repository{pub(crate) conn:Connection}
impl Repository{pub fn open(path:&Path)->AppResult<Self>{Ok(Self{conn:db::open(path)?})}}
