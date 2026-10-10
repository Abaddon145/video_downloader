# FFmpeg Media Tools Implementation Plan
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans inline, task by task.
**Goal:** 交付 v0.2.0 Windows 媒体工具和安装包。
**Architecture:** 独立媒体状态/队列，复用native Job、IPC和原子JSON；先底层再界面。
**Tech Stack:** 现有Rust/Tauri2/React19/TypeScript/捆绑FFmpeg；不加依赖。
**Spec:** ../specs/2026-10-10-media-tools-design.md；用户原始任务书及WebM确认具有约束力。
## Global Constraints
Windows10/11x64，v0.2.0，保持下载接口和视觉；WebM仅Copy；无FFI/任意参数/GPU/高级编码；结构化请求、本地协议、捆绑校验；绝不覆盖；独立串行媒体队列，取消进程树，重启手动重新开始。
## Review Focus
1. 第二音轨/字幕不兼容不得被静默丢弃（Task1）。
2. 合法扩展名中的网络播放列表不得联网（Task1/2）。
3. 发布时同名竞争不得覆盖（Task2）。
4. 探测/取消/退出竞态不得留下进程或错误完成状态（Task2）。
5. 迟到探测结果和事件不得覆盖新选文件/丢任务（Task3）。
### Task 1: 模型/探测/命令/进度
Files: src-tauri/src/ffmpeg/{mod,models,probe,command,progress,tests}.rs; lib.rs模块声明。
Interfaces: MediaInfo{videos,audios,subtitles}; MediaRequest{inputPath,outputDir,operation}; probe::parse(json,path), probe::arguments(path), command::build(request,info,temp_output), progress::parse(block,total_duration).
- [ ] 写六类JSON、五类命令、全部轨道兼容、Copy约束、时间边界、微秒进度测试。
- [ ] cargo test --offline，观察缺少接口失败。
- [ ] 最小实现上述接口，定向与完整Rust测试通过。
- [ ] 中文提交及账本记录。
### Task 2: 输出安全与执行队列
Files: ffmpeg/{output,runner}.rs; lib.rs IPC/三处退出。
Interfaces: MediaService::new(app,data,resources), start/snapshot/probe/create/cancel/retry/open_output/shutdown; MediaSnapshot{tasks,ready,error}; MediaTask{request,status,phase,progress,speed,processedTime,totalDuration,eta,error,logs,createdAt,finishedAt,outputPath}.
- [ ] 写中文/空格/长名/非法路径、同名竞争、恢复、取消状态测试，运行观察失败。
- [ ] 独立原子状态与单worker调度、同锁Job登记、结构化进度、非空输出、不覆盖发布、限定清理。
- [ ] 注册probe_media/get_media_snapshot/create_media_task/cancel_media_task/retry_media_task/open_media_output；探测spawn_blocking。
- [ ] 全Rust验证，提交账本。
### Task 3: 模块化界面
Files: src/types/media.ts, src/media.ts, src/media.test.ts, src/pages/MediaToolsPage.tsx, src/components/media/{MediaInfoPanel,MediaTaskRow}.tsx; App.tsx导航/styles.css/package.json test。
Interfaces: Rust camelCase模型，媒体snapshot事件；纯函数构建结构化请求；getCurrentWebviewWindow().onDragDropEvent本地拖拽。
- [ ] 先写时间/操作约束/请求/迟到结果/事件合并测试，npm test失败。
- [ ] 文件摘要、五操作、简单高级选项、50条任务页、折叠日志信息；键盘标签/减少动画/880px布局。
- [ ] npm test/build及离线UI全部入口检查，提交账本。
### Task 4: 实机/文档/版本/安装包
Files: README.md, docs/acceptance.md, package*.json, Cargo.toml/lock, tauri.conf.json, App.tsx第三方许可/版本；artifacts保存脚本证据产物，不提交二进制。
- [ ] 合成MKV H264/AAC、HEVC、多轨、10分钟低分辨率素材，实机验证九场景、ffprobe输出、原文件hash及进程退出。
- [ ] 更新v0.2.0和第三方FFmpeg来源/GPL构建许可，保留许可文件；如实记录限制。
- [ ] npm test → npm run build → cargo test --offline → NSIS；diff check。
- [ ] 一次整分支独立审查，关键问题一次RED→GREEN修复，完整回归后提交。
- [ ] 上传独立分支和草稿PR，保持PR2不变；交付安装包/摘要/验收，不合并或Release。

