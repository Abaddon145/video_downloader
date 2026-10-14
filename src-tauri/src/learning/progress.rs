use super::{
    db,
    models::*,
    repository::{now, Repository},
    service::LearningService,
};
use crate::domain::AppResult;
use rusqlite::{params, OptionalExtension};
pub fn sequence() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}
impl Repository {
    pub fn validate_progress(&self, p: &ProgressInput) -> AppResult<()> {
        if p.position_ms < 0
            || p.duration_ms < 0
            || p.duration_ms > 604800000
            || p.position_ms > p.duration_ms
            || ![0.5, 0.75, 1., 1.25, 1.5, 1.75, 2.].contains(&p.playback_rate)
            || p.sequence == 0
            || p.sequence > 9007199254740991
        {
            return Err("播放进度或倍速无效".into());
        }
        self.item(&p.item_id)?;
        Ok(())
    }
    pub fn save_progress(&self, p: &ProgressInput) -> AppResult<()> {
        self.validate_progress(p)?;
        let item = self.item(&p.item_id)?;
        let old = item.progress;
        let seq: u64 = self
            .conn
            .query_row(
                "SELECT last_sequence FROM playback_progress WHERE item_id=?1",
                [&p.item_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db::error)?
            .unwrap_or(0);
        if p.sequence <= seq {
            return Ok(());
        }
        let duration = if p.duration_ms > 0 {
            p.duration_ms
        } else {
            old.duration_ms
        };
        let position = if p.position_ms < 5000 && old.position_ms >= 5000 {
            old.position_ms.min(duration)
        } else if p.position_ms < 5000 && !p.ended {
            0
        } else {
            p.position_ms
        };
        let completed = old.completed
            || p.ended
            || duration > 0
                && p.position_ms as f64 / duration as f64 >= self.settings()?.completion_threshold;
        let last = if p.position_ms >= 5000 || p.ended {
            Some(now())
        } else {
            old.last_played_at
        };
        self.conn.execute("INSERT INTO playback_progress(item_id,position_ms,duration_ms,completed,playback_rate,last_played_at,updated_at,last_sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(item_id) DO UPDATE SET position_ms=excluded.position_ms,duration_ms=excluded.duration_ms,completed=excluded.completed,playback_rate=excluded.playback_rate,last_played_at=excluded.last_played_at,updated_at=excluded.updated_at,last_sequence=excluded.last_sequence",params![p.item_id,position,duration,completed,p.playback_rate,last,now(),p.sequence]).map_err(db::error)?;
        Ok(())
    }
    pub fn mark_completed(&self, id: &str, completed: bool) -> AppResult<()> {
        let item = self.item(id)?;
        let p = item.progress;
        self.conn.execute("INSERT INTO playback_progress(item_id,position_ms,duration_ms,completed,playback_rate,last_played_at,updated_at,last_sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(item_id) DO UPDATE SET position_ms=excluded.position_ms,completed=excluded.completed,last_sequence=excluded.last_sequence,updated_at=excluded.updated_at",params![id,if completed{p.position_ms}else{0},p.duration_ms,completed,p.playback_rate,p.last_played_at,now(),sequence()]).map_err(db::error)?;
        Ok(())
    }
    pub fn continue_learning(&self) -> AppResult<Vec<ContinueLearningItem>> {
        let ids=self.rows("SELECT p.item_id FROM playback_progress p JOIN series_items i ON i.id=p.item_id WHERE p.completed=0 AND p.position_ms>=5000 AND p.last_played_at IS NOT NULL ORDER BY p.last_played_at DESC,p.item_id LIMIT 6",[],|r|r.get::<_,String>(0))?;
        ids.iter()
            .map(|id| {
                let item = self.item(id)?;
                Ok(ContinueLearningItem {
                    series: self.get_series(&item.series_id)?,
                    item,
                })
            })
            .collect()
    }
}
impl LearningService {
    pub fn live_progress(&self, p: ProgressInput, persist: bool) -> AppResult<()> {
        self.with(|r| {
            r.validate_progress(&p)?;
            let mut live = self.live.lock().map_err(|_| "进度保存暂不可用")?;
            let current = live.entry(p.item_id.clone()).or_insert_with(|| p.clone());
            if p.sequence > current.sequence {
                *current = p;
            }
            if persist {
                r.save_progress(current)?;
            }
            Ok(())
        })?;
        if persist {
            self.changed("progress", "");
        }
        Ok(())
    }
    pub fn flush_progress(&self) -> AppResult<()> {
        if self.live.lock().map_err(|_| "进度保存暂不可用")?.is_empty() {
            return Ok(());
        }
        self.with(|r| {
            let live = self.live.lock().map_err(|_| "进度保存暂不可用")?;
            for p in live.values() {
                if r.item(&p.item_id).is_ok() {
                    r.save_progress(p)?;
                }
            }
            Ok(())
        })
    }
}
