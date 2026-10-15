# 映流 v0.4.1 交付报告

验收日期：2026-10-15。分支：`codex/learning-playback-thumbnail-v0.4.1`。基线：`c580e5d7d0e20ed17457a18ecf590876f5ec1ec0`。[Draft PR #6](https://github.com/Abaddon145/video_downloader/pull/6) 已创建。安装包校验与源码提交记录在随包交付清单中；不合并、不创建 Release。

## 1 修改文件完整列表

- `README.md`
- `docs/learning-playback-evidence/edge-cases.json`
- `docs/learning-playback-evidence/fixture-manifest.json`
- `docs/learning-playback-evidence/large-library.json`
- `docs/learning-playback-evidence/lesson-rows.json`
- `docs/learning-playback-evidence/lessons-880.png`
- `docs/learning-playback-evidence/library-1600.png`
- `docs/learning-playback-evidence/media-pro.json`
- `docs/learning-playback-evidence/migration.json`
- `docs/learning-playback-evidence/native-regression.json`
- `docs/learning-playback-evidence/playback.json`
- `docs/learning-playback-evidence/player-880.png`
- `docs/learning-playback-evidence/thumbnails.json`
- `docs/learning-playback-evidence/wmv-cache-resume.png`
- `docs/learning-playback-execution.md`
- `docs/learning-playback-final-review.md`
- `docs/learning-playback-v0.4.1-delivery.md`
- `docs/learning-playback-v0.4.1.md`
- `docs/superpowers/plans/2026-10-14-learning-playback-thumbnail.md`
- `docs/superpowers/specs/2026-10-14-learning-playback-thumbnail-design.md`
- `docs/superpowers/specs/learning-playback-thumbnail-taskbook.md`
- `package-lock.json`
- `package.json`
- `scripts/qa/prepare-learning-fixtures.mjs`
- `scripts/qa/verify-download-ui.mjs`
- `scripts/qa/verify-learning-edge-cases.mjs`
- `scripts/qa/verify-learning-integration.mjs`
- `scripts/qa/verify-learning-large.mjs`
- `scripts/qa/verify-learning-lesson-rows.mjs`
- `scripts/qa/verify-learning-migration.mjs`
- `scripts/qa/verify-learning-native.mjs`
- `scripts/qa/verify-learning-playback.mjs`
- `scripts/qa/verify-learning-player-lifecycle.mjs`
- `scripts/qa/verify-learning-preparation.mjs`
- `scripts/qa/verify-learning-thumbnails.mjs`
- `scripts/qa/verify-learning-ui.mjs`
- `scripts/qa/verify-media-pro-review-ui.mjs`
- `scripts/qa/verify-media-pro.mjs`
- `scripts/qa/verify-media-ui.mjs`
- `src-tauri/Cargo.lock`
- `src-tauri/Cargo.toml`
- `src-tauri/src/ffmpeg/command.rs`
- `src-tauri/src/ffmpeg/models.rs`
- `src-tauri/src/ffmpeg/runner.rs`
- `src-tauri/src/ffmpeg/tests.rs`
- `src-tauri/src/learning/cache.rs`
- `src-tauri/src/learning/commands.rs`
- `src-tauri/src/learning/compatibility.rs`
- `src-tauri/src/learning/db.rs`
- `src-tauri/src/learning/download.rs`
- `src-tauri/src/learning/importer.rs`
- `src-tauri/src/learning/mod.rs`
- `src-tauri/src/learning/models.rs`
- `src-tauri/src/learning/player.rs`
- `src-tauri/src/learning/repository.rs`
- `src-tauri/src/learning/schema-v5.sql`
- `src-tauri/src/learning/service.rs`
- `src-tauri/src/learning/tests.rs`
- `src-tauri/src/learning/thumbnail.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/media/batch.rs`
- `src-tauri/src/media/subtitle.rs`
- `src-tauri/src/native.rs`
- `src-tauri/src/service.rs`
- `src-tauri/tauri.conf.json`
- `src/App.tsx`
- `src/features/learning/ImportDialog.tsx`
- `src/features/learning/LearningLibraryPage.tsx`
- `src/features/learning/LearningPlayer.tsx`
- `src/features/learning/LearningSettingsDialog.tsx`
- `src/features/learning/LessonRow.tsx`
- `src/features/learning/LessonThumbnail.tsx`
- `src/features/learning/PlaybackCompatibilityStatus.tsx`
- `src/features/learning/lib.test.ts`
- `src/features/learning/lib.ts`
- `src/styles.css`
- `src/types/learning.ts`

## 2 数据库 Migration

SQLite4→5在事务内添加 thumbnail_source/source_thumbnail_url/thumbnail_key。旧远程图片地址迁至source_thumbnail_url，旧本地图片保留为manual；原课程、分类、收藏、排序、下载绑定、进度和笔记保留。旧本地手动图片后台复制归一化为受管缓存，原图片保留；真实v4迁移检查通过。迁移失败或未来版本不覆盖旧库。旧设置读取时补齐auto/50GiB/30天。

## 3 Playback Compatibility Planner设计

只接受普通本地文件；ffprobe真实轨道决定导入与播放。基于容器、首个视频/音频、位深、像素格式、实际MediaCapabilities HEVC能力以及用户偏好选择Direct/Remux/Transcode。错误回退最多两级，避免反复转换。

## 4 Direct播放规则

MP4/MOV内H.264八位420与AAC/MP3，或兼容WebM VP8/VP9与Opus/Vorbis。HEVC按当前WebView的8/10位能力选择；像素格式补充缺失的位深信息。原始偏好可先尝试源文件。

## 5 Remux规则

兼容编码、容器不适合时复制首个AV轨道到MP4/WebM；MKV H.264/AAC实测无损封装且字幕单独提取。MP4 faststart，HEVC复制时标记hvc1。普通媒体工具Remux仍保持原来的完整轨道检查。

## 6 Transcode Proxy规则

WMV3/VC-1/WMA、AVI MPEG4、MPEG2、FLV1等自动生成H.264/AAC MP4，CRF22、AAC192kbps、faststart。奇数画面尺寸向上调整到偶数以满足H.264420编码。输出独立保存，原媒体不覆盖。

## 7 GPU fallback

完全沿用MediaService的可用编码器探测、媒体设置、硬件编码及失败后CPU重试。16项真实MediaPro回归包含硬件不可用的CPU回退；本机可用硬件编码器为空，实际使用CPU；没有对每一种独立GPU硬件做物理设备覆盖。

## 8 Proxy Cache结构

`learning-cache/playback/{SHA256 key}/`包含生成视频和info.json；字幕独立放在subtitles，图片独立放在thumbnails。参数数组启动原有受控Job/FFmpeg队列，未增加第二个处理器或生产依赖。同源并发请求共享任务与会话引用，处理完成到info.json发布之间也继续共享；发布失败允许重试，手动重新生成已发布缓存仍会新建任务。

## 9 Proxy Cache失效策略

key含canonical路径、尺寸、纳秒修改时间、编码/容器和目标配置。命中时验证缓存文件及元数据；发布前再次检查源文件。源文件变化实际触发新代理与新缩略图。清理保护会话使用、排队和生成中的key；目录链接与越界记录不进入清理。

## 10 Thumbnail Resolver设计

手动→远程缓存→源视频帧→系列封面→默认图标。单后台worker、每课去重，生成不占用播放器任务队列；数据库按旧路径/来源/URL/key条件更新，迟到结果不能覆盖手动选择或重新定位；处理中再次定位会为最新输入补排队，未变化的失败不会循环重试。

## 11 Remote Thumbnail Cache

保存每课源站URL，通过现有代理配置和系统curl下载；仅HTTP/HTTPS，重定向最多3次、30秒、20MiB。标准化JPEG后离线可用，失败继续尝试源帧/封面，不阻断课程与播放。实测远程图片缓存后断网重启仍保留。

## 12 FFmpeg抽帧规则

320×180 JPEG，q=3，按比例缩放并pad。≥30秒12%，5–30秒25%，<5秒50%，不取首帧。16:9、4:3、9:16、1:1均通过真实图像尺寸/编码检查。稀疏TS关键帧输入定位失败时，精确输出定位重试一次，每次30秒上限。

## 13 Thumbnail Cache策略

源文件指纹独立失效；缺失视频保留现有图。手动JPEG/PNG/WebP复制并标准化，替换确认由界面和后端共同检查。清理仅移除数据库无引用文件，保留系列封面、当前课图与原图片；实际文件SHA检查通过。

## 14 Learning Player UI变化

点击课程自动准备并播放；显示解析/排队/转换进度、预计时间与取消。可重新生成兼容视频、选择播放偏好、查看技术详情、重试、打开目录及重新定位。沿用倍速、字幕、进度、笔记和下一课。

## 15 Lesson List UI变化

128×72/96×54课程图、两行标题、时长、进度条、完成标记、当前课高亮与缺失状态。图片lazy加载并失败回退。原排序、改名、重新定位、移除等操作收进可键盘操作的菜单，全文可在菜单中读取。

## 16 自动测试结果

Rust91项、Node22项及生产构建通过。前端覆盖原7项学习库、准备取消、3项课程行、3项生命周期、StrictMode，以及13项下载界面、14项媒体界面和4项MediaPro界面。截图检查1600/1440/1200/1000/880，以及125%/150%浏览器设备像素模拟。独立审查3项Important均完成RED→GREEN修复；无Critical/Minor，详见[审查记录](learning-playback-final-review.md)。

## 17 Windows实机结果

旧版手动缩略图迁移1项、多格式19项、SSA/远程超限/最后引用取消3项、缩略图4项、旧学习播放6项、下载关联/笔记/重定位8项、大库4项、MediaPro16项通过。使用独立测试数据目录与真实WebView2/FFmpeg，合计61项桌面检查；旧下载队列、Cookie提示/导入、代理校验、暂停取消重试、历史、字幕、批量处理等回归通过。真实B站/YouTube网络和登录限制不由本地媒体测试代替；本次下载回归使用真实yt-dlp与本地HTTP源。

| 测试素材 | 实际编码 | 最终策略 | 播放/Seek/1.5x/暂停/保存 |
|---|---|---|---|
| 01-MP4-H264-AAC.mp4 | h264, aac | direct | 通过 |
| 02-MKV-H264-AAC.mkv | h264, aac, subrip | remux | 通过 |
| 03-MKV-HEVC-AAC.mkv | hevc, aac | remux | 通过 |
| 04-MOV-H264.mov | h264, aac | direct | 通过 |
| 05-AVI-MPEG4.avi | mpeg4, mp3 | transcode | 通过 |
| 06-WMV-WMV3-WMA.wmv | wmv3, wmapro | transcode | 通过 |
| 07-WMV-VC1-WMA.wmv | vc1, wmapro | transcode | 通过 |
| 08-MPEG-MPEG2.mpg | mpeg2video, mp2 | transcode | 通过 |
| 09-TS-H264.ts | h264, aac | remux | 通过 |
| 10-M2TS-H264.m2ts | h264, aac | remux | 通过 |
| 11-WebM-VP9-Opus.webm | vp9, opus | direct | 通过 |
| 12-FLV.flv | mp3, flv1 | transcode | 通过 |

## 18 WMV结果

使用FFmpeg官方WMV3/WMAPro与VC-1/WMAPro源片段，保留原编码裁取测试段。两种都自动生成H.264/AAC并播放。WMV观看约50%后退出重开恢复位置和1.5x；第二次缓存命中且没有新完整转码；3个并发请求仅一个任务，取消其中一个不影响其余请求。

## 19 MKV结果

H.264/AAC带内嵌SRT的MKV通过无损AV封装播放，字幕独立转换；没有无意义重编码。源时间变化后代理与图片分别重建。

## 20 HEVC结果

本机WebView能力：HEVC=True，HEVC10=True。实际HEVC/AAC MKV采用Remux并可播放；无HEVC能力与10位能力缺失的判断由Rust矩阵验证。没有修改系统编解码组件。

## 21 大库性能结果

真实SQLite100分类、1000系列、10000课；500集系列每页50项，页面DOM保持50课。首页查询18ms（单次本机测量，不代表所有设备）。五种宽度无横向溢出；真实隐藏窗口后进度保存、弹窗键盘焦点与损坏数据库隔离通过。DPI为WebView设备指标模拟，未修改Windows显示缩放。

## 22 原文件SHA256保护结果

12种源文件前后相同；手动图片及4种比例视频也单独检查。修改时间失效测试只改mtime，不改内容。

- `01-MP4-H264-AAC.mp4`：`e4bf3ff314cb1a9610adc27a29be624f8416bff1154a61e3821bf69e9d0e57f3`
- `02-MKV-H264-AAC.mkv`：`245c23d5bf6f4be337cc7922da9e8804195242f6dfc489199f0df14337ebdcfd`
- `03-MKV-HEVC-AAC.mkv`：`d23a38f93eb5b9c6343f12505481bb35e82fd57ae0d805f5169f10b8bcd5c304`
- `04-MOV-H264.mov`：`5c586f5d343cdbc3dbfc1ccf888558ff9142960ace2b5d00eb166b72fd7d900d`
- `05-AVI-MPEG4.avi`：`e4a3cbac7cce2ef917375d57086d92546bc6a7ac5b34f87647a7e191e7a78e9d`
- `06-WMV-WMV3-WMA.wmv`：`82df61ed90b5e4dee35088fab2908f712f805c125c43af67bf3e7a68d7b542bc`
- `07-WMV-VC1-WMA.wmv`：`31a06915c686fe0e28d8cea35bee9c4a0b5ed2b8a86502a965cff72f2be2869b`
- `08-MPEG-MPEG2.mpg`：`bc042c874b0cf94f92d612153686a8d4e908a0511728a2b3dc9731213cd2d224`
- `09-TS-H264.ts`：`9b69a2cf932345f1e165fde181a7e39da1b56a301cf45a67f45f45656c144cf5`
- `10-M2TS-H264.m2ts`：`97b7d571d3e3953ad4e692d417f5ee46d3a1a64314d7da06e6c0066861f20b54`
- `11-WebM-VP9-Opus.webm`：`c47a7030301fbf9365514e3767690e736a5c03b8ff8c2927211430d37a943aa5`
- `12-FLV.flv`：`5e3d5cba0bddd1bd8e83e89e5e965c1ae66b061a323cd2012cbe7a3ebee4c36f`

测试来源：[WMV3/WMAPro](https://samples.ffmpeg.org/V-codecs/WMV9/halo2_wmp9_WMV3+audio0x162.wmv)、[VC-1/WMAPro](https://samples.ffmpeg.org/V-codecs/WVC1/FlightSimX_720p60_51_15Mbps.wmv)。测试媒体只保存在本机验收目录，不随源码或安装包分发。

## 23 NSIS构建结果

`npm run tauri -- build --bundles nsis --verbose` 退出码0，产出 `视频下载器_0.4.1_x64-setup.exe`，150252739字节（143.29MiB）。发布程序产品版本0.4.1；安装器x64、SimpChinese、当前用户安装，配置了WebView2 embedBootstrapper及桌面/开始菜单入口。生成脚本与构建日志已核对。NSIS6010为模板未使用函数警告，构建成功。

安装包SHA256：`6783d728378f522e2fca56338cc7fe828dc99b1d5b26996a977bdffe5bfd2e6a`。构建提交：`7a7b0e5a35dc16d3cd85f31e64aea354f5e1d872`。最终源码提交只在该提交后补充文档与验收记录，源码ZIP内包含提交标识；最终ZIP大小、SHA256及提交ID见随包 `delivery-manifest.json`、`SHA256SUMS.txt`。Git归档通过CRC、版本与排除项检查，不包含node_modules、target、测试视频、数据库或个人Cookie。

yt-dlp/FFmpeg/ffprobe/Deno实际文件SHA256与捆绑清单4项全部一致；上游许可、版本与来源清单随包提供。见[构建日志](learning-playback-evidence/nsis-build.log)、[Rust测试日志](learning-playback-evidence/rust-tests.log)、[Node测试日志](learning-playback-evidence/node-tests.log)、[执行记录](learning-playback-execution.md)与[审查修复](learning-playback-final-review.md)。当前使用独立测试数据运行发布程序，没有覆盖用户现有安装；未验证干净Windows10或缺少WebView2的安装环境。

## 24 已知限制

播放器兼容缓存需要磁盘空间与第一次等待；损坏、加密或FFmpeg不支持的输入仍会失败并保留原文件。文本字幕可以转换，PGS等位图字幕不做OCR。远程图片可能因源站限制失效并回退。GPU性能依设备；Windows10与所有独立GPU/Codec Pack组合未逐一物理覆盖。源文件变化依文件尺寸/时间戳检测，不为每次打开计算整片哈希。

## 25 下一阶段建议

基于用户实际素材继续增加兼容样例与故障诊断；积累真实使用反馈后再决定是否增加其他编码策略。本次不加入AI、OCR、语音转写、账号/云服务、HLS或完整剪辑器。
