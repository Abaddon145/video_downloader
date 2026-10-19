# Phase 1C — Obsidian 联动与笔记移动

代码位置：`artifacts/v051/staging`，固定 PR #8 基线上实施。未修改用户默认数据、`.obsidian`、源 Markdown 或 DCC 仓库；未新增依赖，未执行 Git、发布或安装操作。

## 实现

- `KnowledgeSettings.obsidian` 是可缺省的新字段，旧设置 JSON 可继续加载。映流 `vaultId` 与用户提供的 Obsidian 16 位 Vault ID 分开保存。
- 绑定携带当前映流根目录身份；切换根目录、同路径替换目录或使用过期 Vault 请求均不能打开旧绑定。
- 默认生成 `obsidian://open?vault=…&file=…`；名称必须明确选择、填写并确认；绝对 `path` 仅为显式兼容选项。使用已有 `url::Url` 编码，拒绝 UNC 路径，不自动切换兼容方式。
- 使用 Windows `AssocQueryStringW` 只读检查协议处理程序。协议可用不等于安装正常或应用打开成功。URI、文件夹打开、显示与复制给用户的路径共享 Windows 本地路径规范化。
- 打开前复用 `with_vault()`、`Vault::read()` 检查实际根身份、文件存在性、读取权限与路径边界。
- 请求结果区分 `launchAccepted` / `launchFailed`。仅明确点击“我已看到目标笔记，确认联动正常”才能保存用户确认；后端核对成功测试、预期绑定与笔记路径，重新配置使旧测试失效。
- 向导包含根目录、登记说明、Vault 管理器、文件夹、复制路径、标识配置、测试、故障排查、安装说明及继续编辑。工具栏和笔记按钮都先处理未保存内容，可保存后打开或保留草稿打开磁盘版本。
- 异步响应校验组件生命周期、请求序号、Vault ID 和笔记路径；切换当前文档或 Vault 关闭旧向导，绑定保存回执不能覆盖已切换的 Vault。
- 移动与重命名共用事务：`BEGIN IMMEDIATE` → 文件移动 → 检查两张关联表更新 → COMMIT。失败回滚数据库并回移文件；补偿失败报告原路径、目标路径与错误，成功前不更新索引、不发送成功事件。
- 搜索索引仍是可重建缓存，保持既有 warning 策略。

## 修改文件

1. `src-tauri/src/knowledge/models.rs`
2. `src-tauri/src/knowledge/obsidian.rs`
3. `src-tauri/src/knowledge/service.rs`
4. `src-tauri/src/knowledge/service_tests.rs`
5. `src-tauri/src/knowledge/commands.rs`
6. `src-tauri/src/lib.rs`（仅新 knowledge 命令注册）
7. `src/types/knowledge.ts`
8. `src/features/knowledge/KnowledgePage.tsx`
9. `src/features/knowledge/ObsidianConnection.tsx`（新增）
10. `src/features/knowledge/knowledge.css`
11. `scripts/qa/verify-knowledge-ui.mjs`

## RED 证据

旧实现观察到的预期失败：

- `compatibility_uri_rejects_network_paths`：普通 UNC 路径被旧 URI 构造器接受。
- `obsidian_settings_are_optional_and_preserve_separate_vault_identity`：旧 DTO 拒绝未知 `obsidian` 字段。
- `move_note_rolls_back_file_and_first_update_when_second_update_fails`：主任务在原生 Windows 测试环境观察到 `association failure must fail the move` 断言失败。
- `move_note_restores_file_when_commit_fails`：主任务观察到旧实现仍返回成功，`is_err()` 断言失败。

首次受限环境执行原生 Vault 测试出现 `ERROR_ACCESS_DENIED` / `ERROR_INVALID_HANDLE`，没有计作有效 RED。随后由主任务在支持原生句柄的执行环境验证上述移动 RED。

## 自动验证

- `tsc --noEmit`：通过；没有通过 `npm run build` 的 dist junction 向只读工作树写入。
- `npm test`：37/37 通过。
- `node --check scripts/qa/verify-knowledge-ui.mjs`：通过。
- 新增 Rust 回归 11 项：URI/编码/状态 3 项；设置兼容、登记与用户确认、根身份、读取被拒绝、显式兼容、移动目录及两种事务故障共 8 项。
- Rust knowledge 原生 GREEN：等待主任务返回结果。
- 浏览器 QA：脚本已扩展，等待主任务运行结果；本报告不把脚本编写或模拟场景当作真实 Obsidian 验收。

浏览器新增断言：首次未配置不发送 URI、登记后仍待实测、URI 接受不自动确认、明确用户确认、880px 对话框、未保存草稿保留、协议不可用、打开失败及重新配置。成功运行会输出 `obsidian-1440.png`、`obsidian-880.png` 和 `knowledge-ui.json`。

## Windows 人工验收（尚待执行）

| 场景 | 应观察结果 |
|---|---|
| 已安装、已登记 Vault，应用关闭或后台运行 | 发送请求后人工确认实际 Vault 与目标笔记；系统接受请求不等于应用已打开 |
| 已安装但当前根目录未登记 | 向导提供登记说明与 Vault 管理器，不自动修改配置 |
| 未安装 / URI 协议不可用 | 显示协议不可用或请求失败，保留映流编辑 |
| 多 Vault、同名笔记 | 使用明确 Vault ID 与根目录相对路径，只打开所绑定目标 |
| 中文、空格、`#`、`&`、`%`、Unicode 文件名 | 人工检查目标笔记，文件内容哈希与原文保持不变 |
| Windows 长路径前缀 | 去除本地盘符的 verbatim 前缀，目标保持一致 |
| 笔记移动后重新打开 | 新路径可读，两张关联表仍指向新位置 |
| 根目录移动、替换、电脑迁移、Obsidian 删除登记 | 重新选择并配置；不存在可靠应用回执时由用户判定登记情况 |
| 目标不存在、越界、读取被拒绝 | 请求发送前拒绝，编辑草稿保留 |
| 未保存草稿 | 取消保留草稿；保存后打开走哈希冲突检查；选择磁盘版本时不丢当前编辑 |
| 外部 Obsidian 修改 | 既有冲突提示与条件保存继续有效 |

Windows 文件名本身不允许 `?`；该字符在纯 URI 参数编码测试中验证，不能伪造为真实 Windows 文件名测试。

## 限制与来源

- 未实现 Obsidian 插件、后台通信或应用级回执。保存的联动正常状态仅代表用户曾明确确认。
- 文件系统与 SQLite 不是同一事务资源；现有补偿处理运行中的失败，进程在文件移动与数据库提交之间被终止时仍需后续恢复手段。未增加未要求的持久化移动日志。
- 数据库更新触发器和延迟外键故障是测试注入，不是实际 DeepSeek 或实际 Obsidian。
- 官方 URI 说明：<https://help.obsidian.md/Extending+Obsidian/Obsidian+URI>。`choose-vault` 打开 Vault 管理器；Vault 切换器支持复制 Vault ID；`path` 覆盖 `vault/file`。

## 主任务最终验证

固定基线172Rust；本阶段最终193通过、0失败、8项既有显式忽略。前端37通过，生产构建通过。Obsidian浏览器模拟原生接口22项、课程面板35项通过；实际Obsidian应用打开回执及用户原课程真实API复测未执行。
