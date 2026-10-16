# 映流 v0.4.2：原文件直接学习播放

学习库默认使用捆绑的 mpv 在映流窗口内直接读取原文件，不再等待整段兼容转码。WMV、MKV、AVI、MOV、MP4、WebM 等格式共用原播放条、倍速、字幕、时间点笔记、进度和上下课功能。数据库仍为 v5。

## 使用

安装新版后进入学习库，点击课程即可。标题右侧显示“直接播放原文件”。默认使用可用的硬件解码，必要时使用软件解码，不预先编码整段视频。

播放失败可重试、重新定位，或主动点“转换为兼容视频”。设置中的“兼容视频”偏好仍使用原 H.264/AAC 缓存路径。故障不会自动触发整段转码。

Space、左右方向键、Shift+方向键、M、F、N/P 保留；原生画面中的操作也会取消自动下一课倒计时。全屏下控件获得焦点时 Escape 也可退出。关闭窗口进入托盘时暂停学习播放并保存，下载继续后台运行。

## 验收

- 157 秒 WMV3/WMAPro 原片：旧实现 5 秒仍未就绪，首帧纯解码约 433ms；新版内核约 445ms 就绪，真实 UI 点击约 897ms 就绪。
- 12 种真实编码/容器逐课点击：约 698–792ms 就绪；包含 WMV3、VC1、H.264、HEVC、MPEG4、MPEG2、VP9，均未创建学习播放转码任务。
- 各格式可跳至 80%、切换 1.5 倍速、暂停保存。内嵌和本地文本字幕、时间点笔记、重启恢复通过。
- 真实原生验收 11 组，加托盘隐藏/立即退出和损坏媒体 2 组全部通过。覆盖迟到取消、新课保护、故障窗口回收、3 秒 IPC 超时、缺失/损坏组件、原片 SHA 不变、手动兼容转换。
- 界面验收 9 项；Rust 96 项；Node 23 项；原媒体工具回归 16 项；真实 yt-dlp 下载/学习库/兼容播放集成回归 8 项全部通过。
- 已查看实际窗口截图：应用内画面、笔记弹窗、880 宽窗口、全屏与键盘操作。125%/150% 缩放坐标计算有测试；未宣称不同实体 GPU、多显示器或真实不同系统 DPI 均已通过。

以上时间为本机指定样片的结果，不是所有硬盘、编码和设备的时延保证。数据和截图在 [验收证据](evidence/native-learning-v0.4.2/)。

## 实现和组件

mpv 使用受控 Windows 子窗口和 Job Object，固定两条本地 IPC 连接。媒体只允许已登记的本地普通文件；禁止外部配置、脚本和网络引用。事件和控制都按会话归属处理，取消或错误清理所属窗口和进程。

mpv 为 mpv.io 列出的 shinchiro Windows 构建，固定 20261015 / 492bae0768。归档 SHA256：`f9becefe40e2a29d2ce5f3ab231ff094f9cf3e4038ff014cfe4a0c09ad90e79e`。来源、构建源、许可证和二进制摘要随包交付。

FFmpeg 下载改为固定的 2026-10-07 发行归档，替代会变化的 latest；经逐文件 SHA 校验，ffmpeg.exe 和 ffprobe.exe 与 v0.4.1 完全相同。下载、编辑、转码的运行版本没有变化。

## 复核

```powershell
npm ci --ignore-scripts
npm run prepare:tools
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked --offline
npm run build
npm run tauri -- build --bundles nsis
```

真实桌面脚本：`scripts/qa/verify-learning-native-playback.mjs`，指定 `PLAYWRIGHT_MODULE`、`QA_FIXTURE_MANIFEST`、`QA_ORIGINAL_WMV` 和 `QA_OUTPUT_DIR`。原下载集成脚本设置 `QA_LEGACY_WEB=1` 可验收主动选择兼容视频的路径。

升级保留原学习库和笔记。新版安装包增加播放内核；源码 ZIP 不包含下载的第三方可执行文件，运行 prepare:tools 按固定摘要准备。
