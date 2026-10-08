# 本机验收记录

日期：2026-10-08。Windows x64；Node.js 24.21.0、Rust 1.98.1 MSVC。

## 已验证

- `npm test`：2/2，通过分享链接提取、去重与每页 50 条分页检查。
- `npm run build`：TypeScript 与生产资源构建通过。
- `cargo test --manifest-path src-tauri/Cargo.toml`：14/14，覆盖链接与代理校验、未知进度、预览编码、参数数组/禁止覆盖、并发限制、暂停恢复、重启恢复、DPAPI、日志脱敏、原子 JSON 恢复、完整子进程树终止、更新启动失败回滚。
- Edge 实际渲染：1200px / 880px、200% DPI、Ctrl+L、批量分享链接识别、历史导航、Cookie 文件回退入口、减少动画模式，未发现页面异常或横向溢出。
- 空状态布局修正：测试先发现操作按钮超出面板，再验证按内容计算高度后正常。

## 发布包与真实平台

待安装包生成后记录实际结果。不会把模拟数据记为真实平台下载成功。

## 组件

官方 yt-dlp 2026.08.19、Deno 2.9.7、yt-dlp/FFmpeg-Builds 的 Windows GPL 构建。发行文件 SHA256 固定在 `components.json`，解包后二进制摘要在 `tools/binaries.json`。
