# Phase 1A — reference recovery implementation

Workspace: `artifacts/v051/staging` (PR8 baseline). Only the assigned AI reference/parser/model/module/test files are edited here; no Git operations or provider API calls.

## Contract and rulings

- Local timestamp correction is limited to 3,000 ms against an individual existing cited cue intersected with the real scope. Unknown IDs, original out-of-scope times, and larger mismatches are not locally accepted.
- Strict validation still validates the final result. Reference failures may be omitted independently, but malformed JSON, oversized arrays/text, and invalid importance remain errors.
- The synchronous cache parser normalizes/degrades without network access. A separate live parsing seam retains invalid candidate indexes for at most one small repair request.
- Warnings use an optional/default `warnings: Vec<String>` result field, bounded to eight entries of 200 characters. Local omission adds `部分重点无法核对已略过` and survives cache reload. Existing JSON requires no migration or prompt-version bump.
- Repair evidence will contain whole source cue text with exact source IDs, bounded in count and bytes. It never sends raw media, new images, credentials, file paths, or valid unrelated points.

## Verification so far

1. Added reference tests before production behavior.
2. `cargo test --locked reference_ -- --nocapture`: RED as intended (two failures at `AI重点时间与引用字幕不一致`, one existing-behavior pass).
3. Implemented local parser recovery and warnings.
4. `cargo test --locked --lib reference_ -- --nocapture`: GREEN, 3 passed. Existing hard-link cache warning remains environmental.
5. Added bounded live-HTTP repair/cache/lifecycle tests and scope/warning edge cases. Parent ran compile-compatible live RED tests with Windows DPAPI access: all three failed as intended (missing successful repair, missing second request, missing privacy followup guard).
6. Implemented live repair only after that RED: at most one 2,048-token repair for the first eight failed points, up to twelve whole cues / 12,000 evidence bytes and 20,000 serialized message bytes. Existing valid points are excluded. Nearby metadata for up to six already-provided frames can be included, without sending images.
7. Only `timeMs` and `sourceSegmentIds` may change, only for offered failed indexes. Patched IDs must be in the small evidence subset, not merely the larger original scope. Repair subtitles require an exact cue interval match; repaired visual anchors must equal an actual offered frame time. Duplicate/unknown indexes, forged/unprovided IDs, out-of-scope patch times, schema errors, oversized responses and provider truncation do not publish invalid content.
8. Ownership/cancellation, current privacy consent, and source fingerprint are checked before and after the repair. Every received repair reply records `reference_repair` usage before post-response checks. Network/schema/repair failures omit only unresolved points and add the durable local warning. Cancellation, privacy changes, source changes, and usage persistence errors remain task errors.
9. Wired chunk analysis through `chat_for(..., Chunk)` and the live seam; cache reads still use synchronous normalization/degradation. Root wires summary recovery and warning inheritance.
10. Root reported elevated reference tests GREEN, 7/7. The combined full-suite log `phase1-rust.txt` showed 191 passed, 1 failed, 8 ignored; the sole failure was Windows accepted-socket `WouldBlock` in the new mock HTTP fixture, not a product assertion.
11. Set each accepted test stream to blocking mode before its existing three-second read timeout. This fixes inherited nonblocking behavior from the listener without changing product code or loosening retry/time limits. Root reruns full verification; no sandbox Cargo rerun was attempted.

## Known limits

- A repair can assess content relevance from the supplied bounded evidence, but programmatic validation establishes source identity and time ranges, not semantic entailment.
- More than eight unresolved points or evidence exceeding the bound is locally omitted with a warning rather than spawning more paid repair requests.
- No production credentials or provider APIs have been used; live tests use local HTTP and synthetic DPAPI-encrypted test keys.

## Assigned file list

- `src-tauri/src/ai/analysis.rs`
- `src-tauri/src/ai/references.rs` (new)
- `src-tauri/src/ai/reference_tests.rs` (new)
- `src-tauri/src/ai/models.rs`
- `src-tauri/src/ai/mod.rs`

The root agent owns `client.rs`, `summary.rs`, `qa.rs`, `usage.rs`, and their integration tests. The root provides `RequestPurpose::ReferenceRepair`, its 2,048-token budget, usage-operation support, and the summary call to the live parsing seam.

## 主任务最终验证

固定基线172Rust；本阶段最终193通过、0失败、8项既有显式忽略。前端37通过，生产构建通过。Obsidian浏览器模拟原生接口22项、课程面板35项通过；实际Obsidian应用打开回执及用户原课程真实API复测未执行。
