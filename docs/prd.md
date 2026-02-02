# PRD + Implementation Spec: **Intermediary**
Updated on: 2026-02-02
Owners: JL · Agents
Depends on: ADR-000, ADR-006, ADR-007

## 1. Product overview

**Product name:** Intermediary
**Platform:** Windows 10/11 (macOS optional later)
**Problem:** High-friction file/context handoff between local repos (often in WSL) and ChatGPT/browser-based workflows.
**Outcome:** A single-window “handoff console” that surfaces recently changed files and generates standardized zip bundles that can be dragged directly into ChatGPT (or anywhere).

---

## 2. Goals

### Primary goals

1. **Zero-Explorer workflow** for the “share context with ChatGPT” loop.
2. **One-click bundle generation** with configurable excludes/includes.
3. **Reliable ‘latest’ semantics** (no accidental stale bundles, no manual renaming).
4. **Fast access to relevant docs/code** via a “recently changed” feed.

### Success metrics

* Time from “agent made changes” → “user shares correct bundle/docs with ChatGPT” reduced to < 30 seconds.
* < 1% incidence of “wrong bundle/version uploaded” in normal usage.
* Bundle creation consistency: every bundle includes a manifest with provenance.

---

## 3. Non-goals

* Full file manager replacement (no directory browsing UI beyond what’s needed).
* Git operations UI (commit/push/merge not required).
* Direct ChatGPT API integration (drag-and-drop to browser is the target).
* Cloud sync / multi-device.

---

## 4. Target user

* Solo developer using agentic coding workflow.
* **v0:** Repos may be in the WSL Linux filesystem or on Windows drives. Windows/UNC paths are converted to WSL `/mnt/<drive>/...` paths for the agent.
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
* **Three columns per tab:**

  1. **Docs** (recently changed doc-like files)
  2. **Code** (recently changed code-like files)
  3. **Zip bundles** (bundle presets + recently built outputs)

### File item row

* Filename + relative path
* Last modified time (relative: "2m ago")
* Size (optional)
* Status badge: `staged` / `source-only` / `building...` / `error`

### File row interactions

* **Click row** → copies `@<repo-relative-path>` to clipboard (forward slashes)
* **Drag handle** → stages file, starts OS drag, also copies `@<path>` to clipboard
* **Star button** → toggles starred status (does not copy or drag)

### Drag interaction

* Each row has a **drag handle** region.
* Dragging the row begins an OS-level drag containing the staged file path.
* For WSL sources, the app ensures the file is copied to staging first.

### Bundle interaction

* Each preset has a **Build** button.
* Built bundles appear as the latest bundle row for the preset (single row).
* The built bundle row is draggable.

### Visual style

* Dark mode by default.
* Glassmorphic panel styling, rounded corners, subtle borders, neon accent per tab.
* No UI clutter: the app is a staging deck, not a file explorer.

### Pane views (Recent / Starred)

Each Docs and Code pane has two views:

* **Recent** (default): Shows recently changed files
* **Starred**: Shows user-starred files

Header controls:

* `[DOCS]`/`[CODE]` title is a button → returns to Recent view
* Star icon button on right → switches to Starred view (disabled when empty)

If starred count becomes zero, pane auto-switches back to Recent.

---

## 7. Functional requirements

### 7.1 Repository configuration

Users add and remove repositories via the UI:

* **Add repository**: Click the "+" button in the tab bar to open a directory picker. The selected Windows/UNC path is converted to a WSL path for the agent.
* **Remove repository**: Click the "×" button on a tab (or in a group dropdown), confirm via modal. Removes the repo and its bundle selections.
* **Empty state**: When no repos are configured, a centered prompt with "Add Repository" button is shown.

Each repo has:

* `repoId`, `label` (auto-generated from folder name, editable)
* `wslPath`: WSL paths (Windows/UNC selections are converted to WSL `/mnt/<drive>/...` for the agent)
* Classification rules:

  * `docsGlobs` (e.g. `docs/**`, `**/*.md`, `**/*.mdx`)
  * `codeGlobs` (e.g. `src/**`, `packages/**`)
  * `ignoreGlobs` (e.g. `**/node_modules/**`, `**/.git/**`, `**/dist/**`, `**/target/**`)

### 7.2 File change tracking

* Maintain a per-repo in-memory index of recent file changes:

  * Store last N (configurable via Options, default 200, range 25-2000)
  * Debounce rapid consecutive writes (default 250ms)
* Show lists filtered into Docs/Code columns by globs.
* Persist recent-file history under `staging/state/recent_files/<repoId>.json` to survive app/agent restarts.

### 7.2.1 Starred files

* Users can star/unstar any file in Docs or Code columns
* Starred files persist in config per repo: `{ docs: string[], code: string[] }`
* Starred files that aren't in the recent list show "—" for modification time
* Dragging a starred file stages and drags normally (staging uses current file state)

### 7.3 Staging system

* All draggables originate from a **staging directory** on the Windows filesystem:

  * **Default root:** `%LOCALAPPDATA%\Intermediary\staging`
  * **Custom root:** Users can set an `outputWindowsRoot` override in config. When set, staging uses that path as the root.
  * Files: `staging\files\<repoId>\...`
  * Bundles: `staging\bundles\<repoId>\<presetId>\...`
* Staging rules:

  * **Auto-stage on change is the default behavior** (reduces drag latency at cost of disk churn).
  * `autoStage` is a boolean option (global default + per-repo override) to disable auto-staging.
  * When `autoStage` is off, **stage-on-drag** is the fallback: on drag start, ensure staged copy exists and is up-to-date.
  * Use atomic write: copy to temp name then rename to final.

### 7.4 Zip bundle presets

Per repo, user can define multiple presets:

* Preset name, description
* **v0 selection UI:** top-level folders only (no nested subfolder selection). User toggles which top-level folders to include.
* **Include root files toggle:** single boolean, default ON. When ON, includes files at repo root (README, package.json, etc.).
* Always-ignored directories: `node_modules`, `.git`, `dist`, `build`, `target`, `.next`, `.cache`, `logs`, `.turbo`, `.vercel`, `__pycache__`, `.mypy_cache`, `.pytest_cache`, `coverage`
* Always-ignored files: `.DS_Store`, `Thumbs.db`, `.env`, `.env.local`
* Advanced include/exclude globs: later enhancement
* Output naming template
* Output destination: staging bundles folder

### 7.5 Bundle provenance manifest

Every generated zip includes:

* `BUNDLE_MANIFEST.json` containing:

  * `generatedAt` (ISO timestamp)
  * `repoId`, `repoRoot`
  * `presetId`, `presetName`
  * `selection` (includeRoot + topLevelDirsIncluded)
  * `git` info (headSha/shortSha/branch, best-effort)
  * `fileCount`, `totalBytesBestEffort`

### 7.6 Naming scheme

Bundles should be self-identifying:

* `{repoId}_{presetId}_{YYYYMMDD_HHMMSS}_{gitShort?}.zip`
* Only the most recent bundle per repo + preset is retained (older bundles deleted before write).

### 7.7 Error handling

* If WSL agent not reachable: show banner “WSL agent offline” + degrade to manual refresh/polling (if enabled).
* If staging copy fails: show per-item error and log.
* If bundle build fails: show build error output (truncate) and keep last good build.
* Reconnects may re-run `clientHello`; the agent treats the handshake as idempotent and safe to call multiple times.

---

## 8. Non-functional requirements

* **Performance:** must handle large repos by honoring ignore globs.
* **Reliability:** must not depend on Windows watching `\\wsl$` directly (known unsupported in some cases). ([GitHub][1])
* **Security:** no telemetry by default; repo access restricted to configured roots.
* **Offline:** works without network.

---

## 9. Technical architecture

### 9.1 Why a WSL agent exists

Windows-side filesystem watchers (like `ReadDirectoryChangesW`) are not reliable/available for WSL UNC paths `\\wsl$...`. ([GitHub][1])
So:

* Watch inside WSL using inotify (reliable for Linux FS).
* Communicate changes to the Windows UI.

### 9.2 Components

#### A) Windows UI app (Tauri recommended)

* Frontend: React/TS (or Svelte if you prefer)
* Backend: Rust commands (config, staging ops, IPC glue)
* Native drag-out:

  * Use `tauri-plugin-drag` / `drag-rs` style plugin: provides `start_drag` accepting absolute file paths. ([Docs.rs][6])

> Note: Tauri core historically didn’t prioritize drag-out natively. ([GitHub][4])
> This is why you validate early.

#### B) WSL agent (daemon)

Responsibilities:

* Watch repos (chokidar/inotify)
* Provide “recent changes” feed
* Build zip bundles to staging (via `/mnt/c/...`)
* Stage individual files on request

Implementation: Node.js + TypeScript using chokidar for watch and Rust `im_bundle_cli` for bundle creation.

#### C) IPC between UI and agent

* Local WebSocket (`127.0.0.1:<port>`) or named pipe.
* JSON messages.

### 9.3 Message protocol (current)

Requests use `{ kind: "request", requestId, payload }` and responses use `{ kind: "response", requestId, status, payload|error }`.

Agent → UI events:

* `fileChanged { repoId, path, kind, changeType, mtime, staged? }`
* `snapshot { repoId, recent: FileEntry[] }`
* `bundleBuilt { repoId, presetId, windowsPath, aliasWindowsPath, bytes, fileCount, builtAtIso }`
* `error { scope, message, details? }`

UI → Agent commands:

* `clientHello { config, stagingWslRoot, stagingWinRoot, autoStageOnChange? } -> clientHelloResult`
* `setOptions { autoStageOnChange? } -> setOptionsResult`
* `watchRepo { repoId } -> watchRepoResult`
* `refresh { repoId } -> refreshResult`
* `stageFile { repoId, path } -> stageFileResult`
* `buildBundle { repoId, presetId, selection } -> buildBundleResult`
* `getRepoTopLevel { repoId } -> getRepoTopLevelResult`
* `listBundles { repoId, presetId } -> listBundlesResult`

### 9.4 Staging path translation

* Agent writes staged outputs to `/mnt/<drive>/Users/<you>/AppData/Local/Intermediary/staging/...`
* UI references the same file via Windows path:

  * `C:\Users\<you>\AppData\Local\Intermediary\staging\...`

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
* Recent changes list for docs + code (from WSL agent)
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
* Single latest bundle per preset (delete prior bundles before write)

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

* **Repo location:** Repos may be in WSL Linux FS or on Windows drives (user adds via directory picker). The WSL agent is required.
* **Initial repo set:** Empty by default. Users add repos via the "+" button in the tab bar.
* **Grouped repos:** Repos with matching `groupId` share a tab with a dropdown. Useful for worktrees of the same project.
* **Bundle selection UI:** Top-level folders only + "include root files" toggle (default ON). No nested subfolder selection in v0.
* **Staging strategy:** Auto-stage on change is default ON. Boolean toggle (global + per-repo) to disable; stage-on-drag is fallback when off.

---

If you build this cleanly, it’s a genuinely good portfolio piece because it’s not another Todo App cosplay. It’s a **workflow tool with real OS integration**, and it solves a problem that only exists because modern dev is 40% building things and 60% shuttling context between other things.

[1]: https://github.com/microsoft/WSL/issues/7674 "`ReadDirectoryChangesW` method is unsupported on `\\\wsl$` paths · Issue #7674 · microsoft/WSL · GitHub"
[2]: https://superuser.com/questions/1808946/file-explorer-does-not-automatically-refresh-changes-when-connected-to-wsl-file "https://superuser.com/questions/1808946/file-explorer-does-not-automatically-refresh-changes-when-connected-to-wsl-file"
[3]: https://electronjs.org/docs/latest/tutorial/native-file-drag-drop "Native File Drag & Drop | Electron"
[4]: https://github.com/tauri-apps/tauri/issues/6664 "[feat] Support for dragging files from Tauri window to filesystem · Issue #6664 · tauri-apps/tauri · GitHub"
[5]: https://github.com/gsidhu/tauri-drag "GitHub - gsidhu/tauri-drag: Draggable for GUI apps on Windows and Mac"
[6]: https://docs.rs/crate/tauri-plugin-drag/0.2.0/source/src/lib.rs "https://docs.rs/crate/tauri-plugin-drag/0.2.0/source/src/lib.rs"
