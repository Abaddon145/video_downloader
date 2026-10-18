use super::{
    analysis::{parse_result, Scope},
    client::{DeepSeekClient, RequestPurpose},
    jobs::Control,
    models::*,
    service::AiService,
};
use crate::domain::AppResult;
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path, sync::Arc};
pub const FINAL_TEXT_BYTES: usize = 120000;
const MAX_SUMMARY_REQUESTS: u32 = 64;
const MAX_RECOVERY_DEPTH: u8 = 8;
const SUMMARY_OUTPUT_LIMITS: &str = "全片总结必须简洁，完整JSON总正文不超过8000字。概述最多1000字；章节最多20个、重点最多20个、教程步骤最多16个、术语最多20个，每条说明最多100字。合并相似内容，不重复逐段复述；详细依据仍保留在分段结果中。所有原有来源和时间校验规则继续有效。";
const COMPACT_OUTPUT_LIMITS: &str = "上次响应因输出过长被截断。这次必须返回完整的精简JSON，不得解释重试过程。总正文不超过2500字，概述最多400字，章节最多8个、重点最多8个、步骤最多6个、术语最多8个，每条说明最多60字。仍保留所有必需字段，允许数组为空，不得伪造来源。";
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
        let mut remaining_requests = MAX_SUMMARY_REQUESTS;
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
                    .summarize_group(
                        id,
                        &analysis,
                        &units,
                        &segments,
                        &frames,
                        control,
                        client,
                        &mut remaining_requests,
                        0,
                    )
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
                    .summarize_group(
                        id,
                        &analysis,
                        &group,
                        &segments,
                        &frames,
                        control,
                        client,
                        &mut remaining_requests,
                        0,
                    )
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
        remaining_requests: &mut u32,
        recovery_depth: u8,
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
        let mut messages = final_messages(
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
        // The cache identity remains the source summaries, so prior successful work survives retries.
        let original_system = messages[0]["content"]
            .as_str()
            .ok_or("总结规则无效")?
            .to_owned();
        let mut compact = false;
        let mut result = loop {
            control.check()?;
            self.check_consent(&analysis.options)?;
            Self::check_source(
                Path::new(&analysis.source_path),
                &analysis.source_fingerprint,
            )?;
            if *remaining_requests == 0 {
                return Err("本次整理已达到安全请求预算，已保存完成的内容；可稍后继续分析".into());
            }
            messages[0]["content"] = json!(format!(
                "{}\n{}",
                original_system,
                if compact {
                    COMPACT_OUTPUT_LIMITS
                } else {
                    SUMMARY_OUTPUT_LIMITS
                }
            ));
            let key = self.key()?;
            *remaining_requests -= 1;
            let reply = control
                .reply(client.chat_for(
                    key.as_bytes(),
                    &analysis.model,
                    messages.clone(),
                    RequestPurpose::Summary,
                ))
                .await?;
            self.with(|r| r.record_usage(Some(id), "summary", &reply.model, &reply.usage))?;
            control.check()?;
            self.check_consent(&analysis.options)?;
            Self::check_source(
                Path::new(&analysis.source_path),
                &analysis.source_fingerprint,
            )?;
            if reply.truncated {
                if !compact {
                    compact = true;
                    continue;
                }
                if units.len() <= 1 || recovery_depth >= MAX_RECOVERY_DEPTH {
                    return Err(
                        "AI多次未返回完整总结，已保留分段结果，可继续分析；不会保存不完整内容"
                            .into(),
                    );
                }
                let middle = units.len() / 2;
                let mut smaller = Vec::new();
                for group in [&units[..middle], &units[middle..]] {
                    let result = Box::pin(self.summarize_group(
                        id,
                        analysis,
                        group,
                        segments,
                        frames,
                        control,
                        client,
                        remaining_requests,
                        recovery_depth + 1,
                    ))
                    .await?;
                    smaller.push(Unit {
                        start: group.first().unwrap().start,
                        end: group.last().unwrap().end,
                        result,
                    });
                }
                break Box::pin(self.summarize_group(
                    id,
                    analysis,
                    &smaller,
                    segments,
                    frames,
                    control,
                    client,
                    remaining_requests,
                    recovery_depth + 1,
                ))
                .await?;
            }
            break self
                .parse_result_with_repair(
                    id,
                    analysis,
                    reply.checked_content()?,
                    &scope,
                    control,
                    client,
                )
                .await?;
        };
        for warning in units.iter().flat_map(|unit| &unit.result.warnings) {
            if !result.warnings.contains(warning) {
                if warning == super::references::OMITTED_KEYPOINT_WARNING {
                    result.warnings.truncate(7);
                }
                if result.warnings.len() < 8 {
                    result.warnings.push(warning.clone());
                }
            }
        }
        super::analysis::validate_result(&result, &scope)?;
        let bytes = serde_json::to_vec(&result).map_err(|_| "无法保存全片总结")?;
        self.owned(id, control, |_| {
            std::fs::create_dir_all(&directory).map_err(|_| "无法创建总结缓存")?;
            crate::native::save_atomic(&file, &bytes)
        })?;
        Ok(result)
    }
}
