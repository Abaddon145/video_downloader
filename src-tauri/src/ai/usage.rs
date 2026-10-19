use super::{client::TokenUsage, repository::Repository};
use crate::{domain::AppResult, learning::db};
impl Repository {
    pub fn record_usage(
        &self,
        analysis_id: Option<&str>,
        operation: &str,
        model: &str,
        usage: &TokenUsage,
    ) -> AppResult<()> {
        if ![
            "chunk",
            "summary",
            "question",
            "explain",
            "reference_repair",
            "note_organize",
        ]
        .contains(&operation)
            || model.len() > 128
            || [
                usage.input_tokens,
                usage.output_tokens,
                usage.cached_tokens,
                usage.cache_miss_tokens,
                usage.reasoning_tokens,
            ]
            .into_iter()
            .flatten()
            .any(|n| n > i64::MAX as u64)
        {
            return Err("AI用量记录无效".into());
        }
        let id = crate::ffmpeg::runner::new_id().replace("media-", "usage-");
        let json = serde_json::to_string(usage).map_err(|_| "无法整理AI用量")?;
        self.conn.execute("INSERT INTO ai_usage(id,analysis_id,operation,model,input_tokens,output_tokens,cached_tokens,usage_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",rusqlite::params![id,analysis_id,operation,model,usage.input_tokens,usage.output_tokens,usage.cached_tokens,json,crate::service::now()]).map_err(db::error)?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageModel {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub unknown_requests: u64,
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageView {
    pub analyses: u64,
    pub questions: u64,
    pub note_requests: u64,
    pub models: Vec<UsageModel>,
}
impl super::service::AiService {
    pub fn usage(&self, from_ms: i64, to_ms: i64) -> AppResult<UsageView> {
        if from_ms < 0 || to_ms <= from_ms || to_ms > crate::service::now() as i64 + 86400000 {
            return Err("用量统计时间范围无效".into());
        }
        self.with(|r|{
   let (analyses,questions)=r.conn.query_row("SELECT count(DISTINCT CASE WHEN operation IN ('chunk','summary') THEN analysis_id END),COALESCE(sum(CASE WHEN operation IN ('question','explain') THEN 1 ELSE 0 END),0) FROM ai_usage WHERE created_at>=?1 AND created_at<?2",rusqlite::params![from_ms,to_ms],|r|Ok((r.get(0)?,r.get(1)?))).map_err(db::error)?;
   let mut statement=r.conn.prepare("SELECT model,COALESCE(sum(input_tokens),0),COALESCE(sum(output_tokens),0),COALESCE(sum(cached_tokens),0),sum(CASE WHEN input_tokens IS NULL OR output_tokens IS NULL OR cached_tokens IS NULL THEN 1 ELSE 0 END) FROM ai_usage WHERE created_at>=?1 AND created_at<?2 GROUP BY model ORDER BY model").map_err(db::error)?;
   let models=statement.query_map(rusqlite::params![from_ms,to_ms],|r|Ok(UsageModel{model:r.get(0)?,input_tokens:r.get(1)?,output_tokens:r.get(2)?,cached_tokens:r.get(3)?,unknown_requests:r.get(4)?})).map_err(db::error)?.collect::<Result<Vec<_>,_>>().map_err(db::error)?;
   let note_requests=r.conn.query_row("SELECT count(*) FROM ai_usage WHERE operation='note_organize' AND created_at>=?1 AND created_at<?2",rusqlite::params![from_ms,to_ms],|r|r.get(0)).map_err(db::error)?;
   Ok(UsageView{analyses,questions,note_requests,models})
  })
    }
}
