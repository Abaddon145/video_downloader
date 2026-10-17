# AI Video Knowledge v0.5.0 Implementation Plan

> **For agentic workers:** Use executing-plans inline, phase by phase. User supplied the written specification and mandatory Phase0–12 execution order; execute continuously without new design permission gates. One independent whole-branch review at the end, not one implementer/reviewer per phase.

**Goal:** 实现用户文档0–125节的AI视频分析与开放Markdown Vault，保留v0.4.2。
**Architecture:** AiService + KnowledgeService 复用 learning.db、FFmpeg、native::Job/DPAPI/原子替换和现有mpv控制；文件是真源，AI结构是验证过的数据。
**Tech Stack:** 现有Tauri2/Rust/React/TypeScript/SQLite；必要reqwest/base64/YAML与CodeMirror/Markdown。
**Spec:** docs/superpowers/specs/2026-10-17-ai-video-knowledge-design.md 和本会话用户执行文档。

## Global Constraints
固定DeepSeek HTTPS域名，Key不进React/普通设置/SQLite/日志；仅授权文字/JPEG上传；源文件只读；普通Markdown可独立使用；不碰.obsidian；阶段失败停止；固定组件URL/SHA；保留旧测试与原生播放器；默认不自动付费分析。

## Review Focus
1. 取消/退出/重试旧任务迟到与并发Key/Vault切换：旧结果不得发布到新任务或错误Vault。
2. 外部Obsidian编辑、同名文件、junction/symlink交换：不得静默覆盖或逃逸，源笔记/附件不得被清缓存删除。
3. AI伪造时间/来源ID、字幕prompt injection、超长/截断结果：严格拒绝后可重试，不执行指令。
4. 无音轨/纯音频/长片/Unicode路径/模型损坏：可选阶段与恢复正确，不阻塞UI或影响下载/mpv。
5. 真实API缺Key、401/402/429/超时：有界脱敏，原始用量准确，费用及上传只按用户主动授权。

## Shared interfaces
AiSettingsView={settings,hasApiKey}; setters do not return secrets. ai_set_deepseek_key opens native entry without a key argument. AiService owns DB/settings/runtime; KnowledgeService owns vault/index/conflict writes. Commands are typed and async for IO. IDs generated in Rust, no model paths/SQL/commands.
AnalysisMode quick/standard/deep; AiStatus as spec. VideoAnalysisResult contains contentType/oneSentenceSummary/overview/chapters/keyPoints/tutorialSteps/terms/actionItems/tags, timestamps ms, segment/frame IDs validated against supplied context. AiQuestionResult={answer,references:[{startMs,endMs,label}]}. Page<T> uses50 by default. FileRead={path,content,hash}; FileWrite uses expectedHash and returns saved/conflict. NoteRef={vaultId,path}. Progress/events includeid/session generation.

### Phase 0: Freeze baseline
- [x] 806fe45 branch, npm ci --ignore-scripts, prepare:tools, npm test, npm run build, cargo test --locked; evidence baseline.txt. Expected:23 Node/96 Rust and build pass before changes.

### Phase 1: Schema and service boundaries
Files: learning/schema-v6.sql, learning/db.rs, learning/tests.rs; ai/{mod,models,service,commands,repository}.rs; knowledge/{mod,models,service,commands}.rs; lib.rs.
Interfaces: open existing DB via learning::db::open. AiService/KnowledgeService separate Arc state; analysis/usage/vault data tables and constraints. FTS optional; existing settings API remains compatible.
- [x] RED: add v5→v6 preservation/emptyDB/future-version/foreign-key tests; extend old expected version rather than remove tests. Expected new schema assertions fail.
- [x] GREEN: transaction v6, optional FTS, functional list/read empty data commands, no fake analysis success; recovery active→interrupted. Expected records/notes/progress preserved and startup no-FTS safe.
- [x] Verify: cargo test --locked, npm test/build; commit feat(ai): add v6 analysis database schema.

### Phase 2: Key/settings/client
Files: ai/{settings,client,prompts,usage}.rs, native credential entry, ai models/commands, AI Settings component/types.
- [x] RED→GREEN: DPAPI opaque file/change/delete/no-key tests, raw key absent in views/state/errors; native entry only. Fixed endpoint client timeout15/120, retry2 1s/3s for429/500/502/503/504, bounded response,401/402/network messages. Fake HTTP transport fixtures clearly marked; real connection only with user-entered key.
- [x] Verify full suites; commit secure settings/client. Do not implement video analysis until pass.

### Phase 3: Local Whisper
Files: prepare-tools/components/licenses, ai/transcription.rs and model management, repository transcript commands.
- [x] Pin verified CPUx64 archive+DLLs; Base/Small immutable model URL/SHA/bytes from primary metadata, actual verification; no model inNSIS. Download/import progress/temporary/rename/cancel/retry.
- [x] RED→GREEN real local WAV pipeline16kHz/mono, timestamp JSON, no-audio skip/audio-only; missing/corrupt CLI/model, Unicode, interruption and process-tree cancellation; source hash unchanged. Commit local transcription after suites.

### Phase 4: Keyframes
Files: ai/keyframes.rs, centralized mode constants.
- [x] RED→GREEN scene+fallback180/90/45s, caps12/24/60, JPEG≤1280 quality80, SHA dedupe/time provenance. Real MP4/MKV/WMV/AVI/HEVC/4K/noaudio, cancellation and source changes. Commit only green.

### Phase 5: Chunk analysis/runtime
Files: ai/{analysis,jobs,repository,client}.rs.
- [ ] RED→GREEN natural transcript cuts/size budgets≤6frames/request, consent filtering, no raw media/audio uploads, validated chunk results and token usage. Resume completed blocks; generations/cancel abort processes/network and prevent late writes; source fingerprint each boundary. Mock429/5xx/retry and staged restart. Commit chunk pipeline.

### Phase 6: Summary and QA
Files: ai/{analysis,prompts,qa,usage}.rs.
- [ ] RED→GREEN final summary from chunk results, validated times/IDs/order/length, regenerate reuses transcript/frames. TopK search+neighbors QA, current±30s explanation/near frame, inadequate context says unknown; no full-transcript per ask. Real API smoke only if configured; otherwise explicit not-run. Commit summary/QA.

### Phase 7: AI workspace
Files: features/ai/{AiAnalysisPage,AnalysisQueue,AnalysisCard,AnalysisProgress,AnalysisResult,VideoSummary,VideoChapters,VideoAsk,AiUsage,AiSettings}.tsx, types/ai.ts, App routing, pricing config.
- [ ] RED→GREEN media-centered queue/history/filter/details, consent modal, mode/model/local transcript/model install, cancel/retry/resume/delete, summary/chapters/usage/errors. Typed snapshots ignore oldIDs, transcript paging, controls accessible880wide; no technical jargon outside details. Commit AI UI.

### Phase 8: Vault backend
Files: knowledge/{vault,markdown,index,links,search,obsidian,service,commands,models}.rs.
- [ ] RED→GREEN new/existing Vault, Rootguard absolute/UNC/.. /Windowsnames/junction/symlink/race checks, ignored dirs/5MiB limits; expected_hash conflict safe atomic writing; create/rename/move/trash/reindex pagedtree; text/frontmatter/wiki/tags/backlinks/search; URI encoding/noObsidian graceful. Files source of truth; source references never full path bydefault. Commit backend.

### Phase 9: Vault UI
Files: features/knowledge/{KnowledgePage,FileTree,MarkdownEditor,MarkdownPreview,KnowledgeSearch,Backlinks,TagPanel,KnowledgeToolbar}.tsx, types/knowledge.ts.
- [ ] RED→GREEN CodeMirror edit/preview/split, safe Markdown noHTML, local images/wiki links, filetree/sidebar toggles/search/outline/tags/backlinks, external reload/compare/conditional overwrite, focus changes trigger bounded scan. Views lazy/paged,880wide+scales. Commit knowledge UI.

### Phase 10: Player integration
Files: learning player/sidebar/lesson rows, existing playbackHandle, media page/download-complete entry.
- [ ] RED→GREEN summary/transcript/notes/knowledge65/35 tabs, seek existing handle for chapters/search/QA/internal links, segment explanation, AI actions from all requested surfaces; download autoAI defaultOFF and explicit consent. Preserve mpv modal geometry/fullscreen/keybindings/cancel/progress. Commit integration.

### Phase 11: Generated notes and screenshot notes
Files: knowledge note generation, ai completed workflow, screenshot helper and UI.
- [ ] RED→GREEN summary/transcript2–5min/knowledge/templates/attachments, Rust-cleaned paths, normal YAML/wiki/tags/time links without absolutepaths. defaultauto-noteON, reuse write-conflict protection/user edits. Screenshot actual time→FFmpeg→attachment+Markdown+time note, source readonly, safe partial failure. Delete analysis preserves notes bydefault; jointdelete needsconfirm. Commit notes.

### Phase 12: QA/delivery
- [ ] Complete API status/JSON/injection/provenance/response limits, no-key leaks to dist/state/DB/log/diagnostics, captured upload body contains only allowed text/JPEG. Whisper failure/download/model/source SHA, video+language+duration+complexpath matrix; Vault conflicts/escapes/reindex/Obsidian all covered. Distinguish mock vs real and host limits.
- [ ] Existing download/MediaPro/mpv/learning/notes/subtitle/thumb/tray/restore/fullscreen/keyboard tests pass. Real screenshot review of AI/Vault/player/880width/scales. One final independent branch review, one verified fix pass.
- [ ] Versions0.5.0 package/Cargo/tauri; React Tauri getVersion. Docs ai-video-analysis-v0.5.0/knowledge-vault-v0.5.0/deepseek-privacy-v0.5.0/ai-video-analysis-delivery, evidence, README. NSIS/source/versions/licenses/SHA verified. NewDraftPR, no merge/Release. Archive ledger/keep worktree.
