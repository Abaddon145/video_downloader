# Phase 4 backend implementation and verification

Date: 2026-10-18. Implementation was confined to `artifacts/v051/staging`; this report and test logs are in `artifacts/v051`. No Git operations, external network requests, real DeepSeek calls, real credentials, package changes, or frontend edits were performed.

## Result

`cargo test --locked` completed successfully: **219 passed, 0 failed, 8 ignored**; binary and doc-test targets also passed. Full output: `phase4-rust-final.txt`. The existing incremental hard-link fallback warning and linker output notice remain; there are no test failures. Tests use isolated temporary databases/Vaults, deliberately fake DPAPI test credentials, and loopback HTTP fixtures only. Windows atomic replacement in those fixtures required execution outside the restricted filesystem sandbox.

The Phase 3 baseline had 199 passing tests. Phase 4 adds 18 template tests and 2 knowledge-generation tests. An earlier full run exposed two obsolete expectations: regeneration mutated the old ID, and a paid-cache fixture lacked the new template identity. Both were updated to assert the new contract while retaining source, transcript, historical summary, queue and billing protections. The actual v6 migration test preserves the original summary JSON byte-for-byte and transcript text, and rejects a future schema without rewriting its version.

## Delivered behavior

- Ten immutable builtins: `general`, `course`, `software`, `ue5-cg`, `film`, `programming`, `language`, `meeting`, `troubleshooting`, `custom-starter`. Each defines purpose-specific focus and columns. Official templates can be copied but cannot be overwritten/deleted. Custom saves create a new version using `expectedVersion`; favorites are separate from version identity. Deletion hides a custom template while analysis snapshots remain usable.
- `AnalysisTemplate` has the agreed camelCase fields. `sections` is `{id,title}[]`; `detail` is `concise|balanced|detailed`. Limits: name 80, description 1000, focus 4000, section title 80, 1–12 columns, Markdown 16000 characters, complete template 128 KiB; at most 200 custom IDs and 100 versions per ID, including retained deleted history.
- Schema v7 adds immutable analysis template identity/snapshot, fallback warning and explicit material-reuse source, plus template version/preferences tables. Legacy analyses remain `general`, version 0, snapshot null, and retain their original prompt/model/results. New analyses use `v0.5.1-templates-1`. Unknown, corrupt or unavailable selected templates fall back to general with a warning. A fallback cache hit returns the current request's warning without modifying the old record.
- Queue admission freezes the complete validated template with `favorite=false`. Task, chunk and hierarchical summary cache identities include the purpose snapshot. Current template edits/deletion cannot alter queued jobs or prior analyses. Failed/interrupted/cancelled ordinary retries retain their original ID and completed checkpoints. Regeneration creates a new analysis ID and leaves original summary/chunks/notes/history intact.
- Purpose output is an optional bounded `sections` array. Each section must match a frozen ID/title; each item has bounded title/content, verified subtitle/frame IDs and an optional strictly verified timestamp. Live HTTP results, DB chunk cache and file summary cache use the same checks. Section-only references are carried through hierarchical merging. Custom preferences remain serialized user-message data behind unchanged fixed system rules, privacy checks and source checks.
- Explicit `reuseAnalysisId` can copy matching, validated local materials. Subtitle reading remains first; only old transcripts explicitly marked `whisper` are reusable when subtitle extraction yields no current subtitles. Missing/unknown/old-subtitle origin is not treated as Whisper. Frames require matching depth, source, duration, permissions, SHA, JPEG bounds and owned paths. Copies receive fresh IDs and owned JPEG files under the new analysis; deleting the old analysis cannot delete the new material.
- Full analysis (`force=true`) clears the persisted material-reuse source. There is no automatic historical-material scan, so full analysis remains full after restart. Template switching and regeneration explicitly name their source. Cached source IDs are validated before building paths, origin markers are bounded, and copy operations retain cancellation/privacy/source checks.
- Markdown uses a fixed placeholder whitelist and single-pass replacement. Empty/whitespace format uses the safe default, ordinary Markdown quotes/comparisons are supported, raw HTML tags/unknown/unclosed placeholders are rejected. Values inserted into placeholders are never interpreted again. Three existing Markdown files, versioned directories, attachments, metadata hashes and edit/conflict protections remain. Frontmatter now records template identity/hash/name and prompt version. `timeReferences=false` suppresses video timestamp links in generated notes while retaining source validation.
- The global auto-note setting is a default for new tasks. The confirmation page's frozen single-task choice controls that task. Upload consent revocation remains unchanged. A completed paid-cache hit with notes selected performs only local generation into the currently selected Vault; no AI request or usage row is added, original options/summary remain unchanged, and no-Vault/export failures leave a warning and preserve prior notes/results.

## Commands

`ai_list_templates()`, `ai_template_versions({id})`, `ai_save_template({template,expectedVersion})`, `ai_copy_template({id,version})`, `ai_set_template_favorite({id,favorite})`, `ai_delete_template({id,expectedVersion})`.

`ai_start_analysis.request` additionally accepts optional/null `templateId`, `templateVersion`, `reuseAnalysisId`, `autoGenerateNote`. Existing fields remain. `ai_retry_analysis(...,regenerate=true)` returns the new analysis ID. `VideoAnalysis` adds `templateId`, `templateVersion`, `templateSnapshot`, `templateWarning`. All commands are registered in the Tauri handler.

## Verification record

Observed RED→GREEN steps included: schema 6→7; missing template catalog/version/protection interface; strict parser rejecting the new sections; request DTO rejecting new options; queue returning general for a selected language purpose; section-only sources missing from final material; missing material-copy interface; missing template/section Markdown output; empty Markdown rejected on save; inconsistent persisted version accepted; unknown transcript origin reused; missing local cached-note interface; fallback warning lost on cache hit. New interface steps initially failed compilation for the absent interface; runtime defects failed concrete assertions before their fixes.

Additional loopback HTTP tests verify language-purpose output through chunk and summary requests, persisted usage, offline reuse of both validated cache layers, fixed-system/custom-data separation, and rejection of an unknown returned column without publishing its result while retaining the billing fact. Material tests cover independent copies, deletion of the old cache, cancellation, permission revocation, source changes, tampered JPEGs, explicit Whisper provenance, and full-analysis state reloading. Markdown tests render all ten purposes, preserve placeholder-like source text literally, suppress disabled time links and verify empty-format defaults.

Commands run against `staging/src-tauri` with `CARGO_TARGET_DIR=C:/Users/14582/Documents/ChatGPT/视频下载软件/src-tauri/target` and `C:/Users/14582/.cargo/bin/cargo.exe`:

- Incremental `test --locked --lib <test/filter>` runs for each RED→GREEN step.
- `test --locked --lib` → first 207 passed/2 failed/8 ignored (`phase4-rust-first.txt`), then 209/0/8 (`phase4-rust-second.txt`).
- `test --locked --lib template_` → 16/0 at that checkpoint (`phase4-template-tests.txt`); two further template regressions were added afterwards.
- Final **`test --locked` → 219/0/8**, plus binary/doc-test success (`phase4-rust-final.txt`).
- Rustfmt was applied only to the owned Rust files, with `skip_children=true` for module roots.

## Files

New: `ai/templates.rs`, `ai/template_tests.rs`, `ai/materials.rs`, `learning/schema-v7.sql`.

Integrated: `ai/mod.rs`, `models.rs`, `commands.rs`, `jobs.rs`, `prompts.rs`, `analysis.rs`, `references.rs`, `repository.rs`, `summary.rs`, `learning/db.rs`, `knowledge/generated.rs`, `lib.rs`. Existing request fixtures/callers were updated in `ai/downloads.rs`, `ai/analysis_tests.rs`, `ai/live_tests.rs`, `knowledge/generated_tests.rs`; the loopback helper in `ai/reference_tests.rs` is shared between test modules. Schema expectations in `learning/tests.rs` are now 7 without removing data assertions.

## Limits and remaining validation

Real DeepSeek output and real Whisper/media end-to-end checks were not run; the 8 existing integration tests remain ignored with their resource/credential requirements. All ten purpose definitions/Markdown exports are exercised locally, while the HTTP round-trip uses a deterministic language-purpose response. Browser/native integration and independent review belong to the root task. Unknown old transcript provenance deliberately causes fresh local recognition instead of speculative reuse. No arbitrary template language, path/include facility, new dependency, or shared-file cache/reference-counting framework was introduced.
