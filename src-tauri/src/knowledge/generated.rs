use super::{
    service::KnowledgeService,
    vault::{Document, Vault},
};
use crate::{
    ai::{jobs::Control, models::*, service::AiService, transcription as t},
    domain::AppResult,
    learning::{cache, db},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc};
pub const NAMES: [&str; 3] = ["视频总结.md", "完整转录.md", "关键知识.md"];
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneratedFile {
    pub path: String,
    pub hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteReference {
    pub vault_id: String,
    pub path: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedNote {
    pub vault_id: String,
    pub path: String,
    pub transcript_path: String,
    pub knowledge_path: String,
    pub warning: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePlan {
    pub reference: NoteReference,
    pub files: Vec<GeneratedFile>,
    pub warning: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleDeleted {
    pub deleted_paths: Vec<String>,
    pub record_deleted: bool,
    pub warning: Option<String>,
}
pub struct PreparedNote {
    pub vault_id: String,
    pub target: String,
    pub draft: Option<String>,
}
pub struct Pack {
    pub folder: String,
    pub version: String,
    pub summary_hash: String,
    pub notes: Vec<(String, String)>,
    pub assets: Vec<(String, Vec<u8>)>,
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn metadata_hash(value: &impl Serialize) -> AppResult<String> {
    let mut json = serde_json::to_value(value).map_err(|_| "笔记元数据无效")?;
    if let Some(map) = json.as_object_mut() {
        map.remove("generated_metadata_hash");
    }
    json.sort_all_objects();
    Ok(hash(
        &serde_json::to_vec(&json).map_err(|_| "笔记元数据无效")?,
    ))
}
pub fn course_folder(title: &str, item: &str) -> String {
    let safe = title
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*[]#".contains(c) {
                ' '
            } else {
                c
            }
        })
        .take(60)
        .collect::<String>();
    format!(
        "课程-{}-{}",
        safe.trim().trim_end_matches('.'),
        &hash(item.as_bytes())[..10]
    )
}
pub fn clock(ms: i64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        ms / 3600000,
        (ms / 60000) % 60,
        (ms / 1000) % 60
    )
}
pub fn time_link(item: Option<&str>, ms: i64) -> String {
    let label = clock(ms);
    match item {
        Some(item) => {
            let mut u = url::Url::parse("yingliu://video/").unwrap();
            u.path_segments_mut().unwrap().push(item);
            u.query_pairs_mut().append_pair("t", &ms.to_string());
            format!("[{label}]({u})")
        }
        None => label,
    }
}
fn note(
    a: &VideoAnalysis,
    title: &str,
    summary_hash: &str,
    body: String,
    assets: &[(String, Vec<u8>)],
) -> AppResult<String> {
    let mut meta = serde_json::json!({"source":"yingliu","duration":a.duration_ms,"ai_model":a.model,"analysis_mode":a.analysis_mode.as_str(),"analyzed_at":a.completed_at.unwrap_or(a.updated_at),"analyzed_at_unit":"unix_milliseconds","generated_by":"yingliu","analysis_id":a.id,"item_id":a.item_id,"title":title,"summary_hash":summary_hash,"generated_body_hash":hash(body.as_bytes()),"tags":a.summary.as_ref().map(|s|&s.tags),"attachments":assets.iter().map(|(p,b)|serde_json::json!({"path":p,"sha256":hash(b)})).collect::<Vec<_>>()});
    meta["generated_metadata_hash"] = serde_json::Value::String(metadata_hash(&meta)?);
    let yaml = serde_yaml::to_string(&meta).map_err(|_| "无法整理笔记元数据")?;
    let text = format!("---\n{yaml}---\n{body}");
    if text.len() > 5 * 1024 * 1024 {
        return Err("完整笔记超过5MiB，已保留分析结果，请拆分课程后重试".into());
    }
    Ok(text)
}
pub fn render(
    a: &VideoAnalysis,
    segments: &[TranscriptSegment],
    assets: Vec<(String, Vec<u8>)>,
) -> AppResult<Pack> {
    let summary = a.summary.as_ref().ok_or("分析尚无完整总结")?;
    let title = a.title.as_deref().unwrap_or("未命名课程");
    let summary_hash = hash(&serde_json::to_vec(summary).map_err(|_| "总结无效")?);
    let mut body = format!(
        "# {title}\n\n{}\n\n{}\n\n[[完整转录]] · [[关键知识]]\n\n## 章节\n",
        summary.one_sentence_summary, summary.overview
    );
    for c in &summary.chapters {
        body += &format!(
            "\n### {} {}\n\n{}\n",
            time_link(a.item_id.as_deref(), c.start_ms),
            c.title,
            c.summary
        )
    }
    for point in &summary.key_points {
        body += &format!(
            "\n## {} {}\n\n{}\n",
            time_link(a.item_id.as_deref(), point.time_ms),
            point.title,
            point.content
        );
    }
    for (path, _) in &assets {
        body += &format!("\n![[{path}]]\n")
    }
    let mut transcript = format!("# {title} · 完整转录\n\n[[视频总结]] · [[关键知识]]\n");
    let mut bucket = None;
    for segment in segments {
        let group = segment.start_ms / 180000;
        if bucket != Some(group) {
            bucket = Some(group);
            transcript += &format!(
                "\n## {} — {}\n",
                clock(group * 180000),
                clock(((group + 1) * 180000).min(a.duration_ms.unwrap_or(i64::MAX)))
            )
        }
        transcript += &format!(
            "\n{} {}\n",
            time_link(a.item_id.as_deref(), segment.start_ms),
            segment.text
        )
    }
    if segments.is_empty() {
        transcript += "\n本次分析没有可用转录。\n"
    }
    let mut knowledge = format!("# {title} · 关键知识\n\n[[视频总结]] · [[完整转录]]\n");
    for point in &summary.key_points {
        knowledge += &format!(
            "\n## {} {}\n\n{}\n",
            time_link(a.item_id.as_deref(), point.time_ms),
            point.title,
            point.content
        )
    }
    for step in &summary.tutorial_steps {
        knowledge += &format!(
            "\n## 步骤 {} · {} {}\n\n{}\n{}\n",
            step.order,
            time_link(a.item_id.as_deref(), step.time_ms),
            step.title,
            step.description,
            step.notes.join("\n")
        )
    }
    for term in &summary.terms {
        knowledge += &format!(
            "\n### {} {}\n\n{}\n",
            time_link(a.item_id.as_deref(), term.time_ms),
            term.name,
            term.definition
        )
    }
    for action in &summary.action_items {
        knowledge += &format!("\n- [ ] {action}\n")
    }
    let notes = [body, transcript, knowledge]
        .into_iter()
        .enumerate()
        .map(|(i, b)| note(a, title, &summary_hash, b, &assets).map(|text| (NAMES[i].into(), text)))
        .collect::<AppResult<Vec<_>>>()?;
    Ok(Pack {
        folder: course_folder(title, a.item_id.as_deref().unwrap_or(&a.id)),
        version: format!(
            "分析-{}-{}",
            &hash(a.id.as_bytes())[..12],
            &summary_hash[..16]
        ),
        summary_hash,
        notes,
        assets,
    })
}
fn frontmatter(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start_matches('\u{feff}');
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return None;
    }
    let start = first.len();
    let mut end = start;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Some((&text[start..end], &text[end + line.len()..]));
        }
        end += line.len();
        if end > 65536 {
            return None;
        }
    }
    None
}
pub fn metadata<'a>(doc: &'a Document, analysis: &str) -> AppResult<(serde_yaml::Value, &'a str)> {
    let parsed = super::markdown::parse_note(&doc.path, &doc.text);
    if parsed.analysis_id.as_deref() != Some(analysis) || !parsed.warnings.is_empty() {
        return Err("文件不是可验证的应用生成笔记，已保留原文件".into());
    }
    let (yaml, body) = frontmatter(&doc.text).ok_or("生成笔记元数据无效")?;
    if yaml.len() > 65536 {
        return Err("笔记元数据过大".into());
    }
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).map_err(|_| "笔记元数据无效")?;
    if value["generated_by"].as_str() != Some("yingliu") {
        return Err("文件不属于应用生成内容".into());
    }
    Ok((value, body))
}
pub fn pack_result(vault: &str, base: &str, warning: Option<String>) -> GeneratedNote {
    GeneratedNote {
        vault_id: vault.into(),
        path: format!("{base}/{}", NAMES[0]),
        transcript_path: format!("{base}/{}", NAMES[1]),
        knowledge_path: format!("{base}/{}", NAMES[2]),
        warning,
    }
}
pub fn verify_pack(v: &Vault, base: &str, a: &VideoAnalysis, summary_hash: &str) -> AppResult<()> {
    for name in NAMES {
        let doc = v.read(&format!("{base}/{name}"))?;
        let (meta, _) = metadata(&doc, &a.id)?;
        if meta["summary_hash"].as_str() != Some(summary_hash) {
            return Err("目标目录包含其他版本或用户内容，已保留".into());
        }
    }
    Ok(())
}
pub fn plan_files(v: &Vault, a: &VideoAnalysis, vault: &str) -> AppResult<DeletePlan> {
    if a.markdown_vault_id.as_deref() != Some(vault) {
        return Err("分析关联的知识库与当前知识库不一致".into());
    }
    let path = a.markdown_path.as_ref().ok_or("尚未生成知识笔记")?;
    let (base, name) = path.rsplit_once('/').ok_or("生成笔记路径无效")?;
    if name != NAMES[0] {
        return Err("只允许清理应用生成版本包".into());
    }
    let summary = v.read(path)?;
    let (summary_meta, _) = metadata(&summary, &a.id)?;
    let mut files = vec![];
    let mut warnings = vec!["仅处理当前关联版本；旧版本、用户修改稿和其他文件保留".to_owned()];
    for name in NAMES {
        let doc = v.read(&format!("{base}/{name}"))?;
        let (meta, body) = metadata(&doc, &a.id)?;
        if meta["summary_hash"] != summary_meta["summary_hash"] {
            return Err("笔记版本归属不一致".into());
        }
        let mut original_meta = meta.clone();
        let expected_metadata_hash = original_meta
            .as_mapping_mut()
            .and_then(|m| m.remove(serde_yaml::Value::String("generated_metadata_hash".into())))
            .and_then(|v| v.as_str().map(str::to_owned));
        let actual_metadata_hash = metadata_hash(&original_meta)?;
        if expected_metadata_hash.as_deref() != Some(actual_metadata_hash.as_str())
            || meta["generated_body_hash"].as_str() != Some(hash(body.as_bytes()).as_str())
        {
            warnings.push(format!("已保留用户修改稿：{}", doc.path));
            continue;
        }
        files.push(GeneratedFile {
            path: doc.path,
            hash: doc.hash,
        });
    }
    if files.len() == NAMES.len() {
        if let Some(assets) = summary_meta["attachments"].as_sequence() {
            for asset in assets.iter().take(6) {
                let rel = asset["path"].as_str().ok_or("附件归属记录无效")?;
                let sha = asset["sha256"].as_str().ok_or("附件摘要无效")?;
                if sha.len() != 64
                    || !sha.bytes().all(|b| b.is_ascii_hexdigit())
                    || rel.rsplit('/').next() != Some(format!("{sha}.jpg").as_str())
                    || rel.split('/').count() != 2
                {
                    return Err("附件归属记录无效".into());
                }
                let path = format!("{base}/{rel}");
                let bytes = v.read_bytes(&path, 8 * 1024 * 1024)?;
                if hash(&bytes) != sha {
                    warnings.push(format!("已保留修改后的附件：{path}"));
                    continue;
                }
                files.push(GeneratedFile {
                    path,
                    hash: sha.into(),
                });
            }
        }
    }
    Ok(DeletePlan {
        reference: NoteReference {
            vault_id: vault.into(),
            path: path.clone(),
        },
        files,
        warning: Some(warnings.join("；")),
    })
}
impl AiService {
    fn note_material(
        &self,
        id: &str,
    ) -> AppResult<(
        VideoAnalysis,
        Vec<TranscriptSegment>,
        Vec<(String, Vec<u8>)>,
    )> {
        let a = self.analysis(id)?;
        if a.summary.is_none() {
            return Err("请先完成视频分析，再生成知识笔记".into());
        }
        Self::check_source(Path::new(&a.source_path), &a.source_fingerprint)?;
        let (segments, frames) = self.with(|r| Ok((r.all_transcript(id)?, r.keyframes(id)?)))?;
        let mut assets = vec![];
        for frame in frames.iter().take(6) {
            let path = cache::owned(&self.data.join("ai/cache").join(id), Path::new(&frame.path))?;
            let _lock = t::lock_source(&path)?;
            if std::fs::metadata(&path).map_err(|_| "无法读取截图")?.len()
                > crate::ai::keyframes::MAX_JPEG_BYTES
            {
                return Err("截图附件过大".into());
            }
            let bytes = std::fs::read(&path).map_err(|_| "无法读取截图")?;
            if hash(&bytes) != frame.sha256 {
                return Err("截图已改变，已停止生成笔记".into());
            }
            crate::ai::keyframes::jpeg_dimensions(&bytes)?;
            assets.push((format!("Attachments/{}.jpg", frame.sha256), bytes));
        }
        Ok((a, segments, assets))
    }
    pub fn generate_note(
        &self,
        k: &KnowledgeService,
        id: &str,
        vault: &str,
    ) -> AppResult<GeneratedNote> {
        self.generate_note_inner(k, id, vault, None)
    }
    fn generate_note_inner(
        &self,
        k: &KnowledgeService,
        id: &str,
        vault: &str,
        control: Option<&Arc<Control>>,
    ) -> AppResult<GeneratedNote> {
        let (mut a, segments, mut assets) = self.note_material(id)?;
        if a.markdown_vault_id.as_deref() == Some(vault) {
            if let Some(reference) = self.analysis_note(k, id)? {
                a.markdown_path = Some(reference.path);
            }
        }
        let attachment_folder = k.settings()?.attachments_folder;
        for (path, _) in &mut assets {
            *path = path.replacen("Attachments/", &format!("{attachment_folder}/"), 1);
        }

        let _source = t::lock_source(Path::new(&a.source_path))?;
        Self::check_source(Path::new(&a.source_path), &a.source_fingerprint)?;
        let pack = render(&a, &segments, assets)?;
        if let Some(c) = control {
            c.check()?
        }
        let prepared = k.prepare_generated(&a, &pack, vault)?;
        let result = (|| {
            let runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
            if runtime.exiting {
                return Err("软件正在退出".into());
            }
            if let Some(c) = control {
                c.check()?;
                if !runtime
                    .active
                    .as_ref()
                    .is_some_and(|(active, current)| active == id && Arc::ptr_eq(current, c))
                {
                    return Err("分析已取消或中断".into());
                }
                if !self.settings()?.settings.auto_generate_notes {
                    return Err("已关闭自动生成知识笔记".into());
                }
            } else if runtime
                .active
                .as_ref()
                .is_some_and(|(active, _)| active == id)
                || runtime.pending.iter().any(|active| active == id)
            {
                return Err("请等待该分析结束后生成知识笔记".into());
            }
            Self::check_source(Path::new(&a.source_path), &a.source_fingerprint)?;
            self.with(|r|{let current=r.get(id)?;if current.summary.as_ref().and_then(|v|serde_json::to_vec(v).ok()).map(|v|hash(&v)).as_deref()!=Some(pack.summary_hash.as_str()){return Err("分析总结已变化，请重新生成笔记".into())}
    k.publish_prepared(&prepared)?;let result=pack_result(vault,&prepared.target,None);
    r.conn.execute("UPDATE video_analyses SET markdown_path=?2,markdown_vault_id=?3,note_warning=NULL,updated_at=?4 WHERE id=?1",rusqlite::params![id,result.path,result.vault_id,crate::service::now()]).map_err(|_|"笔记已安全保存，但关联未能更新，请重新生成以恢复关联".to_owned())?;Ok(result)
   })
        })();
        if result.is_err() {
            k.discard_prepared(&prepared);
        }
        let mut result = result?;
        result.warning = k
            .index_generated(&prepared.vault_id, &prepared.target)
            .err();
        self.emit_analysis(id, "ai-analysis-updated");
        Ok(result)
    }
    pub(crate) fn export_auto_note(&self, id: &str, control: &Arc<Control>) -> AppResult<()> {
        use tauri::Manager;
        control.check()?;
        let a = self.analysis(id)?;
        if !a.options.auto_generate_note
            || a.options.local_only
            || !self.settings()?.settings.auto_generate_notes
        {
            return Ok(());
        }
        let k = self
            .app
            .as_ref()
            .and_then(|app| app.try_state::<Arc<KnowledgeService>>())
            .map(|k| k.inner().clone());
        match k{Some(k)=>self.export_auto_note_with(&k,id,control),None=>self.owned(id,control,|r|{r.conn.execute("UPDATE video_analyses SET note_warning='请在知识库中选择Vault后手动生成知识笔记' WHERE id=?1",[id]).map_err(db::error)?;Ok(())})}
    }
    pub(crate) fn export_auto_note_with(
        &self,
        k: &KnowledgeService,
        id: &str,
        control: &Arc<Control>,
    ) -> AppResult<()> {
        let a = self.analysis(id)?;
        if !a.options.auto_generate_note || !self.settings()?.settings.auto_generate_notes {
            return Ok(());
        }
        control.check()?;
        let result = a
            .note_target_vault_id
            .as_deref()
            .ok_or_else(|| "分析开始时未选择知识库，请手动生成知识笔记".to_owned())
            .and_then(|vault| self.generate_note_inner(k, id, vault, Some(control)));
        control.check()?;
        if let Err(error) = result {
            self.owned(id, control, |r| {
                r.conn
                    .execute(
                        "UPDATE video_analyses SET note_warning=?2 WHERE id=?1",
                        rusqlite::params![id, error],
                    )
                    .map_err(db::error)?;
                Ok(())
            })?
        }
        Ok(())
    }
    pub fn analysis_note(
        &self,
        k: &KnowledgeService,
        id: &str,
    ) -> AppResult<Option<NoteReference>> {
        let a = self.analysis(id)?;
        let (Some(vault), Some(path), Some(summary)) =
            (&a.markdown_vault_id, &a.markdown_path, &a.summary)
        else {
            return Ok(None);
        };
        let summary_hash = hash(&serde_json::to_vec(summary).map_err(|_| "总结无效")?);
        if k.read(vault, path).ok().is_some_and(|doc| {
            metadata(&doc.data, id)
                .is_ok_and(|(meta, _)| meta["summary_hash"].as_str() == Some(summary_hash.as_str()))
        }) {
            return Ok(Some(NoteReference {
                vault_id: vault.clone(),
                path: path.clone(),
            }));
        }
        let recovered = k.find_analysis_note(vault, id, &summary_hash)?;
        if let Some(reference) = &recovered {
            self.with(|r| {
                let current = r.get(id)?;
                if current.markdown_vault_id != a.markdown_vault_id
                    || current.markdown_path != a.markdown_path
                    || current
                        .summary
                        .as_ref()
                        .and_then(|value| serde_json::to_vec(value).ok())
                        .map(|bytes| hash(&bytes))
                        .as_deref()
                        != Some(summary_hash.as_str())
                {
                    return Err("分析关联或总结已变化，请重新打开笔记".into());
                }
                r.conn
                    .execute(
                        "UPDATE video_analyses SET markdown_path=?2 WHERE id=?1",
                        rusqlite::params![id, reference.path],
                    )
                    .map_err(db::error)?;
                Ok(())
            })?;
        }
        Ok(recovered)
    }

    pub fn plan_note_delete(
        &self,
        k: &KnowledgeService,
        id: &str,
        vault: &str,
    ) -> AppResult<DeletePlan> {
        k.plan_generated_delete(&self.analysis(id)?, vault)
    }
    pub fn delete_note_bundle(
        &self,
        k: &KnowledgeService,
        id: &str,
        vault: &str,
        files: &[GeneratedFile],
    ) -> AppResult<BundleDeleted> {
        let a = self.analysis(id)?;
        let runtime = self.runtime.lock().map_err(|_| "AI任务状态不可用")?;
        if runtime
            .active
            .as_ref()
            .is_some_and(|(active, _)| active == id)
            || runtime.pending.iter().any(|active| active == id)
        {
            return Err("请先结束该分析".into());
        }
        drop(runtime);
        let mut result = k.delete_generated_files(&a, vault, files)?;
        if result
            .warning
            .as_deref()
            .is_some_and(|s| s.starts_with("部分"))
        {
            return Ok(result);
        }
        match self.delete_analysis(id) {
            Ok(warning) => {
                result.record_deleted = true;
                if let Some(w) = warning {
                    result.warning = Some(w)
                }
            }
            Err(error) => result.warning = Some(format!("笔记已移入回收站，分析记录保留：{error}")),
        }
        Ok(result)
    }
}
