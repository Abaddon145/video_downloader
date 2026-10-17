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
        if !["chunk", "summary", "question", "explain"].contains(&operation)
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
