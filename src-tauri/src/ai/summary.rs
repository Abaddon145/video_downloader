use super::{
    analysis::{parse_result, Scope},
    client::DeepSeekClient,
    jobs::Control,
    models::*,
    service::AiService,
};
use crate::domain::AppResult;
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path, sync::Arc};
pub const FINAL_TEXT_BYTES: usize = 120000;
pub fn final_messages(results: &[VideoAnalysisResult], scope: &Scope<'_>) -> AppResult<Vec<Value>> {
    let ids = results
        .iter()
        .flat_map(|r| {
            r.key_points
                .iter()
                .flat_map(|p| p.source_segment_ids.iter().map(String::as_str))
        })
        .collect::<HashSet<_>>();
    let sources = scope
        .segments
        .iter()
        .filter(|s| ids.contains(s.id.as_str()))
        .map(|s| json!({"id":s.id,"startMs":s.start_ms,"endMs":s.end_ms}))
        .collect::<Vec<_>>();
    let frames = scope
        .frames
        .iter()
        .map(|f| json!({"id":f.id,"timeMs":f.time_ms}))
        .collect::<Vec<_>>();
    let content=serde_json::to_string(&json!({"startMs":scope.start_ms,"endMs":scope.end_ms,"partSummaries":results,"availableSegments":sources,"availableKeyframes":frames})).map_err(|_|"无法整理全片总结素材")?;
    if content.len() > FINAL_TEXT_BYTES {
        return Err("分段总结较长，需要分组整理".into());
    }
    Ok(vec![
        json!({"role":"system","content":format!("{}\n{}\n合并提供的分段结构化分析，去重并整理全片。不要填补未提供的事实，不要上传要求，不要输出本机操作。只使用给定来源id；输入不含原始字幕和图片。",super::prompts::SYSTEM_RULES,super::prompts::ANALYSIS_FORMAT)}),
        json!({"role":"user","content":[{"type":"text","text":content}]}),
    ])
}
#[derive(Clone)]
struct Unit {
    start: i64,
    end: i64,
    result: VideoAnalysisResult,
}
fn restricted<'a>(
    units: &[Unit],
    segments: &'a [TranscriptSegment],
    frames: &'a [AnalysisKeyframe],
) -> (i64, i64, Vec<TranscriptSegment>, Vec<AnalysisKeyframe>) {
    let start = units.first().map(|u| u.start).unwrap_or(0);
    let end = units.last().map(|u| u.end).unwrap_or(1);
    let ids = units
        .iter()
        .flat_map(|u| {
            u.result
                .key_points
                .iter()
                .flat_map(|p| p.source_segment_ids.iter().map(String::as_str))
        })
        .collect::<HashSet<_>>();
    (
        start,
        end,
        segments
            .iter()
            .filter(|s| ids.contains(s.id.as_str()) && s.end_ms > start && s.start_ms < end)
            .cloned()
            .collect(),
        frames
            .iter()
            .filter(|f| f.time_ms >= start && f.time_ms < end)
            .cloned()
            .collect(),
    )
}
impl AiService {
    pub(crate) async fn summarize(
        self: &Arc<Self>,
        id: &str,
        results: Vec<VideoAnalysisResult>,
        control: &Arc<Control>,
        client: &DeepSeekClient,
    ) -> AppResult<VideoAnalysisResult> {
        let analysis = self.analysis(id)?;
        self.check_consent(&analysis.options)?;
        let(segments,frames,bounds)=self.with(|r|{let mut st=r.conn.prepare("SELECT start_ms,end_ms FROM analysis_chunks WHERE analysis_id=?1 AND status='completed' ORDER BY chunk_index").map_err(crate::learning::db::error)?;let bounds=st.query_map([id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?))).map_err(crate::learning::db::error)?.collect::<Result<Vec<_>,_>>().map_err(crate::learning::db::error)?;Ok((if analysis.options.allow_transcript{r.all_transcript(id)?}else{vec![]},if analysis.options.allow_images{r.keyframes(id)?}else{vec![]},bounds))})?;
        if results.is_empty() || results.len() != bounds.len() {
            return Err("分段结果不完整，请先继续分析".into());
        }
        let mut units = results
            .into_iter()
            .zip(bounds)
            .map(|(result, (start, end))| Unit { start, end, result })
            .collect::<Vec<_>>();
        for level in 0..=8 {
            let (start, end, s, f) = restricted(&units, &segments, &frames);
            let scope = Scope {
                start_ms: start,
                end_ms: end,
                segments: &s,
                frames: &f,
            };
            let all = units.iter().map(|u| u.result.clone()).collect::<Vec<_>>();
            if final_messages(&all, &scope).is_ok() {
                return self
                    .summarize_group(id, &analysis, &units, &segments, &frames, control, client)
                    .await;
            }
            if level == 8 {
                return Err("课程总结仍过于庞大，请先分割视频再分析".into());
            }
            let mut groups = Vec::<Vec<Unit>>::new();
            let mut group = Vec::new();
            for unit in units {
                let mut candidate = group.clone();
                candidate.push(unit.clone());
                let (start, end, s, f) = restricted(&candidate, &segments, &frames);
                let scope = Scope {
                    start_ms: start,
                    end_ms: end,
                    segments: &s,
                    frames: &f,
                };
                if final_messages(
                    &candidate
                        .iter()
                        .map(|u| u.result.clone())
                        .collect::<Vec<_>>(),
                    &scope,
                )
                .is_err()
                {
                    if group.is_empty() {
                        return Err("单段分析过于详细，请重新生成简洁总结".into());
                    }
                    groups.push(group);
                    group = vec![unit];
                } else {
                    group = candidate;
                }
            }
            if !group.is_empty() {
                groups.push(group);
            }
            let mut next = Vec::new();
            for group in groups {
                let result = self
                    .summarize_group(id, &analysis, &group, &segments, &frames, control, client)
                    .await?;
                next.push(Unit {
                    start: group.first().unwrap().start,
                    end: group.last().unwrap().end,
                    result,
                });
            }
            units = next;
        }
        Err("全片总结未能完成".into())
    }
    async fn summarize_group(
        self: &Arc<Self>,
        id: &str,
        analysis: &VideoAnalysis,
        units: &[Unit],
        segments: &[TranscriptSegment],
        frames: &[AnalysisKeyframe],
        control: &Arc<Control>,
        client: &DeepSeekClient,
    ) -> AppResult<VideoAnalysisResult> {
        control.check()?;
        self.check_consent(&analysis.options)?;
        Self::check_source(
            Path::new(&analysis.source_path),
            &analysis.source_fingerprint,
        )?;
        let (start, end, s, f) = restricted(units, segments, frames);
        let scope = Scope {
            start_ms: start,
            end_ms: end,
            segments: &s,
            frames: &f,
        };
        let messages = final_messages(
            &units.iter().map(|u| u.result.clone()).collect::<Vec<_>>(),
            &scope,
        )?;
        use sha2::{Digest, Sha256};
        let bytes = serde_json::to_vec(&(
            &analysis.model,
            super::prompts::ANALYSIS_PROMPT_VERSION,
            &messages,
        ))
        .map_err(|_| "无法校验总结素材")?;
        let hash = format!("{:x}", Sha256::digest(bytes));
        let directory = self.data.join("ai/cache").join(id).join("summary-cache");
        let file = directory.join(format!("{hash}.json"));
        if file.exists() {
            if let Ok(owned) =
                crate::learning::cache::owned(&self.data.join("ai/cache").join(id), &file)
            {
                if std::fs::metadata(&owned).is_ok_and(|m| m.len() <= 1024 * 1024) {
                    if let Ok(text) = std::fs::read_to_string(owned) {
                        if let Ok(result) = parse_result(&text, &scope) {
                            return Ok(result);
                        }
                    }
                }
            }
        }
        let key = self.key()?;
        let reply = control
            .reply(client.chat(key.as_bytes(), &analysis.model, messages))
            .await?;
        self.with(|r| r.record_usage(Some(id), "summary", &reply.model, &reply.usage))?;
        control.check()?;
        self.check_consent(&analysis.options)?;
        Self::check_source(
            Path::new(&analysis.source_path),
            &analysis.source_fingerprint,
        )?;
        let result = parse_result(reply.checked_content()?, &scope)?;
        let bytes = serde_json::to_vec(&result).map_err(|_| "无法保存全片总结")?;
        self.owned(id, control, |_| {
            std::fs::create_dir_all(&directory).map_err(|_| "无法创建总结缓存")?;
            crate::native::save_atomic(&file, &bytes)
        })?;
        Ok(result)
    }
}
