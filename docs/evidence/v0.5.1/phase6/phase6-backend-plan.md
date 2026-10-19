# Phase 6 知识工作台后端最小实施契约

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement task-by-task. 本文件只完成只读勘察与实施契约；不代表实现或测试已完成。

**Goal:** 为 Task 8 提供多标签安全保存、版本历史、收藏、Markdown 模板、附件及 AI 整理草稿接口。

**Architecture:** Markdown 是真源，现有 `KnowledgeService` 是唯一 Vault 入口。不可重建的元数据和历史存入映流数据区现有 `learning.db` 的独立表；搜索索引继续可清理。AI 只返回供用户预览的草稿，最终写入统一经过 `knowledge_write`。

**Tech Stack:** 现有 Rust / Tauri / rusqlite / serde / sha2 / Windows handle / DeepSeekClient；不加依赖。

**Spec:** `C:/Users/14582/.codex/worktrees/content-updates/视频下载软件/docs/superpowers/specs/2026-10-18-v051-execution-document.md` 第 27–28 节；同目录 `../plans/2026-10-18-v051-workspace-ai-knowledge.md` Task 8。

## Global Constraints

- 实施目标为主工作区 `artifacts/v051/staging/`，以下代码路径均相对此目录。
- 本轮仅勘察与写本契约；不读真实用户数据库、Vault、Key，不联网，不执行 Git 或全量测试。
- Markdown/YAML/Wiki/任务列表/代码块/附件/Obsidian 特殊语法保持原文；无法证明富文本往返无损时用源码模式。
- 不写 `.obsidian`，不把系统接受 URI 说成 Obsidian 已打开。
- 不复制 DCC 私有 JSON，不另建保存队列后端，不自动注册多个可写 Vault。
- 实施/复核使用 GPT-6.1 Sol high，禁止 Astra；真实 API 验收另需明确少量授权。

## Review Focus

1. A/B Vault 有同名路径时，旧请求只能失败，不能写入新 Vault；切回 A 后收藏和历史仍存在。
2. 保存期间继续输入与迟到回执由前端按 NoteKey/editVersion 处理，不能重置后来输入。
3. 写入前历史持久化失败必须保留源文件；写入成功后的索引失败必须仍回 `saved`。
4. 目录移动后的收藏/模板/历史/附件记录与课程关联共同提交，任一 SQL 失败共同回滚并回移文件。
5. AI 授权撤销、取消、源文件变化、输入草稿变化、截断返回均不得自动替换源 Markdown。

## 已核对的真实边界

| 位置 | 当前行为与后续约束 |
| --- | --- |
| `knowledge/service.rs::with_vault` | 只接受 settings 中当前 vaultId；重新打开根目录并核验身份。所有新增笔记命令继续走此入口。 |
| `KnowledgeService::set_vault` | Vault 改变时调用 `index::clear`；索引只有当前 Vault 的数据。 |
| `knowledge/index.rs` / `learning/schema-v6.sql` | `knowledge_links`、`knowledge_tags` 外键级联依附 `knowledge_index`；它们是派生数据，不能承载收藏、模板标记或历史。 |
| `KnowledgeService::write` | `Vault::write` 返回 Saved/Conflict 后更新索引；索引失败只是 warning。当前所有编辑器写入调用此接口。 |
| `Vault::save/write/read` | 5 MiB、UTF-8、expectedHash、临时文件 sync+handle rename、二次哈希检查、根身份/子路径/reparse 边界已有。复用，不能改成 `fs::write(root.join(path))`。Windows 无原生 hash-CAS rename 的极短外部竞争窗口已在代码注明。 |
| `KnowledgeService::move_note` | 已有 SQLite Immediate 事务、video_analyses/video_notes 路径更新、SQL/commit 失败后的文件回移和明确补偿失败信息；扩展此事务，不另建移动流程。 |
| `Vault::create_raster/read_bytes/trash_raster` | 已支持 ≤8 MiB 栅格图片写入/回收，预览 asset 接口只给栅格图片。现有 `Vault::list` 只列目录和 `.md`，不能当附件列表。 |
| `knowledge/generated.rs` / service prepare/publish | 已有分析结果/截图生成包，采用 create-new，不是任意 Markdown 整理。继续复用，不把它变成覆盖原笔记入口。 |
| `ai/settings.rs` / `qa.rs` / `usage.rs` | 当前只有字幕/图片授权，没有任意笔记文本授权；usage operation 白名单也没有 note_organize。Key 只经 AiService.key 取用。 |
| `KnowledgePage.tsx::save` | 当前单文档回执调用 accept，会替换 text；多标签/保存中输入修复归前端，不需要后端 editVersion 持久化。 |

## 公共 DTO（TS 表示，Rust camelCase；新增输入 deny_unknown_fields）

复用现有 `KnowledgeReference{vaultId,path}`、`KnowledgeView<T>`、`KnowledgeDocument`、`KnowledgeWrite`、`KnowledgeMutation`；现有命令参数和 Saved/Conflict 结构不改。

```ts
type NoteKey = KnowledgeReference;
interface NoteMetadata { path:string; favorite:boolean; template:boolean; available:boolean }
interface NoteMetadataPage { items:NoteMetadata[]; total:number; page:number }
interface NoteVersion { id:string; path:string; hash:string; bytes:number; createdAt:number }
interface NoteVersionPage { items:NoteVersion[]; total:number; page:number }
interface NoteVersionContent extends NoteVersion { text:string }
interface NoteAttachment {
  path:string; name:string; hash:string; bytes:number; importedAt:number;
  state:'ready'|'changed'|'missing';
}
interface NoteAttachmentPage { items:NoteAttachment[]; total:number; page:number }
interface NoteOrganizeRequest {
  requestId:string; vaultId:string; path:string;
  expectedHash:string; text:string; allowNoteText:boolean;
}
interface NoteOrganizeDraft {
  requestId:string; vaultId:string; path:string;
  sourceHash:string; inputHash:string; markdown:string;
}
```

分页沿用每页 50，page 最小 1；稳定排序后再分页。版本 ID 使用现有 new_id，不接受路径充当版本 ID；哈希使用现有 SHA-256 工具。`createdAt`/`importedAt` 为 Unix 毫秒。

## Task B1 — 元数据、历史与现有保存/移动闭环

**所有权:** 一个后端实施者独占本任务共享文件；前端只消费已定 DTO。

**Files:** Modify `learning/db.rs`, `knowledge/service.rs`, `knowledge/models.rs`, `knowledge/commands.rs`, `knowledge/mod.rs`, `lib.rs`; Create `learning/schema-v8.sql`, `knowledge/workspace.rs`; Test `knowledge/service_tests.rs`, `learning/tests.rs`。

**存储契约:**

- schema_version/user_version 从 7 升到 8，保留 v0–7 迁移和未来版拒绝；迁移事务失败不留半张表。
- `knowledge_note_metadata(vault_id TEXT, path TEXT, favorite INTEGER NOT NULL DEFAULT 0, template INTEGER NOT NULL DEFAULT 0, PRIMARY KEY(vault_id,path))`。
- `knowledge_note_versions(id TEXT PRIMARY KEY, vault_id TEXT NOT NULL, path TEXT NOT NULL, content_hash TEXT NOT NULL, text TEXT NOT NULL, bytes INTEGER NOT NULL, created_at INTEGER NOT NULL)`，索引 `(vault_id,path,created_at,id)`。
- `knowledge_attachments(vault_id TEXT, path TEXT, name TEXT NOT NULL, content_hash TEXT NOT NULL, bytes INTEGER NOT NULL, imported_at INTEGER NOT NULL, PRIMARY KEY(vault_id,path))`；由 B2 使用。
- 三表均不外键引用 `knowledge_index`，不参与 `index::clear/remove` 或 reindex pruning。每次查询显式限定 vault_id。
- 版本保存旧文件完整 UTF-8 文本与哈希，保留 BOM/CRLF 等实际字符；不是 AST 重序列化。历史保留在映流数据区，不放 Vault 隐藏目录。
- v0.5.1 不自动删除历史。`ponytail:` 历史随保存增长；用户可见容量与明确的保留/清理策略另行添加，不能静默清理用户历史。

**命令契约（除注明外都返回 KnowledgeView<T>）:**

| Command | 参数 | T / 结果 |
| --- | --- | --- |
| `knowledge_note_metadata` | vaultId, path | NoteMetadata；不存在行时 false/false，笔记本不接受 |
| `knowledge_set_note_metadata` | vaultId, path, favorite:boolean, template:boolean | NoteMetadata；先 Vault.read 确认当前普通 Markdown；不改源文本 |
| `knowledge_note_collection` | vaultId, kind:`favorites`\|`templates`, page | NoteMetadataPage；不得用当前搜索索引判定历史数据是否存在；页内核验实际文件，缺失项 available=false，禁用打开，不改变分页总数 |
| `knowledge_note_versions` | vaultId, path, page | NoteVersionPage；仅头信息，不一次返回全部正文 |
| `knowledge_note_version` | vaultId, path, versionId | NoteVersionContent；三者共同匹配，否则拒绝 |
| `knowledge_restore_note_version` | vaultId, path, versionId, expectedHash | KnowledgeWrite；恢复仍走统一写入及冲突保护 |

**统一写入顺序:**

1. 在当前 `with_vault` 状态锁下 `v.read(path)`，先比 expectedHash；不匹配立即返回 Conflict，不插入历史。
2. text 与当前内容相同直接返回 Saved，不新增版本。
3. 把将被替换的完整当前文档作为快照提交至 SQLite；连续相同的最新快照 hash 不重复插入。数据库失败则返回错误，尚未写文件。
4. 继续调用现有 `v.write(path,text,expectedHash)`，其二次检查仍是最终文件边界。若外部此时改稿，返回 Conflict，保留已持久化的真实旧快照；它是“保存前快照”，不能标成一次成功保存。
5. Saved 后沿用 index warning 行为；不得因索引失败返回普通 Err，使前端以为文件未保存。恢复历史使用同一过程，旧的当前内容也会形成快照。

首次 create 不需要历史；后续所有覆盖（普通保存、自动保存、历史恢复、用户接受 AI 草稿）统一经过此方法。既有 generated/screenshot create-new 路径继续保持不覆盖。

**移动与回收:** `move_note` 现有 Immediate 事务内加入上述三表按 `(vault_id, path==from OR from+'/')` 的精确/后代更新；不能用裸 LIKE 模糊匹配。任何唯一键冲突/SQL/commit 失败沿用整体回滚和 FS 回移。历史 ID 不变。`index::remove` 仍只清索引。笔记回收保留版本以供恢复，收藏/模板不在列表中显示失效文件；新建同路径时先重置其元数据，历史作为该 NoteKey 的旧快照供显式预览，绝不自动恢复。附件不随笔记删除自动回收。

- [ ] RED：`workspace_metadata_and_history_survive_index_clear_and_vault_roundtrip`：A/B 同路径互不混用，A→B→A 记录仍在。
- [ ] RED：`write_checkpoints_exact_bytes_before_replacement`：含 BOM/CRLF/YAML/Wiki/任务/代码/中文快照字节与替换前一致；重复无变化保存不增版本。
- [ ] RED：`history_insert_failure_keeps_source_and_conflict_never_overwrites`：触发器注入失败源 hash 不变；expectedHash 不符不新增版本；外部二次冲突保留草稿。
- [ ] RED：`restore_is_hash_guarded_and_preserves_current_version`：错 hash 返回 Conflict；正确恢复且当前版进入历史；别的 Vault/path/versionId 无法读取。
- [ ] RED：扩展现有 `move_note_rolls_back_file_and_first_update_when_second_update_fails`、`move_note_restores_file_when_commit_fails`、`move_directory_updates_descendants_but_preserves_other_vaults_and_prefix_siblings`，断言新三表一起移动/回滚，前缀相邻路径不受影响。
- [ ] GREEN：实现上述接口与断言；仅跑 `cargo test --locked knowledge::service_tests` 与迁移定向测试。保留已有“索引失败但文件已保存”回归。

## Task B2 — 附件复制与 Markdown 模板

**所有权:** 附件实施者拥有新 `knowledge/attachments.rs` 及其测试；对 service/models/commands/lib 的接线由 B1 所有者统一合入，避免并改同一大文件。

**Files:** Create `knowledge/attachments.rs`, `knowledge/attachment_tests.rs`; Modify `knowledge/vault.rs`（只增加通用附件的 handle 边界方法，不改现有 Markdown 边界）。

**接口:**

- `knowledge_import_attachment(vaultId)` → `KnowledgeView<NoteAttachment> | null`。Rust 原生文件选择器选中后打开本地普通文件，拒绝目录/reparse，先有界读再复制；取消返回 null。前端不提交任意 sourcePath/base64。
- `knowledge_attachments(vaultId,page)` → `KnowledgeView<NoteAttachmentPage>`，列出映流导入记录并核对实际文件状态；可读时 hash/bytes 为本次读取事实，与导入 hash 不同为 changed，删除须展示变化并以当前 hash 再确认；无法读取为 missing，此时保留导入 hash/bytes 并禁用文件动作。不扫描整棵 Vault，不假装 `knowledge_tree` 会列普通附件。
- `knowledge_delete_attachment(vaultId,path,expectedHash)` → KnowledgeMutation。显式确认后才移入 Vault 回收目录；哈希不匹配拒绝。可能被其他 Markdown 引用，不做自动“孤儿清理”。
- `knowledge_export_attachment(vaultId,path)` → boolean。Rust 保存对话框选择新目标，create_new，禁止覆盖任何已有文件，复制后 sync；不执行附件。
- 新增 `Vault::create_attachment(rel,bytes)`、`attachment_info(rel)`、`trash_attachment(rel,expectedHash)`：复用 root/parent/child/inspect/temp/rename 边界，通用附件 ≤20 MiB，图片导入仍 ≤8 MiB；扩展名不提供执行能力。现有图片 asset 预览保持其白名单和大小限制。
- 附件目标固定在 settings.attachmentsFolder 下，使用 new_id 加合法扩展名，原文件名仅展示。所有方法在 service 再校验目标位于该目录，不能用附件命令读/回收普通笔记或 `.obsidian`。
- 先 create-new 文件再写附件记录；SQL 失败用 expectedHash 回收刚创建文件，补偿失败明确报告实际目标路径，不把失败说成成功。导入不直接编辑笔记；前端把 Vault 相对 Wiki 图片/附件链接插入当前草稿，最终照常保存。
- 收藏/模板使用 B1 的元数据。模板就是一篇普通 Markdown 的 template 标记；预览读取原文，“从模板新建”复用 `knowledge_create`，不做占位符执行/任意路径插值，不复制模板的收藏标记，不新增第二种私有存储。Phase 4 的 AI 分析用途模板保持原接口，二者不是同一用途。

- [ ] RED：`attachment_import_is_bounded_create_new_and_vault_scoped`：中文/空格/特殊字符名、0 字节、8/20 MiB 边界、越界路径、同名不覆盖、根替换/reparse；保留源附件 hash。
- [ ] RED：`attachment_db_failure_compensates_and_delete_checks_hash`：注入 INSERT 失败后无假成功，补偿失败报实际路径；外部修改后删除失败。
- [ ] RED：`markdown_template_creates_exact_source_without_overwrite`：模板 YAML/Wiki/附件链接原文一致，目标存在失败，源模板 hash 不变。
- [ ] GREEN：只跑 `cargo test --locked knowledge::attachment_tests` 及受影响 Vault 定向测试。

## Task B3 — AI 整理返回可预览草稿

**所有权:** AI 实施者拥有 `ai/note_organize.rs`、测试、AI settings/client/usage 接线；只调用 KnowledgeService.read，不自行打开 Vault 或写 Markdown。

**Files:** Create `ai/note_organize.rs`, `ai/note_organize_tests.rs`; Modify `ai/settings.rs`, `ai/service.rs`, `ai/client.rs`, `ai/usage.rs`, `ai/mod.rs`；由接线所有者登记命令与 TS DTO。

**接口与边界:**

- `ai_organize_note(request:NoteOrganizeRequest)` → NoteOrganizeDraft；`ai_cancel_note_organize(requestId)` → void。每次显式点击先展示将发送的笔记文本与用途，再 allowNoteText=true；后台/自动保存/模板应用不自动触发。
- `AiSettings.allow_note_text` 新字段默认 false（旧设置兼容），不能把 allow_transcript 当 Markdown 授权；请求级 true 与当前设置 true 必须同时成立。
- requestId 长度 1–128；输入 text 非空、无 NUL、UTF-8 ≤48 KiB；超过时明确要求缩小选择，不静默截断。`knowledge.read(vaultId,path)` 校验当前 Vault 与 expectedHash，inputHash 由 Rust 对传入 text 算出。可以整理未保存草稿，但不能绕过源基线校验。
- 只发送明确提供的文本，路径/文中指令均作为数据；不跟随 Wiki/URL、不读取附件、其他笔记、课程字幕或图片。
- 复用安全固定 HTTPS DeepSeekClient 与 Rust key；新增 `RequestPurpose::NoteOrganize` 输出 8192 tokens、沿用 2 MiB HTTP 响应上限；一次逻辑请求，无整理结果自动修复/层级重试。响应严格 `{markdown:string}` JSON，非空、无 NUL、≤32 KiB；truncated/error/无效 JSON 拒绝。
- 每个已取得的 ChatReply 在 checked_content/解析/最终隐私检查之前调用 `record_usage(None,"note_organize",model,usage)`，包括截断和校验失败；扩展 operation 白名单。用量面板新增 `noteRequests` 默认 0 的统计字段，不能伪装成视频问答/分析；仍汇总进 token 总数。
- AiService 为一个活动整理请求持有现有 `Arc<Control>`，不另建作业数据库；取消及 allow_note_text 从 true→false 时 cancel。不得持有 KnowledgeService/AI DB/settings 锁跨 await。若复用现有 HTTP 状态重试，每个物理 attempt 前重新检查 Control、当前设置、Vault 与 source hash；给现有 client 增加最小检查回调，不能只在整个 chat_for 前检查一次。
- 收到响应后先记 usage，再次检查授权/取消/Vault/source hash。回执 sourceHash+inputHash 只证明请求快照；前端比较当前 NoteKey、基线 hash、草稿 hash 后才可接受。
- 结果显示在可预览草稿中，由用户显式“应用到草稿”或另存；AI 命令绝不调用 knowledge_write/create。应用后还需走正常编辑/历史/expectedHash 保存。关闭或丢弃预览不修改源文件。

- [ ] RED：`organize_requires_note_consent_and_reads_only_supplied_text`：全局默认拒绝、请求未授权拒绝，字幕授权不能代替；无外链/附件读取。
- [ ] RED：`organize_rechecks_consent_cancel_and_source_before_retry`：模拟首个可重试响应后撤销/取消/切 Vault/改源；服务器请求次数停在 1。
- [ ] RED：`organize_records_usage_for_invalid_or_truncated_replies`：本地模拟 ChatReply 均有一条 note_organize 用量，token 不伪造，失败不返回草稿。
- [ ] RED：`organize_preview_never_mutates_source`：成功/失败/取消前后源 hash、`.obsidian` hash 均相同；回复包含 source/input hash；输入输出边界与严格 JSON 拒绝。
- [ ] GREEN：仅跑 `cargo test --locked ai::note_organize_tests`，使用已有本地 HTTP 模拟模式；不使用真实 Key/网络。

## 前后端所有权与接线顺序

1. **B1 先冻结 DTO/schema**，由一个接线所有者修改 `models.rs/commands.rs/lib.rs/service.rs`；B2/B3 的独立实现按明确签名交给该所有者接线。不要同时让三个实施者改同一命令表或迁移版本。
2. **前端所有者**拥有 `NoteSession.ts` / `NoteSaveQueue.ts` / `KnowledgeWorkspace.tsx` / 工具面板 / `src/types/knowledge.ts`。`NoteKey=vaultId+path`；每文档串行队列，回执用闭包捕获的 editVersion 只更新该文档基线。无须更改现有 knowledge_write DTO。
3. **当前 Vault 模型保持不变**：其他 Vault 标签可保留本地草稿并暂停，不得继续发写请求。切换先检查所有未保存草稿/队列，切回后重新 read+hash；后端的“已切换”错误不能丢弃草稿。
4. **移动协议**：前端排空所涉文档保存队列后才调用 move/rename；成功将所有后代 NoteKey 和附件面板路径迁移，失败保留旧 key。后端移动事件可补充 `from/to`，新增字段保持现有监听兼容；没有真实落盘成功不能刷新为 saved。
5. **历史/模板/AI 预览**都先展示内容，不直接替换 session.text；用户接受后增加 editVersion，再交同一保存队列。富文本转换、大纲/导图及源模式回退由前端所有者验证，后端不参与 Markdown 规范化。

## 本轮交付与实施验证范围

- 本文件依据实际 staging 源码完成，只执行文件读取/搜索及本文件写入；未修改产品代码、未运行测试、未访问真实数据、未调用 API。
- 本契约中的测试名称是待新增的验收目标，不能记为已通过。执行前重新核对 B1/B2/B3 涉及文件，按 RED→最小实现→定向 GREEN 逐项推进。
- Phase 7 再统一全量测试、真实文件 hash/UI/Windows 人工矩阵；本契约不批准发布、合并、正式安装包或真实 AI 请求。
