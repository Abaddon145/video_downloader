# Phase 7 — AI workspace

124 ordinary Rust tests, 26 Node tests, production frontend and custom-protocol desktop builds passed (phase7.txt). Three opt-in real integrations remain excluded from the ordinary suite and are recorded separately in earlier phase evidence.
Browser fixture checks: 9 groups passed. These include model controls never submitting paid analysis; privacy toggles; local-only action; transcript pages of 50; references; stale task events; manual retry; 880px/keyboard escape; direct Learning Library search/pagination/unavailable file protection and itemId association. UI data and API replies in this browser test are simulated.
Real desktop checks: 5 groups passed (phase7-desktop.json). No API key or paid request; real imported verified Base model/FFmpeg/Whisper/SQLite; serial tasks; pending and active cancellation; real timestamped transcript UI; exit/restart restores interrupted tasks without automatic processing. UI round trip observed 1ms during the test. This is one host, not a performance claim for all Windows devices.
Original JFK WAV SHA256 remained 59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e.

UI uses the existing desktop palette, system fonts, SVG icons and restrained motion. Search guidance was narrowed to professional media workspace after an irrelevant education-theme result; no unrelated design system/dependency was installed. Screenshots at1440/880 and real desktop were inspected; inherited dialog label spacing was corrected. Model controls explicitly type=button inside the analysis form; a regression assertion failed then passed.
DeepSeek official rate card checked 2026-10-17: https://api-docs.deepseek.com/quick_start/pricing/. pricing.ts contains USD per-million-token low/high estimates only. Unknown usage/model suppresses the estimate. Raw facts remain SQLite; no prices in business logic. Final bill belongs to the DeepSeek account.
No real DeepSeek model call has been executed without a user-configured key.

User steering adds direct Learning Library video selection in this page. Search matches lesson and series titles, uses parameter binding, pages50, checks local-file validity and retains item_id; derived lesson titles survive rename and history remains available if the lesson is removed. Missing/download placeholders cannot be selected.
The user's subsequent request for per-video AI buttons and results beside playback will be implemented next as the player portion of Phase10, ahead of the Vault phases; Vault-only tabs and generated notes still follow their backend dependencies.
