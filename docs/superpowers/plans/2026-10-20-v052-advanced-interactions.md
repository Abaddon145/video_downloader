# 映流 v0.5.2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: subagent-driven-development. Use explicit difficulty-based model routing; never GPT-6 Astra.

**Goal:** 用统一命令、快捷键、实时 Markdown 编辑和真实后台状态，减少学习与知识整理中的点击及数据丢失风险。

**Architecture:** 以 e8f0740 的完整 v0.5.1 为基线，在现有 React/Tauri 服务上增加交互层。CommandRegistry 连接页面已经存在的动作；CodeMirror decorations 只改变呈现；任务中心聚合现有下载、MediaService、AiRuntime，不创建第二套引擎。保留 NoteSessions、CAS、历史与 native mpv 会话。

**Tech Stack:** React 19, TypeScript, CodeMirror 6, Tauri 2, Rust, SQLite, mpv.

**Spec:** ../specs/2026-10-20-v052-execution-document.md（用户提供并要求执行）。

## Global Constraints

- 独立分支 codex/v0.5.2-advanced-interactions；PR9仍为未合并Draft，新PR以其分支为基线。
- 每Phase独立提交，失败先修复再推进依赖阶段；不得删测试、覆盖用户安装、合并或发布Release。
- 所有源视频只读，Markdown保留原文；不修改 .obsidian；Key仅Rust安全层可读取。
- 命令、菜单和工具栏复用同一个业务动作；危险动作保留确认；AI入口仍附着课程。
- 未持久化操作显示临时状态，不伪造百分比、暂停能力、历史或实测。
- 输入法组合输入/编辑器撤销/弹层焦点优先于全局及播放器快捷键。

## Review Focus

1. 迟到异步回调和卸载：命令注册、任务订阅及AI草稿不能串到另一课程/笔记；测试清理及目标变化。
2. Markdown复杂语法/中文IME：预览 decorations 不改原文；选区工具只用CM事务；测试YAML/Wiki/CRLF/代码原文。
3. 文件冲突/撤销：仅逆转自己成功的操作，当前磁盘或分类变化时拒绝盲目回滚；测试失败保留记录。
4. 原生窗口覆盖与小容器：弹层沿用data-native-overlay，布局变更不重建mpv，临时收缩不写回持久尺寸。
5. 跨系统任务状态：源ID带kind，初始快照不能覆盖较新事件，无可靠百分比时只展示阶段；测试重试及能力矩阵。

## Phase 0 — 基线

- [ ] 记录最新分支/PR9状态；原样保存执行文档。
- [ ] npm test、npm run build（含tsc）、cargo test --locked，保留依赖环境失败及修复证据。
- [ ] 核对现有mpv/Obsidian边界与v0.5.1实测，不把历史证据当新实测。
- [ ] 提交基线及计划。

## Phase 1 — 通用交互基础

Files: src/features/commands/{CommandRegistry,ShortcutManager,CommandProvider,ShortcutSettings}；src/components/ContextMenu.tsx；共享反馈及Undo模块。

- [ ] 先写快捷键冲突/焦点/IME/注册清理、撤销失败保持队列测试，再实现。
- [ ] 注册描述包含id/title/category/keywords/enabled/reason/execute；支持可定制绑定、严格导入导出、帮助与恢复默认。
- [ ] 扩展现有菜单快捷键/图标/禁用原因；统一可点击通知与撤销提示，保留既有notify入口。
- [ ] Phase测试及类型检查后提交。

## Phase 2 — 命令面板

Files: src/features/commands/CommandPalette.tsx；App.tsx、LearningLibraryPage、LearningPlayer、KnowledgePage的注册接缝。

- [ ] 测试中文/英文/模糊排序、仅必要ID最近记录、不可执行与执行异常。
- [ ] Ctrl+K/Esc/方向/Enter/Tab、焦点恢复、执行中去重、原生覆盖；注册实际导航、当前视频/笔记/布局/目录/任务命令。
- [ ] Ctrl+P统一搜索课程和笔记；使用现有分页查询，结果跳转调用现有链接入口。
- [ ] UI键盘验证、截图、提交。

## Phase 3 — 知识库

Files: MarkdownEditor.tsx、MarkdownLivePreview.ts、SelectionToolbar.tsx、KnowledgePage.tsx、knowledge.css。

- [ ] 先写源码保持、选区操作及复杂语法安全测试。
- [ ] 默认实时预览，提供源码/预览/分屏；当前光标行显示原语法；复用安全附件及链接规则，未知语法显示原文。
- [ ] 高频格式/链接/图片/时间点/AI靠近正文，低频保留更多；状态源于NoteSessions实际保存回执。
- [ ] 选区AI走既有权限、预览、hash/editVersion确认；不自动覆盖。
- [ ] UI及源码无损回归、提交。

## Phase 4 — 播放器与布局

Files: LearningPlayer.tsx、NativeLearningSurface.tsx、playbackHandle.ts、learning css、Rust native player commands。

- [ ] 先测播放控制参数、容器断点与原生尺寸；扩展现有控制而非创建播放器。
- [ ] 容器布局、临时适配不写回尺寸；焦点限定Space/J/L/F11；音轨、逐帧、比例、控制栏隐藏。
- [ ] 时间轴悬停时间、已有缩略图、章节/知识标记、时间点复制/笔记；不等待缩略图。
- [ ] 多尺寸与真实mpv回归、提交；物理DPI若无法实测明确记录。

## Phase 5 — 后台任务中心

Files: src/features/tasks/*，复用源状态类型和事件。

- [ ] 测试downloads/media/AI适配，类型+ID、可靠进度、动作能力、事件与快照竞争。
- [ ] 全局订阅持久任务；缩略图/手动笔记接入生命周期明确的临时操作，不伪造历史。
- [ ] 分页/筛选/详情/输出/脱敏错误/底层实际支持的取消重试；完成通知点击目标。
- [ ] 并行任务与离页回归、提交。

## Phase 6 — AI与学习联动/撤销

Files: PlayerAiSidebar.tsx、VideoNotes.tsx、NoteTools.tsx、knowledge shared actions、undo。

- [ ] 测试字幕引用时间来源、草稿应用拒绝过期、撤销失败不丢状态。
- [ ] 字幕搜索/复制/跳转/选择引用到新旧笔记、提问、章节总结复用现有AI隐私与预览。
- [ ] 笔记移动重命名/收藏及课程分类调整使用守卫逆操作；回收站恢复保留冲突检查。
- [ ] 验证、提交。

## Phase 7 — 最终验收交付

- [ ] 版本及锁文件统一0.5.2；Node/Rust/生产/NSIS构建。
- [ ] 1000/5000课程、1000笔记、长文/字幕、键盘/窗口尺寸/模拟DPI实测并记录范围。
- [ ] 隔离数据真实桌面mpv/Markdown/布局恢复，不接触用户安装及凭据。
- [ ] docs/v0.5.2-delivery.md、修改文件/阶段提交/截图/已知限制。
- [ ] 独立审查与回归修复、推送、创建并附加Draft PR；不合并或发布。

## Ledger

2026-10-20：PR9仍未合并，当前基线e8f0740。复用已附加managed worktree并创建独立分支。基线首轮构建发现依赖安装缺少锁文件已有Tiptap，按原锁文件npm ci修复，保留首轮失败记录，不调整源码或测试掩盖环境问题。
