# 映流 v0.5.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在PR8完整基线上修复三项P0，并升级可自定义的视频学习和Markdown知识工作台。
**Architecture:** 复用现有AiService、KnowledgeService、LearningService、原生mpv及原子保存。先解决后端可靠性，再复用统一工作区配置和菜单；Markdown始终为真源。独立模块分阶段测试和提交。
**Tech Stack:** Tauri2 / Rust / SQLite / React19 / TypeScript / CodeMirror6；富文本仅在可证明往返不丢内容时使用Tiptap。
**Spec:** ../specs/2026-10-18-v051-execution-document.md（用户提供的执行设计原文）

## Global Constraints
- 基线986c1e0ab14ef8f13e349edd116e5484cdefcacc，包含PR8全部功能；分支codex/v0.5.1-workspace-ai-knowledge-fixes。
- 不修改DCC源仓库；参考PR14固定63f37a4136fc30f0b75bf3f662f0d085bb2cea64。
- 不合并、不发布Release、不发布正式安装包；仅Draft PR。
- 源视频只读，密钥仅Rust安全层；布局导出白名单不包含AppSettings、凭据或API Key。
- 不修改.obsidian，不自动注册Vault；系统接受URI仅表示请求已发送。
- 保留v0.5.0数据、Markdown/YAML/Wiki/附件/任务/代码块；转换不无损则回退源码。
- 使用现有深色专业风格，880px可操作；原生mpv CSS坐标只在Rust乘一次DPI。
- 按难度分派GPT-6.1 Sol高/中推理、常规文档GPT-6 Luna；禁用GPT-6 Astra。

## Review Focus
- 分析成功分段不重付费用；取消、撤销隐私授权、源文件变化必须阻断局部修复及总结恢复请求。
- 多Vault、目录移动、同名笔记不得打开错误文件；未登记状态不得误报成功。
- 自动保存期间继续输入、切换标签或外部Obsidian改稿不丢草稿，冲突不静默覆盖。
- 原生视频子窗不得遮住右键菜单；切换布局不重新挂载播放器，不丢进度。
- 损坏/未来版本/异常大布局文件和未知Markdown节点安全回退，错误不修改源内容。

## Task 0 / Phase 0 — 基线、备份与证据
Files: docs/evidence/v0.5.1/, docs/superpowers/specs/2026-10-18-v051-execution-document.md。
- [x] 核对PR8/参考PR14真实HEAD，建立新分支；保留上轮未提交修复。
- [x] SQLite backup API生成一致性私有快照并quick_check；只读复制Vault并逐文件SHA核对，密钥不复制。
- [x] 对固定HEAD运行npm test / npm run build / cargo test --locked，记录当前基线，不把已有新回归当作旧基线。
- [x] 将本计划与用户原文保存仓库，记录分阶段实际结果和提交SHA。

## Task 1 / Phase 1A — AI引用局部恢复
Files: ai/analysis.rs, ai/references.rs, ai/prompts.rs, ai/models.rs, ai/reference_tests.rs, frontend VideoSummary。
Interfaces: normalize_keypoint_references(result, scope)区分合法、最多3000ms小偏差、待修复；repair_keypoint_references只接收失败重点及其有界引用材料。
- [ ] RED：精确/500ms/3000ms修正；较远或不相关引用不强行锚定；未知ID/原时间越界不直接接受；有效重点保留；引用局部修复后二次严格校验。
- [ ] 先本地有据校正；仍无效的重点最多一次小预算AI修复。对仍无效重点局部降级，结果显示“部分重点无法核对已略过”，其他有效内容继续。
- [ ] 不关闭scope/来源/长度校验；不保存截断JSON；本地解析、实时请求、缓存走相同规范化边界。所有新增请求记录usage，前后检查取消/授权/指纹。
- [ ] 校验旧任务继续仍复用已完成段/转录/关键帧；模板版本兼容规则写明并测试。
- [ ] GREEN：引用/恢复/隐私测试以及cargo test全量；提交fix(ai): recover invalid keypoint references locally。

## Task 2 / Phase 1B — 总结截断与预算
Files: ai/client.rs, ai/summary.rs, ai/qa.rs, ai/analysis.rs, ai/analysis_tests.rs。
Interfaces: RequestPurpose::{Chunk,Summary,Question,ReferenceRepair}给出独立输出预算；ChatReply保留结构化truncated状态和原始usage事实。
- [ ] RED：finish_reason=length不会入库；一次恢复成功入缓存；连续截断有界终止；取消/撤销授权阻止下一请求；每次响应均计usage；重启复用阶段缓存。
- [ ] 保持固定官方HTTPS和响应上限。不同用途预算集中常量，按已核验的模型容量设置；不盲目扩大响应到无限。
- [ ] 最终总结先限制输出篇幅；截断后一次简洁重试；仍截断且有多段时按结构化分段二分整理并缓存中间结果，再合并。层数/总请求预算受限，不要求手动缩短长课程。
- [ ] GREEN：真实HTTP客户端+本地模拟异常/usage/恢复测试；cargo test全量；提交fix(ai): bound summary output and resume hierarchical recovery。

## Task 3 / Phase 1C — Obsidian与笔记移动
Files: knowledge/obsidian.rs, models.rs, service.rs, commands.rs, service_tests.rs, KnowledgePage.tsx, ObsidianConnection.tsx, TS DTO/API。
Interfaces: 独立ObsidianBinding{yingliuVaultId,obsidianVaultIdentifier,kind,confirmedByUser}; connection DTO为未配置/协议不可用/待登记/待实测/用户确认/请求失败。
- [ ] RED：URI特殊字符仅编码一次、不同根绑定失效、未登记不直接发打开成功、缺失/越界文件不外发。
- [ ] 复用url::Url，绑定后vault+relativefile；绝对path仅显式兼容方式。只读AssocQueryStringW检测协议，choose-vault打开管理器；不读取密钥、不写Obsidian配置。
- [ ] 首次点击显示引导、当前根、复制/资源管理器/配置ID或明确确认名称/测试/稍后；未保存笔记先经已有保存保护。请求发送后由用户确认实际效果。
- [ ] 移动关联更新放入SQLite事务，失败撤销事务并回移文件；回移失败明确报实际新路径供恢复，禁止吞SQL错误。注入第二张表更新失败验证两张表/文件位置一致。
- [ ] GREEN：后端全量和浏览器交互；记录Windows真实Obsidian人工验收，不将URI启动称应用级回执；提交fix(knowledge): bind registered Obsidian vaults and preserve moved links。

## Task 4 / Phase 2 — 学习库与播放器布局
Files: learning/LibrarySeriesView.tsx, LearningLibraryPage.tsx, LearningPlayer.tsx, NativeLearningSurface.tsx, components/ResizeHandle.tsx, workspace/types.ts, Rust learning查询。
Interfaces: LibraryView=cards|list|table；PlayerLayout=standard|stacked|theater|focus；ResizeHandle(value,onChange,onCommit,min,max,orientation)。
- [ ] RED：三视图数据/动作一致；排序先数据库再分页；列宽/可见列/分类折叠重启保留；拖动/键盘分隔线有界；四布局切换不重建mpv。
- [ ] 一次查询系列时长、最近学习、AI/文件聚合状态；复用原控制器和课程动作，不复制数据请求。
- [ ] 原生遮挡钩子覆盖dialog和data-native-overlay；窄屏降为上下，full screen优先，减少动画。
- [ ] GREEN：npm test/build、真实mpv布局/跳转/全屏/DPI人工矩阵；截图；提交feat(learning): add configurable library and player layouts。

## Task 5 / Phase 3 — 统一右键菜单
Files: components/ContextMenu.tsx,学习系列/课程/下载/媒体/知识树目标动作适配。
Interfaces: MenuItem{id,label,disabled?,danger?,children?,run?}；onAction必须携带真实目标ID/路径和选中集。
- [ ] RED：右键/Shift+F10、上下/左右/Enter/Escape、焦点返回、屏幕边界、高菜单滚动、禁用项、目标和多选正确。
- [ ] 统一菜单基础组件，业务复用已有act/lessonAction/runBatch/冲突与危险确认链。无实现的菜单项不能仅做占位。
- [ ] 原生mpv菜单遮挡回归；输入区域保留常规编辑操作；批量操作不静默删除源文件。
- [ ] GREEN：交互QA覆盖四模块；截图；提交feat(ui): unify contextual actions and keyboard menus。

## Task 6 / Phase 4 — 用途模板
Files: ai/templates.rs, template_tests.rs, AnalysisConfirm.tsx, AiTemplateManager.tsx, TS类型/设置、AI缓存和Markdown生成。
Interfaces: AnalysisTemplate{id,version,name,description,focus,sections,detail,extractParameters,extractTerms,exercises,timeReferences,markdownTemplate,builtin,favorite}。
- [ ] RED：10类模板、官方不可改/可复制、自定义版本、深度独立、未知/损坏模板回退；用途配置不能越权上传/输出规则。
- [ ] 记录templateId/templateVersion入分析历史和所有AI结果缓存身份；使用新增可选字段默认通用模板，旧记录兼容。复用匹配转录/画面，不跨模板复用最终结果、不覆盖历史。
- [ ] 对结构化结果增加有界可选用途栏目；每模板真实输出展示/对应Markdown模板，不使用任意路径或可执行模板。
- [ ] GREEN：迁移/缓存/安全测试全量，确认页及模板管理QA截图；提交feat(ai): add versioned purpose templates and custom analysis goals。

## Task 7 / Phase 5 — 全局工作区
Files: workspace/models.ts, defaults.ts, preferences.ts, WorkspaceSettings.tsx, WorkspaceProvider.tsx, Rust workspace.rs及原子JSON设置。
Interfaces: WorkspaceSettings{version,appearance,currentLayout,presets}；内置default/learning/focus/batch/knowledge；自定义预设独立ID。
- [ ] RED：舒适/标准/紧凑、合法数值/枚举、损坏/未来版回默认、导出仅白名单不含Key、重启恢复和默认还原。
- [ ] 复用Task4工作区字段，主题/强调色/字号/侧栏/卡片/面板/动画/折叠/列设置贯穿主模块，不新增第二套布局状态。
- [ ] 新建/保存/重命名/删除自定义预设，内置不可删除；导入大小限制及完整校验。
- [ ] GREEN：全量和880px/125%/150%模拟QA，明确模拟与物理DPI差异；截图；提交feat(workspace): persist appearance and safe layout presets。

## Task 8 / Phase 6 — Markdown知识工作台
Files: knowledge/NoteSession.ts, NoteSaveQueue.ts, MarkdownRoundTrip.ts, KnowledgeWorkspace.tsx, NoteEditorTools.tsx, NoteStructureView.tsx, NoteMindMapDialog.tsx, Rust知识历史/附件接口。
Interfaces: NoteKey=vaultId+path；NoteSession{document,draft,editVersion,saveState,conflict}；每文档串行save(expectedHash,editVersion)；回执仅更新基线，保留后续输入。
- [ ] RED：边保存边编辑不丢字、多标签跨Vault隔离、冲突暂停、关闭/离开检查全部草稿；移动前排空队列并迁移会话key。
- [ ] 普通树状笔记本、多标签、自动保存、版本历史、收藏、模板、附件、AI整理；复用Vault边界和源Markdown，不移植DCC私有JSON存储。
- [ ] Tiptap转换先证明代表性Markdown无损往返；YAML/Wiki/嵌入/特殊语法不支持则保留原文并切源码，不静默清理。
- [ ] 大纲/导图复用已有有界AST；不引入无关大框架。版本历史保存在映流数据区，恢复仍检查expectedHash；附件复制校验路径/上限。
- [ ] GREEN：测试真实文件并hash核对，外部Obsidian修改/同名/中文/特殊字符矩阵，UI截图；提交feat(knowledge): add safe multi-note editing and Markdown-first workspace。

## Task 9 / Phase 7 — 交付
- [ ] 统一package/Cargo/Tauri0.5.1，UI继续getVersion，不硬编码版本。
- [ ] 全部旧/新测试、npm build、cargo test --locked、独立审查；有真实API凭据仅软件安全层和明确少量授权后运行。模拟与真实结果分开。
- [ ] 输出文件清单、阶段SHA、自动结果、Windows人工清单、Obsidian回执限制、修复前后对照、各模块截图和未解决项。
- [ ] 创建以PR8分支为基线的独立Draft PR，附本任务；不合并、不发布Release或正式安装包。

## Rulings
- 用户已提供详细执行设计并持续授权按难度分派实施；本计划拆分独立阶段，不重复要求确认既定范围。
- 已有时间修复未提交且会把远距离冲突锚定到来源；新文档明确要求小偏差校正/局部修复，故该实现视为未定稿，用新回归替换其不符合新设计的期望，保留旧安全断言。
- 现有172Rust/37Node为固定基线；上轮未交付修复174Rust通过仅作为工作中状态，不冒充原始基线。
- 无可靠应用回执时，Obsidian联动“已验证”只能来自明确用户确认；不引入插件或后台服务。

## Execution checkpoint

2026-10-18：Phase0完成，Phase1三项P0及移动关联修复完成开发验证。Rust193/0/8ignored，Node37，build通过；Obsidian22、课程面板35浏览器模拟检查通过。实际Obsidian和原课程真实DeepSeek复测留人工项。继续Phase2，不将Phase2–7标记完成。

Phase2开发和自动验证完成：Node42、Rust195/0/8ignored、build通过；浏览器布局与持久化回归通过，真实mpv/DPI留最终人工矩阵。继续Phase3。
