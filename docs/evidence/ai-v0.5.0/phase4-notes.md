# Phase4 实际验证

107项普通Rust测试、23项Node测试、生产构建通过。2项依赖本地素材的测试默认为ignored：Whisper已在Phase3单独执行，关键帧在本阶段用 `cargo test --locked keyframes_real_ -- --ignored --nocapture` 真实执行通过，63.86秒。phase4-real.txt为此次进程开始后的实际stdout，phase4.txt为全量常规回归。

真实矩阵12格式：MP4 H264、MKV H264、MKV HEVC、MOV、AVI MPEG4、WMV3、VC1、MPEG2、TS、M2TS、VP9 WebM、FLV。每个源文件前后完整SHA保持一致。另生成颜色切换视频确认真实检测到2个场景，生成4K无音轨视频确认输出1280×720，检查取消及纯音频分支。

采样配置集中管理：快速180秒/12张、标准90秒/24张、深度45秒/60张，场景检测补充；长片按数量上限均匀覆盖。JPEG最大尺寸1280、每张2MiB，qscale5（约80质量，不同内容实际压缩率不同），SHA去重。关键帧时间来自FFmpeg实际输出PTS。

实测暴露并修复两个TS问题：JPEG编码器拒绝有限色域，现显式转换完整色域；快速定位可能无帧，现一次顺序读取批量提取已选时间点，避免每张回到开头重解码。[FFmpeg滤镜依据](https://ffmpeg.org/ffmpeg-filters.html)；仅本地协议和已支持格式参与处理，源文件只读。

重跑需AI_QA_MATRIX指向既有matrix.json，AI_QA_RESOURCES指向已校验的src-tauri/resources。此阶段未向DeepSeek发送任何素材。
