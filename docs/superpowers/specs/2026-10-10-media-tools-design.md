# 映流 v0.2.0 媒体工具设计
依据用户 2026-10-10 FFmpeg Core 任务书，基于 de5a33f 保留下载能力。用户已确认 WebM 仅兼容编码 Copy，VP9/AV1 转码后续再做。

新增媒体工具导航：单文件选择/原生拖拽，异步 ffprobe 后摘要、五种操作、独立任务列表。保持现有石墨灰/青蓝/Windows中文字体/Lucide/圆角；高级选项、完整轨道信息和日志折叠。无对应轨道禁用操作，所有按钮可键盘操作。

MP4/MKV/MOV 按容器白名单支持 H.264/H.265/Copy 与 AAC/MP3/Opus/Copy，WebM 仅 Copy。质量映射 H.264 CRF18/23/28，HEVC20/26/30，高级CRF0–51、原始/720/1080/2160高度、原始/24/25/30/60fps、128/192/256/320kbps；Copy 禁止改变分辨率/帧率。裁剪单片段支持关键帧无损模式与精确H.264/AAC模式。提取MP3默认320kbps、M4A/AAC/WAV/FLAC/Opus及兼容Copy，不承诺提升原始音质。截图PNG默认/JPG/WebP。

默认转码/裁剪/截图使用首条非封面视频和首条音轨，提取首音轨，UI明确说明。Remux保留全部视频/音频/字幕轨道，遇不兼容轨道拒绝，禁止静默丢失。附件/复杂字幕转换/轨道选择后续再做。
默认输出原目录，可选择其他本地目录；操作后缀/截图时间自动命名。媒体串行独立队列，50条分页，取消/失败/中断可重新开始，没有伪暂停或转码续传。下载历史不混用模型。

ffmpeg 模块含 models/probe/command/progress/output/runner，独立 MediaService/media-state.json/media-snapshot-updated。复用native::spawn/Job/原子保存/脱敏；不复用Cookie/代理上下文。显式捆绑exe，binaries.json校验两个exe，components.json保留发行包来源/版本/摘要，两种摘要不可混淆；媒体就绪独立于下载。
IPC结构化枚举与字段，未知字段/非法时间/容器/路径拒绝。canonicalize本地regular file，拒绝网络/设备；探测及处理限制file/pipe协议和常见媒体demuxer，阻止播放列表主动联网。
ffprobe JSON含文件/视频/音频/字幕/metadata，未知值null。stdout/logs有界。FFmpeg -progress pipe:1 -nostats完整块读取微秒，发布成功前百分比<100，ETA可未知。错误中文，技术详情折叠且脱敏。

目标目录/.video-downloader/media-temp/<id>/存临时文件，-n/-nostdin。退出码0且输出非空后Windows不覆盖移动，重名竞争尝试(1)/(2)。不使用native::replace_file发布，不覆盖原文件。清理限本任务范围。状态保存失败回滚内存，Job登记和取消同锁；关闭窗口托盘继续，三处退出入口终止全部Job，未完任务中断，重启手动重新开始。

测试覆盖六种JSON/五种命令/全轨兼容/时间/路径/发布竞争/进度/重启/进程清理/迟到探测结果。合成小媒体验证九个真实桌面验收场景，含10分钟低分辨率素材。最后依次npm test/build/cargo test/NSIS，核对摘要，更新README/验收文档，交付v0.2.0。不新增依赖、不合并PR、不创建Release。

