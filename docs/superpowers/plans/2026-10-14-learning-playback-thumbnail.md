# Learning Playback and Thumbnail Enhancement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans inline, task-by-task. Steps use checkbox syntax.

**Goal:** 交付可工作的v0.4.1多格式自动播放、课程缩略图、源码、NSIS安装包、新Draft PR及25项交付报告。
**Architecture:** compatibility与thumbnail模块编排现有MediaService；独立缓存与会话引用，SQLite v5，React沿用当前学习页。
**Tech Stack:** 现有Tauri2/Rust/rusqlite/React19/TypeScript/FFmpeg/GPU/Windows Job，不增加生产依赖。
**Spec:** docs/superpowers/specs/learning-playback-thumbnail-taskbook.md、2026-10-14-learning-playback-thumbnail-design.md。

## Global Constraints
0.4.1；DB4→5事务迁移，旧记录/下载结构保留；原视频SHA不变；进度绑itemId；缓存各自独立；普通文件和受控本地参数；50条分页；缩略图320×180 JPEG单后台worker；播放器不排在缩略图后；自动/50GiB/30天默认；手动图覆盖确认；共享任务引用保护；只清理拥有的非使用/无引用缓存；新Draft PR，不Merge/Release；不扩展AI/HLS等P2。

## Review Focus
1. StrictMode/快速切课/同源多请求/取消：只启动一个代理，不能让旧取消杀掉新请求或授权迟到文件。Task3/6。
2. 生成中源文件改变、重定位、GPU失败：不发布过期代理/图，回退不无限循环，不覆盖进度/笔记。Task2/3/4。
3. 缓存目录链接/篡改元数据/共享图片/活动代理清理：不得删除原媒体、用户图片或仍使用缓存。Task2/3/4。
4. 手动图设置与迟到远程/抽帧竞态、远程重定向/超限/断网：保持优先级，错误不阻断播放，失败不循环重试。Task4/5。
5. 10000课、500集、窄侧栏/高DPI/准备期快速关闭：查询/渲染/FFmpeg并发有界，进度最后有效值可恢复。Task3/5/6。

### Task 1: Migration and Real-Video Import
**Files:** learning/db.rs,models.rs,repository.rs,importer.rs,player.rs,download.rs,tests.rs; media/batch.rs; ffmpeg/probe.rs; ImportDialog/types。
**Interfaces:** consumes Repository/MediaInfo; produces schema5, LearningItem thumbnailSource/sourceThumbnailUrl/thumbnailKey; LearningSettings playbackPreference(auto/original/compatible), playbackCacheGb=50, playbackCacheDays=30; discover_filtered(paths,predicate) preserving native path guards; prepare(path,info) accepts actual video without extension gate.
- [x] RED tests learning_v4_upgrade_preserves_progress_notes_and_source_thumbnail, learning_import_accepts_video_regardless_extension, learning_scan_covers_all_required_formats; old future-version rejection remains. Run focused learning tests. Expected: current v4/extension logic fails.
- [x] GREEN transaction migration, backward-compatible settings defaults/validation, shared discovery predicate; selector includes17 formats. Expected: source data, progress, note, favorite/sort/binding remain; corrupt/future failures isolated.
- [x] Verify cargo test --manifest-path src-tauri/Cargo.toml --locked --offline --lib; npm test; npm run build. Expected: all pass; mark and commit.

### Task 2: Compatibility Planner and Cache Rules
**Files:** learning/compatibility.rs,cache.rs (shared owned-path/fingerprint only), tests.rs; ffmpeg/models.rs,command.rs,runner.rs.
**Interfaces:** consumes Task1 settings/MediaInfo; produces BrowserPlaybackSupport{hevc,hevc10}, PlaybackMode/PlaybackPlan, plan(info,support,preference,fallback); cache fingerprint/key/check/owned cleanup; MediaRequest.learning_playback defaultfalse, MediaService.task(id)/create_playback(request), AV-only Remux and existing GPU fallback.
- [x] RED planner matrix MP4/MOV/H264/AAC/MP3, WebM/VP8/VP9/Opus/Vorbis, MKV copy, WMV/AVI/MPEG fallback, conditional HEVC8/10, no video unsupported; cache key changes on size/mtime/profile; validated hit and linked/outside cache rejection; playback-remux drops subtitle/data/secondary AV without altering ordinary Remux. Expected: missing planner/rules fail.
- [x] GREEN minimum plan/cache helpers and controlled queue seam; CRF22/AAC192, faststart, tagged HEVC copy where supported. Expected: no duplicate FFmpeg runner, ordinary Media Pro behavior intact.
- [x] Verify full Rust and Node suites + build; commit. Expected: zero failures.

### Task 3: Automatic Playback, Shared Proxy and Cleanup
**Files:** learning/player.rs,compatibility.rs,commands.rs,service.rs; LearningPlayer.tsx,PlaybackCompatibilityStatus.tsx,types/learning.ts,lib.ts/tests; lib.rs command registration.
**Interfaces:** learning_prepare_player(itemId,session,proxy,support?,fallback?,force?) retains path; returns plan; learning-playback-updated(session,itemId,state,percent,eta); learning_cancel_player(session); learning_cache_info()/learning_clean_cache(kind); cache manager references session/key/task; cached text subtitles.
- [x] RED shared-job/session tests and UI preparing/progress/error bounded fallback; cancellation of last vs shared requester; cleanup protects ready/processing leases and linked paths; proxy cleanup keeps DB progress/notes; subtitle cache fingerprint includingSSA. Expected: current manual-only/nonpersistent preview fails.
- [x] GREEN reuse create_playback/task/GPU queue, key dedup, completion/source verification and info atomic save; ready/cached path authorization; await/invalidate sessions; automatic play and cancel, max50GiB/age30 protected cleanup. Expected: one task for3 simultaneous requests, direct/remux/transcode/cache restoration works.
- [x] Verify Rust/Node/build and focused browser checks. Expected: green; commit.

### Task 4: Thumbnail Resolver and Background Integration
**Files:** learning/thumbnail.rs,cache.rs,repository.rs,service.rs,importer.rs,download.rs,commands.rs; native.rs/service.rs minimal sharedcurl binary seam; tests.rs。
**Interfaces:** schedule_thumbnails(ids) dedups item; thumbnail worker resolves manual→remote→generated→fallback; learning_set_thumbnail(id,path)/learning_regenerate_thumbnail(id,confirmed); database path/source/key CAS; cache_info/cleanup thumbnails.
- [x] RED priorities, sampling thresholds30/5sec,320×180 padded4:3/9:16/1:1, remote failure/generated fallback, source stamp invalidation, manual-CAS/relink races, dedup/concurrency1 and unreferenced-only cleanup. Expected: missing resolver fails.
- [x] GREEN async singleworker plus required six triggers, boundedHTTP(S)/20MiB/30sec through existing proxy/curl/native, no original writes, failure memo, atomic image publish and guarded DB update; originals/manual choices protected. Expected: import returns before all images, playback queue independent, offline remote cache survives restart.
- [x] Verify Rust/Node/build and native media image checks. Expected: green; commit.

### Task 5: Lesson Rows, Thumbnail Controls and Cache Settings
**Files:** LessonRow.tsx,LessonThumbnail.tsx,LearningLibraryPage.tsx,LearningSettingsDialog.tsx,styles.css,types/lib/tests and UI QA。
**Interfaces:** row receives item/series/playing/actions; thumbnail renders local cache or cover/placeholder; native details action menu; settings preferences/usage/cleanup and existingGPU label.
- [ ] RED UI checks fallback/lazy image, duration,50%bar,completion,onlycurrenthighlight,missing retained image, download state,manual replacement confirmation; widths1600/1440/1200/1000/880 and125/150DPI. Expected: old text rows fail.
- [ ] GREEN128×72/96×54 fixed aspect,2line title,accessible SVG/labels/menu/keyboard/focus,cache settings without duplicate GPU control. Expected: no overflow/broken images, all actions work.
- [ ] Verify full Node/Rust/build + UI checks. Expected: green; commit.

### Task 6: Windows Acceptance and Delivery
**Files:** scripts/qa (reused harness + v041 checks), versions/README; docs/learning-playback-v0.4.1.md,delivery/execution/review/evidence。
**Interfaces:** consumes all prior features; produces actual mixed-format fixtures/test results, inspected screenshots, version0.4.1 NSIS, Git source ZIP/SHA256, newDraft PR basedv0.4.
- [ ] RED meaningful actual playback/thumbnail/cache/slowcancel/protectedfiles checks before any stabilizing fix; prepare listed12 codec/container fixtures and4 ratios, realWMV3/VC1 samples only local. Expected: checks identify unfinished cases rather than mocks.
- [ ] GREEN satisfy actual flows, regression Download/Playlist/Cookie/proxy/control/history, old16MediaPro and v04learning progress/notes/next/update; large100/1000/10000,500/page50 and five widths/DPI. Expected: checked source SHA unchanged, no duplicate full proxy/image jobs.
- [ ] Verify npm test; cargo test --manifest-path src-tauri/Cargo.toml --locked; npm run build; npm run tauri -- build --bundles nsis. One independent whole-branch review; one RED→GREEN Important/Critical fix pass; defer Minors in ledger; no re-review. Expected: green tests and installer, actual limitations recorded.
- [ ] Produce25-section report/file list/usage/component notices/evidence, pushnewbranch/Create+attachDraft PR; archiveledger thendeleteonlyplan-owned temp workspace; retainmanagedworktree. Expected: concrete source, installer, hashes and PR links delivered; no merge/Release.
