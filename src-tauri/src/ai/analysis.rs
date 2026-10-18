use super::{
    keyframes::{self, MAX_IMAGES_PER_REQUEST},
    models::*,
    transcription as t,
};
use crate::domain::AppResult;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeSet, HashSet},
    path::Path,
};
pub const MAX_TRANSCRIPT_BYTES: usize = 18000;
pub const MAX_CHUNKS: usize = 4096;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisChunkInput {
    pub id: String,
    pub analysis_id: String,
    pub chunk_index: u32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub segments: Vec<TranscriptSegment>,
    pub frames: Vec<AnalysisKeyframe>,
}
pub struct Scope<'a> {
    pub start_ms: i64,
    pub end_ms: i64,
    pub segments: &'a [TranscriptSegment],
    pub frames: &'a [AnalysisKeyframe],
}
impl AnalysisChunkInput {
    pub fn scope(&self) -> Scope<'_> {
        Scope {
            start_ms: self.start_ms,
            end_ms: self.end_ms,
            segments: &self.segments,
            frames: &self.frames,
        }
    }
}
fn string(s: &str, max: usize, required: bool) -> AppResult<()> {
    if (required && s.trim().is_empty()) || s.chars().count() > max || s.contains('\0') {
        Err("AI结果中的文本为空或超过长度限制".into())
    } else {
        Ok(())
    }
}
fn strings(values: &[String], count: usize, len: usize) -> AppResult<()> {
    if values.len() > count {
        return Err("AI结果条目过多".into());
    }
    for s in values {
        string(s, len, true)?;
    }
    Ok(())
}
fn time(time: i64, scope: &Scope<'_>) -> AppResult<()> {
    if time < scope.start_ms || time > scope.end_ms {
        Err("AI返回的时间点超出真实媒体范围".into())
    } else {
        Ok(())
    }
}
pub fn validate_result(r: &VideoAnalysisResult, s: &Scope<'_>) -> AppResult<()> {
    if s.start_ms < 0 || s.end_ms <= s.start_ms {
        return Err("分析的真实时间范围无效".into());
    }
    if !["tutorial", "lecture", "interview", "documentary", "other"]
        .contains(&r.content_type.as_str())
    {
        return Err("AI返回的内容类型无效".into());
    }
    string(&r.one_sentence_summary, 1000, true)?;
    string(&r.overview, 20000, true)?;
    strings(&r.action_items, 100, 2000)?;
    strings(&r.tags, 32, 64)?;
    if r.chapters.len() > 200
        || r.key_points.len() > 200
        || r.tutorial_steps.len() > 200
        || r.terms.len() > 200
    {
        return Err("AI分析结果条目过多".into());
    }
    let segments = s
        .segments
        .iter()
        .map(|seg| seg.id.as_str())
        .collect::<HashSet<_>>();
    let frames = s
        .frames
        .iter()
        .map(|f| f.id.as_str())
        .collect::<HashSet<_>>();
    let mut previous = s.start_ms;
    for c in &r.chapters {
        time(c.start_ms, s)?;
        time(c.end_ms, s)?;
        if c.start_ms >= c.end_ms || c.start_ms < previous {
            return Err("AI章节必须按时间排序且不得重叠".into());
        }
        previous = c.end_ms;
        string(&c.title, 200, true)?;
        string(&c.summary, 6000, true)?;
        strings(&c.keywords, 24, 100)?;
        if c.keyframe_ids.len() > 60
            || c.keyframe_ids
                .iter()
                .any(|id| !frames.contains(id.as_str()))
        {
            return Err("AI引用了未提供的画面".into());
        }
    }
    for p in &r.key_points {
        time(p.time_ms, s)?;
        string(&p.title, 200, true)?;
        string(&p.content, 6000, true)?;
        if !(1..=5).contains(&p.importance)
            || p.source_segment_ids.len() > 32
            || p.source_segment_ids
                .iter()
                .any(|id| !segments.contains(id.as_str()))
        {
            return Err("AI重点的来源或重要程度无效".into());
        }
        if p.source_segment_ids.is_empty() {
            if !s
                .frames
                .iter()
                .any(|f| (f.time_ms - p.time_ms).abs() <= 15000)
            {
                return Err("AI重点缺少可验证的字幕或画面来源".into());
            }
        } else if !s.segments.iter().any(|seg| {
            p.source_segment_ids.contains(&seg.id)
                && p.time_ms >= seg.start_ms
                && p.time_ms <= seg.end_ms
        }) {
            return Err("AI重点时间与引用字幕不一致".into());
        }
    }
    let mut previous = s.start_ms;
    for (index, step) in r.tutorial_steps.iter().enumerate() {
        time(step.time_ms, s)?;
        if step.order != index as u32 + 1 || step.time_ms < previous {
            return Err("AI教程步骤顺序无效".into());
        }
        previous = step.time_ms;
        string(&step.title, 200, true)?;
        string(&step.description, 6000, true)?;
        strings(&step.notes, 32, 2000)?;
    }
    for term in &r.terms {
        time(term.time_ms, s)?;
        string(&term.name, 200, true)?;
        string(&term.definition, 4000, true)?;
    }
    Ok(())
}
pub fn parse_result(text: &str, scope: &Scope<'_>) -> AppResult<VideoAnalysisResult> {
    if text.len() > 1024 * 1024 {
        return Err("AI分析JSON过大".into());
    }
    let result = serde_json::from_str(text).map_err(|_| "AI未返回符合约定的JSON结构，请重试")?;
    validate_result(&result, scope)?;
    Ok(result)
}
pub fn build_chunks(
    id: &str,
    duration_ms: i64,
    mode: AnalysisMode,
    segments: &[TranscriptSegment],
    frames: &[AnalysisKeyframe],
    allow_text: bool,
    allow_images: bool,
) -> AppResult<Vec<AnalysisChunkInput>> {
    if !allow_text && !allow_images {
        return Err("至少需要允许发送字幕或关键帧中的一种".into());
    }
    if duration_ms <= 0 || duration_ms > 604800000 {
        return Err("媒体真实时长无效".into());
    }
    let segments = if allow_text { segments } else { &[] };
    let frames = if allow_images { frames } else { &[] };
    if segments.is_empty() && frames.is_empty() {
        return Err("没有可发送的字幕或画面，请检查隐私设置与本地处理结果".into());
    }
    if segments.iter().any(|s| {
        s.analysis_id != id
            || s.start_ms < 0
            || s.end_ms <= s.start_ms
            || s.end_ms > duration_ms
            || s.text.len() > 16000
    }) || frames
        .iter()
        .any(|f| f.analysis_id != id || f.time_ms < 0 || f.time_ms >= duration_ms)
    {
        return Err("本地分析素材的来源或时间无效".into());
    }
    let target = match mode {
        AnalysisMode::Quick => 600000,
        AnalysisMode::Standard => 300000,
        AnalysisMode::Deep => 180000,
    };
    let mut boundaries = BTreeSet::from([0, duration_ms]);
    let mut at = target;
    while at < duration_ms {
        boundaries.insert(at);
        at += target;
    }
    loop {
        if boundaries.len() > MAX_CHUNKS + 1 {
            return Err("分析分段数量过多，请先使用媒体工具分割视频".into());
        }
        let bounds = boundaries.iter().copied().collect::<Vec<_>>();
        let mut split = None;
        for pair in bounds.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let selected = segments
                .iter()
                .filter(|s| s.start_ms < b && s.end_ms > a)
                .collect::<Vec<_>>();
            let images = frames
                .iter()
                .filter(|f| f.time_ms >= a && f.time_ms < b)
                .collect::<Vec<_>>();
            if selected.iter().map(|s| s.text.len()).sum::<usize>() <= MAX_TRANSCRIPT_BYTES
                && images.len() <= MAX_IMAGES_PER_REQUEST
            {
                continue;
            }
            let mut cuts = selected
                .iter()
                .map(|s| s.start_ms)
                .chain(images.iter().map(|f| f.time_ms))
                .filter(|t| *t > a && *t < b)
                .collect::<Vec<_>>();
            cuts.sort_unstable();
            cuts.dedup();
            if cuts.is_empty() {
                return Err("字幕或画面过于密集，无法在安全上传限制内分段".into());
            }
            split = Some(cuts[cuts.len() / 2]);
            break;
        }
        if let Some(split) = split {
            boundaries.insert(split);
        } else {
            break;
        }
    }
    let bounds = boundaries.into_iter().collect::<Vec<_>>();
    let mut chunks = Vec::new();
    for pair in bounds.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let selected = segments
            .iter()
            .filter(|s| s.start_ms < b && s.end_ms > a)
            .map(|s| {
                let mut s = s.clone();
                s.start_ms = s.start_ms.max(a);
                s.end_ms = s.end_ms.min(b);
                s
            })
            .collect::<Vec<_>>();
        let images = frames
            .iter()
            .filter(|f| f.time_ms >= a && f.time_ms < b)
            .cloned()
            .collect::<Vec<_>>();
        if selected.is_empty() && images.is_empty() {
            continue;
        }
        let index = chunks.len() as u32;
        let mut chunk = AnalysisChunkInput {
            id: String::new(),
            analysis_id: id.into(),
            chunk_index: index,
            start_ms: a,
            end_ms: b,
            segments: selected,
            frames: images,
        };
        use sha2::{Digest, Sha256};
        let bytes = serde_json::to_vec(&chunk).map_err(|_| "无法构造分析分段")?;
        chunk.id = format!("{id}-chunk-{index}-{:x}", Sha256::digest(&bytes));
        chunks.push(chunk);
    }
    Ok(chunks)
}
pub fn messages(chunk: &AnalysisChunkInput, cache: &Path) -> AppResult<Vec<Value>> {
    if chunk.frames.len() > MAX_IMAGES_PER_REQUEST
        || chunk.segments.iter().map(|s| s.text.len()).sum::<usize>() > MAX_TRANSCRIPT_BYTES
    {
        return Err("本次AI上传超过安全数量或长度限制".into());
    }
    let transcript = chunk
        .segments
        .iter()
        .map(|s| json!({"id":s.id,"startMs":s.start_ms,"endMs":s.end_ms,"text":s.text}))
        .collect::<Vec<_>>();
    let frames = chunk
        .frames
        .iter()
        .map(|f| json!({"id":f.id,"timeMs":f.time_ms}))
        .collect::<Vec<_>>();
    let mut content = vec![
        json!({"type":"text","text":serde_json::to_string(&json!({"startMs":chunk.start_ms,"endMs":chunk.end_ms,"transcript":transcript,"keyframes":frames})).map_err(|_|"无法整理视频分析素材")?}),
    ];
    for frame in &chunk.frames {
        let path = crate::learning::cache::owned(cache, Path::new(&frame.path))?;
        let _lock = t::lock_source(&path)?;
        let meta = std::fs::metadata(&path).map_err(|_| "关键帧无法读取")?;
        if meta.len() > keyframes::MAX_JPEG_BYTES
            || path
                .extension()
                .is_none_or(|e| !e.eq_ignore_ascii_case("jpg"))
        {
            return Err("只允许上传经筛选的JPEG关键帧".into());
        }
        if t::file_hash(&path)? != frame.sha256 {
            return Err("关键帧文件已变化，请重新生成画面".into());
        }
        let bytes = std::fs::read(path).map_err(|_| "关键帧无法读取")?;
        let (w, h) = keyframes::jpeg_dimensions(&bytes)?;
        if w > 1280 || h > 1280 {
            return Err("关键帧尺寸超出上传范围".into());
        }
        content.push(json!({"type":"image_url","image_url":{"url":format!("data:image/jpeg;base64,{}",STANDARD.encode(bytes))}}));
    }
    Ok(vec![
        json!({"role":"system","content":format!("{}\n{}",super::prompts::SYSTEM_RULES,super::prompts::ANALYSIS_FORMAT)}),
        json!({"role":"user","content":content}),
    ])
}

impl super::service::AiService {
    pub(crate) fn check_consent(&self, options: &AnalysisOptions) -> AppResult<()> {
        let current = self.settings()?.settings;
        if (options.allow_transcript && !current.allow_transcript)
            || (options.allow_images && !current.allow_images)
        {
            return Err("上传隐私设置已变化，已停止后续AI请求，请确认设置后重试".into());
        }
        Ok(())
    }
    pub(crate) async fn analyze_chunks(
        self: &std::sync::Arc<Self>,
        id: &str,
        control: &std::sync::Arc<super::jobs::Control>,
        client: &super::client::DeepSeekClient,
    ) -> AppResult<Vec<VideoAnalysisResult>> {
        let analysis = self.analysis(id)?;
        self.check_consent(&analysis.options)?;
        let (segments, frames) = self.with(|r| Ok((r.all_transcript(id)?, r.keyframes(id)?)))?;
        let chunks = build_chunks(
            id,
            analysis.duration_ms.ok_or("缺少真实媒体时长")?,
            analysis.analysis_mode,
            &segments,
            &frames,
            analysis.options.allow_transcript,
            analysis.options.allow_images,
        )?;
        self.owned(id, control, |r| r.prepare_chunks(id, &chunks))?;
        let concurrency = self.settings()?.settings.chunk_concurrency as usize;
        for batch in chunks.chunks(concurrency) {
            let first =
                self.analyze_chunk(id, &analysis, &chunks, batch[0].clone(), control, client);
            if batch.len() == 2 {
                let second =
                    self.analyze_chunk(id, &analysis, &chunks, batch[1].clone(), control, client);
                let (a, b) = tokio::join!(first, second);
                a?;
                b?;
            } else {
                first.await?;
            }
        }
        self.with(|r| {
            chunks
                .iter()
                .map(|chunk| {
                    r.cached_chunk(chunk)?
                        .ok_or_else(|| "分段分析尚未完成".into())
                })
                .collect()
        })
    }
    async fn analyze_chunk(
        self: &std::sync::Arc<Self>,
        id: &str,
        analysis: &VideoAnalysis,
        chunks: &[AnalysisChunkInput],
        chunk: AnalysisChunkInput,
        control: &std::sync::Arc<super::jobs::Control>,
        client: &super::client::DeepSeekClient,
    ) -> AppResult<()> {
        control.check()?;
        if self.with(|r| r.cached_chunk(&chunk))?.is_some() {
            return Ok(());
        }
        self.check_consent(&analysis.options)?;
        let source = Path::new(&analysis.source_path);
        Self::check_source(source, &analysis.source_fingerprint)?;
        self.owned(id, control, |r| r.chunk_status(&chunk.id, "running", None))?;
        let key = self.key()?;
        let body = messages(&chunk, &self.data.join("ai/cache").join(id))?;
        let reply = control
            .reply(client.chat(key.as_bytes(), &analysis.model, body))
            .await;
        let result = match reply {
            Ok(reply) => {
                // Usage is a billing fact even when a later schema/source/cancellation check rejects the result.
                self.with(|r| r.record_usage(Some(id), "chunk", &reply.model, &reply.usage))?;
                control.check()?;
                Self::check_source(source, &analysis.source_fingerprint)?;
                reply
                    .checked_content()
                    .and_then(|text| parse_result(text, &chunk.scope()))
            }
            Err(error) => Err(error),
        };
        match result {
            Ok(result) => {
                self.owned(id, control, |r| {
                    r.chunk_status(&chunk.id, "completed", Some(&result))
                })?;
                let completed = self.with(|r| r.completed_chunks(id))?;
                self.stage(
                    id,
                    control,
                    "analyzing_chunks",
                    &format!("正在理解课程 {completed} / {}", chunks.len()),
                    0.5 + 0.35 * (completed as f64 / chunks.len() as f64),
                )?;
                Ok(())
            }
            Err(error) => {
                let _ = self.owned(id, control, |r| r.chunk_status(&chunk.id, "failed", None));
                Err(error)
            }
        }
    }
}
