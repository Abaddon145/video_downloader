# 映流 v0.4.0 学习库交付与验收

日期：2026-10-14。基于 v0.3.0 / PR #4 的 `50aa2a9`，分支 `codex/learning-library-v0.4`。P0 和 P1（收藏、系列更新）已实现；交付新 Draft PR，保留既有 PR，不合并、不发布 GitHub Release。

## 1. 修改文件完整列表

见 [完整清单](learning-library-files.txt)，包含以下文件以及本报告的最终更新：

```text
README.md
docs/learning-library-delivery.md
docs/learning-library-evidence/catalog-ui.json
docs/learning-library-evidence/integration.json
docs/learning-library-evidence/large.json
docs/learning-library-evidence/library-1600.png
docs/learning-library-evidence/library-880.png
docs/learning-library-evidence/lifecycle-red.json
docs/learning-library-evidence/lifecycle.json
docs/learning-library-evidence/media.json
docs/learning-library-evidence/native.json
docs/learning-library-evidence/note-1230.png
docs/learning-library-evidence/nsis-build.txt
docs/learning-library-evidence/player-1600.png
docs/learning-library-evidence/player-880.png
docs/learning-library-evidence/player-dpi150.png
docs/learning-library-evidence/rust-test.txt
docs/learning-library-evidence/strictmode.json
docs/learning-library-execution.md
docs/learning-library-files.txt
docs/learning-library-review.md
docs/learning-library.md
docs/superpowers/plans/2026-10-13-learning-library.md
docs/superpowers/specs/2026-10-13-learning-library-design.md
docs/superpowers/specs/learning-library-taskbook.md
package-lock.json
package.json
scripts/qa/verify-learning-integration.mjs
scripts/qa/verify-learning-large.mjs
scripts/qa/verify-learning-native.mjs
scripts/qa/verify-learning-player-lifecycle.mjs
scripts/qa/verify-learning-ui.mjs
scripts/qa/verify-preview-strictmode.mjs
src-tauri/Cargo.lock
src-tauri/Cargo.toml
src-tauri/resources/THIRD-PARTY-NOTICES.txt
src-tauri/resources/learning-components.json
src-tauri/resources/licenses/SQLite-Public-Domain.txt
src-tauri/resources/licenses/learning-rust/ahash-0.8.12/LICENSE-APACHE
src-tauri/resources/licenses/learning-rust/ahash-0.8.12/LICENSE-MIT
src-tauri/resources/licenses/learning-rust/fallible-iterator-0.3.0/LICENSE-APACHE
src-tauri/resources/licenses/learning-rust/fallible-iterator-0.3.0/LICENSE-MIT
src-tauri/resources/licenses/learning-rust/fallible-streaming-iterator-0.1.9/LICENSE-APACHE
src-tauri/resources/licenses/learning-rust/fallible-streaming-iterator-0.1.9/LICENSE-MIT
src-tauri/resources/licenses/learning-rust/hashbrown-0.14.5/LICENSE-APACHE
src-tauri/resources/licenses/learning-rust/hashbrown-0.14.5/LICENSE-MIT
src-tauri/resources/licenses/learning-rust/hashlink-0.9.1/LICENSE-APACHE
src-tauri/resources/licenses/learning-rust/hashlink-0.9.1/LICENSE-MIT
src-tauri/resources/licenses/learning-rust/libsqlite3-sys-0.30.1/LICENSE
src-tauri/resources/licenses/learning-rust/rusqlite-0.32.1/LICENSE
src-tauri/resources/licenses/learning-rust/vcpkg-0.2.15/LICENSE-APACHE
src-tauri/resources/licenses/learning-rust/vcpkg-0.2.15/LICENSE-MIT
src-tauri/resources/licenses/learning-rust/zerocopy-0.8.57/LICENSE-APACHE
src-tauri/resources/licenses/learning-rust/zerocopy-0.8.57/LICENSE-BSD
src-tauri/resources/licenses/learning-rust/zerocopy-0.8.57/LICENSE-MIT
src-tauri/resources/licenses/learning-rust/zerocopy-derive-0.8.57/LICENSE-APACHE
src-tauri/resources/licenses/learning-rust/zerocopy-derive-0.8.57/LICENSE-BSD
src-tauri/resources/licenses/learning-rust/zerocopy-derive-0.8.57/LICENSE-MIT
src-tauri/src/engine.rs
src-tauri/src/learning/commands.rs
src-tauri/src/learning/db.rs
src-tauri/src/learning/download.rs
src-tauri/src/learning/importer.rs
src-tauri/src/learning/mod.rs
src-tauri/src/learning/models.rs
src-tauri/src/learning/player.rs
src-tauri/src/learning/progress.rs
src-tauri/src/learning/repository.rs
src-tauri/src/learning/schema-v1.sql
src-tauri/src/learning/schema-v3.sql
src-tauri/src/learning/service.rs
src-tauri/src/learning/tests.rs
src-tauri/src/lib.rs
src-tauri/src/models.rs
src-tauri/src/service.rs
src-tauri/tauri.conf.json
src/App.tsx
src/components/media/editor/VideoPreview.tsx
src/features/learning/Editors.tsx
src/features/learning/ImportDialog.tsx
src/features/learning/LearningLibraryPage.tsx
src/features/learning/LearningPlayer.tsx
src/features/learning/LearningSettingsDialog.tsx
src/features/learning/PlaylistImportOptions.tsx
src/features/learning/SeriesUpdateDialog.tsx
src/features/learning/VideoNotes.tsx
src/features/learning/lib.test.ts
src/features/learning/lib.ts
src/styles.css
src/types.ts
src/types/learning.ts
```

## 2. 数据库设计

独立应用数据目录 `learning.db`：categories → series → series_items；playback_progress、video_notes 关联课时。category 删除将系列转到未分类；系列/课时记录删除级联其进度与笔记。learning_settings 保存观看偏好，schema_version / PRAGMA user_version 记录版本。

路径规范键、来源键和请求键分别用于本地去重、来源去重与下载关联；保留 source_video_id / source_playlist_index / source_playlist_id、完整原下载参数、实际完成文件与任务 ID。SQLite 外键、WAL、事务、忙等待和参数化语句统一通过 Repository 处理。统计字段由触发器在同一事务内维护，界面使用实际关联记录计算。

## 3. Migration 说明

v1 建表和索引；v2 增加收藏、排序、路径/来源键、原下载参数与学习设置；v3 增加进度序号和统计触发器，防止迟到位置覆盖；v4 增加下载请求键索引。迁移在事务内执行，保留已有数据；未来版本或损坏数据库拒绝打开。修复后重试可重新连接。旧 `DownloadTask` / `DiskState` 与 JSON 文件结构不迁移。

## 4. 新增 Rust 模块

`src-tauri/src/learning/`：commands.rs, db.rs, download.rs, importer.rs, mod.rs, models.rs, player.rs, progress.rs, repository.rs, service.rs, tests.rs。数据库失败留在学习库模块内，下载和 Media Pro 可继续运行。媒体探测、FFmpeg 参数、预览代理和 Windows Job 复用已有受控服务。

## 5. 新增 Tauri Commands

```text
learning_home
learning_series
learning_item
learning_change
learning_scan
learning_import
learning_relink
learning_delete_preview
learning_open_file
learning_prepare_player
learning_load_subtitle
learning_live_progress
learning_save_progress
learning_flush_progress
learning_notes
learning_save_note
learning_enqueue_downloads
learning_download_item
learning_check_updates
```

所有输入经后端验证；只通过已登记课时授权播放资产。结构、进度、笔记与下载状态通过 `learning-library-updated` 更新。窗口隐藏/退出会保存最近有效位置，未完成任务仍沿用原来的手动恢复逻辑。

## 6. 新增 React 页面与组件

`src/features/learning/`：Editors.tsx, ImportDialog.tsx, LearningLibraryPage.tsx, LearningPlayer.tsx, LearningSettingsDialog.tsx, PlaylistImportOptions.tsx, SeriesUpdateDialog.tsx, VideoNotes.tsx，共享 `src/types/learning.ts`。中文石墨灰/青蓝界面；原生文件选择和 dialog；每页 50 条；鼠标拖动与上下按钮排序；键盘操作、减少动画、错误重试和窄窗口布局。

实机截图：[学习库](learning-library-evidence/library-1600.png)、[880px 播放器](learning-library-evidence/player-880.png)、[时间点笔记](learning-library-evidence/note-1230.png)、[150% DPI 模拟](learning-library-evidence/player-dpi150.png)。截图使用生成的离线教程与测试目录。

## 7. Downloader → Learning Library 集成

预览勾选后先在 SQLite 建立完整播放列表目录，并冻结每课原下载参数；仅所选课时交给原 `Service.enqueue`。正常任务照旧排队、解析、下载、处理和发布，不改变原下载状态机。下载完成只绑定实际存在、在任务下载目录内、可探测的视频。

完成回调可在显式绑定前按请求键识别待下载课时；入队后补绑任务 ID，启动时对未可用课时补偿关联。失败、取消和未选课时保留；重试沿用原任务或原参数。按视频 ID 优先/URL 兜底去重，即使来源解析器名称或标题变化也不重复。源更新复用原解析器、Cookie 与代理设置。

## 8. 自动测试与 Frontend QA

- `npm test`：21/21。
- `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline`：73 个库测试；含迁移、进度/乱序、重启、笔记、来源去重、下载绑定、文件保护和大库。最终完整执行结果附在执行记录。
- `npm run build`：通过。
- 下载界面 13/13；Media Pro 界面 14/14；旧审查回归 4/4；学习库界面 7/7；StrictMode 会话回归 1/1；播放器生命周期回归 3/3（合计42/42）。
- 可重复运行的界面/实机检查位于 `scripts/qa/verify-learning-*.mjs` 与 `verify-preview-strictmode.mjs`。需 Windows、已构建发布程序、运行中的开发界面（默认5226）、Playwright模块和 Python（大库/中断关联夹具）。通过 `PLAYWRIGHT_MODULE` / `PYTHON_PATH` 指定现有安装；`CARGO_TARGET_DIR`、`QA_OUTPUT_DIR`、`QA_BASE_URL` 可覆盖默认路径。原生检查共用9228，须顺序执行；Media Pro检查使用9227。
- 旧预览源码下回归检查失败，恢复每次 effect 独立会话后通过。来源解析器名称变化时先复现重复课时，再修复为 ID 优先。
- 100 分类 / 1,000 系列 / 10,000 课时基准：种子写入约 358 ms，组合查询约 16 ms；实际桌面首页接口约 18 ms，DOM 保持 50 系列/50课时。性能数字只代表本机本次运行。

## 9. Windows 实机验收

最终发布程序共 34/34：学习库 18 项（6 + 8 + 4）及旧 Media Pro 16 项。细节见同目录 `learning-library-evidence/*.json`。

- 十段本地视频按 1–10 导入；确认前不建库，课时、封面和字幕可用。
- 第三课 50% / 1.5× 主动退出后恢复到同一课时与位置；暂停改为1.75×并定位500秒后在定时保存前原生退出，重开仍保留；达到 90% 完成；5 秒续播可取消并真正播放下一课。
- 真实 yt-dlp 下载本机 HTTP 离线播放列表，包含成功、失败、未选择课时；复用较早暂停任务、失败原任务重试、稍后下载、重复导入、重启补绑定均通过。
- 12:30 笔记在真实播放器记录、输入 Space 不触发播放、点击跳转，重启仍保留且可搜索。
- 视频移动后提示重新定位，重定位后当前播放器直接可播放且保留笔记；移除当前课时停止播放；默认移除保留视频，删除文件必须再次确认；共享引用由后端阻止。
- 实际 Windows 关闭窗口事件隐藏窗口、暂停播放、保存位置；dialog 的 Tab 保持在对话框内。
- 1,000 系列 / 10,000 课时真实数据库、500 集系列分页；1600/1440/1200/1000/880 视口无横向溢出，150% DPI 模拟与减少动画通过。
- 损坏 SQLite 原文件保留、旧下载/媒体功能可用、修复数据库后无需重启即可重试。
- Media Pro：H.264/H.265、硬件不可用回退、裁剪、字幕、批量、抽帧、联系表、封面、标签、取消进程树与重启动作通过。源文件 SHA256 保持一致，已有输出不覆盖。

## 10. 构建与交付

`npm run tauri -- build --bundles nsis` 成功，生成中文 Windows x64 `视频下载器_0.4.0_x64-setup.exe`（143.18 MiB / 150,136,486 字节）。包含 WebView2 引导、原下载组件、SQLite 来源/校验与新增依赖许可证。交付安装包、Git 源码 ZIP、SHA256SUMS、使用说明与本报告；最终文件存放于本地 `artifacts/releases/2026-10-14-learning-library-v0.4.0/`；[Draft PR #5](https://github.com/Abaddon145/video_downloader/pull/5) 已创建并附到当前任务。

安装包构建通过，实机验收运行对应发布程序。未执行覆盖个人已安装版本的安装/卸载，也未验证干净 Windows 10 和缺少 WebView2 的安装环境。

## 11. 已知问题与验证范围

独立审查 Critical 0、Important 4、Minor 0；四项均已修复并通过失败复现与回归验证，见[审查记录](learning-library-review.md)和[执行记录](learning-library-execution.md)。本轮网络下载采用本机 HTTP 离线播放列表，真实使用原 yt-dlp 生命周期；本轮没有重测 B站/YouTube 实时网络或个人 Cookie 下载。站点仍受上游、地区、登录和网络条件影响。

高 DPI 为 WebView2 150% 模拟，并非修改 Windows 物理显示设置。GPU 使用原有 CPU 及不可用设备回退验证，未验证本机没有的硬件加速成功路径。预览代理沿用 6小时/5分钟/1GiB 限制，字幕为文本轨道；位图字幕不转换。

## 12. 延期项目

P0/P1 无功能延期。AI 总结、OCR、字幕全文检索/转录、云同步、在线服务和多轨编辑按任务书保留后续范围。独立审查无延期 Minor；没有未处理的 Critical/Important，也没有代用户裁决的 Ruling。
