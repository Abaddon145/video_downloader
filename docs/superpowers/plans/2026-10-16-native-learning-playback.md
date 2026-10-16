# Embedded Native Learning Playback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans inline, task-by-task. Steps use checkbox syntax.

**Goal:** 交付v0.4.2内置原生学习播放，跳过整片预转码并保留原学习功能、源码、Windows安装包与新Draft PR。
**Architecture:** 保留现有学习页与手动WebView兼容缓存；默认使用受控mpv进程、Windows子窗口及本地JSON IPC。复用原进度、笔记、文件验证与Job Object。
**Tech Stack:** 现有Tauri2/Rust/windows-sys/React/TypeScript；新增捆绑mpv.exe，不增加生产JS或Rust依赖。
**Spec:** docs/superpowers/specs/2026-10-16-native-learning-playback-design.md（用户已确认）。

## Global Constraints
Windows10/11 x64；版本0.4.2；DB v5不变；原媒体/进度/笔记保留；视频仍在映流内；默认播放不创建兼容转码；手动兼容路径保留；禁用户配置/脚本/外部引用/网络媒体；固定IPC命令和本地文件校验；子窗口与进程树归所属会话；首帧本机目标3秒、硬验收5秒；新Draft PR，不merge/Release，不覆盖旧交付。

## Review Focus
1. StrictMode、旧start迟到、新课已开始、立即关闭：旧进程/窗口必须回收且不得停止新课。Task1/3真实竞态检查。
2. Native画面覆盖WebView：DPI/滚动/弹窗/全屏时位置、隐藏与键盘焦点必须正确。Task2/3。
3. 暂停/倍速/隐藏/退出同时保存：获取实际位置后按原序列保存，旧事件不覆盖新课，结束只推进一次。Task1/2/3。
4. IPC无响应、组件损坏/缺失、异常媒体：有限超时、可重试、无孤儿进程，不自动回到整片转码。Task1/3。
5. 路径中的Unicode/引号、字幕切换、先重新定位/移除再返回：只打开经过验证的当前普通文件，不授权任意命令/窗口，原片不修改。Task1/3。

### Task 1: Controlled Native Engine and Learning API
**Files:** 新增src-tauri/src/learning/native_player.rs、native_window.rs；修改learning/mod.rs,service.rs,player.rs,compatibility.rs,commands.rs,lib.rs；scripts/prepare-tools.mjs,extract.ps1；resources/components.json,THIRD-PARTY-NOTICES.txt,licenses/,mpv-input.conf；tauri.conf.json资源与版本、package/lock、Cargo/lock自身版本、App版本。
**Interfaces:**
- NativeViewport{left,top,width,height:f64,visible:bool}（CSS像素、有限值，后端按主窗口scale_factor转换并限制client范围）。
- NativeSnapshot{session,itemId,ready,position,duration,rate,paused,volume,muted,ended,subtitles:[{id,label}],subtitleId,error}，时间秒，volume0..1，rate七档。
- NativeAction tagged type：pause{paused},seek{position},rate{rate},volume{volume},mute{muted},subtitle{id?},loadSubtitle{path},fullscreen{enabled}；只生成固定mpv命令。
- learning_native_start(itemId,session,viewport,autoplay)->NativeSnapshot；learning_native_control(session,action)->NativeSnapshot；learning_native_viewport(session,viewport)->()；learning_native_save_progress(session,ended?)->PlaybackProgress；事件learning-native-updated与learning-native-shortcut均含session/itemId。
- learning_prepare_player增加native?:bool（默认false保留已有调用），prepare_player增加native_requested参数。native=true且非proxy/Compatible、fallback=0时返回plan.mode='native'、原文件，不生成视频或提取整片字幕；force重试仍走native。其他参数保留旧路径。
- LearningService.native_start/native_control/native_viewport/native_save/native_stop/native_pause/native_shutdown；cancel_player统一取消所属native和旧会话。窗口隐藏/所有退出路径调用暂停保存/关闭，不影响下载托盘行为。
- native_window只负责Create/Resize/Show/Destroy属于当前主窗口的STATIC子窗口；创建通过主线程，任何全局会话锁不能跨主线程等待。创建中取消以session所有权复核回收，句柄不接受前端输入。
- Native会话固定两条IPC连接：一条只读事件，一条串行请求/响应（request_id匹配），避免同步管道读写互阻。命令回应3秒上限；ready等待5秒；超时杀所属Job并返回错误。事件线程用弱引用避免会话循环存活。
- MPV固定可信输入配置只包含现有学习快捷键/客户端消息，默认脚本与外部配置关闭；硬件解码auto-safe，允许软件解码。prepare脚本支持固定7z提取、归档和binary SHA校验；runtime版本检查。
- 进度仍走LearningService.live_progress/flush_progress，原生定期更新/5秒持久化；暂停、切课、隐藏、退出与结束前读实际状态，序列来自progress::sequence。原生前端不得再提交过时的HTML快照。
- 固定组件：mpv-x86_64-20261015-git-492bae0768.7z；GitHub发行asset SHA256 f9becefe40e2a29d2ce5f3ab231ff094f9cf3e4038ff014cfe4a0c09ad90e79e；URL https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20261015/mpv-x86_64-20261015-git-492bae0768.7z。保留其原构建信息/许可和mpv源码commit，安装包包含所有必需运行文件。
- [x] RED：运行既有artifacts/v042/measure-startup.mjs失败基线（已观测433ms首帧、5秒未就绪）。新增Rust tests native_rejects_unsafe_controls、native_viewport_is_finite_and_bounded、native_ipc_matches_response_and_ignores_events；断言NaN/Inf/越界rate、非字幕/链接路径、不匹配request_id不被接受。运行cargo test --manifest-path src-tauri/Cargo.toml --locked --offline --lib；预期新增边界未实现而失败。
- [x] GREEN：实现上述内核、API与组件准备。事件以playback-restart确认首帧，不用file-loaded冒充出画；用户选择的普通文件才可loadfile，保留手动缓存功能。原生初始恢复位置/默认倍速在加载前设置；原生媒体错误不自动触发整片转码。预期真实WMV原生API5秒内ready且媒体队列无新增learningPlayback任务。
- [x] Verify：npm test；cargo test --manifest-path src-tauri/Cargo.toml --locked --offline；npm run build；发布程序无bundle构建及真实API启动检查。预期全部通过，提交feat(learning): embed controlled mpv playback and preserve session progress。

### Task 2: Reuse Learning Controls with Native Surface
**Files:** 新增src/features/learning/NativeLearningSurface.tsx、playbackHandle.ts；修改LearningPlayer.tsx,lib.ts/lib.test.ts,src/types/learning.ts,styles.css；scripts/qa原学习UI检查按需更新。
**Interfaces:**
- PlaybackHandle只含当前控件已使用的currentTime,duration,playbackRate,volume,muted,paused,play(),pause()及native可选persist(ended):Promise<PlaybackProgress>；HTMLVideoElement本身可作为web句柄。
- NativeLearningSurface接收itemId/session/autoplay/handleRef以及loaded/time/rate/play/pause/ended/error/shortcut回调。其IPC动作串行，persist等待先前动作，再由后端读取真实进度；组件取消后停止发送/丢弃迟到事件。
- Geometry用ResizeObserver/滚动/窗口resize/DPI与modal open状态同步，viewport命令独立于播放动作；可见画面严格限当前视频区域，弹窗时隐藏。原生全屏使用主窗口全屏+播放器固定布局；退出/离开恢复。
- LearningPlayer默认调用native:true，native模式渲染surface，既有web fallback渲染video；复用同一控制条、笔记、上下课和倒计时。Native失败仅展示重试与“转换为兼容视频”手动操作，nextPlaybackFallback('native',0)==null。
- 字幕原生选择使用track id，加载本地字幕直接调用loadSubtitle；web字幕路径不变。用户看见“直接播放原文件”，实现细节仅在技术详情出现。
- [x] RED：lib.test.ts新增native_failure_never_silently_starts_full_transcode；浏览器检查native_request_is_default、stale_native_events_are_ignored、native_pause_seek_rate_persist_in_order、modal_hides_native_surface。预期旧界面请求和fallback行为失败。
- [x] GREEN：接入共享句柄和事件；保留当前快捷键、音量、字幕、进度、笔记。预期原12类学习UI场景及新原生状态通过，不增加设置页控制。
- [x] Verify：npm test；npm run build；Rust全套；学习UI/生命周期/窄窗口/StrictMode检查。预期全绿，提交feat(learning): play original courses inside the existing learning interface。

### Task 3: Real Startup Acceptance and Delivery
**Files:** 新增scripts/qa/verify-learning-native-playback.mjs；复用旧fixture/desktop/MediaPro/UI harness；README、docs/native-learning-playback-v0.4.2.md、delivery/review/execution/evidence。
**Interfaces:** 原生实际截图/首帧时间/进程和视频队列证据、12格式操作结果、安装器与源码摘要、新Draft PR based codex/learning-playback-thumbnail-v0.4.1。
- [x] RED→GREEN：同一157秒WMV3与VC1、12格式UI点击后真实首帧，5秒内ready且无整片转码；seek至80%立即继续、1.5倍速、字幕、笔记、暂停后立刻隐藏/退出恢复。保留旧基线结果对比。实际图像/原生窗口存在及视频尺寸证明出画，不能只断言进程存在。
- [x] 验证快速切课/StrictMode/连续取消、新会话不被旧取消杀死；移动/移除文件；弹窗/缩放/880宽/125%与150%模拟/全屏/键盘；组件缺失/损坏/IPC不响应/坏媒体均有限失败，无孤儿进程，原片SHA相同；手动兼容转换与下载/旧16MediaPro回归。预期全部通过，限制逐项记录。
- [x] 验证npm test、cargo test --manifest-path src-tauri/Cargo.toml --locked --offline、npm run build。执行一次独立整分支审查，集中一次Important/Critical RED→GREEN修复，不重新审查；Minors/Rulings全部记录。
- [ ] 构建npm run tauri -- build --bundles nsis并等退出0，再做任何同路径程序启动；核对0.4.2/中文/x64/WebView2/所有组件SHA与许可。新目录交付installer+git archive源码ZIP+SHA256，核对文档链接、源提交和只含文档的构建后变化。
- [ ] 提交/推送独立分支，创建并附加新Draft PR（不改旧PR历史、不merge/Release）；归档ledger与证据，验证绝对路径后只清理本计划临时工作区，保留源工作树。预期用户得到可安装、可核验的具体文件与PR链接。
