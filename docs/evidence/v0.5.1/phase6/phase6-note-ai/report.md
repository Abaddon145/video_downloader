# Phase 6 B3 — AI 笔记整理草稿

实施范围仅 staging/src-tauri/src/ai。没有修改 knowledge、lib.rs、TS、package；未使用 Git、用户 Vault/数据库/Key、真实 API 或外部网络。HTTP 测试仅绑定 127.0.0.1 临时端口，服务数据在随机临时目录。

## 命令与 DTO

注册路径由 root 统一接线：

- `ai::note_organize::ai_organize_note(request)`：返回 `NoteOrganizeDraft`，命令注入现有 `Arc<AiService>` 与 `Arc<KnowledgeService>`。
- `ai::note_organize::ai_cancel_note_organize(requestId)`：返回 void，只取消匹配 requestId 的活动请求。

```ts
interface NoteOrganizeRequest {
  requestId: string;
  vaultId: string;
  path: string;
  expectedHash: string;
  text: string;
  allowNoteText: boolean;
}
interface NoteOrganizeDraft {
  requestId: string;
  vaultId: string;
  path: string;
  sourceHash: string;
  inputHash: string;
  markdown: string;
}
```

Rust request 使用 camelCase + deny_unknown_fields；allowNoteText 必填。AiSettings.allowNoteText 默认 false，旧设置 JSON 缺字段可读取。UsageView.noteRequests 为 number，SQL count 在无记录时为 0；前端旧数据兼容可使用 `noteRequests ?? 0`。

## 实现核对

- 两级笔记授权分别检查；字幕/图片授权不能替代笔记授权。请求 ID 为 1–128 字节；文本非空、无 NUL、最多 48 KiB UTF-8。
- 只调用 KnowledgeService.read 校验当前 Vault、路径与 expectedHash；不自行打开 Vault。inputHash 为 Rust 对传入 text 的 SHA-256。
- 只发送请求提供的文本，路径、源正文、附件和其他笔记不加入 payload。内容指令/链接被声明为数据，无工具或文件写入执行路径。
- 新 RequestPurpose::NoteOrganize 限 8192 输出 tokens；复用固定 HTTPS、禁止重定向、敏感 Key header 与 2 MiB HTTP 响应限额。
- 只进行一次逻辑整理，不进行内容修复调用。保留既有 HTTP 状态重试；新 chat_for_checked 回调紧邻每次物理 send 前检查取消、授权、当前 Vault 和源 hash。旧 chat_for caller 继续使用默认无操作回调。
- 每个取得的 ChatReply 先写 ai_usage(operation=note_organize, analysis_id=NULL)，再处理授权/源变化/截断/JSON 错误。用量独立 noteRequests，不冒充 questions 或 analyses，tokens 保留实际返回值并纳入 model 合计。
- 严格 JSON `{markdown:string}`，拒绝额外字段、空文本、NUL、超过 32 KiB 和截断。响应校验后再次检查状态，再返回带双 hash 的草稿。
- AiService 只保存一个现有 Arc<Control>，RAII 清理活动槽。取消、全局撤权、shutdown 均取消控制器；没有新作业表，没有锁跨 await。
- 不调用 knowledge_write/create；成功、失败、取消均不改源 Markdown 或 .obsidian。最终应用/保存属于前端显式确认与既有保存流程。
- 按 root 要求，ai/template_tests.rs 中最高 schema 期望由 7 改为 8，保留旧分析兼容断言。

## 验证记录

1. `red.txt`：先新增旧设置序列化测试，实际失败 `allowNoteText: Null != false`。
2. `green-first.txt`：初始整合测试 3/5；两项失败来自 Windows accept 继承 nonblocking，明确改测试 socket 为 blocking 后恢复。
3. `green-second.txt`：`cargo test --locked --manifest-path artifacts/v051/staging/src-tauri/Cargo.toml ai::note_organize_tests --lib`，5/5 通过。
4. `client-compat.txt`：`cargo test --locked --manifest-path artifacts/v051/staging/src-tauri/Cargo.toml ai::client --lib`，既有客户端测试 3/3 通过。
5. 最终重跑 `green.txt` / `migration-compat.txt` 曾遇 B2 同时编辑期间的 Vault 附件方法缺失，待 B2 完成后更新最终结果。

测试覆盖双授权与旧字段默认、仅发送明确文本、四种重试阻断、迟到响应状态复查、无效/截断响应事实用量、输入输出 KiB 边界、strict JSON、双 hash、源与 .obsidian 不变、撤权取消 Control。

自查：旧 chat_for 无调用语义变化；未引入新依赖、作业数据库或任意笔记写入口。未运行全量测试或真实 AI 验收；前端仍须在应用草稿前比较当前 NoteKey、基线 hash、草稿 hash。
