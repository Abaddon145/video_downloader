use super::models::*;
use crate::{domain::AppResult, learning::db};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;
pub struct Repository {
    pub(crate) conn: Connection,
}
const COLUMNS:&str="id,item_id,source_path,source_fingerprint,status,analysis_mode,model,prompt_version,language,duration_ms,progress,stage,summary_json,markdown_path,markdown_vault_id,error,created_at,updated_at,completed_at,options_json,transcript_done,keyframes_done";
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<VideoAnalysis> {
    let mode: String = r.get(5)?;
    let json: Option<String> = r.get(12)?;
    let summary = json
        .map(|s| {
            serde_json::from_str(&s).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    12,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        })
        .transpose()?;
    let raw: String = r.get(19)?;
    if raw.len() > 4096 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let options: AnalysisOptions = serde_json::from_str(&raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(19, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(VideoAnalysis {
        options,
        transcript_done: r.get(20)?,
        keyframes_done: r.get(21)?,
        id: r.get(0)?,
        item_id: r.get(1)?,
        source_path: r.get(2)?,
        source_fingerprint: r.get(3)?,
        status: r.get(4)?,
        analysis_mode: match mode.as_str() {
            "quick" => AnalysisMode::Quick,
            "deep" => AnalysisMode::Deep,
            _ => AnalysisMode::Standard,
        },
        model: r.get(6)?,
        prompt_version: r.get(7)?,
        language: r.get(8)?,
        duration_ms: r.get(9)?,
        progress: r.get(10)?,
        stage: r.get(11)?,
        summary,
        markdown_path: r.get(13)?,
        markdown_vault_id: r.get(14)?,
        error: r.get(15)?,
        created_at: r.get(16)?,
        updated_at: r.get(17)?,
        completed_at: r.get(18)?,
    })
}
impl Repository {
    pub fn open(path: &Path) -> AppResult<Self> {
        Ok(Self {
            conn: db::open(path)?,
        })
    }
    pub fn recover(&self) -> AppResult<usize> {
        self.conn.execute("UPDATE video_analyses SET status='interrupted',stage='分析被中断，可继续分析',updated_at=?1 WHERE status IN ('queued','extracting_audio','transcribing','extracting_keyframes','analyzing_chunks','finalizing','writing_note')",[crate::service::now()]).map_err(db::error)
    }
    pub fn list(&self, status: Option<&str>, page: u32) -> AppResult<AnalysisPage> {
        let status = status.filter(|s| !s.is_empty());
        if status.is_some_and(|s| {
            !["active", "completed", "failed", "cancelled", "interrupted"].contains(&s)
        }) {
            return Err("分析筛选条件无效".into());
        }
        let filter="(?1 IS NULL OR status=?1 OR (?1='active' AND status IN ('queued','extracting_audio','transcribing','extracting_keyframes','analyzing_chunks','finalizing','writing_note')))";
        let total: u64 = self
            .conn
            .query_row(
                &format!("SELECT count(*) FROM video_analyses WHERE {filter}"),
                [status],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        let page = page.max(1).min(((total + 49) / 50).max(1) as u32);
        let mut stmt=self.conn.prepare(&format!("SELECT {COLUMNS} FROM video_analyses WHERE {filter} ORDER BY created_at DESC,id LIMIT 50 OFFSET ?2")).map_err(db::error)?;
        let items = stmt
            .query_map(rusqlite::params![status, (page - 1) * 50], row)
            .map_err(db::error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db::error)?;
        Ok(AnalysisPage { items, total, page })
    }
    pub fn get(&self, id: &str) -> AppResult<VideoAnalysis> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM video_analyses WHERE id=?1"),
                [id],
                row,
            )
            .optional()
            .map_err(db::error)?
            .ok_or_else(|| "找不到分析记录".into())
    }
}

impl Repository {
    pub fn save_transcript(
        &mut self,
        id: &str,
        language: Option<&str>,
        segments: &[TranscriptSegment],
    ) -> AppResult<()> {
        let tx = self.conn.transaction().map_err(db::error)?;
        let fts = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='transcript_fts')",
                [],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false);
        if fts {
            tx.execute("DELETE FROM transcript_fts WHERE analysis_id=?1", [id])
                .map_err(db::error)?;
        }
        tx.execute("DELETE FROM transcript_segments WHERE analysis_id=?1", [id])
            .map_err(db::error)?;
        for segment in segments {
            tx.execute("INSERT INTO transcript_segments(id,analysis_id,segment_index,start_ms,end_ms,text) VALUES(?1,?2,?3,?4,?5,?6)",rusqlite::params![segment.id,id,segment.segment_index,segment.start_ms,segment.end_ms,segment.text]).map_err(db::error)?;
            if fts {
                tx.execute(
                    "INSERT INTO transcript_fts(id,analysis_id,text) VALUES(?1,?2,?3)",
                    rusqlite::params![segment.id, id, segment.text],
                )
                .map_err(db::error)?;
            }
        }
        tx.execute(
            "UPDATE video_analyses SET language=?2,transcript_done=1,updated_at=?3 WHERE id=?1",
            rusqlite::params![id, language, crate::service::now()],
        )
        .map_err(db::error)?;
        tx.commit().map_err(db::error)
    }
    pub fn transcript(&self, id: &str, page: u32) -> AppResult<TranscriptPage> {
        self.get(id)?;
        let total: u64 = self
            .conn
            .query_row(
                "SELECT count(*) FROM transcript_segments WHERE analysis_id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        let page = page.max(1).min(((total + 49) / 50).max(1) as u32);
        let mut stmt=self.conn.prepare("SELECT id,analysis_id,segment_index,start_ms,end_ms,text FROM transcript_segments WHERE analysis_id=?1 ORDER BY segment_index LIMIT 50 OFFSET ?2").map_err(db::error)?;
        let items = stmt
            .query_map(rusqlite::params![id, (page - 1) * 50], |r| {
                Ok(TranscriptSegment {
                    id: r.get(0)?,
                    analysis_id: r.get(1)?,
                    segment_index: r.get(2)?,
                    start_ms: r.get(3)?,
                    end_ms: r.get(4)?,
                    text: r.get(5)?,
                })
            })
            .map_err(db::error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db::error)?;
        Ok(TranscriptPage { items, total, page })
    }
}

impl Repository {
    pub fn save_keyframes(&mut self, id: &str, frames: &[AnalysisKeyframe]) -> AppResult<()> {
        let tx = self.conn.transaction().map_err(db::error)?;
        tx.execute("DELETE FROM analysis_keyframes WHERE analysis_id=?1", [id])
            .map_err(db::error)?;
        for f in frames {
            tx.execute("INSERT INTO analysis_keyframes(id,analysis_id,time_ms,path,sha256,scene_score) VALUES(?1,?2,?3,?4,?5,?6)",rusqlite::params![f.id,id,f.time_ms,f.path,f.sha256,f.scene_score]).map_err(db::error)?;
        }
        tx.execute(
            "UPDATE video_analyses SET keyframes_done=1,updated_at=?2 WHERE id=?1",
            rusqlite::params![id, crate::service::now()],
        )
        .map_err(db::error)?;
        tx.commit().map_err(db::error)
    }
    pub fn keyframes(&self, id: &str) -> AppResult<Vec<AnalysisKeyframe>> {
        self.get(id)?;
        let mut stmt=self.conn.prepare("SELECT id,analysis_id,time_ms,path,sha256,scene_score FROM analysis_keyframes WHERE analysis_id=?1 ORDER BY time_ms LIMIT 60").map_err(db::error)?;
        let rows = stmt
            .query_map([id], |r| {
                Ok(AnalysisKeyframe {
                    id: r.get(0)?,
                    analysis_id: r.get(1)?,
                    time_ms: r.get(2)?,
                    path: r.get(3)?,
                    sha256: r.get(4)?,
                    scene_score: r.get(5)?,
                })
            })
            .map_err(db::error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db::error)?;
        Ok(rows)
    }
}

impl Repository {
    pub fn all_transcript(&self, id: &str) -> AppResult<Vec<TranscriptSegment>> {
        let mut statement=self.conn.prepare("SELECT id,analysis_id,segment_index,start_ms,end_ms,text FROM transcript_segments WHERE analysis_id=?1 ORDER BY segment_index LIMIT 200001").map_err(db::error)?;
        let rows = statement
            .query_map([id], |r| {
                Ok(TranscriptSegment {
                    id: r.get(0)?,
                    analysis_id: r.get(1)?,
                    segment_index: r.get(2)?,
                    start_ms: r.get(3)?,
                    end_ms: r.get(4)?,
                    text: r.get(5)?,
                })
            })
            .map_err(db::error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db::error)?;
        if rows.len() > 200000 {
            return Err("转录段落过多，请先分割视频".into());
        }
        Ok(rows)
    }
    pub fn prepare_chunks(
        &mut self,
        id: &str,
        chunks: &[super::analysis::AnalysisChunkInput],
    ) -> AppResult<()> {
        self.get(id)?;
        let tx = self.conn.transaction().map_err(db::error)?;
        let existing=tx.prepare("SELECT id,chunk_index FROM analysis_chunks WHERE analysis_id=?1 ORDER BY chunk_index").map_err(db::error)?.query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,usize>(1)?))).map_err(db::error)?.collect::<Result<Vec<_>,_>>().map_err(db::error)?;
        if !existing.is_empty()
            && (existing.len() != chunks.len()
                || existing.iter().any(|(saved, index)| {
                    chunks.get(*index).is_none_or(|input| input.id != *saved)
                }))
        {
            return Err("本地素材或上传权限已变化，请重新分析或重新生成总结".into());
        }
        for c in chunks {
            tx.execute("INSERT OR IGNORE INTO analysis_chunks(id,analysis_id,chunk_index,start_ms,end_ms,status) VALUES(?1,?2,?3,?4,?5,'queued')",rusqlite::params![c.id,id,c.chunk_index,c.start_ms,c.end_ms]).map_err(db::error)?;
        }
        tx.execute("UPDATE analysis_chunks SET status='queued' WHERE analysis_id=?1 AND status IN ('running','failed')",[id]).map_err(db::error)?;
        tx.commit().map_err(db::error)
    }
    pub fn cached_chunk(
        &self,
        chunk: &super::analysis::AnalysisChunkInput,
    ) -> AppResult<Option<VideoAnalysisResult>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT result_json FROM analysis_chunks WHERE id=?1 AND status='completed'",
                [&chunk.id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db::error)?
            .flatten();
        value
            .map(|json| super::analysis::parse_result(&json, &chunk.scope()))
            .transpose()
    }
    pub fn chunk_status(
        &self,
        id: &str,
        status: &str,
        result: Option<&VideoAnalysisResult>,
    ) -> AppResult<()> {
        if !["running", "completed", "failed"].contains(&status) {
            return Err("分段分析状态无效".into());
        }
        let json = result
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| "无法保存分段分析")?;
        let changed=self.conn.execute("UPDATE analysis_chunks SET status=?2,result_json=COALESCE(?3,result_json) WHERE id=?1 AND status<>'completed' AND EXISTS(SELECT 1 FROM video_analyses a WHERE a.id=analysis_chunks.analysis_id AND a.status='analyzing_chunks')",rusqlite::params![id,status,json]).map_err(db::error)?;
        if changed != 1 {
            return Err("分段分析已取消或状态已变化".into());
        }
        Ok(())
    }
    pub fn completed_chunks(&self, id: &str) -> AppResult<u64> {
        self.conn
            .query_row(
                "SELECT count(*) FROM analysis_chunks WHERE analysis_id=?1 AND status='completed'",
                [id],
                |r| r.get(0),
            )
            .map_err(db::error)
    }
}
