# 映流 v0.2.0 交付报告

已实现 FFmpeg Media Tools：媒体信息、无损封装、H.264/H.265 转换、单段裁剪、音频提取、PNG/JPG/WebP 截图和独立媒体任务列表。延续原有下载功能与深色界面，不增加依赖。

## 文件与模块

- 新增 `src-tauri/src/ffmpeg/`：models、probe、command、progress、output、runner、tests。负责类型、探测、受控参数、进度、文件发布、独立队列、恢复与校验。
- 修改 `src-tauri/src/lib.rs`：六个媒体命令、启动服务、三处退出钩子。
- 新增 `src/types/media.ts`、`src/media.ts`、`src/media.test.ts`、`src/pages/MediaToolsPage.tsx`、`src/components/media/{MediaInfoPanel,MediaTaskRow}.tsx`。
- 修改 `src/App.tsx` 与 `src/styles.css`：媒体导航、第三方许可入口与一致的深色界面。
- 更新 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`Cargo.lock`、`tauri.conf.json` 为 v0.2.0，补充 README、媒体说明、验收与设计/实施文档。

## 验证与构建

Node 15/15；Rust 41/41；生产构建、中文 x64 NSIS 构建成功。媒体界面离线检查 13/13，原有下载界面回归 13/13。真实 Windows WebView2 本地媒体 13/13，覆盖任务书九个场景、原文件 SHA256、解码一致性、多轨容器检查、队列独立与重启恢复。

异常结束残留临时结果的首次重试另做真实回归，成功。截图、输出检查及测试记录随本地验收产物保存。检查基于合成媒体，不读取私人登录数据，不将本地测试记作源站下载验收。

一次独立全分支审查发现四个 Important，全部先复现失败再修复：扩展路径绕过、初始化快照迟到、残留临时文件首次重试、盘符根目录默认输出。完整回归通过，没有延期 Minor，也未追加第二轮审查。

## 实施取舍（Rulings I made）

1. Windows 原生进度账本代替技能提供的 Bash 包装脚本，保留步骤、提交和失败→通过证据。判断错误的成本：仅账本格式。
2. 默认串行处理一个媒体任务、保存源目录，转换类使用第一条对应轨道并显示说明；无损操作保留全部兼容轨道。判断错误的成本：后续需增加并发设置、默认目录设置和轨道选择。
3. 验证构建的正式程序及组件，不覆盖用户已有安装来测试同一 NSIS 注册身份。判断错误的成本：全新安装问题仍需干净虚拟机验收。
4. 审查未将 WebM 缺少 VP9/AV1 转码列为问题：遵循用户明确的 Copy only 决定。若需求改变，后续增加对应编码。
5. 审查未将默认首轨列为问题：保留已公开的首轨默认与完整信息显示。判断错误的成本：后续增加轨道选择。

## 限制与下一阶段

Windows 10/11 x64 为目标；本次实际测试为 Windows 11，未另测干净 Windows 10/无 WebView2 安装。输出经 ffprobe/解码检查，未人工逐一使用所有播放器播放。

媒体不支持暂停续传，取消/中断后完整重新处理；WebM 仅复制兼容源流；当前容器兼容名单较保守，Opus 视频音轨仅 MKV。多轨选择、GPU、AV1 高级转码、多段裁剪、滤镜、字幕编辑、目录批量和下载后自动处理留待下一阶段。优先建议增加轨道选择与批量处理，再按实际需求接 GPU/字幕。

原有 B站 HTTP 412 和个人浏览器 Cookie 验证边界继续保留，本次媒体功能不改变源站访问结果。源码采用独立分支和草稿 PR 交付，安装包在本地交付，不创建 GitHub Release。
