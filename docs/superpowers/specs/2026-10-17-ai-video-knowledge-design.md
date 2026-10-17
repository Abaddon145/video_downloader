> 2026-10-17用户更新（优先于下文旧UI描述）：不设置独立AI分析一级模块。AI只在学习库对应课程内使用，播放器右侧固定“总结、字幕、问答、笔记”，课程显示简明分析状态和进度；分析历史/取消/重试/重新生成/删除在课程内。知识库仍为一级导航。AiService保持独立。新版原文见docs/evidence/ai-v0.5.0/updated-execution-document.md，该文件实际在第25节Rust处结束，其他未覆盖的安全、知识库和交付要求沿用上一版。

# 映流 v0.5.0 AI 视频分析与开放知识库设计

依据：用户在本会话提供的《映流 v0.5.0 AI视频分析 + Obsidian兼容知识库 Codex执行文档》0–125 节。该文档是产品规格与阶段顺序；本文件明确落实到现有仓库的边界，不减少其完成定义。

## 目的和基线

产品升级为“映流 Yingliu，本地视频学习与知识工作台”。保留 v0.4.2 的所有下载、Cookie/代理/托盘、媒体编辑、mpv 原生播放、多格式、学习库、进度、笔记、字幕和缩略图能力。

基线为未合并 PR7 的 codex/native-learning-playback-v0.4.2，806fe45caa4a7ffa2981c63358e5accd786bbbb0。复用已附加的隔离工作树，分支 codex/ai-video-knowledge-v0.5.0。Phase0 实际结果在 docs/evidence/ai-v0.5.0/baseline.txt。

## 架构

保留 LearningService，新增 AiService、KnowledgeService，各自连接现有 learning.db，统一使用 learning::db::open 的事务迁移、WAL 与外键。服务不在网络或外部进程等待期间持有数据库锁。App.tsx 只接导航、路由和全局入口；两个新模块独立。

AI 数据流：只读本地源 → FFmpeg 16kHz 单声道 PCM WAV / 精选 JPEG → 本地 whisper.cpp 时间码 → 自然字幕边界分块 → DeepSeek 结构化块结果 → 二次总结 → SQLite → 普通 Markdown 文件。无音轨走视觉，纯音频走语音。绝不上传原视频或 WAV。

AiRuntime 一次一个分析任务，默认一个块请求，最多两个。状态 queued/extracting_audio/transcribing/extracting_keyframes/analyzing_chunks/finalizing/writing_note/completed/failed/cancelled/interrupted。重启将活动状态改 interrupted，不自动产生费用；继续复用已完成转录、帧和块；重新总结复用本地材料；完整重做重新生成材料。每关键阶段核对大小、mtime、Windows 文件标识和快速哈希。取消中止所属进程树和网络 future，禁止迟到结果/笔记发布。

## AI 安全、客户端和模型

只连接 https://api.deepseek.com，不提供自定义 Endpoint，不跟随重定向。Rust reqwest 执行 HTTPS JSON，连接15秒/请求120秒；429/500/502/503/504 最多重试2次，退避1秒、3秒；401/402/断网/超时提供脱敏中文错误。响应、单图、请求体、文本和数组都设上限。图片只允许用户授权的本地精选 JPEG，最大宽1280，质量约80，每请求最多6张。

模型配置集中管理：默认 deepseek-flash，视觉只用已确认支持图像的模型；Pro不作为依赖。官方 vision/model 文档已确认 Flash 图片能力。请求使用 JSON 输出；Rust 再验证字段、枚举、长度、来源ID、时间在真实范围、章节 start<end 和排序。字幕/OCR/截图均是不可信数据；System明确其命令不可执行，模型无工具、SQL、路径或本机命令权限。Prompt版本 v0.5.0-1。

Key 使用 native::protect/unprotect 存在 ai/deepseek.key，普通设置与 state.json/SQLite/诊断均不含 Key。为了满足“React永远拿不到完整Key”，更换按钮调用 Windows 原生凭据录入，Tauri 命令不接收前端明文 Key 参数；React只得到 hasApiKey。删除清理密钥及所属临时文件，换钥覆盖加密存储，不产生明文备份。真实 API 验收须由用户在该窗口配置 Key，不通过聊天收集。

Whisper CPU x64 CLI 与必要 DLL 固定发行、URL、SHA256 和许可，加入 prepare-tools；不得 latest。Base/Small 不进安装器，首次按用户选择下载或导入本地模型，可信上游 SHA256校验、临时文件、进度/取消/重试、通过后原子发布。模型和进程参数在每次运行前核验，原视频只读。

三档集中常量：快速180秒/最多12帧；标准90秒/最多24帧；深度45秒/最多60帧。场景检测加固定时间兜底，去重并记录 SHA/时间/场景分数。字幕按自然边界与长度估计切块，通常约5分钟；最终总结只发块结构和必要摘要，不重发所有图片。

## SQLite v6

新建 schema-v6.sql，在一个事务内从v5升级；拒绝>6；schema_version和PRAGMA user_version均为6。新增 video_analyses、transcript_segments、analysis_chunks、analysis_keyframes、ai_usage、knowledge_index、knowledge_links、knowledge_tags及必要状态设置。分析删除不删除源文件；usage保留原始账单事实，分析删除后关联置空。课程删除不连带删除开放笔记。

FTS5仅加速：支持则建独立 transcript_fts/knowledge_fts 显式更新；不支持或无匹配时采用受限LIKE，不能让迁移/启动失败。字幕和笔记搜索分页，QA只检索TopK相邻片段，绝不每问上传全片；不足时明确不知道。回答引用必须由已检索来源且在视频范围内，点击跳转复用当前播放器。

## Vault 和文件安全

首次进入可新建 Documents/映流知识库 或选择已有 Vault；生成内容默认 Vault/映流/，附件目录 Attachments。Markdown文件为唯一真源；SQLite只做可重建索引和关联。绝不打包Obsidian、不修改.obsidian、不安装插件。

KnowledgeService所有路径均为当前Vault相对路径，拒绝绝对路径、UNC、设备、..、保留Windows名和受保护/隐藏目录；校验canonical root、普通文件、symlink/junction和父目录稳定性。索引忽略.obsidian/.git/node_modules/.yingliu-trash、系统隐藏目录，文件上限5MiB，树/正文分页。写入持有目录保护、唯一临时文件、sync、原子replace；expected_hash冲突时返回外部版本，不静默覆盖。显式覆盖仍使用最新哈希进行条件保存。

支持Frontmatter、[[Note]]/[[Folder/Note]]/[[Note#Heading]]、标签、相对图片与Obsidian附件、反向链接、全文搜索和大纲。CodeMirror6编辑，react-markdown+remark-gfm安全预览，不启用raw HTML或dangerouslySetInnerHTML。内部yingliu://video/<item>?t=<ms>解析为受校验的播放器操作，本版不注册Windows全局协议。Obsidian URI正确编码，未安装可继续内置编辑。

AI建议标签不生成路径；Rust复用naming清理目录。Frontmatter含标题、source、item_id、analysis_id、duration、模型、模式、时间、tags，默认不写本地绝对源路径。生成视频总结/按2–5分钟聚合完整转录/关键知识及附件；同名不覆盖，重做保护用户外部改动。截图笔记取实际播放时间、FFmpeg准确取帧、保存附件与时间点笔记，取消和故障保留用户已发布笔记。

## UI、隐私与费用

导航：下载、学习库、AI分析、知识库、媒体工具、历史、设置。延续现有暗色媒体工作台，880宽可用；播放器宽屏约65/35，AI总结/字幕/笔记/知识库Tab，窄屏上下布局。AI页围绕视频、任务/阶段/结果/章节/问答/使用量，不做聊天机器人首页。知识库三栏可收起，编辑/预览/分屏，默认编辑，外部修改比较/覆盖/重载。

入口覆盖课程菜单、播放器、本地媒体和下载完成可选项；自动分析默认OFF。开始前确认模式与字幕/关键帧上传范围；两者都禁用时只允许本地转录。自动生成知识笔记设置默认ON；结果页仍可查看、生成和打开笔记。用户可仅字幕或仅图片；不自动改隐私开关。

Usage记录真实input/output/cache/model/operation/time，价格只在独立可更新配置中估算，明确以账户账单为准。日志不含Key、Authorization、全文字幕或完整请求/响应。临时WAV默认成功后清理；清理不删除转录、结果或Vault附件。删除记录默认保留源和Markdown，删除生成笔记须再次确认。

版本统一 package/Cargo/tauri 为0.5.0，React用Tauri应用版本API而非硬编码。新增库只用必要的reqwest/base64/YAML、CodeMirror/Markdown，不建设通用Provider、向量服务或新播放器/进程/加密框架。

## 完成与验证

严格Phase0–12顺序、每阶段测试通过才继续、分阶段提交。保持既有测试，v5迁移保留所有数据；覆盖API错误/非法JSON/越界/Key泄漏/上传范围、Whisper组件/模型/取消、无音轨/音频、12类媒体与语言/时长、复杂路径、Vault冲突与逃逸、原片前后SHA、UI截图/窄窗口/缩放和完整旧功能回归。

真实API与mock测试明确区分，无Key不伪造真实成功。不同实体Windows/GPU/DPI未实测的明确记录。最终交付源码、中文NSIS、实际结果、许可/来源/SHA以及新Draft PR“映流 v0.5.0：AI视频分析与Obsidian兼容知识库”。不merge、不发布Release。阶段记录和证据保存 docs/evidence/ai-v0.5.0/。
