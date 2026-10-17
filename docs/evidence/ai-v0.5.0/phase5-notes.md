# Phase5 实际验证

117项常规Rust测试、23项Node测试、前端生产构建、custom-protocol桌面构建通过，完整phase5.txt。两项素材依赖测试默认ignored，已在Phase3/4单独真实运行。本阶段DeepSeek调用均为本机HTTP服务模拟，不是真实DeepSeek账户调用。

实际执行生产分段管线（GUI事件句柄为空，其余存储/加密/请求/验证代码不变），模拟503两次重试耗尽后从失败段续跑，首段只发送一次；仅成功响应记录原始用量。非法/截断分析保留可获得的用量，但不会写入结构化结果。并发实测峰值2；取消不等待服务延迟响应，旧任务不能发布到新所有者。源变化和隐私撤回均停止后续处理。

文字与精选JPEG分别按授权构造请求；每次最多6张、单张2MiB、最大1280尺寸、原始SHA校验及受控缓存路径验证；视频/WAV/绝对路径不进入Body。来源id和时间范围在Rust端校验。分析JSON须严格匹配结构，不接受自然语言或Markdown代码围栏。缓存分段按素材摘要匹配，避免错误复用。

早期开发版v6数据库新增options_json列兼容测试通过；默认旧记录为本地转录，不会自动产生AI费用。公开完整分析队列在Phase6与最终总结一起接入；本阶段未把分段结果伪装成全片完成。

本机曾遇测试exe启动0xc0000139。检查PE发现TaskDialogIndirect导入，mt.exe确认测试exe没有资源，符合[Tauri上游问题](https://github.com/tauri-apps/tauri/issues/14580)及[Microsoft Common Controls清单要求](https://learn.microsoft.com/en-us/windows-hardware/drivers/print/using-theme-manifests)。修复直接在Windows库测试链接Tauri已生成的resource.lib，完整测试和正常应用构建均恢复。
