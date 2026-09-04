# PRD + Implementation Spec: **Intermediary**
Updated on: 2026-09-04
Owners: JL · Agents
Depends on: ADR-000, ADR-006, ADR-007

## 1. Product overview

**Product name:** Intermediary
**Platform:** Maintainer-validated runtime is Windows 10/11. WSL2 is the recommended path for the full WSL-backed workflow, while host-native Windows repo workflows are also validated. The architecture includes host-native paths for other platforms, but macOS and Linux are not yet validated to the same standard.
**Problem:** High-friction context handoff between local repos and ChatGPT/browser-based workflows, especially when users need both trustworthy full-repo bundles and fast access to the latest changed files or screenshots.
**Outcome:** A single-window “handoff console” that surfaces recently changed files, stages drag-and-drop-safe handoff copies, and generates standardized zip bundles with reliable latest-bundle semantics.

---

## 2. Goals

### Primary goals

1. **Zero-Explorer workflow** for the “share context with ChatGPT” loop.
2. **One-click bundle generation** with configurable excludes/includes.
3. **Reliable ‘latest’ semantics** (no accidental stale bundles, no manual renaming).
4. **Fast access to relevant docs/code/images** via a unified Auto Files panel.

### Success metrics

* Time from “agent made changes” → “user shares correct bundle/files with ChatGPT” reduced to < 30 seconds.
* < 1% incidence of “wrong bundle/version uploaded” in normal usage.
* Bundle creation consistency: every bundle includes a manifest with provenance.

---

## 3. Non-goals

* Full file manager replacement (no directory browsing UI beyond what’s needed). Inbound drag-in import and organizing files from the Zip bundles tree (select, cut/copy/paste, delete to quarantine, drag-move, rename) are in scope (§6); editing file content is not.
* A full Git client: branch management, merge/rebase tooling, history, blame, and hunk-level staging are out of scope. Source Control covers status, stage/unstage, commit, discard, per-file diff, push, and pull.
* Direct ChatGPT API integration (drag-and-drop to browser is the target).
* Cloud sync / multi-device.

---

## 4. Target user

* Solo developer using agentic coding workflow.
* **v0:** Repos may be in the WSL Linux filesystem or on host-native drives. Repo roots are persisted with explicit authority as `{ kind: "wsl" | "host", path }`, and backend routing follows root kind (host backend, WSL backend agent).
* Needs frequent repeated "context snapshots" for LLM collaboration.

---

## 5. Core user stories

1. **Recent change drag-out**

   * “I changed a doc/code file. I want to drag it into ChatGPT without hunting for it.”

2. **Bundle build + drag-out**

   * “I want to click ‘Build Context Bundle’ for a repo and immediately drag the zip into ChatGPT.”

3. **Trust the bundle**

   * “I want the bundle name and contents to prove it’s the latest, and what commit it corresponds to.”

4. **Reduce diligence tax**

   * “I don’t want to remember which docs to include. The app should show what changed and offer sane default bundles.”

---

## 6. UX spec

### Layout (single window)

* **Top tab bar:** one tab per repo. Repos with matching `groupId` and `groupLabel` are shown as a single tab with a dropdown switcher (useful for worktrees). A "+" button adds new repos.
* **Two main columns per tab:**

  1. **Auto Files** (docs, code, and images ranked by auto, latest, or activity mode)
  2. **Rail** with a segmented icon rocker (archive-box / git-branch / console glyphs) between **Zip bundles** (bundle presets + recently built outputs, with Git-status decorations on changed files/directories in the explorer tree), **Source Control** (working-tree status and commit controls), and **Terminal** (PowerShell 7 sessions per repo). The active rail persists globally; the SOURCE cell carries the change count beside its glyph. A drag divider between the two columns sets the rail's share of the deck width (20-70 %, default 35 %, double-click resets), persisted as `uiState.railWidthPercent`; all three rail sections share it.

### Responsive mode behavior

* The user-selected `uiMode` (`standard` or `handset`) is a preferred baseline, not a hard lock.
* Runtime layout can auto-switch while resizing:
  * Enter `standard` at `>= 980px`
  * Return to `handset` at `<= 860px`
* Maximized windows always render `standard`.
* Hysteresis is required to avoid mode flapping around the breakpoint band.

### File item row

* Rank, filename, relative path, and file-kind icon
* Quiet activity dot meter, trend marker, last active time, update count, and 24-hour pulse strip
* Size (optional)
* Status badge: `staged` / `source-only` / `building...` / `error`

### File row interactions

* **Drag row** → stages file / starts OS drag behavior (no clipboard copy)
* **Double-click row** → opens supported files in the in-app workspace: UTF-8 text files as scratch buffers and common image files as fit-to-panel previews
* **Right-click row** → opens context menu with:
  * Open Containing Folder
  * Open File
  * Copy Relative Path

### Drag interaction

* Each row has a drag surface for initiating OS-level file drag.
* Dragging the row begins an OS-level drag containing the staged file path.
* For WSL sources, the app ensures the file is copied to staging first.

### Drag-in import

External files can be dropped onto the Zip bundles explorer tree to copy them into the repository. This is the one deliberate write path into a repo; general file management stays out of scope.

* Dragging one or more OS files or folders over the tree highlights the destination: a directory row (or anything inside its subtree, including its child rows and the gaps between them) targets that directory; a top-level file row or the blank space below the tree targets the repo root.
* Holding the drag over a collapsed directory for about 700 ms expands it, so nested destinations are reachable without leaving the drag; the tree never collapses during a drag, and the list auto-scrolls near its edges.
* Dropping copies each file to `<directory>/<name>`; a folder is copied recursively (symlinks skipped, bounded at 10,000 entries). The copy runs in the agent that owns the repo root under the same per-repo mutation lock as Source Control, so it never interleaves with a commit or discard. For WSL roots the agent translates the dropped Windows paths (`C:\…` → `/mnt/c/…`; `\\wsl$\<distro>\…` → `/…` for the running distro only).
* Existing files are never overwritten silently: the drop is refused with nothing written, a confirm modal lists the conflicting file paths, and Replace overwrites them atomically. A dropped folder merges into an existing folder of the same name, so its conflicts are the files that collide. Two dropped items resolving to the same destination are refused the same way, and a file landing on an existing folder (or a folder on a file) is refused outright.
* Nothing is staged in Git. Imported files appear as untracked (or modified) in SOURCE, in Auto Files, and in the tree with their badge, driven by the ordinary watcher events. Failures show an inline notice in the ZIPS column; a partial failure reports how many files landed and the watcher reconciles the tree.
* The app's own drag-out payload re-entering the window is ignored, so dragging a staged file out across the tree can never import it.

### Tree selection and worktree actions

The Zip bundles explorer tree is also where files inside the repo are organized. Every action below runs in the agent that owns the repo root, under the same per-repo mutation lock as Source Control, through one `worktreeAction` command; Auto Files rows stay handoff-only.

* **Selection.** Clicking a row selects it (accent selection box); Ctrl-click toggles, Shift-click ranges in visible order. A plain click on a folder selects it and toggles its expansion; the checkbox is the only bundle-inclusion toggle for folders (its name no longer toggles inclusion). File double-click still opens. Right-clicking an unselected row selects it first; actions apply to the whole selection.
* **Keyboard** (tree focused): Up/Down move the selection (Shift extends), Right expands, Left collapses or moves to the parent, Enter opens a file or toggles a folder, Delete, F2 (rename), Ctrl+X / Ctrl+C / Ctrl+V, Escape (cancel rename, else clear selection).
* **Cut, copy, paste.** Paste targets the selected folder, else the selected file's folder, else the root; right-click Paste targets that row's folder, and blank tree space offers Paste into the root. Cut becomes a move (rows dim until pasted); copy becomes a copy and keeps the clipboard.
* **Delete** asks for confirmation, then moves the entries (files or whole folders) into the repository's discard quarantine, where they are kept until the next agent start, so a wrong delete is recoverable by hand like a wrong discard. A tracked file shows as `D` in SOURCE.
* **Move** is a drag within the tree: drag a row (or the selection) onto a folder row or the root with the same highlight and hover-to-expand as the OS drop, or cut and paste. A file landing on an existing file is refused and confirmed through the Replace modal; a folder landing on an existing folder is refused outright (move never merges or destroys a folder the user did not name); a folder cannot be moved into itself. Moves are filesystem renames, so a tracked file shows as `D` plus untracked until both sides are staged, when SOURCE shows `R`.
* **Rename** is inline (F2 or the context menu): Enter or focus loss commits, Escape cancels; an existing name is refused, never replaced; a case-only rename works on case-insensitive volumes.
* Nothing is staged in Git by any of these actions. `.git` is never shown in the tree and can never be a source or destination at any depth (a dropped folder containing one is refused whole). Replace authorizes exactly the files the modal listed: the request carries that list, any collision that appeared since is refused again with a fresh list, and every unauthorized write uses a non-replacing filesystem primitive. Switching repos discards pending confirmations and in-flight results. The tree re-lists the affected folders itself, so ignored files disappear or move without waiting for a watcher event. Contract: `docs/design/zips_tree_write_surface_design.md`.

### Bundle interaction

* Each preset has a **Build** button.
* Built bundles appear as the latest bundle row for the preset (single row).
* The built bundle row is draggable.
* The built bundle row includes a right-side location/download button that reveals the generated ZIP in the host file manager.

### Visual style

* Dark mode by default.
* Glassmorphic panel styling, rounded corners, subtle borders, neon accent per tab.
* No UI clutter: the app is a staging deck, not a file explorer.

### Auto Files filters and ranking

Auto Files is one unified table, not separate Docs and Code or Latest and Active lanes.

* **Auto** is the default mode and blends recency, update frequency, burst activity, and rising-file momentum.
* **Latest** sorts by newest observed activity.
* **Active** sorts by persisted activity strength.
* The table defaults to **All** and exposes icon filters for documents, code, and images.
* Activity metadata is agent-owned and persists with recent-file history so ranking and row telemetry survive app and agent restarts.
* Rows show quiet activity dots, trend arrows, last active time, update count, and a 24-hour pulse strip.

Legacy starred-file config may still exist in older user configs, but the current UI no longer exposes favourites.

### Source Control

The SOURCE rail shows the active repo/worktree as Git sees it and lets the working tree be managed without VS Code.

* Status line: branch (or detached sha), ahead/behind when an upstream exists, short HEAD sha, and refresh / pull / push controls.
* Commit box: message textarea (Ctrl+Enter commits) and a COMMIT button that is disabled until something is staged and a message is present.
* Sections `MERGE CONFLICTS [n]` first (error tone, only when unmerged paths exist; the rail cell, a banner row, and a COMMIT gate flag the same state), `STAGED CHANGES [n]` (with a − unstage-all icon), and `CHANGES [n]` (with a + stage-all icon); rows show the file icon, name over directory, a `[M] [A] [D] [R] [C] [T] [U] [!]` badge, and a hover stage/unstage action. Deleted rows cannot be opened (Open File, Open Containing Folder, and Open Diff are all disabled) — except a deleted image, whose previous version still opens as a one-sided image diff. MERGE CONFLICTS has no section-wide stage — conflicts resolve per row, by design.
* SOURCE's count is distinct changed paths across the three lists plus staged-outside-root paths, not a sum of section rows, so a modified-and-staged file counts once; when the visible lists are empty but outside-root paths exist, the body reads "NO CHANGES IN THIS FOLDER" rather than a hidden zero. The conflict count the rail alert and banner show is unmerged rows plus unmerged paths above the configured root, because Git refuses the whole-index commit for either.
* Row double-click opens the diff in the shared workspace (staged diff for STAGED rows, worktree diff for CHANGES rows, untracked files as all-added; a changed image opens as a side-by-side image diff). Right-click offers Stage/Unstage, Open Diff, Open File, Open Containing Folder, Copy Relative Path, and Discard Changes (confirm modal). A copy row's actions target its destination path only; a rename row's actions target both endpoints, and the discard confirm lists every target path with what happens to it (restored or deleted).
* Warnings surface staged paths outside the configured root (commit then asks for confirmation, including cross-root renames) and truncated status output (STAGE ALL and COMMIT disabled).
* Commit and discard are bound to the snapshot the user reviewed. One snapshot identity covers the branch a commit would move, where it points, the tree it would record, and any merge, cherry-pick, or revert it would conclude, so a commit sent against a repository that has moved since the review is refused rather than silently retargeted; a discard sent against a target whose on-disk file changed, reappeared after being missing, or cannot be identified at all is refused rather than overwriting it. Either way the UI shows "STATE CHANGED — REVIEW AGAIN" and refreshes status automatically. Once Git publishes a commit it stands: if a commit hook rewrote reviewed files or added files nobody reviewed, the result says which, and the unreviewed case gets a warning notice pointing at a soft reset — the app never rewinds a published commit. A discarded file's bytes are kept in the repository's quarantine directory until the next agent start, so a discard the user regrets is still recoverable by hand.
* Refresh is event-driven: the agent watcher emits `sourceControlChanged` for `.git` metadata writes (including linked worktrees' real git dir) and working-tree changes outside the repo's structural ignore globs — a tracked file under those globs still refreshes SOURCE — coalesced agent-side; the UI also refetches on window focus and after every action. No interval polling.
* Git runs inside the agent that owns the repo root (Windows host agent for host roots, WSL agent for WSL roots). Mutations are serialized per repo (by the physical git dir) and are never killed mid-command; WSL-routed reads are cancellable, host in-process reads are bounded by their Git timeout but are not cancellable. Closing the app drains an in-flight mutation to completion — the agent does not exit while a mutation is still reported active, only at a 450 s emergency bound, and on Windows the whole Git process tree (hooks, credential helpers included) is owned and terminated together rather than left behind.

### Terminal

The TERMINAL rail hosts JL's own PowerShell 7 — profile, environment, and aliases loaded — inside the deck, so `claude`, `codex`, `wsl`, and `wb-code` run without leaving the app. Contract: `docs/design/terminal_design.md`; shipped system: `docs/architecture/terminal_architecture.md`.

* One session group per repo/worktree, shown when that tab is active. The first TERMINAL visit for a repo opens one tab; `+` opens more. Twelve retained terminal tabs is the product bound, including exited and failed tabs; each live tab is a real ConPTY-backed pwsh (`-NoLogo`, never `-NoProfile`).
* Host-rooted repos start in their folder. A native WSL root is first proved by a bounded non-login control probe in one resolved, explicit distro; pwsh then starts at the user profile directory and immediately runs `wsl.exe -d <distro> --cd '<repo>'`, like the `wb-code` alias, so the tab lands in bash inside the repo and `exit` returns to pwsh. A missing path fails before ConPTY spawn, and a later entry failure exits pwsh instead of leaving the tab in the wrong shell. A WSL root on a `/mnt/<drive>` path starts in its Windows folder instead.
* Sessions survive rail, repo-tab, handset, and mode switches: a switch parks the session off-screen and never disposes it. Nothing persists across an app restart except which rail is active; TERMINAL re-opens one fresh tab on the next visit.
* Keys follow Windows Terminal: Ctrl+Shift+C / Ctrl+Shift+V copy and paste, Ctrl+C copies a selection (else interrupts), Ctrl+V pastes, right-click copies the selection or pastes, Shift+Enter sends a newline for Claude Code; everything else reaches the program. Paste reads the clipboard through a Tauri command because WebView2 blocks a page-side clipboard read.
* An exited tab keeps its scrollback and offers RESTART / CLOSE; a failed open names the reason and offers RETRY / CLOSE. Closing a tab ends the console-attached tree (pwsh, `wsl.exe`, conhost) as Windows Terminal does — console close first, a Job Object only as the bounded escalation — while GUI apps launched from the shell survive. Removing a repo or rebinding the same id to another root closes its old terminals.
* The Rust registry owns each admitted transaction through opening, running, closing, reaping, and a joined terminal receipt. App exit freezes admission and waits every transaction before the agent supervisor runs, so an in-app `wsl` session never keeps the WSL distro alive at exit (ADR-013 rule 4). Output uses cumulative sent/consumed flow watermarks so a failed or duplicate acknowledgement remains recoverable; explicit close detaches the webview sink before privately draining ConPTY.
* The terminal is a Tauri IPC surface owned by the Tauri process; neither agent, no shell or clipboard plugin, and no CSP or capability change is involved (ADR-010 clause 7). Colours derive from the deck tokens and the tab accent; the font is the deck's mono font.

### Workspace previews

Auto Files rows can open a minimal workspace for supported text and image files. In standard layout, the workspace replaces the Auto Files panel while Zip bundles remain visible. In handset layout, the workspace replaces the deck content until closed.

* Text file workspace buffers are scratch-only: typing is allowed, but there is no save action and no source-file write-back.
* Opening another file or closing the workspace discards scratch edits.
* Text workspaces use a muted grey editor surface with live Markdown semantics for notes and Markdown-like doc files, while code and other text files remain plain scratch buffers.
* Text workspaces show live line and character counts in the bottom-right corner.
* Opened text-file workspace titles are handoff surfaces: dragging the title stages and starts native drag-out, and right-clicking the title exposes the same single-file actions as Auto Files rows.
* Image workspaces support PNG, JPEG, WebP, GIF, BMP, and AVIF previews. Images are rendered from agent-provided preview bytes and can be dragged from the preview surface using the same staged-file drag path as file rows.
* Unsupported files, binary files, invalid UTF-8 text files, oversized text files, and oversized image files do not open as editable text or image previews.

---

## 7. Functional requirements

### 7.1 Repository configuration

Users add and remove repositories via the UI:

* **Add repository**: Click the "+" button in the tab bar to open a directory picker. The selected path is resolved into a path-native `root` (`wsl` or `windows`) and stored without cross-conversion.
* **Remove repository**: Click the "×" button on a tab (or in a group dropdown), confirm via modal. Removes the repo and its bundle selections.
* **Empty state**: When no repos are configured, a centered prompt with "Add Repository" button is shown.

Each repo has:

* `repoId`, `label` (auto-generated from folder name, editable)
* `root`: `{ kind: "wsl" | "host", path }` (WSL paths stay WSL, host-native paths stay host-native)
* Classification rules:

  * `docsGlobs` (e.g. `docs/**`, `**/*.md`, `**/*.mdx`)
* `codeGlobs` (generated broad-language defaults + optional per-repo customization)
  * `ignoreGlobs` (e.g. `**/node_modules/**`, `**/.git/**`, `**/dist/**`, `**/target/**`)

### 7.2 File change tracking

* Maintain a per-repo in-memory index of recent file changes:

  * Store last N (configurable via Options, default 200, range 25-2000)
  * Debounce rapid consecutive writes (default 250ms)
* Show one Auto Files table with Auto/Latest/Active sort modes and All/Documents/Code/Images filters.
* Classify image extensions as a first-class `image` file kind before docs/code globs are applied.
* Persist recent-file history under `staging/state/recent_files/<repoId>.json` to survive app/agent restarts.
* Persist activity metadata with recent-file history: first seen, last seen, update count, current burst count, and 24-hour bucket history.
* Global **classification excludes** (Options) suppress noisy/generated files from Auto Files without affecting bundle contents.

### 7.2.1 Auto Files ranking

* Auto ranking combines capped update frequency, recency decay, burst count, 24-hour bucket activity, and a rising boost for files first seen within the recent activity window and updated repeatedly.
* The ranking is deterministic from agent-provided metadata; the frontend does not persist its own ranking state.
* Deleted files are removed from the recent index and no longer appear in Auto Files.

### 7.3 Staging system

* All draggables originate from a **staging directory** on the host filesystem:

  * **Default root:** `<app_local_data>/staging` (resolved by Tauri per platform)

    | Platform | Default app local data |
    |----------|----------------------|
    | Windows  | `%LOCALAPPDATA%\Intermediary` |
    | macOS    | `~/Library/Application Support/Intermediary` |
    | Linux    | `~/.local/share/intermediary` (or `$XDG_DATA_HOME`) |

  * **Custom root:** Users can set an `outputWindowsRoot` override in config. When set, staging uses that path as the root. (Name is a legacy holdover; accepts any host-native absolute path.)
  * **WSL mirror root (Windows only):** When running on Windows, a WSL-equivalent path (`/mnt/<drive>/...`) is derived automatically for WSL backend access.
  * Files: `staging/files/<repoId>/...`
  * Bundles: `staging/bundles/<repoId>/<presetId>/...`
* Staging rules:

  * **Auto-stage on change is the default behavior** (reduces drag latency at cost of disk churn).
  * `autoStage` is a boolean option (global default + per-repo override) to disable auto-staging.
  * When `autoStage` is off, **stage-on-drag** is the fallback: on drag start, ensure staged copy exists and is up-to-date.
  * Use atomic write: copy to temp name then rename to final.

### 7.4 Zip bundle presets

Per repo, user can define multiple presets:

* Preset name, description
* **Selection UI:** top-level folders plus nested subdirectory exclusions up to repo depth 4. Users toggle top-level folders to include and untick nested folders to exclude from the zip. Re-including a nested folder that matches a recommended directory-name exclude records an exact positive inclusion, so legitimate source paths such as `src/target` remain selected without including other build-output folders named `target`.
* **Include root files toggle:** single boolean, default ON. When ON, includes files at repo root (README, package.json, etc.).
* Recommended global excludes seed new or omitted bundle config: `node_modules`, `.git`, `dist`, `build`, `target`, `.next`, `.cache`, `logs`, `.turbo`, `__pycache__`, `.mypy_cache`, `.pytest_cache`, `coverage`, common cache dirs, generated artifacts, binary/model-weight extensions, and local env/cache files.
* Explicit user-configured `globalExcludes` are authoritative after normalization; recommended entries are not hidden mandatory filters.
* Advanced include/exclude globs: later enhancement
* Output naming template
* Output destination: staging bundles folder

### 7.5 Bundle provenance manifest

Every generated zip uses bundle format version 3 and includes:

* `BUNDLE_MANIFEST.json` containing:

  * `bundleFormatVersion`
  * `generatedAt` (ISO timestamp)
  * `repoId`, `repoRoot`
  * `presetId`, `presetName`
  * `selection` (includeRoot + topLevelDirsIncluded + includedSubdirs + excludedSubdirs + excludedFiles)
  * `effectiveGlobalExcludes` used by the scan
  * versioned `git` capture evidence: captured HEAD/branch/time, comparison base, candidate index tree id, capture status, repo/selection dirty facts, patch deletion mode, selected/omitted counts, generated artifact names, incomplete artifacts, and structured failure/drift reasons
  * `fileCount`, `totalBytesBestEffort`
* `BUNDLE_GIT_STATUS.txt` containing selected staging/worktree status, diff stat/name-status, untracked-file orientation, and privacy-safe omitted-change evidence
* `BUNDLE_GIT_DIFF.patch` containing the selected tracked final-state delta from captured HEAD without binary payload encoding
* `BUNDLE_HANDOFF.md` containing the project-neutral read order and repo-local operator guidance

The selected file set is the privacy boundary for Git paths and content. Clean repositories emit explicit clean evidence and an empty patch. Git absence/failure preserves normal bundle creation with unavailable/partial evidence, while HEAD/status/selected-byte movement produces an unstable verdict.

### 7.6 Naming scheme

Bundles should be self-identifying:

* `{repoId}_{presetId}_{YYYYMMDD_HHMMSS}_{gitShort?}.zip`
* Bundle replace semantics are **atomic replace, not delete-before-write**: build to a temp file, atomically rename to the final name, then prune older bundles for that repo+preset.
* Only the most recent successful bundle per repo + preset is retained.

### 7.7 Error handling

* If WSL backend is not reachable: show banner/error event for WSL operations, while Windows-root repos continue to function.
* If staging copy fails: show per-item error and log.
* If bundle build fails (including timeout): show build error output (truncate), do not replace the current bundle, and keep the last good build.
* Reconnects may re-run `clientHello`; the agent treats the handshake as idempotent and safe to call multiple times.
* File-row context menu actions validate repo-relative input before any OS launch; invalid/traversal paths must fail with an explicit error.

### 7.8 Agent lifecycle

* The app auto-starts the host agent on launch by default (UI endpoint on `agentPort`).
* The app starts the WSL backend agent only when any configured repo root has `kind: "wsl"` (on `agentPort + 1`).
* Auto-start can be disabled in Options.
* Optional WSL distro override is supported for agent launch.
* Users can manually restart the agent from Options.
* On OS resume (sleep/wake), the UI triggers automatic recovery: it reconnects the WebSocket session and rehydrates repo/bundle state. Users may briefly see reconnect status, and stale transport errors should clear once backend status returns online.

---

## 8. Non-functional requirements

* **Performance:** must handle large repos by honoring ignore globs.
* **Reliability:** must not depend on Windows watching `\\wsl$` directly (known unsupported in some cases). ([GitHub][1])
* **Security:** no telemetry by default; repo access restricted to configured roots.
* **Offline:** works without network.

---

## 9. Technical architecture

### 9.1 Why backend split exists

Windows-side filesystem watchers (like `ReadDirectoryChangesW`) are not reliable/available for WSL UNC paths `\\wsl$...`. ([GitHub][1])
WSL-side inotify is also not the correct watcher surface for Windows drive mounts (`/mnt/c/...`) in this product shape.
So:

* Route host roots to a host-native backend.
* Route WSL roots to a WSL backend agent using inotify.
* Keep a single host-agent endpoint for UI stability.

### 9.2 Components

#### A) UI app (Tauri)

* Frontend: React/TS (or Svelte if you prefer)
* Backend: Rust commands (config, staging ops, IPC glue)
* Native drag-out:

  * Use `tauri-plugin-drag` / `drag-rs` style plugin: provides `start_drag` accepting absolute file paths. ([Docs.rs][6])

> Note: Tauri core historically didn’t prioritize drag-out natively. ([GitHub][4])
> This is why you validate early.

#### B) Host agent (daemon)

Responsibilities:

* Expose one WebSocket endpoint to the UI
* Route per-repo operations by root kind (`windows` vs `wsl`)
* Execute Windows-root operations locally (watch/refresh/stage/build/list/top-level)
* Forward WSL-root operations to the WSL backend agent
* Merge forwarded events into the UI event stream

Implementation: Rust host agent using Tokio WebSocket server/client, local backend ops, and routing map.

#### C) WSL backend agent (daemon)

Responsibilities:

* Watch WSL-root repos (notify/inotify)
* Provide WSL recent changes feed
* Build/stage/list/top-level for WSL roots
* Stream events and responses back to host agent

Implementation: Rust WSL backend agent using notify for watch and the `im_bundle` library for bundle creation.

#### D) IPC between UI and host/backend

* UI ↔ Host: local WebSocket (`127.0.0.1:<hostPort>`)
* Host ↔ WSL backend: internal local WebSocket (`127.0.0.1:<hostPort+1>` configurable)
* JSON messages using existing request/response/event envelopes.

### 9.3 Message protocol (current)

Requests use `{ kind: "request", requestId, payload }` and responses use `{ kind: "response", requestId, status, payload|error }`.

Host agent → UI events:

* `fileChanged { repoId, path, kind, changeType, mtime, activity?, staged? }`
* `snapshot { repoId, recent: FileEntry[] }`
* `repoTopologyChanged { repoId }`
* `bundleBuilt { repoId, presetId, hostPath, aliasHostPath, bytes, fileCount, builtAtIso }`
* `sourceControlChanged { repoId }`
* `error { scope, message, details? }`

UI → Host agent commands:

* `clientHello { config, stagingHostRoot, stagingWslRoot?, autoStageOnChange? } -> clientHelloResult`
* `setOptions { autoStageOnChange? } -> setOptionsResult`
* `watchRepo { repoId } -> watchRepoResult`
* `refresh { repoId } -> refreshResult`
* `stageFile { repoId, path } -> stageFileResult`
* `readTextFile { repoId, path } -> readTextFileResult`
* `readImageFile { repoId, path } -> readImageFileResult`
* `buildBundle { repoId, presetId, buildId, selection } -> buildBundleResult`
* `cancelBundleBuild { repoId, presetId, buildId } -> cancelBundleBuildResult`
* `getRepoTopLevel { repoId } -> getRepoTopLevelResult`
* `listBundles { repoId, presetId } -> listBundlesResult`
* `importFiles { repoId, directory, sources, onConflict } -> importFilesResult` (`onConflict`: `"refuse"` | `{ replace: [paths] }`; refused conflicts return `ENTRY_CONFLICT` with the conflicting paths, and a replace authorizes only those paths)
* `worktreeAction { repoId, action } -> worktreeActionResult` (action kinds: delete, move, copy, rename; conflicts return `ENTRY_CONFLICT`, cross-kind collisions `ENTRY_KIND_MISMATCH`)
* `sourceControlStatus { repoId } -> sourceControlStatusResult`
* `sourceControlDiff { repoId, path, originalPath?, area } -> sourceControlDiffResult`
* `sourceControlAction { repoId, action } -> sourceControlActionResult` (action kinds: stage, unstage, discard, commit, push, pull)

Integrated terminal — **Tauri IPC commands in the Tauri process, not agent protocol** (no WebSocket, no agent):

* `terminal_open { sessionId, repoRoot, cols, rows }` + output `Channel` -> `TerminalOpened { sessionId, pid, windowsBuildNumber, startDir, initialCommand }`; the channel carries raw pty bytes and, after the last byte, one `{ kind: "exit", sessionId, code, reason }` frame
* `terminal_write` (raw request body = bytes; session id in the `tauri-terminal-session` header)
* `terminal_resize { sessionId, cols, rows }`
* `terminal_ack { sessionId, consumedTotal }` (idempotent cumulative flow-control watermark)
* `terminal_close { sessionId }` -> `{ outcome: "exited" | "escalated" | "stillAlive", code? }`
* `terminal_clipboard_text` -> string (clipboard read in Rust for paste)

### 9.4 Staging path translation

* Host agent and WSL backend both write staged outputs under the same app staging roots.
* On Windows, the WSL backend writes via the WSL mount of the host staging root:
  * WSL path: `/mnt/<drive>/Users/<you>/AppData/Local/Intermediary/staging/...`
  * Host path: `C:\Users\<you>\AppData\Local\Intermediary\staging\...`
* On macOS and Linux, there is no WSL backend; all staging is host-native.

---

## 10. Drag-out implementation notes

### Electron baseline (fallback plan)

Electron supports dragging files out using `webContents.startDrag(item)` in response to a drag start event. ([Electron][3])
If Tauri drag-out proves flaky in your environment, Electron is the pragmatic fallback.

### Tauri plan

Use `tauri-plugin-drag`-style command to start native drag with absolute file paths (and an icon). The plugin code explicitly expects absolute paths for dragged files. ([Docs.rs][6])

---

## 11. MVP scope

### Must-have

* 1 repo tab (hardcoded or simple config import)
* Recent changes list for docs + code (via host agent routing)
* Manual “Build Bundle” for one preset
* Staging directory writes
* Drag-out of:

  * staged doc file
  * staged code file
  * built zip bundle

### Should-have

* Multi-repo tabs
* Config UI editor (or edit JSON and reload)
* Bundle manifest injection
* Single latest bundle per preset (keep last good on failures; prune older bundles only after successful build finalize + rename)

### Nice-to-have

* System tray mode
* Global hotkey “Build + focus app”
* “Save clipboard as report.md to repo” (captures your ChatGPT output into your workflow automatically)

---

## 12. Technical spikes (to de-risk the hard parts)

1. **Drag-out spike**

   * Build a Tauri window with a single draggable list item.
   * On drag start: create a temp file in staging and drag it into:

     * Desktop
     * Explorer folder
     * Browser upload zone (ChatGPT)
   * Pass/fail decides Tauri vs Electron.

2. **WSL watcher spike**

   * WSL daemon emits file events for a test directory.
   * UI renders “recent changes” reliably.

---

## 13. Decisions (locked)

The following assumptions are locked for v0:

* **Repo location:** Repos may be in WSL Linux FS or on Windows drives (user adds via directory picker). Host agent is always required; WSL backend is required only when at least one WSL root exists.
* **Initial repo set:** Empty by default. Users add repos via the "+" button in the tab bar.
* **Grouped repos:** Repos with matching `groupId` share a tab with a dropdown. Useful for worktrees of the same project.
* **Bundle selection UI:** Top-level folders + nested subdirectory exclusions up to repo depth 4 + "include root files" toggle (default ON).
* **Staging strategy:** Auto-stage on change is default ON. Boolean toggle (global + per-repo) to disable; stage-on-drag is fallback when off.

---

If you build this cleanly, it’s a genuinely good portfolio piece because it’s not another Todo App cosplay. It’s a **workflow tool with real OS integration**, and it solves a problem that only exists because modern dev is 40% building things and 60% shuttling context between other things.

[1]: https://github.com/microsoft/WSL/issues/7674 "`ReadDirectoryChangesW` method is unsupported on `\\\wsl$` paths · Issue #7674 · microsoft/WSL · GitHub"
[2]: https://superuser.com/questions/1808946/file-explorer-does-not-automatically-refresh-changes-when-connected-to-wsl-file "https://superuser.com/questions/1808946/file-explorer-does-not-automatically-refresh-changes-when-connected-to-wsl-file"
[3]: https://electronjs.org/docs/latest/tutorial/native-file-drag-drop "Native File Drag & Drop | Electron"
[4]: https://github.com/tauri-apps/tauri/issues/6664 "[feat] Support for dragging files from Tauri window to filesystem · Issue #6664 · tauri-apps/tauri · GitHub"
[5]: https://github.com/gsidhu/tauri-drag "GitHub - gsidhu/tauri-drag: Draggable for GUI apps on Windows and Mac"
[6]: https://docs.rs/crate/tauri-plugin-drag/0.2.0/source/src/lib.rs "https://docs.rs/crate/tauri-plugin-drag/0.2.0/source/src/lib.rs"
