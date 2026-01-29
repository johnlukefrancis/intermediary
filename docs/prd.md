# PRD + Implementation Spec: **Intermediary**

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
* Repos may live in **WSL Linux filesystem** or Windows filesystem.
* Needs frequent repeated “context snapshots” for LLM collaboration.

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

* **Top tab bar:** one tab per repo (“triangle rain”, “textureportal”, “blankstop”…).
* **Three columns per tab:**

  1. **Docs** (recently changed doc-like files)
  2. **Code** (recently changed code-like files)
  3. **Zip bundles** (bundle presets + recently built outputs)

### File item row

* Filename + relative path
* Last modified time (relative: “2m ago”)
* Size (optional)
* Status badge: `staged` / `source-only` / `building...` / `error`

### Drag interaction

* Each row has a **drag handle** region.
* Dragging the row begins an OS-level drag containing the staged file path.
* For WSL sources, the app ensures the file is copied to staging first.

### Bundle interaction

* Each preset has a **Build** button.
* Built bundles appear in “Zip bundles” list, top-sorted by newest.
* Each built bundle row is draggable.

### Visual style

* Dark mode by default.
* Glassmorphic panel styling, rounded corners, subtle borders, neon accent per tab.
* No UI clutter: the app is a staging deck, not a file explorer.

---

## 7. Functional requirements

### 7.1 Repository configuration

Each repo has:

* `repoId`, `label`
* `location`:

  * `windows`: absolute Windows path
  * `wsl`: distro + Linux path
* Classification rules:

  * `docsGlobs` (e.g. `docs/**`, `**/*.md`, `**/*.mdx`)
  * `codeGlobs` (e.g. `src/**`, `packages/**`)
  * `ignoreGlobs` (e.g. `**/node_modules/**`, `**/.git/**`, `**/dist/**`)

### 7.2 File change tracking

* Maintain a per-repo in-memory index of recent file changes:

  * Store last N (configurable, default 200)
  * Debounce rapid consecutive writes (default 250ms)
* Show lists filtered into Docs/Code columns by globs.

### 7.3 Staging system

* All draggables originate from a **staging directory** on the Windows filesystem:

  * Example: `%LOCALAPPDATA%\Intermediary\staging\<repoId>\...`
* Staging rules:

  * When a file changes, optionally auto-stage (configurable).
  * On drag start, ensure staged copy exists and is up-to-date.
  * Use atomic write: copy to temp name then rename to final.

### 7.4 Zip bundle presets

Per repo, user can define multiple presets:

* Preset name, description
* Include globs
* Exclude globs
* Output naming template
* Output destination: staging bundles folder

### 7.5 Bundle provenance manifest

Every generated zip includes:

* `INTERMEDIARY_MANIFEST.json` containing:

  * repoId / repo path
  * timestamp
  * git short SHA (if repo is git)
  * dirty status + list of changed files (optional)
  * include/exclude patterns used
  * app version

### 7.6 Naming scheme

Bundles should be self-identifying:

* `{repoId}_{presetId}_{gitShort}_{YYYY-MM-DD_HH-mm-ss}.zip`
* Additionally maintain a stable alias:

  * `{repoId}_{presetId}_LATEST.zip` (overwritten)

### 7.7 Error handling

* If WSL agent not reachable: show banner “WSL agent offline” + degrade to manual refresh/polling (if enabled).
* If staging copy fails: show per-item error and log.
* If bundle build fails: show build error output (truncate) and keep last good build.

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

* Watch repos (inotify)
* Provide “recent changes” feed
* Build zip bundles to staging (via `/mnt/c/...`)
* Stage individual files on request

Implementation options:

* Rust binary using `notify` (Linux backend) + zip crate
* Node script using chokidar + archiver (if you want fastest iteration)

#### C) IPC between UI and agent

* Local WebSocket (`127.0.0.1:<port>`) or named pipe.
* JSON messages.

### 9.3 Message protocol (suggested)

Agent → UI:

* `hello { agentVersion, distro, reposDetected? }`
* `fileChanged { repoId, path, kind, mtime }`
* `snapshot { repoId, recent: FileEntry[] }`
* `bundleBuilt { repoId, presetId, windowsPath, size, mtime, gitShort }`
* `error { scope, message, details? }`

UI → Agent:

* `watchRepo { repoId }`
* `refresh { repoId }`
* `stageFile { repoId, path } -> { windowsPath }`
* `buildBundle { repoId, presetId } -> { windowsPath }`

### 9.4 Staging path translation

* Agent writes staged outputs to `/mnt/c/Users/<you>/AppData/Local/Intermediary/staging/...`
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
* Stable LATEST zip alias
* Cleanup/retention

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

## 13. Open decisions

* Where do your repos actually live today (WSL Linux FS vs `/mnt/c/...`)? This impacts whether the agent is required.
* Preferred bundle contents defaults per repo (e.g., always include `README`, `package.json`, `src/**`, `docs/**`?).
* Do you want auto-stage on change (more disk churn, less drag latency) or stage-on-drag (less churn, small latency)?

---

If you build this cleanly, it’s a genuinely good portfolio piece because it’s not another Todo App cosplay. It’s a **workflow tool with real OS integration**, and it solves a problem that only exists because modern dev is 40% building things and 60% shuttling context between other things.

[1]: https://github.com/microsoft/WSL/issues/7674 "`ReadDirectoryChangesW` method is unsupported on `\\\wsl$` paths · Issue #7674 · microsoft/WSL · GitHub"
[2]: https://superuser.com/questions/1808946/file-explorer-does-not-automatically-refresh-changes-when-connected-to-wsl-file "https://superuser.com/questions/1808946/file-explorer-does-not-automatically-refresh-changes-when-connected-to-wsl-file"
[3]: https://electronjs.org/docs/latest/tutorial/native-file-drag-drop "Native File Drag & Drop | Electron"
[4]: https://github.com/tauri-apps/tauri/issues/6664 "[feat] Support for dragging files from Tauri window to filesystem · Issue #6664 · tauri-apps/tauri · GitHub"
[5]: https://github.com/gsidhu/tauri-drag "GitHub - gsidhu/tauri-drag: Draggable for GUI apps on Windows and Mac"
[6]: https://docs.rs/crate/tauri-plugin-drag/0.2.0/source/src/lib.rs "https://docs.rs/crate/tauri-plugin-drag/0.2.0/source/src/lib.rs"
