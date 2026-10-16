# v0.4.2 实施记录

# SDD ledger — plan: docs/superpowers/plans/2026-10-16-native-learning-playback.md
Baseline: fc1fa80 app, docs cdd2fb2/912dd21; linked managed worktree reused on codex/native-learning-playback-v0.4.2. User approved written spec and plan, inline execution.
Pre-flight: backend NativeSnapshot/Action/Viewport feed UI; old prepare API native defaultsfalse; existing progress and manual compatibility retained. Native locks must not span main-thread window creation. No new production dependency crates.
Todo: Task1 complete; Task2 complete; Task3 complete.
Initial real RED: 157s WMV first-frame decode433ms, existing prepare not ready after5005ms while full transcode62%; deterministic five-second startup assertion failed.
Task 1: complete (commits 912dd21..eabc94b, tests: C:/Users/14582/.cargo/bin/cargo.exe test --manifest-path src-tauri/Cargo.toml --locked --offline → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 2: complete (commits eabc94b..5774288, tests: npm test → ℹ duration_ms 104.9814)

Task 3: real native acceptance 11/11 and lifecycle 2/2; UI 9/9; Rust96, Node23; MediaPro16/16; real download/compatibility integration8/8. All original hashes unchanged.
Final: fixed cancelled old start claiming a newer owner — native_cancelled_claim_preserves_newer_owner RED→GREEN.
Final: fixed failed native persistence blocking explicit conversion — manual_compatibility_remains_available_after_native_failure RED→GREEN.
Final: fixed failed native session/window retention — real forced engine exit RED→GREEN.
Final: fixed native transport shortcut autoplay countdown — input boundary and UI countdown RED→GREEN.
Final: regraded Escape Minor to Important for keyboard control consistency; fixed fullscreen Escape from button focus RED→GREEN. No deferred minors.
Ruling: system DPI125/150 coordinate conversion and simulated UI are covered; different physical GPUs/multi-monitor DPI are not all available on this host — document this boundary, cost: device-specific behavior still needs confirmation there.
Final: pending packaging assessment is handled by NSIS build/resource/hash checks; license scope is component notices and source completeness, no broader legal opinion requested.
Task 3: fixed inherited moving FFmpeg latest archive using same-commit immutable upstream archive; both exe SHA256 values identical to v0.4.1.

Task 3: NSIS build exited0, 184843083 bytes, sha256 97ceda526e38b57ee516565730c1ad2423de1bbc6742c7228deafab3477eeba8; PE x64 and both versions0.4.2, all five tools+mpv input/fonts/licenses present in NSIS file entries.
Task 3: post-bundle release smoke ready660ms, no conversion jobs, seek100s/rate1.5/pause persist passed.
Task 3: pushed independent branch and created/attached Draft PR7, base fc1fa80; no merge/Release.
Task 3: complete (commits 5774288..aed4b25, tests: C:/Users/14582/.cargo/bin/cargo.exe test --manifest-path src-tauri/Cargo.toml --locked --offline → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)

Final: all three tasks complete. Installer, initial source ZIP and checksum manifest verified before task-done; final documentation archival changes no application code. Worktree retained for PR feedback.
