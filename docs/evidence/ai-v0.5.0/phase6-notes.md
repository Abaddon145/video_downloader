# Phase 6 — Summary, questions and durable analysis runtime

Validation: 122 ordinary Rust tests passed; 3 opt-in integrations excluded from the ordinary run. 23 Node tests and production build passed (phase6.txt).
Separately executed ai_real_local_pipeline_mocked_summary_failure_resume_and_regenerate_without_whisper: passed in 16.32 seconds. Real local FFmpeg/Whisper Base/source file/SQLite; DeepSeek HTTP responses were simulated using an actual localhost HTTP fixture, not the real API.
Original JFK WAV SHA256 before/after: 59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e.
Summary failure after completed chunk: retry made one summary request. Explicit regeneration made two requests (one chunk, one summary), despite an intentionally damaged model after first transcription, proving no repeat Whisper run.

Final requests contain structured part results and validated source identifiers/times, not raw subtitles or images. Bounded hierarchical grouping supports large summaries. Valid intermediate summaries are cached by request hash. All result times/source IDs pass the existing strict validator.
Questions retrieve at most 8 local hits plus bounded neighbors (12 segments/12KB), including FTS-unavailable fallback. References must stay within provided evidence. No evidence returns the fixed unknown answer without loading a key or sending a request. Current-segment explanation uses the local ±30 second range and at most one nearby verified JPEG.

Independent single-worker queue persists options/checkpoints, manually resumes interruptions, cancels pending/active work and guards late publication. Default repeated requests reuse matching cached work. force=true starts a new complete analysis. Regenerate preserves local transcript/frames but explicitly requests fresh AI analysis. No automatic paid restart.

No real DeepSeek API request was executed: no user key has been configured. These fixture tests establish request, privacy, retry and validation behavior; they do not establish live model quality or account availability.
