# Phase11 real desktop verification

Final command: node scripts/qa/verify-generated-course-notes-desktop.mjs with QA_APP pointing to the debug Windows executable, a copied WMV original fixture, Python SQLite summary fixtures, and QA_OUTPUT_DIR=docs/evidence/ai-v0.5.0/phase11/desktop-final.

Result: exit 0, 9 passed, 0 failed; real Windows/Tauri, real filesystem and mpv; seeded completed summary/transcript rows; 0 AI requests. Final report is desktop-final/generated-course-notes-desktop.json. Captured native pause time and saved VideoNote time both equal 8174 ms; the real JPEG is 6312 bytes and renders in Markdown preview. Source SHA256 and both Vault sentinels remain unchanged.

The initial seek-to-six-second assertion was a fixture assumption failure. Reproduction observed one native session: first ready at 0.267 s, direct absolute+exact seek requested 6 s, result 8.074 s, pause stable at 8.074 s. A second run using ordinary UI playback instead of seeking observed positions 4.04 s, then 5.00 s repeatedly, then 8.108 s. This WMV has a decoded timestamp gap around the intended six-second frame. No persistence writes were forced to satisfy the check. The final script reaches the actual eight-second decoded frame through UI playback, pauses through the UI, and requires the saved VideoNote timestamp to match the native paused state within 100 ms, alongside the dialog's displayed eight-second time.

A separate QA synchronization correction opens the analysis-history disclosure only if closed: after default deletion React preserves its open state, so blindly clicking it again hid the next deletion button. Second bundle export also waits for its persisted reference before planning deletion.

Frontmatter checks cover plain YAML and required source/duration/ai_model/analysis_mode/analyzed_at fields. All three Markdown files are generated and open in Knowledge preview. Repeat export preserves a manual edit. Stale references are rejected after Vault switching. Default delete preserves the three Markdown files. Explicit joint delete shows all three planned paths, checks SHA256 values, and removes only those confirmed generated files while preserving the earlier user-edited bundle and source.

At 1440px and 880px, screenshots capture generation actions, generated Markdown preview, real screenshot preview, and the explicit joint-delete confirmation; horizontal overflow checks pass. Playwright captures the WebView surface, so native mpv's separate child window may appear black in sidebar screenshots; the JPEG attachment and native state assertions verify actual decoded media capture.

One intermediate evidence overwrite encountered Windows fs UNKNOWN opening the existing report JSON. The app was shut down by its own exit command, then the complete run succeeded in a fresh desktop-final directory. The canonical report in the parent directory is copied from that final successful run. No product files were changed by this QA task.
