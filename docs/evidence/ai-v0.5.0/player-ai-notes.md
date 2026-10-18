# Learning player AI integration — Phase10 playback portion

User requested direct per-video AI buttons in Learning Library and results visible beside playback. This playback portion was brought forward after Phase7 under that explicit steering; Vault integration and the remaining Phase10 surfaces still await Phases8/9.

Implemented:
- Visible AI action next to each local lesson; unavailable lessons cannot start analysis. Known active/completed records update the button state.
- Opens the actual selected lesson, preserving item_id and explicit consent before paid analysis. Existing analysis displays immediately.
- Desktop video/AI split about65/35; window880 stacks content. Summary, chapters, terminology, transcript, video questions/current-segment explanation, and the original timestamp notes share the sidebar.
- Chapter/question references reuse the existing PlaybackHandle seek (milliseconds converted to seconds). No second player or transcode path.
- Original notes remain mounted across tab switches. Native modal visibility handling is reused. Native/HTML fullscreen hides the sidebar and expands the existing surface.
- Backend association verifies source fingerprint; relinking a lesson cannot show analysis for its old file. Old task/course events cannot replace newer selected results.

Verification:
- 125 ordinary Rust tests passed, 3 opt-in real integrations excluded from the ordinary run (earlier phases record their explicit runs).
- 27 Node tests passed; production build passed. player-ai-checks.txt records the command outputs.
- 7 browser fixture UI checks passed: course action/consent, side-by-side summary, existing native seek interface, question reference seek, timestamp notes, fullscreen,880 responsive.
- 5 real desktop groups passed (player-ai-desktop.json). Actual WMV source/mpv/SQLite; summary supplied as clearly labeled UI fixture through the normal frontend event channel. No real DeepSeek request was made.
- Chapter click observed through actual learning-native-updated events at2.002seconds. A real note saved at2000ms. No transcode tasks; fullscreen restored; AI page selected the imported actual learning video.
- Original WMV SHA256 before/after:3934e0daba8f29c0cd785389813399dbaab39efb31abe65121b1b7c5486870c9.

Diagnostics and fixes:
- Extra layout container initially collapsed native fullscreen surface height to0; browser regression assertion failed then passed after restoring flex sizing on the shared fullscreen containers.
- Initial real-desktop test instrumentation attempted to replace Tauri's protected invoke function, so no calls were recorded although playback had already jumped. Removed the hook and verified native state events and persisted note rows instead. No production IPC weakening or test-only production endpoint was added. Question reference behavior remains covered by actual localhost mocked-client backend tests and the browser fixture; no claim of a live question API test.
- A stopped Vite session caused connection-refused during UI rerun; restarted the local-only QA server and reran both UI scripts successfully. No product change for that environment issue.
- WebView screenshots exclude the native mpv child surface; a black video area in the CDP capture is not a claim about rendered native frames. The actual native readiness/state/seek are verified above. Screenshots reviewed atwide/880 and actualdesktop.

Pending for complete v0.5.0: Vault backend/UI, generated Markdown/screenshot notes, remaining player Knowledge tab and media/download entry points, full release QA/docs/version/NSIS/review/new PR. Real DeepSeek quality/account verification still needs a user-configured key.
