# DCC PR #14 reference and reuse notes

Reference: Abaddon145/DCC- PR #14, pinned head `63f37a4136fc30f0b75bf3f662f0d085bb2cea64`.
Read through the GitHub connector on 2026-10-18. This is source inspection, not a runtime acceptance result.
No DCC source, configuration, account files, or credentials were copied into Yingliu. No Phase 2 implementation was started.

## Sources actually inspected

Paths below refer to the pinned reference commit:

- [KnowledgeWorkspace.tsx](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/components/KnowledgeWorkspace.tsx)
- [ContextMenu.tsx](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/components/ContextMenu.tsx)
- [KnowledgeMenu.tsx](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/components/KnowledgeMenu.tsx)
- [NoteEditorTools.tsx](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/components/NoteEditorTools.tsx)
- [NoteMindMapDialog.tsx](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/components/NoteMindMapDialog.tsx)
- [knowledge.ts: NoteSaveQueue](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/lib/knowledge.ts)
- [noteMarkdown.ts](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/lib/noteMarkdown.ts)
- [noteSaveGuard.ts](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/lib/noteSaveGuard.ts)
- [noteWorkspace.ts](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/lib/noteWorkspace.ts)
- [noteMindMap.ts](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/src/lib/noteMindMap.ts)
- [package.json](https://github.com/Abaddon145/DCC-/blob/63f37a4136fc30f0b75bf3f662f0d085bb2cea64/package.json)

`src/lib/NoteSaveQueue.ts` does not exist at this commit; the class lives in `knowledge.ts`.

## Reuse decisions

| Reference surface | Observed behavior | Yingliu decision |
| --- | --- | --- |
| KnowledgeWorkspace | Notebook tree, separate note list, tabs, history, favorites, formats, attachments, structure/map modes; data model is JSON document + revision | Recreate the workflow around existing Markdown Vault and file tree. Do not transplant DCC's library/notebook UUID database or JSON primary storage. Preserve deep dark Yingliu colors and cyan accent. |
| ContextMenu | Body portal, measured screen positioning, first enabled focus, outside pointer/scroll dismiss, Escape; flat actions | Borrow positioning and dismissal behavior as design reference. It does not itself supply Shift+F10, arrow navigation, groups, submenus, focus restoration, or native mpv occlusion. Implement those required contracts in one Yingliu component. |
| KnowledgeMenu | Separate flat menu preserves editor selection using pointerdown.preventDefault; supports ArrowUp/Down/Home/End, Tab/Escape dismiss | Merge required keyboard behavior into the common menu. Preserve Tiptap selection as editor state separately from menu focus; avoid retaining two global menu implementations. |
| NoteSaveQueue | One in-flight request, newest pending input, serial drain, latest revision rebasing, failed request does not replace newer pending draft; optional serialized recovery writes | Adapt the queue algorithm to vaultId + normalized relative path + expectedHash. Treat conflict as a stopped queue, not successful acceptance. Queue acknowledgement updates saved baseline/hash without replacing current draft. |
| Workspace autosave integration | 600 ms debounce; flush captures map draft, waits attachment imports, drains queue; accepted callback updates savedRef and does not set editor content | Preserve that separation. Keep explicit save and leave guard. Pending map edits and attachment imports participate in flush. |
| Save barrier | Snapshot registered save guards before awaiting; nested barriers lock editing and flush before library operations | Reuse the principle for Vault changes, rename/move, close, export and history restore. Yingliu already has App-level registerLeaveGuard; extend it instead of adding competing navigation guards. |
| Tabs | Metadata includes pinned/view/layout/color/scroll/selection, bounded to 50; chosen/closed note flushes before loading replacement | Identify tabs by vaultId + relative path, not DCC UUID. Restore only validated metadata, never unsaved text from layout imports. Update all matching keys after successful moves. |
| NoteEditorTools | Tiptap-bound toolbar, selection-aware context menu, formatting commands, heading outline, safe http/https links; DCC clipboard and asset APIs | Implement a small editor toolbar using Yingliu safe link/attachment boundary. Do not copy dcc-media URLs, custom clipboard package, screenshot commands or library IDs. Existing CodeMirror remains source fallback. |
| NoteMindMapDialog | mind-elixir, bounded normalized data, 50-step local undo, draft commit on close/save/export, read-only mode, safe external links | Keep full requested mind map capability in Phase 6, but integrate through a dedicated draft commit contract. Derived heading view can reuse Yingliu headings() without changing Markdown. Editable map metadata must have an explicit Markdown-compatible representation and no-loss fallback. |
| Markdown bridge | Tiptap MarkdownManager with custom DCC nodes; rejects most embedded HTML; structure serialization can rewrite content into indented bullets | Do not transplant as a general Obsidian round-trip layer. It contains no explicit frontmatter/Wiki Links/unknown-Obsidian preservation contract. Use capability detection, preserve raw source and opaque segments, and fall back to source when unsupported. |

DCC depends on Tiptap 3.31.3 packages and mind-elixir 5.15.1; Yingliu currently has neither. Yingliu already has CodeMirror, react-markdown, remark-gfm, unified and remark-parse. Add only the editor/map dependencies needed for the explicitly requested features, after Phase 1 P0 fixes.

## Concrete target contracts and integration hazards

- A note session owns `vaultId, path, savedDocument, draft, editVersion, conflict, saveState`. Save captures the submitted editVersion and expectedHash. Only that acknowledgement advances the baseline; newer text remains dirty.
- `flush()` commits nested map draft, awaits pending asset imports, drains saves, and rejects on error/conflict. Close/Vault switch/move must stop on rejection.
- Existing Yingliu `KnowledgePage.save()` calls `accept()`, which overwrites current text and resets the editor key. This is safe only while its current busy lock disables editing; do not reuse it unchanged for editable-background autosave.
- Existing `knowledge_write` already returns a saved/conflict union and hash. Preserve this optimistic-concurrency contract for Obsidian external edits.
- Existing `move_note()` currently discards errors updating `video_analyses` and `video_notes`. Fix via checked transaction plus filesystem compensation before adding tree move UX. Do not update tab/queue paths until the service confirms the operation.
- Existing `NativeLearningSurface` measures CSS rect, observes resizing/mutations/scroll, and Rust converts once using the window scale factor. Layout changes must preserve mounted playback session.
- Native surface currently hides only for document hidden or modal dialog. Add an explicit shared DOM overlay marker for portal menus and actual hidden/collapsed state; include marker changes in mutation observation. Otherwise mpv can cover context menus. Exclude menu keys from player seek shortcuts.
- Library sorts must execute before backend 50-item pagination. Current series model lacks aggregated last-learned/AI/file state; supply one backend aggregate query rather than per-card requests.
- Workspace import/export must use an explicit layout-only schema, never AppSettings/DiskState. No credentials, note drafts, or absolute private file paths in exported presets.

## License finding

Yingliu's local root LICENSE is MIT, copyright 2026 14582.
At the pinned DCC commit, connector reads of root LICENSE, LICENSE.md and LICENSE.txt returned 404; DCC package.json declares no license. This establishes only that those common license locations did not provide a grant, not that every file or dependency is unlicensed. Repository access and shared ownership are not a substitute for recording redistribution terms.

This plan uses behavior and architecture as reference and independently adapts Yingliu code, with no verbatim DCC source vendoring. If actual source is copied later, establish the applicable grant/owner authorization and preserve required notices. Verify licenses from the exact resolved Tiptap and mind-elixir packages when adding them; no dependency license was inferred from package names.

## Required no-loss checks before Phase 6 completion

These are planned checks; none is reported as passing here.

1. Deferred-save test: send draft A, type B while A is in flight, resolve A, assert visible draft B survives and next write uses A's returned hash; final disk content B.
2. Failure test: A in flight, enqueue B, reject A, assert B remains recoverable and retry writes B; failed recovery-copy writes must remain visible.
3. External-edit test: save with stale hash after Obsidian changes file; assert queue stops in conflict, disk external version and local draft both survive, no automatic overwrite.
4. Two-tab test: same basename in two folders and two Vaults; saves and conflicts address only exact vaultId/path. Tab switch, close and Vault switch await flush or leave the original session open.
5. Mutation injection: fail each SQL association update after file move, and separately fail compensation; assert either original file and both links restore or explicit recoverable partial-failure details are returned. Never report silent success.
6. Race test: outstanding save plus rename, directory move or Vault switch cannot create a new stale-path file. Moving a directory rewrites all affected tab/session/queue paths and course/analysis references.
7. Markdown corpus: BOM/CRLF, YAML comments and nested metadata, Unicode, Wiki aliases/anchors/embeds, task lists, fenced code with literal HTML, tables, image paths with spaces/#/%/&, footnotes, callouts, block IDs, raw HTML, unknown plugin syntax. Opening and leaving without edits must preserve source bytes; unsupported rich mode must refuse conversion and retain editable source. Supported edits must preserve all untouched opaque content and attachment targets.
8. History restore test: inspect previous version read-only; restoring creates a recoverable new version using current expectedHash; stale disk version triggers conflict. Missing history never damages current file.
9. Asset/map test: saving or closing while image import or focused map edit is pending commits it first; failed import/export retains draft, no dangling attachment reference is written; invalid/oversized map is rejected without modifying Markdown.
10. Menu/UI acceptance: disabled actions skipped by keyboard; Shift+F10 targets focused object; arrows/groups/submenus/Escape/focus restoration work near every screen edge; mpv menu overlay and layout drag verified at 100/125/150/200% Windows DPI. These require actual desktop acceptance, not a DOM mock.

