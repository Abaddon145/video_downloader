# 映流 v0.4.2 交付记录

- Draft PR：[内置原生播放，课程无需整段预转码 #7](https://github.com/Abaddon145/video_downloader/pull/7)。基于 v0.4.1 的 #6；未合并、未发布 Release。
- 程序构建提交：`667df6180e49c5bd09988b3d32507aee167c53aa`。构建后仅增加交付文档/证据；源码 ZIP 的实际提交与摘要记录在随包 `delivery-manifest.json` 中。
- 中文 Windows x64 安装包：`视频下载器_0.4.2_x64-setup.exe`，184843083 字节，176.28 MiB。
- 安装包 SHA256：`97ceda526e38b57ee516565730c1ad2423de1bbc6742c7228deafab3477eeba8`。
- 源码包：`映流_v0.4.2_源码.zip`。逐文件摘要在 `SHA256SUMS.txt`；组件版本、来源与运行文件摘要在 `components.json`、`binaries.json`。许可和使用说明随包提供。

安装器构建已退出 0；应用和安装器的 FileVersion/ProductVersion 均为 0.4.2，应用 PE 为 x86_64。生成的 NSIS 文件清单包含 mpv、可信输入配置、字体配置、所有下载组件与许可；简体中文和 WebView2 embedBootstrapper 配置已核对。未修改用户当前安装。

安装包生成后，发布程序再次实测 157 秒 WMV 约 660ms 就绪；定位 100 秒、1.5 倍速、暂停保存通过，没有新增转码任务。首次构建和完整验收见 [功能/验收报告](native-learning-playback-v0.4.2.md)、[审查处置](native-learning-playback-final-review.md) 和 [证据](evidence/native-learning-v0.4.2/)。

本轮未执行个人现有安装覆盖、干净 Windows 10 或无 WebView2 环境的安装；不同实体 GPU/多显示器系统 DPI 未全面覆盖。已有的原文件、学习进度和笔记保护由隔离数据测试验证。
