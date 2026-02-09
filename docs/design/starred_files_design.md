# Starred Files Design
Updated on: 2026-02-04
Owners: JL · Agents
Depends on: ADR-000, ADR-007

---

## Goal in visible behavior terms

* **Star/unstar** any Docs/Code file, per repo, persisted.
* **Docs/Code pane header becomes a 2-page switch**:

  * `[DOCS]` / `[CODE]` button = “Recent changes” page
  * **Star icon** = “Starred” page (disabled/grey when empty)
* **Right-click a file row** = context menu includes **Copy Relative Path** (`<repo-relative-path>`)
* **Drag a file** (via a handle/row drag surface) = stages file + starts OS drag (no clipboard copy)
* **Configurable cap** (default stays 200), exposed in Options.

Also: you *already* have a cap today. It’s just hardcoded twice:

* Agent MRU capacity is hardcoded to **200** in `crates/im_agent/src/repos/repo_watcher.rs`
* UI slices to **200** in `app/src/hooks/use_repo_state.ts`

So the “2000 files lag” nightmare is currently prevented… accidentally. We’ll make it intentional and configurable.

## Behavior table

| Situation        | Input                                  | Expected behavior                                                       |
| ---------------- | -------------------------------------- | ----------------------------------------------------------------------- |
| Copy path fast   | Right-click row → Copy Relative Path   | Clipboard becomes `docs/.../file.md` (repo-relative, plain path)         |
| Share file       | Drag via drag-handle                   | File is staged, OS drag starts                                            |
| Pin a file       | Click star on a row                    | File is added/removed from that repo’s starred list (docs/code)         |
| Switch lists     | Click `[DOCS]` / `[CODE]`              | Pane shows “recent changes” list                                        |
| Switch to pinned | Click Star icon in header              | Pane shows “starred files” list (only if count > 0)                     |
| Limit growth     | Change “Recent files limit” in Options | Agent stores + UI shows up to N items (default 200)                     |

## Invariants (don’t break these)

1. `Copy Relative Path` writes exactly the **repo-relative path** (forward slashes, no prefix).
2. Starred lists are **unique** per repo+kind, stable order (most-recently-starred first).
3. If starred count is 0 for that pane, header star button is **disabled** and pane cannot stay on “starred”.
4. Recent files limit is **one value** used by both agent MRU storage and UI slicing.
5. Left-click/drag interactions must **not** write to clipboard.

## One design (the end state)

### Data + persistence

* Add to persisted config:

  * `recentFilesLimit: number` (default 200, range say 25–2000)
  * `starredFiles: Record<repoId, { docs: string[]; code: string[] }>`
* `recentFilesLimit` also goes into `AppConfig` so it’s sent to the agent in `clientHello`.
* Agent uses it as MRU capacity and includes it in the watcher fingerprint so changing it resets watchers correctly.

### UI interactions

* File rows become:

  * **Right-click row** → context menu actions, including **Copy Relative Path**
  * **Drag handle** → stage + start drag
  * **Star button** → toggle starred (no copy, no drag)
* Docs/Code headers become:

  * Title is a **button** returning to Recent view.
  * Star icon on right toggles Starred view and is disabled when empty.

### Performance

* Since the agent itself won’t keep more than `recentFilesLimit`, you can’t “accidentally” render 5000 rows unless you explicitly choose to.

## Rejected (noncompliant / annoying) approaches

* **Make the entire row start drag on mouse down** while leaving copy behavior implicit
  Conflicts with explicit context-menu copy actions and causes accidental drags.
* **Keep MRU capacity hardcoded and only cap UI rendering**
  Looks fine until disk/state grows and the agent is still churning through huge lists. You want one knob.

## Tradeoffs

* Changing `recentFilesLimit` will likely trigger a watcher reset (by design). You might see the list briefly refresh.
* Adding drag handles is a minor UX change: you’ll drag from the handle instead of “anywhere”.
* Starred items that aren’t in the recent list may not have meaningful “time ago” until they change again. They’ll still stage the latest file on drag, which is the part you actually care about.

---

## (PR ladder)

## 1) Config v11 + starred persistence + recentFilesLimit wiring (TS + Rust + agent)

feat/config-v11-starred-and-recent-files-limit

```txt
Task
- Implement persisted config v11 fields:
  - recentFilesLimit (default 200)
  - starredFiles (per repo: docs[], code[])
- Wire recentFilesLimit through AppConfig -> agent watcher MRU capacity.
- Refactor ConfigProvider actions to keep files small (ADR-000), and add:
  - setRecentFilesLimit(number)
  - toggleStarredFile(repoId, kind, path)

Context
- PRD expects “store last N (configurable, default 200)” but code currently hardcodes 200 in both agent + UI.
- We need starred files persisted in the same config file (Rust load/save), but starred UI rendering comes in later PRs.
- ADR-000: app/src/hooks/use_config.tsx is already near the soft limit; don’t bloat it further.

Refs
- @docs/prd.md
- @docs/compliance/adr_000_modular_file_discipline.md
- @docs/compliance/adr_005_typescript_native_contracts_and_rails.md
- @docs/compliance/adr_007_architecture_first_execution.md
- @app/src/shared/config/version.ts
- @app/src/shared/config/app_config.ts
- @app/src/shared/config/persisted_config.ts
- @app/src/shared/config/persisted_config_migrations.ts
- @app/src/hooks/use_agent.tsx
- @app/src/hooks/use_repo_state.ts
- @app/src/hooks/use_config.tsx
- @crates/im_agent/src/runtime/config_fingerprint.rs
- @crates/im_agent/src/runtime/state.rs
- @crates/im_agent/src/repos/repo_watcher.rs
- @src-tauri/src/lib/config/types.rs
- @src-tauri/src/lib/config/io.rs
- @src-tauri/src/lib/commands/config.rs

Deliver
- Bump config version to 11 (TS + Rust).
- Add recentFilesLimit:
  - TS: include in AppConfigSchema + DEFAULT_APP_CONFIG + extractAppConfig
  - TS: include in PersistedConfigSchema + defaults
  - UI: use recentFilesLimit instead of hardcoded 200 in @app/src/hooks/use_repo_state.ts
  - Agent: MRU capacity uses recentFilesLimit instead of MRU_CAPACITY constant.
  - Agent: include recentFilesLimit in config fingerprint so changing it resets watchers.
- Add starredFiles to persisted config (TS + Rust) with default empty map:
  - Structure: { [repoId]: { docs: string[], code: string[] } }
- Refactor config actions:
  - Create a new hook module (e.g. @app/src/hooks/use_config_actions.ts) and move the action callbacks out of @app/src/hooks/use_config.tsx.
  - Add actions:
    - setRecentFilesLimit(value): clamps to schema range before saving.
    - toggleStarredFile(repoId, kind, path): toggles unique membership, keeps MRU order; remove repo key if both lists empty.
    - removeRepo must also clean up starredFiles[repoId].
- Keep type contracts strict (no type weakening).
- Run checks: pnpm lint, pnpm typecheck, cargo check.
- If you add new files, run headers + regenerate file ledger per @docs/commands/workflow/closeout_checks.md.

Constraints
- Skills: architecture-first, typescript-native-rails, rust-runtime-rails, workflow-closeout
- Follow ADR-000 (small modules), ADR-005 (no type weakening), ADR-007 (end-state design).
```

## 2) File rows: click copies `@path`, drag-handle drags, star toggle button

feat/file-row-copy-atpath-drag-handle-star-toggle

```txt
Task
- Update file rows so:
  - Click row copies @<repo-relative-path> to clipboard
  - Drag handle stages + starts drag AND copies @path to clipboard
  - Star button toggles starred state without triggering copy/drag

Context
- Bundle row already copies helpful text on drag; we’re extending the same “clipboard assist” idea to file rows.
- We need to avoid UX conflict: “click to copy” vs “mousedown starts drag”.
- PRD mentions a drag handle region for rows, so implement that now.

Refs
- @docs/prd.md
- @docs/compliance/adr_000_modular_file_discipline.md
- @docs/compliance/adr_005_typescript_native_contracts_and_rails.md
- @app/src/components/file_row.tsx
- @app/src/components/file_list_column.tsx
- @app/src/styles/file_row.css
- @app/src/hooks/use_config.tsx (for new actions added in prior PR)
- (Create) @app/src/hooks/use_starred_files.ts

Deliver
- Add @app/src/hooks/use_starred_files.ts:
  - Given repoId, expose:
    - starredDocsPaths, starredCodePaths
    - isStarred(kind, path)
    - toggle(kind, path) (calls ConfigContext.toggleStarredFile)
- Update @app/src/components/file_list_column.tsx to:
  - Use useStarredFiles(repoId) and pass per-row isStarred + toggle callback into FileRow.
- Update @app/src/components/file_row.tsx:
  - Add a drag handle button region (only this button starts drag).
  - Row click copies clipboard text: `@${file.path}`.
  - Drag handle onMouseDown copies clipboard `@${file.path}` (best-effort) then triggers existing onDragStart(file).
  - Add a star button on the right: shows ☆ / ★; clicking toggles; must stopPropagation so it does not copy or drag.
  - Update relative time formatter to show "—" if mtime is invalid/empty (for later starred list placeholders).
  - Update title tooltips to match behavior (e.g. “Click to copy @path” / “Drag to share (also copies @path)”).
- Update @app/src/styles/file_row.css:
  - New grid columns for: marker | drag-handle | info | time | star-button
  - Style buttons to match existing theme tokens (no ugly default button chrome).
- Run pnpm lint + pnpm typecheck.

Constraints
- Skills: typescript-native-rails, architecture-first, workflow-closeout
- Do not change staging behavior semantics; only change UI interactions.
- Keep modules small (ADR-000).
```

## 3) Docs/Code pane headers: button titles + header star icon toggles “Starred” page

feat/panes-starred-page-toggle

```txt
Task
- Make Docs and Code panes each have two pages:
  - Recent changes page (default)
  - Starred files page (enabled only when starred count > 0)
- Pane headers:
  - [DOCS] / [CODE] titles become buttons that switch back to Recent page
  - Star icon button switches to Starred page and is greyed/disabled when empty

Context
- This is the “two pages per pane” behavior.
- Starred entries are persisted paths; if an entry isn’t in the recent list, display can be minimal (time can be "—"), but drag must still work.

Refs
- @docs/compliance/adr_005_typescript_native_contracts_and_rails.md
- @app/src/components/layout/three_column.tsx
- @app/src/tabs/repo_tab.tsx
- @app/src/styles/panels.css
- @app/src/components/file_list_column.tsx
- @app/src/hooks/use_starred_files.ts

Deliver
- Update @app/src/components/layout/three_column.tsx:
  - Add optional header slots for docs + code:
    - docsHeaderLeft/docsHeaderRight, codeHeaderLeft/codeHeaderRight
  - Default behavior unchanged if not provided.
- Update @app/src/styles/panels.css:
  - Add styles for a header icon button (star) and for a title button nested in the panel-title.
  - Disabled star icon should look “greyed out” and not clickable.
- Update @app/src/tabs/repo_tab.tsx:
  - Maintain independent state for docs and code panes: "recent" | "starred".
  - Use useStarredFiles(repoId) to get counts + lists.
  - If currently on "starred" and list becomes empty, auto-switch back to "recent".
  - Build starred FileEntry[] from starred paths:
    - If a path exists in recent list, reuse that entry.
    - Otherwise create a placeholder FileEntry with mtime="" and changeType="change" (FileRow will show time as "—").
  - Pass headerLeft/headerRight into ThreeColumn for docs+code:
    - Title button selects recent view
    - Star icon selects starred view (disabled when empty, highlighted when active)
  - Switch the FileListColumn content + emptyMessage based on view.
- Run pnpm lint + pnpm typecheck.

Constraints
- Skills: typescript-native-rails, architecture-first, workflow-closeout
- Keep the UI change localized; no protocol changes here.
```

## 4) Options setting for Recent Files limit + update PRD

feat/options-recent-files-limit-setting-and-prd

```txt
Task
- Add an Options setting to control recentFilesLimit (how many recent files are stored/shown).
- Update PRD to document:
  - starring files + starred pane view
  - click-to-copy @relative path
  - configurable recent files limit (default 200)

Context
- The underlying config + agent support is already implemented in prior PRs.
- This PR exposes the knob and documents the behavior.

Refs
- @app/src/components/status_bar.tsx
- @app/src/components/options_overlay.tsx
- @app/src/styles/options_overlay.css
- @docs/prd.md

Deliver
- Update StatusBar to pass down:
  - config.recentFilesLimit
  - setRecentFilesLimit from useConfig()
- Update OptionsOverlay:
  - Add a “Recent files limit” number input (min/max consistent with schema).
  - On change, call setRecentFilesLimit with parsed int (ignore NaN).
  - Add a short hint: higher values may impact UI performance.
- Style the number input in @app/src/styles/options_overlay.css so it matches existing chrome (border, background, focus-visible).
- Update @docs/prd.md:
  - Document starring behavior and the two-page panes.
  - Document click-to-copy and drag-handle behavior.
  - Document recentFilesLimit setting and default 200.
- Run pnpm lint + pnpm typecheck.
- If any files were added/removed in earlier PRs and ledger isn’t updated yet, run the closeout header/ledger steps.

Constraints
- Skills: typescript-native-rails, docs-discipline, workflow-closeout
- Follow docs workflow conventions in @docs/environment/docs_workflow.md.
```

That’s the implementation plan. It gives you **pinning**, **instant `@path` copying**, and a **real performance knob**, without turning your file rows into a misclick carnival.
