# Phase 8 知识库后端验收（2026-10-17）

本阶段只验收后端；知识库专用界面、自动AI笔记和截图笔记仍在后续阶段。没有配置或调用真实DeepSeek，也未启动真实Obsidian客户端。

## 实际结果

- 完整 `cargo test --manifest-path src-tauri/Cargo.toml --locked`：150通过、0失败、4个有前置条件的测试明确ignored。新增知识库相关24项常规测试通过。
- 另行运行 `vault_rejects_protected_directory_opened_through_8dot3_alias -- --ignored`：1通过。本机卷支持8.3别名，确实生成保护目录别名并验证拒绝，不能把该结果推断到所有Windows卷。
- `npm test`：28通过；`npm run build`：通过。
- custom-protocol桌面构建通过，真实Windows/Tauri接口6项检查通过：系统文档默认目录、已有Vault/.obsidian不变、普通Markdown与YAML/链接/标签、真实磁盘外部修改冲突、越界路径拒绝、重命名不覆盖/回收、外部扫描/Vault切换隔离。
- 普通测试中另外3个ignored属于先前阶段的真实Whisper/关键帧/端到端分析集成，前序证据保留，本阶段未重复执行。

## RED 与修正

最初缺少Vault及Markdown/索引/搜索服务接口，测试编译失败。随后新增断言发现大写.MD解析、YAML引用扩展、标签空查询及Obsidian Windows路径问题，修复后通过。目录句柄发布最初用Win32 SetFileInformationByHandle RootDirectory，在本机得到错误87；改为NtSetInformationFile相对目录句柄发布，不回退到不安全路径操作。

目录锁和canonicalize并不足以独立防御reparse竞态。现实现使用逐级NtCreateFile相对目录句柄打开、检查后读取同一文件句柄，并使用相对句柄发布/移动。实际对已打开父目录施加FSCTL_SET_REPARSE_POINT的对抗测试通过。另覆盖Junction、硬链接、保护目录短别名、长于260 UTF16路径、5MiB边界、非法UTF8、并发新建不覆盖、根目录替换、被占用目录项和回收目录链接。无法提供稳定目录标识的文件系统会拒绝。

## 数据与并发约定

Markdown始终是文件真源；SQLite仅索引。保存后索引失败仍返回成功文件和警告。Vault切换先清空索引再发布设置，所有命令带当前Vault身份。部分扫描/被占用文件/超限时不清理未确认的旧索引。源视频不由知识库CRUD操作处理，.obsidian不修改。

expected_hash是正常的乐观并发控制，配合原子发布防止半写文件；不声称与不配合的外部编辑器形成完美跨进程CAS。目录扫描不是外部写入的原子快照。单次扫描上限10000项/256MiB，Markdown5MiB，图片附件8MiB且仅栅格格式。真实Obsidian、完整UI、全部DPI及全部Windows设备验证仍待最终阶段。
