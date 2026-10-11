# Bundle Hover Inspection
Updated on: 2026-10-11
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

## Intent and witness

Hovering a file or folder in the ZIPS tree opens a custom glass information
panel matching the existing context menu. Show total size, currently included
size and file count, plus the largest included files inside a folder, so JL can
decide what to exclude. Sizes describe source bytes before ZIP compression;
JL requested no explanatory footer in the card.
Verify the installed Windows UI, hover dismissal and selection changes.

## Owner decision

Extend im_bundle with metadata-only inspection using its canonical selection
predicate. Run reads on an owned blocking worker through each repository's
existing host/WSL route, outside the runtime lock. Preserve repo containment,
Git-directory and symlink boundaries. Bound scans and mark partial results;
cancel obsolete hover requests through the existing cancel-envelope contract.
One hover owner per tree handles delay, dismissal, selected-state changes and
stale replies. Reuse the context-menu surface styling without menu semantics.

## Live receipt

- Identity: /home/johnf/dev/intermediary, master at f67aa76; clean at start.
- Baseline: tree uses native title tooltips and has no size inspection; lazy
  listings return paths only. ContextMenu owns the requested glass surface.
- Frontier: complete. Installed glass hover cards omit the footer at JL’s
  request and use animation-safe placement.
- Evidence: ignored target/bundle_inspection_witness.
- Remaining: none. Temporary fixture removed; all 15 original repository
  configurations and global exclusions match the initial snapshot. The user’s
  latest StJohnsChurch tab is retained.

## Verification

- [x] Four inspection regressions pass: totals/ranking, bounds, cancellation,
  traversal rejection and symlink boundaries. Frontend cancellation regression
  passes within the 117-test TypeScript suite.
- [x] TypeScript typecheck and ESLint pass; 117 frontend tests pass. Rust
  agent check and Windows workspace check pass. Host tests: 39 pass. Agent
  suite: 366 passed, one existing 150 ms linked-worktree lock test timed out
  and passed on its focused rerun. Existing unused_mut warning at
  src-tauri/src/lib/terminal/session_close.rs:70 remains.
- [x] Native fixture: Saved totals 12.5 MB, selected 4.0 MB, 3 of 5 files;
  excluding its 3 MB recording changes selection to 1.0 MB / 2 files.
  Excluded-file hover reports 3 MB total / 0 B selected. Escape, pointer leave
  and context-menu dismissal pass. Placement fix uses untransformed dimensions.
- [x] Final installer rebuilt and installed; installed executable SHA-256
  matches release: 96AA1FD313468DB2E99C3818469463889F9535F7BF8901CD2717223B4FFDB7F2.
  Final native WSL witness on StJohnsChurch/model shows 237.4 KB total,
  53.2 KB selected, 6 of 17 files, five ranked files and no footer. Card is
  inside the viewport and separated from the hovered row. Screenshot:
  target/bundle_inspection_witness/final_hover.png. Normal launch restored.
- Runtime note: the known post-install WSL clientHello timeout recurred;
  a full app restart recovered before the final witness. No startup fix is
  claimed. Existing issue: docs/known_issues.md, P2 Windows reinstall startup.

## Changed files

### Hover UI and shared glass styling

- `app/src/components/bundles/bundle_entry_hover.tsx`
- `app/src/components/bundles/bundle_explorer_directory.tsx`
- `app/src/components/bundles/bundle_explorer_file_row.tsx`
- `app/src/components/bundles/bundle_file_explorer.tsx`
- `app/src/components/bundles/bundle_hover_provider.tsx`
- `app/src/components/context_menu.tsx`
- `app/src/styles/bundle_entry_hover.css`
- `app/src/styles/context_menu.css`

### Protocol and request cancellation

- `app/src/lib/agent/agent_client.ts`
- `app/src/lib/agent/agent_client_inspection_test.ts`
- `app/src/lib/agent/agent_request_timeouts.ts`
- `app/src/shared/protocol.ts`
- `app/src/shared/protocol_bundle_inspection.ts`
- `crates/im_agent/src/protocol/bundle_inspection.rs`
- `crates/im_agent/src/protocol/commands.rs`
- `crates/im_agent/src/protocol/mod.rs`
- `crates/im_agent/src/protocol/responses.rs`
- `crates/im_agent/src/server/connection/request_cancellation.rs`
- `crates/im_host_agent/src/server/request_dispatch.rs`

### Metadata measurement and agent routing

- `crates/im_agent/src/bundles/bundle_inspection.rs`
- `crates/im_agent/src/bundles/mod.rs`
- `crates/im_agent/src/server/connection/dispatch.rs`
- `crates/im_agent/src/server/connection/repo_commands.rs`
- `crates/im_bundle/src/inspection.rs`
- `crates/im_bundle/src/inspection_tests.rs`
- `crates/im_bundle/src/lib.rs`
- `crates/im_host_agent/src/runtime/host_runtime/host_dispatch.rs`
- `crates/im_host_agent/src/runtime/host_runtime/mod.rs`
- `crates/im_host_agent/src/server/connection.rs`
- `crates/im_host_agent/src/server/dispatch.rs`
- `crates/im_host_agent/src/server/mod.rs`
- `crates/im_host_agent/src/wsl/wsl_backend_client/timeouts.rs`

### Documentation and generated outputs

- `docs/architecture/bundle_format_architecture.md`
- `docs/changelog.md`
- `docs/guide.md`
- `docs/implementation/bundle_inspection_implementation.md`
- `docs/inventory/file_ledger.json`
- `docs/inventory/file_ledger.md`
- `src-tauri/resources/agent_bundle/im_agent`
