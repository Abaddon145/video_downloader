# Phase11: generated knowledge and screenshot notes

Verified on this Windows host, 2026-10-18. Existing AI analysis is integrated into each Learning Library course; no standalone AI navigation.

- Rust: 172 passed, 0 failed, 6 explicitly opt-in tests ignored in the ordinary run. Full output: [rust-tests.txt](rust-tests.txt).
- Explicit real FFmpeg screenshot test: 1 passed, source hash unchanged, custom attachments directory: [real-screenshot.txt](real-screenshot.txt).
- Node: 37 passed, 0 failed; production frontend build passed. Knowledge remains a lazy-loaded ~805KB minified chunk; Vite retains its >500KB warning.
- Browser IPC fixtures: 34 course interactions and 18 Knowledge interactions passed, no page errors: [course](course-final/learning-panel-ui.json), [Knowledge](knowledge-outline-final/knowledge-ui.json).
- Real Windows/Tauri/filesystem/mpv: 9 passed, 0 failed, source SHA and Vault .obsidian sentinels unchanged: [desktop report](desktop-final/generated-course-notes-desktop.json). Summary/transcript rows are seeded fixtures; **0 AI requests**, not real DeepSeek acceptance. [Fixture timing diagnosis](desktop-verification.md).

Summary, grouped transcript and knowledge Markdown use normal YAML, Wiki links, local attachments and course time links. Atomic draft-directory publication never overwrites an existing generated version; repeat export preserves manual edits. Current-summary hashes recover and reuse moved note associations. Default auto-generation captures the selected Vault identity at enqueue; absence or switching produces an actionable warning without discarding the completed AI summary.

Screenshot notes freeze the actual player timestamp before pausing, reuse FFmpeg/native Job cancellation, publish a JPEG plus ordinary Markdown, and associate the VideoNote with its Vault document. Editing a linked note opens the Markdown source. Default analysis deletion preserves notes. Joint deletion previews the current associated version, checks ownership and hashes, and preserves edited, unknown and older-version files.

Visual review found that the old regex outline exposed raw video URLs and disagreed with rendered Wiki aliases. A failing Node test reproduced it; the outline now shares the existing remark AST transformation and exact preview IDs. A second test bounds synchronous outline parsing at 256K characters, with a visible explanation for larger notes. Unchanged text reuses the parsed outline. The final 880px screenshot was reviewed: [outline](knowledge-outline-final/knowledge-outline-880.png).

One initial browser attempt failed while the Vite endpoint was stopped; a cold development reload interrupted a subsequent first navigation. After starting and stabilizing the server, the complete unchanged interaction suite passed. No product assertions or existing tests were removed.

Phase12 remains: version/release checks, complete regression, independent branch review, installer/local installation and live DeepSeek acceptance with a newly configured user credential. No live API, real Obsidian program, all-device DPI or Windows 10 clean-install claims are made here.
