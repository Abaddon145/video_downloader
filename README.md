# 映流 · Windows 视频下载器

基于官方 **yt-dlp** 的中文桌面下载器。Windows 10/11 x64，Tauri 2 + React + TypeScript。

## 日常使用

1. 安装后，从桌面或开始菜单打开「视频下载器」。
2. 粘贴视频、播放列表或分享文本，点 **解析链接**。支持一次解析多个链接。
3. 查看封面、编码和大小，勾选播放列表条目，选择视频 / MP3 音频 / SRT 字幕，加入下载。
4. 默认同时下载 2 个任务，保存到系统「下载」目录的「视频下载」文件夹。可在设置中修改。
5. 下载阶段可暂停后续传；合并与转换阶段可取消。点击已完成任务的播放或文件夹按钮打开文件。

**关闭窗口会进入托盘，后台继续下载。** 双击托盘图标重新打开；右键托盘 →「保存任务并退出」主动退出。下次启动后，未完成任务保持暂停，点击恢复才继续。

快捷键：`Ctrl+L` 定位链接输入；`Ctrl+Enter` 解析；`Esc` 关闭预览并停止解析。删除历史记录会保留下载文件。

## 登录、代理与更新

- 默认直接连接，不使用 Cookie。设置页可以读取 Edge / Chrome / Firefox 登录状态。浏览器占用或加密可能使读取失败，错误详情会提示改用文件导入。
- Cookie 文件须为 **Netscape HTTP Cookie File**，UTF-8、小于 2 MiB。导入后由 Windows DPAPI 加密保存；运行内核时临时解密，使用后删除。
- 代理需手动启用并保存，支持 HTTP、HTTPS、SOCKS5，例如 `http://127.0.0.1:7890`。代理凭据同样加密保存。
- 手动检查官方 stable 内核更新；暂停或完成所有解析与下载后再更新。新内核经官方 SHA256 与启动检查后切换，旧版保留用于失败回滚。
- 兼容视频使用官方 `-t mp4` 预设；预览显示实际编码。源站未提供 H.264/AAC 时，选择「源站最高画质」保留原编码。
- 平台可用性继承 yt-dlp，并受到网络、源站登录、地区和权限限制。实际验证结果见 [验收记录](docs/acceptance.md)。

## 从源码运行

需要 Node.js、Rust stable MSVC、Visual Studio C++ 构建工具及 Windows WebView2。依赖锁文件已包含在仓库中。

```powershell
npm ci --ignore-scripts
npm run prepare:tools
npm run tauri -- dev
```

组件准备脚本从官方发行地址下载 yt-dlp、FFmpeg/ffprobe、Deno，**校验固定 SHA256 后才允许打包**。下载使用本机静态系统代理（若存在）；软件内的代理仍默认关闭。FFmpeg 上游 `latest` 发行文件可能变更，校验不匹配时必须重新核对官方摘要，不应跳过校验。

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri -- build --bundles nsis
```

安装包输出在 `src-tauri/target/release/bundle/nsis/`。安装程序为当前用户安装，中文界面，并在需要时安装 WebView2。

## 文件与许可

设置、加密 Cookie、任务记录和可更新内核保存在 `%LOCALAPPDATA%/com.local.video-downloader/`。任务和设置采用原子 JSON 写入并保留最近一次有效备份。

保存目录中的 `.video-downloader` 存放每个任务的临时文件，用于暂停续传和保护已有文件。取消任务会保留已下载内容；确认不需要继续时可以自行清理对应临时目录。

组件版本、来源与摘要：`src-tauri/resources/components.json`、生成的 `tools/binaries.json`；上游许可：`resources/licenses/`、`THIRD-PARTY-NOTICES.txt`。桌面界面源码采用 MIT；官方 yt-dlp 源码采用 Unlicense，捆绑的 PyInstaller 独立 exe 和 FFmpeg GPL 构建保留各自上游许可。

本地验收可以设置 `VIDEO_DOWNLOADER_DATA_DIR` 将测试数据与日常数据分开；默认无需设置。
