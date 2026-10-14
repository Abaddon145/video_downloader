# Learning Library Implementation Plan
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax.
**Goal:** 实现可工作的v0.4学习库并交付源码、安装包、新Draft PR。
**Architecture:** SQLite独立repository，LearningService编排旧下载与受控媒体服务；React独立学习流程。
**Tech Stack:** Tauri2/Rust/rusqlite bundled/React19/TypeScript，现有UI依赖。
**Spec:** docs/superpowers/specs/2026-10-13-learning-library-design.md 和 learning-library-taskbook.md
## Global Constraints
v0.4.0；旧DownloadTask/DiskState不迁移；学习库DB失败隔离；不复制上游源码；不自动移动文件；删除默认只移除记录且文件删除二次确认；P2不实现；新分支/Draft PR，不合并/Release。
## Review Focus
1. 正在播放时直接窗口关闭/托盘退出、快速切课：最后有效位置和倍速必须持久，迟到旧快照不覆盖新位置。
2. 完成回调与学习关联写入的跨存储竞态/重启：不漏绑定、不产生重复、不改旧下载生命周期。
3. 文件缺失/重定位/共享路径/链接目录：只授权实际学习媒体；明确删除不得误删别的文件。
4. React StrictMode模拟卸载/切课生成代理：辅助进程清理，不复用已取消session，不把旧字幕绑定新课。
5. 大库搜索/分页/排序/重复导入：统计即时正确，500集不一次渲染，重复源ID/规范路径不得重复插入。

### Task 1: Learning Database
Files: learning/db.rs,models.rs,repository.rs,tests.rs; Cargo.
Interfaces: consumes existing Service/MediaService + prior Repository; produces Repository::open(path), versioned schema, typed LearningCategory/Series/Item/Progress/Note/Settings.
- [x] RED: schema migrates empty and v1 retaining data; future/corrupt DB rejected without removal. Run focused check; Expected: required behavior fails before implementation.
- [x] GREEN: implement minimum safe behavior in files above; Expected: focused check passes.
- [x] Verify: cargo test --manifest-path src-tauri/Cargo.toml --offline; Expected: zero failures; append ledger and commit task.

### Task 2: Category + Series
Files: learning/repository.rs,commands.rs,service.rs; src/features/learning/LibraryPage,Editors; App.
Interfaces: consumes existing Service/MediaService + prior Repository; produces learning_home(query,category,page,favorites), learning_series(id,page), learning_change(Change).
- [x] RED: CRUD, contained-category confirmation, record-only deletion, transactional sort, search and 50 paging. Run focused check; Expected: required behavior fails before implementation.
- [x] GREEN: implement minimum safe behavior in files above; Expected: focused check passes.
- [x] Verify: npm test + cargo test + npm run build; Expected: zero failures; append ledger and commit task.

### Task 3: Local Import
Files: learning/importer.rs; features/learning/ImportDialog; repository.
Interfaces: consumes existing Service/MediaService + prior Repository; produces learning_scan(paths), learning_import(input) -> ImportResult.
- [x] RED: numeric prefix/natural 1/2/10; canonical duplicate paths; absent/link skipped; scan does not mutate DB; confirm probes duration. Run focused check; Expected: required behavior fails before implementation.
- [x] GREEN: implement minimum safe behavior in files above; Expected: focused check passes.
- [x] Verify: cargo test + npm test + UI import; Expected: zero failures; append ledger and commit task.

### Task 4: Learning Player
Files: features/learning/LearningPlayer,PlayerControls; learning/service.
Interfaces: consumes existing Service/MediaService + prior Repository; produces learning_prepare_player(itemId,session,proxy) -> media; previous/next/rate/volume/fullscreen/subtitles.
- [x] RED: registered file asset only; stale source/subtitles ignored; missing file relink; keyboard ignored in input. Run focused check; Expected: required behavior fails before implementation.
- [x] GREEN: implement minimum safe behavior in files above; Expected: focused check passes.
- [x] Verify: npm test + cargo test + npm run build; Expected: zero failures; append ledger and commit task.

### Task 5: Continue Learning
Files: learning/repository/service; player/ContinueLearning; settings.
Interfaces: consumes existing Service/MediaService + prior Repository; produces learning_live_progress(input), learning_save_progress(input), markCompleted; settings.
- [x] RED: 5 second durable saves; hidden/unmount/exit flush; half resume/rate; 90 percent; manual reset; next5seconds cancel. Run focused check; Expected: required behavior fails before implementation.
- [x] GREEN: implement minimum safe behavior in files above; Expected: focused check passes.
- [x] Verify: cargo test + npm test + real shutdown/resume; Expected: zero failures; append ledger and commit task.

### Task 6: Notes
Files: learning/repository; features/learning/VideoNotes.
Interfaces: consumes existing Service/MediaService + prior Repository; produces learning_notes(item), learning_save_note(input), learning_change(deleteNote).
- [x] RED: create/edit/delete ordered timestamps; timejump; persistence/reopen; notes search. Run focused check; Expected: required behavior fails before implementation.
- [x] GREEN: implement minimum safe behavior in files above; Expected: focused check passes.
- [x] Verify: cargo test + npm test + UI notes; Expected: zero failures; append ledger and commit task.

### Task 7: Downloader Integration
Files: learning/service/importer; service.rs minimal callback; PlaylistImportOptions; App.
Interfaces: consumes existing Service/MediaService + prior Repository; produces learning_enqueue_downloads(requests,imports); attach_download(task); learning_download_item(id).
- [x] RED: playlistIndex order, ID/URL dedup, placeholders/failed retention; success actual video path; callback-before-bind/restart idempotence. Run focused check; Expected: required behavior fails before implementation.
- [x] GREEN: implement minimum safe behavior in files above; Expected: focused check passes.
- [x] Verify: cargo test + npm test + offline playlist real tasks; Expected: zero failures; append ledger and commit task.

### Task 8: Series Update
Files: learning/service; features/learning/SeriesUpdateDialog.
Interfaces: consumes existing Service/MediaService + prior Repository; produces learning_check_updates(seriesId), learning_enqueue_downloads with existing series.
- [x] RED: changed title sameID no duplicates; added entries only, source preserved; favorites. Run focused check; Expected: required behavior fails before implementation.
- [x] GREEN: implement minimum safe behavior in files above; Expected: focused check passes.
- [x] Verify: cargo test + UI updates; Expected: zero failures; append ledger and commit task.

### Task 9: Stabilization + Deliver
Files: MediaPro VideoPreview; QA scripts; version/README/docs.
Interfaces: consumes existing Service/MediaService + prior Repository; produces fresh per-effect preview sessions; compatibility and delivery records.
- [ ] RED: StrictMode cleanup regression RED; 100/1000/10000 timings; widths1600/1440/1200/1000/880; oldUI + mediaQA + tenWindows library cases. Run focused check; Expected: required behavior fails before implementation.
- [ ] GREEN: implement minimum safe behavior in files above; Expected: focused check passes.
- [ ] Verify: npm test; cargo test; npm run build; npm run tauri -- build --bundles nsis; fresh independent final review; Expected: zero failures; append ledger and commit task.
