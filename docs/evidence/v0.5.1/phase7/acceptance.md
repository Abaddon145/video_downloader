# Phase7 — 版本与交付验证

2026-10-19：package/Cargo/Tauri及两份锁文件统一0.5.1。Node61/0，Rust238/0/8既有忽略，NSIS构建通过；独立临时数据启动release可执行文件，六组真实Windows/文件系统/mpv验证通过。没有覆盖安装或真实API调用。

真实样本：合成MP4 H.264、WMV、MKV HEVC，无音轨素材，中文/空格/括号/#/&/Unicode路径；原视频SHA与测试.obsidian配置SHA前后一致。历史保存BOM/CRLF/YAML/Wiki/任务/代码字节、CAS冲突、跨Vault与遍历拒绝、真实UI自动保存、原生会话保持与设置重启恢复均通过。

所有主要独立审查问题已修复并增加回归。手动项目/八项忽略/物理DPI/Obsidian回执/网络平台及真实AI服务限制不作通过声明，详见docs/v0.5.1-delivery.md。明文Key样式扫描范围及结果见security-scan.json，构建大小/真实SHA见build-manifest.json。

内部构建产物未上传正式Release，用户现有软件未被覆盖；保持Draft PR。
