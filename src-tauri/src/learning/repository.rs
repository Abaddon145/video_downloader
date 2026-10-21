use super::{db, models::*};
use crate::domain::AppResult;
use rusqlite::{params, Connection, OptionalExtension, Params, Row};
use std::path::Path;
pub struct Repository {
    pub(crate) conn: Connection,
}

#[cfg(test)]
mod safe_undo_tests {
    use super::*;

    #[test]
    fn receipts_are_session_bound_single_use_and_keep_failed_undo_for_retry() {
        let mut r = fixture();
        let mut receipts = std::collections::VecDeque::new();
        let receipt = r
            .reversible_receipt(
                ReversibleLearningChange::MoveItem {
                    id: "i".into(),
                    series_id: "b".into(),
                },
                &mut receipts,
            )
            .unwrap()
            .unwrap();
        assert_eq!(receipt.len(), 32);
        assert!(receipt.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(r.undo_receipt("forged-id", &mut receipts).is_err());
        assert_eq!(r.item("i").unwrap().series_id, "b");
        assert_eq!(receipts.len(), 1);
        assert!(r
            .undo_receipt(&receipt, &mut std::collections::VecDeque::new())
            .is_err());
        let expected = r.item("i").unwrap().sort_order;
        r.conn
            .execute("UPDATE series_items SET sort_order=99 WHERE id='i'", [])
            .unwrap();
        assert!(r.undo_receipt(&receipt, &mut receipts).is_err());
        assert_eq!(receipts.len(), 1);
        assert_eq!(r.item("i").unwrap().sort_order, 99);
        r.conn
            .execute(
                "UPDATE series_items SET sort_order=?1 WHERE id='i'",
                [expected],
            )
            .unwrap();
        assert_eq!(r.undo_receipt(&receipt, &mut receipts).unwrap(), "i");
        assert_eq!(r.item("i").unwrap().series_id, "a");
        assert_eq!(r.item("i").unwrap().sort_order, 4);
        assert!(receipts.is_empty());
        assert!(r.undo_receipt(&receipt, &mut receipts).is_err());
        assert!(r
            .reversible_receipt(
                ReversibleLearningChange::MoveItem {
                    id: "i".into(),
                    series_id: "a".into()
                },
                &mut receipts
            )
            .unwrap()
            .is_none());
        assert!(receipts.is_empty());
        let mut first = String::new();
        for n in 0..31 {
            let receipt = r
                .reversible_receipt(
                    ReversibleLearningChange::RenameItem {
                        id: "i".into(),
                        title: format!("title-{n}"),
                    },
                    &mut receipts,
                )
                .unwrap()
                .unwrap();
            if n == 0 {
                first = receipt;
            }
        }
        assert_eq!(receipts.len(), 30);
        assert!(r.undo_receipt(&first, &mut receipts).is_err());
        assert_eq!(r.item("i").unwrap().title, "title-30");
    }

    fn fixture() -> Repository {
        let r = Repository::open(Path::new(":memory:")).unwrap();
        r.conn.execute_batch("INSERT INTO categories(id,name,created_at,updated_at) VALUES('c1','旧分类',0,0),('c2','新分类',0,0);
            INSERT INTO series(id,category_id,title,description,favorite,created_at,updated_at) VALUES('a','c1','原系列','保留说明',0,0,0),('b','c2','目标系列','目标说明',0,0,0);
            INSERT INTO series_items(id,series_id,title,file_path,sort_order,created_at,updated_at) VALUES('i','a','旧名称','C:/source.mp4',4,0,0);
            INSERT INTO playback_progress(item_id,position_ms,duration_ms,completed,updated_at) VALUES('i',42000,60000,0,0);
            INSERT INTO video_notes(id,item_id,time_ms,content,created_at,updated_at) VALUES('n','i',42000,'保留笔记',0,0);").unwrap();
        r
    }

    #[test]
    fn undo_restores_only_target_fields_and_keeps_other_course_data() {
        let mut r = fixture();
        let moved = r
            .reversible_change(ReversibleLearningChange::MoveItem {
                id: "i".into(),
                series_id: "b".into(),
            })
            .unwrap()
            .unwrap();
        r.conn
            .execute(
                "UPDATE series_items SET title='随后修改的标题' WHERE id='i'",
                [],
            )
            .unwrap();
        r.undo_change(moved).unwrap();
        let item = r.item("i").unwrap();
        assert_eq!(
            (
                item.series_id.as_str(),
                item.title.as_str(),
                item.file_path.as_deref(),
                item.progress.position_ms
            ),
            ("a", "随后修改的标题", Some("C:/source.mp4"), 42000)
        );
        assert_eq!(
            r.conn
                .query_row(
                    "SELECT sort_order FROM series_items WHERE id='i'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            4
        );
        assert_eq!(
            r.conn
                .query_row("SELECT content FROM video_notes WHERE id='n'", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
            "保留笔记"
        );

        let renamed = r
            .reversible_change(ReversibleLearningChange::RenameItem {
                id: "i".into(),
                title: "新名称".into(),
            })
            .unwrap()
            .unwrap();
        r.undo_change(renamed).unwrap();
        assert_eq!(r.item("i").unwrap().title, "随后修改的标题");

        let category = r
            .reversible_change(ReversibleLearningChange::SetSeriesCategory {
                id: "a".into(),
                category_id: Some("c2".into()),
            })
            .unwrap()
            .unwrap();
        r.conn
            .execute(
                "UPDATE series SET description='后来更新的说明' WHERE id='a'",
                [],
            )
            .unwrap();
        r.undo_change(category).unwrap();
        let series = r.get_series("a").unwrap();
        assert_eq!(
            (series.category_id.as_deref(), series.description.as_deref()),
            (Some("c1"), Some("后来更新的说明"))
        );
        let uncategorized = r
            .reversible_change(ReversibleLearningChange::SetSeriesCategory {
                id: "a".into(),
                category_id: None,
            })
            .unwrap()
            .unwrap();
        r.undo_change(uncategorized).unwrap();
        assert_eq!(
            r.get_series("a").unwrap().category_id.as_deref(),
            Some("c1")
        );

        let favorite = r
            .reversible_change(ReversibleLearningChange::SetSeriesFavorite {
                id: "a".into(),
                favorite: true,
            })
            .unwrap()
            .unwrap();
        r.conn
            .execute(
                "UPDATE series SET title='后来的系列名称',category_id=NULL WHERE id='a'",
                [],
            )
            .unwrap();
        r.undo_change(favorite).unwrap();
        let series = r.get_series("a").unwrap();
        assert!(!series.favorite);
        assert_eq!(
            (series.title.as_str(), series.category_id),
            ("后来的系列名称", None)
        );
        let category = r
            .reversible_change(ReversibleLearningChange::SetSeriesCategory {
                id: "a".into(),
                category_id: Some("c2".into()),
            })
            .unwrap()
            .unwrap();
        r.undo_change(category).unwrap();
        assert_eq!(r.get_series("a").unwrap().category_id, None);
        assert!(r
            .reversible_change(ReversibleLearningChange::MoveItem {
                id: "i".into(),
                series_id: "a".into()
            })
            .unwrap()
            .is_none());
        assert!(r
            .reversible_change(ReversibleLearningChange::RenameItem {
                id: "i".into(),
                title: " 随后修改的标题 ".into()
            })
            .unwrap()
            .is_none());
        assert!(r
            .reversible_change(ReversibleLearningChange::SetSeriesCategory {
                id: "a".into(),
                category_id: None
            })
            .unwrap()
            .is_none());
        assert!(r
            .reversible_change(ReversibleLearningChange::SetSeriesFavorite {
                id: "a".into(),
                favorite: false
            })
            .unwrap()
            .is_none());
        assert!(r
            .reversible_change(ReversibleLearningChange::RenameItem {
                id: "i".into(),
                title: " \0 ".into()
            })
            .is_err());
        assert!(r
            .reversible_change(ReversibleLearningChange::SetSeriesCategory {
                id: "a".into(),
                category_id: Some("missing".into())
            })
            .is_err());
    }

    #[test]
    fn undo_rejects_stale_expected_values_without_overwriting_later_edits() {
        let mut r = fixture();
        let moved = r
            .reversible_change(ReversibleLearningChange::MoveItem {
                id: "i".into(),
                series_id: "b".into(),
            })
            .unwrap()
            .unwrap();
        r.conn
            .execute("UPDATE series_items SET sort_order=99 WHERE id='i'", [])
            .unwrap();
        assert!(r.undo_change(moved).is_err());
        assert_eq!(r.item("i").unwrap().series_id, "b");

        let renamed = r
            .reversible_change(ReversibleLearningChange::RenameItem {
                id: "i".into(),
                title: "新名称".into(),
            })
            .unwrap()
            .unwrap();
        r.conn
            .execute("UPDATE series_items SET title='后来名称' WHERE id='i'", [])
            .unwrap();
        assert!(r.undo_change(renamed).is_err());
        assert_eq!(r.item("i").unwrap().title, "后来名称");

        let category = r
            .reversible_change(ReversibleLearningChange::SetSeriesCategory {
                id: "a".into(),
                category_id: Some("c2".into()),
            })
            .unwrap()
            .unwrap();
        r.conn
            .execute("UPDATE series SET category_id=NULL WHERE id='a'", [])
            .unwrap();
        assert!(r.undo_change(category).is_err());
        assert_eq!(r.get_series("a").unwrap().category_id, None);

        let favorite = r
            .reversible_change(ReversibleLearningChange::SetSeriesFavorite {
                id: "a".into(),
                favorite: true,
            })
            .unwrap()
            .unwrap();
        r.conn
            .execute("UPDATE series SET favorite=0 WHERE id='a'", [])
            .unwrap();
        assert!(r.undo_change(favorite).is_err());
        assert!(!r.get_series("a").unwrap().favorite);
        assert_eq!(r.item("i").unwrap().progress.position_ms, 42000);
        assert_eq!(
            r.conn
                .query_row("SELECT content FROM video_notes WHERE id='n'", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
            "保留笔记"
        );
    }
}
pub(crate) fn now() -> i64 {
    crate::service::now() as i64
}
pub(crate) fn id() -> String {
    crate::ffmpeg::runner::new_id().replacen("media-", "learn-", 1)
}
pub(crate) fn text(s: &str, max: usize, required: bool) -> AppResult<String> {
    let s = s.trim();
    if required && s.is_empty()
        || s.chars().count() > max
        || s.chars().any(|c| c == '\0' || c.is_control() && c != '\n')
    {
        return Err(format!("请输入有效文本（最多{max}字）"));
    }
    Ok(s.into())
}
pub(crate) fn optional(s: &Option<String>, max: usize) -> AppResult<Option<String>> {
    s.as_ref()
        .map(|s| text(s, max, false))
        .transpose()
        .map(|s| s.filter(|s| !s.is_empty()))
}
pub(crate) const SERIES_CTE: &str = "WITH latest_analysis AS (
 SELECT a.item_id,a.status,a.updated_at,ROW_NUMBER() OVER(PARTITION BY a.item_id ORDER BY a.created_at DESC,a.id DESC) AS rank
 FROM video_analyses a JOIN series_items i ON i.id=a.item_id AND i.file_path=a.source_path
 WHERE json_extract(a.options_json,'$.localOnly')=0
), series_aggregates AS (
 SELECT i.series_id,COUNT(*) AS items,SUM(COALESCE(p.completed,0)) AS completed,
 SUM(COALESCE(i.duration_ms,p.duration_ms,0)) AS duration,SUM(i.available) AS available,
 SUM(a.status='completed') AS ai_completed,
 SUM(a.status IN ('queued','extracting_audio','transcribing','extracting_keyframes','analyzing_chunks','finalizing','writing_note')) AS ai_active,
 SUM(a.status IN ('failed','interrupted')) AS ai_failed,MAX(p.last_played_at) AS last_played,
 MAX(MAX(i.updated_at,COALESCE(p.updated_at,0),COALESCE(a.updated_at,0))) AS updated
 FROM series_items i LEFT JOIN playback_progress p ON p.item_id=i.id
 LEFT JOIN latest_analysis a ON a.item_id=i.id AND a.rank=1 GROUP BY i.series_id
)";
pub(crate) const SERIES_FROM: &str = "series s LEFT JOIN series_aggregates g ON g.series_id=s.id";
pub(crate) const SERIES_COLS:&str="s.id,s.category_id,s.title,s.description,s.cover_path,s.source_type,s.source_url,s.source_playlist_id,s.author,COALESCE(g.items,0),COALESCE(g.completed,0),COALESCE(g.duration,0),s.favorite,s.sort_order,s.created_at,MAX(s.updated_at,COALESCE(g.updated,0)),COALESCE(g.available,0),COALESCE(g.ai_completed,0),COALESCE(g.ai_active,0),COALESCE(g.ai_failed,0),g.last_played";
pub(crate) const ITEM_COLS:&str="i.id,i.series_id,i.title,i.file_path,i.source_url,i.source_video_id,i.source_playlist_index,i.duration_ms,i.sort_order,i.thumbnail_path,i.download_task_id,i.available,i.download_state,COALESCE(p.position_ms,0),COALESCE(p.duration_ms,i.duration_ms,0),COALESCE(p.completed,0),COALESCE(p.playback_rate,1.0),p.last_played_at,i.thumbnail_source,i.source_thumbnail_url,i.thumbnail_key";
fn category_row(r: &Row) -> rusqlite::Result<LearningCategory> {
    Ok(LearningCategory {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        icon: r.get(3)?,
        color: r.get(4)?,
        sort_order: r.get(5)?,
    })
}
fn series_row(r: &Row) -> rusqlite::Result<LearningSeries> {
    series_row_at(r, 0)
}
pub(crate) fn series_row_at(r: &Row, offset: usize) -> rusqlite::Result<LearningSeries> {
    Ok(LearningSeries {
        id: r.get(offset + 0)?,
        category_id: r.get(offset + 1)?,
        title: r.get(offset + 2)?,
        description: r.get(offset + 3)?,
        cover_path: r.get(offset + 4)?,
        source_type: r.get(offset + 5)?,
        source_url: r.get(offset + 6)?,
        source_playlist_id: r.get(offset + 7)?,
        author: r.get(offset + 8)?,
        total_items: r.get(offset + 9)?,
        completed_items: r.get(offset + 10)?,
        total_duration_ms: r.get(offset + 11)?,
        favorite: r.get(offset + 12)?,
        sort_order: r.get(offset + 13)?,
        created_at: r.get(offset + 14)?,
        updated_at: r.get(offset + 15)?,
        available_items: r.get(offset + 16)?,
        ai_completed_items: r.get(offset + 17)?,
        ai_active_items: r.get(offset + 18)?,
        ai_failed_items: r.get(offset + 19)?,
        last_played_at: r.get(offset + 20)?,
    })
}
pub(crate) fn item_row(r: &Row) -> rusqlite::Result<LearningItem> {
    let id: String = r.get(0)?;
    Ok(LearningItem {
        progress: PlaybackProgress {
            item_id: id.clone(),
            position_ms: r.get(13)?,
            duration_ms: r.get(14)?,
            completed: r.get(15)?,
            playback_rate: r.get(16)?,
            last_played_at: r.get(17)?,
        },
        id,
        series_id: r.get(1)?,
        title: r.get(2)?,
        file_path: r.get(3)?,
        source_url: r.get(4)?,
        source_video_id: r.get(5)?,
        source_playlist_index: r.get(6)?,
        duration_ms: r.get(7)?,
        sort_order: r.get(8)?,
        thumbnail_path: r.get(9)?,
        thumbnail_source: r.get(18)?,
        source_thumbnail_url: r.get(19)?,
        thumbnail_key: r.get(20)?,
        download_task_id: r.get(10)?,
        available: r.get(11)?,
        download_state: r.get(12)?,
    })
}
impl Repository {
    pub fn open(path: &Path) -> AppResult<Self> {
        Ok(Self {
            conn: db::open(path)?,
        })
    }
    pub(crate) fn rows<T>(
        &self,
        sql: &str,
        p: impl Params,
        map: fn(&Row) -> rusqlite::Result<T>,
    ) -> AppResult<Vec<T>> {
        let mut stmt = self.conn.prepare(sql).map_err(db::error)?;
        let rows = stmt.query_map(p, map).map_err(db::error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db::error)
    }
    pub fn categories(&self) -> AppResult<Vec<LearningCategory>> {
        self.rows("SELECT id,name,description,icon,color,sort_order FROM categories ORDER BY sort_order,created_at,id",[],category_row)
    }
    pub fn save_category(&self, c: &CategoryInput) -> AppResult<String> {
        let name = text(&c.name, 80, true)?;
        if name.contains('\n') {
            return Err("分类名称不能换行".into());
        }
        let description = optional(&c.description, 4000)?;
        let icon = optional(&c.icon, 40)?;
        let color = optional(&c.color, 7)?;
        if color.as_ref().is_some_and(|c| {
            c.len() != 7 || !c.starts_with('#') || !c[1..].bytes().all(|b| b.is_ascii_hexdigit())
        }) {
            return Err("请选择有效分类颜色".into());
        }
        if icon.as_ref().is_some_and(|s| {
            ![
                "BookOpen",
                "Boxes",
                "Film",
                "Code2",
                "Languages",
                "Palette",
                "Gamepad2",
                "FileVideo",
            ]
            .contains(&s.as_str())
        }) {
            return Err("请选择可用分类图标".into());
        }
        let id = c.id.clone().unwrap_or_else(id);
        let time = now();
        if c.id.is_some() {
            let n=self.conn.execute("UPDATE categories SET name=?1,description=?2,icon=?3,color=?4,updated_at=?5 WHERE id=?6",params![name,description,icon,color,time,id]).map_err(db::error)?;
            if n == 0 {
                return Err("分类不存在".into());
            }
        } else {
            self.conn.execute("INSERT INTO categories(id,name,description,icon,color,sort_order,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,(SELECT COALESCE(MAX(sort_order)+1,0) FROM categories),?6,?6)",params![id,name,description,icon,color,time]).map_err(db::error)?;
        }
        Ok(id)
    }
    pub fn delete_category(&self, id: &str, move_to_uncategorized: bool) -> AppResult<()> {
        let n: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM series WHERE category_id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        if n > 0 && !move_to_uncategorized {
            return Err(format!("分类包含{n}个系列，请确认移到未分类或取消删除"));
        }
        if self
            .conn
            .execute("DELETE FROM categories WHERE id=?1", [id])
            .map_err(db::error)?
            == 0
        {
            return Err("分类不存在".into());
        }
        Ok(())
    }
    pub fn save_series(&self, s: &SeriesInput) -> AppResult<String> {
        let title = text(&s.title, 240, true)?;
        let description = optional(&s.description, 8000)?;
        let category = optional(&s.category_id, 100)?;
        let url = optional(&s.source_url, 4096)?
            .map(|u| crate::domain::validate_url(&u))
            .transpose()?;
        let id = s.id.clone().unwrap_or_else(id);
        let time = now();
        if s.id.is_some() {
            if self.conn.execute("UPDATE series SET category_id=?1,title=?2,description=?3,cover_path=?4,source_url=?5,favorite=?6,updated_at=?7 WHERE id=?8",params![category,title,description,s.cover_path,url,s.favorite,time,id]).map_err(db::error)?==0{return Err("系列不存在".into());}
        } else {
            self.conn.execute("INSERT INTO series(id,category_id,title,description,cover_path,source_url,favorite,sort_order,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,(SELECT COALESCE(MAX(sort_order)+1,0) FROM series),?8,?8)",params![id,category,title,description,s.cover_path,url,s.favorite,time]).map_err(db::error)?;
        }
        Ok(id)
    }
    pub fn get_series(&self, id: &str) -> AppResult<LearningSeries> {
        self.conn
            .query_row(
                &format!("{SERIES_CTE} SELECT {SERIES_COLS} FROM {SERIES_FROM} WHERE s.id=?1"),
                [id],
                series_row,
            )
            .optional()
            .map_err(db::error)?
            .ok_or_else(|| "系列不存在".into())
    }
    pub fn list_series(
        &self,
        query: &str,
        category: Option<&str>,
        page: u32,
        favorite: bool,
    ) -> AppResult<(Vec<LearningSeries>, i64)> {
        self.list_series_sorted(query, category, page, favorite, "manual")
    }
    pub fn list_series_sorted(
        &self,
        query: &str,
        category: Option<&str>,
        page: u32,
        favorite: bool,
        sort: &str,
    ) -> AppResult<(Vec<LearningSeries>, i64)> {
        let order = match sort {
            "manual" => "s.sort_order,s.created_at DESC,s.id",
            "title" => "s.title COLLATE NOCASE,s.id",
            "updated" => "MAX(s.updated_at,COALESCE(g.updated,0)) DESC,s.id",
            "lastPlayed" => "g.last_played DESC,s.id",
            "progress" => {
                "CAST(COALESCE(g.completed,0) AS REAL)/MAX(COALESCE(g.items,0),1) DESC,s.id"
            }
            "duration" => "COALESCE(g.duration,0) DESC,s.id",
            _ => return Err("资料库排序方式无效".into()),
        };
        let query = text(query, 240, false)?;
        let filter="(?1 IS NULL OR COALESCE(s.category_id,'')=?1) AND (?2=0 OR s.favorite=1) AND (?3='' OR instr(lower(s.title),lower(?3))>0 OR EXISTS(SELECT 1 FROM categories c WHERE c.id=s.category_id AND instr(lower(c.name),lower(?3))>0) OR EXISTS(SELECT 1 FROM series_items i WHERE i.series_id=s.id AND (instr(lower(i.title),lower(?3))>0 OR EXISTS(SELECT 1 FROM video_notes n WHERE n.item_id=i.id AND instr(lower(n.content),lower(?3))>0))))";
        let total: i64 = self
            .conn
            .query_row(
                &format!("SELECT COUNT(*) FROM series s WHERE {filter}"),
                params![category, favorite, query],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        let page = page.max(1).min(((total + 49) / 50).max(1) as u32);
        let list=self.rows(&format!("{SERIES_CTE} SELECT {SERIES_COLS} FROM {SERIES_FROM} WHERE {filter} ORDER BY {order} LIMIT 50 OFFSET ?4"),params![category,favorite,query,(page-1)*50],series_row)?;
        Ok((list, total))
    }
    pub fn item(&self, id: &str) -> AppResult<LearningItem> {
        self.conn.query_row(&format!("SELECT {ITEM_COLS} FROM series_items i LEFT JOIN playback_progress p ON p.item_id=i.id WHERE i.id=?1"),[id],item_row).optional().map_err(db::error)?.ok_or_else(||"课程不存在".into())
    }
    pub fn search_items(&self, query: &str, page: u32) -> AppResult<ItemSearchPage> {
        let query = text(query, 200, false)?.to_lowercase();
        let filter = "FROM series_items i JOIN series s ON s.id=i.series_id LEFT JOIN playback_progress p ON p.item_id=i.id WHERE instr(lower(i.title),?1)>0 OR instr(lower(s.title),?1)>0";
        let total: i64 = self
            .conn
            .query_row(&format!("SELECT count(*) {filter}"), [&query], |row| {
                row.get(0)
            })
            .map_err(db::error)?;
        let page = page.max(1).min(((total + 49) / 50).max(1) as u32);
        let items = self.rows(&format!("SELECT {ITEM_COLS} {filter} ORDER BY COALESCE(p.last_played_at,i.updated_at) DESC,i.id LIMIT 50 OFFSET ?2"), params![query,(page-1)*50], item_row)?;
        Ok(ItemSearchPage { items, total, page })
    }
    pub fn series(&self, id: &str, page: u32) -> AppResult<SeriesDetail> {
        let series = self.get_series(id)?;
        let total = series.total_items;
        let page = page.max(1).min(((total + 49) / 50).max(1) as u32);
        let items=self.rows(&format!("SELECT {ITEM_COLS} FROM series_items i LEFT JOIN playback_progress p ON p.item_id=i.id WHERE i.series_id=?1 ORDER BY i.sort_order,i.created_at,i.id LIMIT 50 OFFSET ?2"),params![id,(page-1)*50],item_row)?;
        Ok(SeriesDetail {
            series,
            items,
            page,
            total,
        })
    }
    pub fn delete_series_records(&self, id: &str) -> AppResult<()> {
        if self
            .conn
            .execute("DELETE FROM series WHERE id=?1", [id])
            .map_err(db::error)?
            == 0
        {
            return Err("系列不存在".into());
        }
        Ok(())
    }
    pub fn rename_item(&self, id: &str, title: &str) -> AppResult<()> {
        let title = text(title, 500, true)?;
        if self
            .conn
            .execute(
                "UPDATE series_items SET title=?1,updated_at=?2 WHERE id=?3",
                params![title, now(), id],
            )
            .map_err(db::error)?
            == 0
        {
            return Err("课程不存在".into());
        }
        Ok(())
    }
    pub fn reversible_receipt(
        &mut self,
        change: ReversibleLearningChange,
        receipts: &mut std::collections::VecDeque<(String, LearningUndoToken)>,
    ) -> AppResult<Option<String>> {
        let receipt: String = self
            .conn
            .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
            .map_err(db::error)?;
        let Some(token) = self.reversible_change(change)? else {
            return Ok(None);
        };
        receipts.push_back((receipt.clone(), token));
        if receipts.len() > 30 {
            receipts.pop_front();
        }
        Ok(Some(receipt))
    }
    pub fn undo_receipt(
        &mut self,
        receipt: &str,
        receipts: &mut std::collections::VecDeque<(String, LearningUndoToken)>,
    ) -> AppResult<String> {
        let index = receipts
            .iter()
            .position(|(id, _)| id == receipt)
            .ok_or("无法撤销：操作凭据不存在、已过期或已使用")?;
        let token = receipts[index].1.clone();
        let id = match &token {
            LearningUndoToken::MoveItem { id, .. }
            | LearningUndoToken::RenameItem { id, .. }
            | LearningUndoToken::SetSeriesCategory { id, .. }
            | LearningUndoToken::SetSeriesFavorite { id, .. } => id.clone(),
        };
        self.undo_change(token)?;
        receipts.remove(index);
        Ok(id)
    }
    pub fn reversible_change(
        &mut self,
        change: ReversibleLearningChange,
    ) -> AppResult<Option<LearningUndoToken>> {
        let token = match change {
            ReversibleLearningChange::MoveItem { id, series_id } => {
                let before = self.item(&id)?;
                self.move_item_to_series(&id, &series_id)?;
                if before.series_id == series_id {
                    return Ok(None);
                }
                let after = self.item(&id)?;
                LearningUndoToken::MoveItem {
                    id,
                    previous_series_id: before.series_id,
                    previous_sort_order: before.sort_order,
                    expected_series_id: after.series_id,
                    expected_sort_order: after.sort_order,
                }
            }
            ReversibleLearningChange::RenameItem { id, title } => {
                let title = text(&title, 500, true)?;
                let previous = self.item(&id)?.title;
                if previous == title {
                    return Ok(None);
                }
                self.rename_item(&id, &title)?;
                LearningUndoToken::RenameItem {
                    id,
                    previous,
                    expected: title,
                }
            }
            ReversibleLearningChange::SetSeriesCategory { id, category_id } => {
                let category_id = optional(&category_id, 100)?;
                let previous = self.get_series(&id)?.category_id;
                if previous == category_id {
                    return Ok(None);
                }
                self.conn
                    .execute(
                        "UPDATE series SET category_id=?1,updated_at=?2 WHERE id=?3",
                        params![category_id, now(), id],
                    )
                    .map_err(db::error)?;
                LearningUndoToken::SetSeriesCategory {
                    id,
                    previous,
                    expected: category_id,
                }
            }
            ReversibleLearningChange::SetSeriesFavorite { id, favorite } => {
                let previous = self.get_series(&id)?.favorite;
                if previous == favorite {
                    return Ok(None);
                }
                self.conn
                    .execute(
                        "UPDATE series SET favorite=?1,updated_at=?2 WHERE id=?3",
                        params![favorite, now(), id],
                    )
                    .map_err(db::error)?;
                LearningUndoToken::SetSeriesFavorite {
                    id,
                    previous,
                    expected: favorite,
                }
            }
        };
        Ok(Some(token))
    }
    pub fn undo_change(&mut self, token: LearningUndoToken) -> AppResult<()> {
        let tx = self.conn.transaction().map_err(db::error)?;
        let changed = match token {
            LearningUndoToken::MoveItem { id, previous_series_id, previous_sort_order, expected_series_id, expected_sort_order } => {
                let changed = tx.execute("UPDATE series_items SET series_id=?1,sort_order=?2,updated_at=?3 WHERE id=?4 AND series_id=?5 AND sort_order=?6", params![text(&previous_series_id,100,true)?, previous_sort_order, now(), id, expected_series_id, expected_sort_order]).map_err(db::error)?;
                if changed == 1 {
                    tx.execute("UPDATE series SET updated_at=?1 WHERE id IN (?2,?3)", params![now(), previous_series_id, expected_series_id]).map_err(db::error)?;
                }
                changed
            }
            LearningUndoToken::RenameItem { id, previous, expected } => tx.execute("UPDATE series_items SET title=?1,updated_at=?2 WHERE id=?3 AND title=?4", params![text(&previous,500,true)?,now(),id,expected]).map_err(db::error)?,
            LearningUndoToken::SetSeriesCategory { id, previous, expected } => tx.execute("UPDATE series SET category_id=?1,updated_at=?2 WHERE id=?3 AND category_id IS ?4", params![optional(&previous,100)?,now(),id,expected]).map_err(db::error)?,
            LearningUndoToken::SetSeriesFavorite { id, previous, expected } => tx.execute("UPDATE series SET favorite=?1,updated_at=?2 WHERE id=?3 AND favorite=?4", params![previous,now(),id,expected]).map_err(db::error)?,
        };
        if changed != 1 {
            return Err("无法撤销：目标已被修改或移除，当前数据已保留".into());
        }
        tx.commit().map_err(db::error)
    }
    pub fn delete_item_records(&self, id: &str) -> AppResult<()> {
        if self
            .conn
            .execute("DELETE FROM series_items WHERE id=?1", [id])
            .map_err(db::error)?
            == 0
        {
            return Err("课程不存在".into());
        }
        Ok(())
    }
    pub fn move_item_to_series(&mut self, id: &str, series_id: &str) -> AppResult<()> {
        let previous = self.item(id)?.series_id;
        self.get_series(series_id)?;
        if previous == series_id {
            return Ok(());
        }
        let tx = self.conn.transaction().map_err(db::error)?;
        let order: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(sort_order),-1)+1 FROM series_items WHERE series_id=?1",
                [series_id],
                |row| row.get(0),
            )
            .map_err(db::error)?;
        tx.execute(
            "UPDATE series_items SET series_id=?1,sort_order=?2,updated_at=?3 WHERE id=?4",
            params![series_id, order, now(), id],
        )
        .map_err(db::error)?;
        tx.execute(
            "UPDATE series SET updated_at=?1 WHERE id IN (?2,?3)",
            params![now(), previous, series_id],
        )
        .map_err(db::error)?;
        tx.commit().map_err(db::error)
    }
    pub fn move_entry(
        &mut self,
        scope: &str,
        id: &str,
        before: Option<&str>,
        direction: Option<&str>,
    ) -> AppResult<()> {
        let table = match scope {
            "category" => "categories",
            "series" => "series",
            "item" => "series_items",
            _ => return Err("排序类型无效".into()),
        };
        let group: Option<String> = if scope == "item" {
            Some(self.item(id)?.series_id)
        } else {
            None
        };
        let filter = if group.is_some() {
            "WHERE series_id=?1"
        } else {
            "WHERE ?1 IS NULL"
        };
        let mut ids = self.rows(
            &format!("SELECT id FROM {table} {filter} ORDER BY sort_order,created_at,id"),
            [group],
            |r| r.get::<_, String>(0),
        )?;
        let old = ids.iter().position(|x| x == id).ok_or("排序项目不存在")?;
        let target = if let Some(dir) = direction {
            if before.is_some() {
                return Err("排序参数冲突".into());
            }
            match dir {
                "up" => old.saturating_sub(1),
                "down" => (old + 1).min(ids.len() - 1),
                _ => return Err("排序方向无效".into()),
            }
        } else {
            let before = before.ok_or("请选择排序位置")?;
            let p = ids
                .iter()
                .position(|x| x == before)
                .ok_or("目标排序项目不存在")?;
            if old < p {
                p - 1
            } else {
                p
            }
        };
        if target == old {
            return Ok(());
        }
        let value = ids.remove(old);
        ids.insert(target, value);
        let tx = self.conn.transaction().map_err(db::error)?;
        for (n, id) in ids.iter().enumerate() {
            tx.execute(
                &format!("UPDATE {table} SET sort_order=?1,updated_at=?2 WHERE id=?3"),
                params![n as i64, now(), id],
            )
            .map_err(db::error)?;
        }
        tx.commit().map_err(db::error)
    }
    pub fn settings(&self) -> AppResult<LearningSettings> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM learning_settings WHERE key='settings'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(db::error)?;
        json.map(|s| serde_json::from_str(&s).map_err(|_| "学习库设置损坏，请保留数据库".into()))
            .unwrap_or_else(|| Ok(LearningSettings::default()))
    }
    pub fn save_settings(&self, s: &LearningSettings) -> AppResult<()> {
        if !(1..=4096).contains(&s.playback_cache_gb)
            || s.playback_cache_days > 3650
            || !s.completion_threshold.is_finite()
            || !(0.01..=1.).contains(&s.completion_threshold)
            || ![0.5, 0.75, 1., 1.25, 1.5, 1.75, 2.].contains(&s.default_playback_rate)
        {
            return Err("完成阈值应为1%–100%，请选择可用倍速".into());
        }
        let json = serde_json::to_string(s).map_err(|e| e.to_string())?;
        self.conn.execute("INSERT INTO learning_settings(key,value) VALUES('settings',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[json]).map_err(db::error)?;
        Ok(())
    }
}

impl Repository {
    pub fn notes(&self, item: &str, page: u32) -> AppResult<NotePage> {
        self.item(item)?;
        let total: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM video_notes WHERE item_id=?1",
                [item],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        let page = page.max(1).min(((total + 49) / 50).max(1) as u32);
        let notes=self.rows("SELECT id,item_id,time_ms,content,created_at,updated_at,markdown_vault_id,markdown_path FROM video_notes WHERE item_id=?1 ORDER BY time_ms,created_at,id LIMIT 50 OFFSET ?2",params![item,(page-1)*50],|r|Ok(VideoNote{markdown_vault_id:r.get(6)?,markdown_path:r.get(7)?,id:r.get(0)?,item_id:r.get(1)?,time_ms:r.get(2)?,content:r.get(3)?,created_at:r.get(4)?,updated_at:r.get(5)?}))?;
        Ok(NotePage { notes, total, page })
    }
    pub fn save_note(&self, n: &NoteInput) -> AppResult<String> {
        let item = self.item(&n.item_id)?;
        if n.time_ms < 0 || n.time_ms > item.duration_ms.unwrap_or(604800000) {
            return Err("笔记时间必须在课程范围内".into());
        }
        let content = text(&n.content, 4000, true)?;
        let id = n.id.clone().unwrap_or_else(id);
        if n.id.is_some() {
            let linked: Option<String> = self
                .conn
                .query_row(
                    "SELECT markdown_path FROM video_notes WHERE id=?1 AND item_id=?2",
                    params![id, n.item_id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(db::error)?
                .flatten();
            if linked.is_some() {
                return Err("请在知识库中编辑此截图笔记".into());
            }

            if self.conn.execute("UPDATE video_notes SET time_ms=?1,content=?2,updated_at=?3 WHERE id=?4 AND item_id=?5",params![n.time_ms,content,now(),id,n.item_id]).map_err(db::error)?==0{return Err("笔记不存在或不属于当前课程".into());}
        } else {
            self.conn.execute("INSERT INTO video_notes(id,item_id,time_ms,content,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?5)",params![id,n.item_id,n.time_ms,content,now()]).map_err(db::error)?;
        }
        Ok(id)
    }
    pub fn delete_note(&self, id: &str) -> AppResult<()> {
        if self
            .conn
            .execute("DELETE FROM video_notes WHERE id=?1", [id])
            .map_err(db::error)?
            == 0
        {
            return Err("笔记不存在".into());
        }
        Ok(())
    }
}
