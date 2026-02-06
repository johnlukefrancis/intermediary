## Goal in user-visible terms

**In scope**

* The app runs on macOS and behaves like Windows: add a repo, see file changes, drag files/bundles out, build bundles, open folders.
* Repo roots stay in their native world:

  * mac repos stay mac paths.
  * Windows repos stay Windows paths.
  * WSL repos stay WSL paths.
* The UI never needs to “pretend” a Windows repo is a WSL repo.

**Out of scope (for this pass)**

* Mac App Store sandbox correctness (security-scoped bookmarks, entitlements). You can ship unsigned/notarized first; sandboxing is a separate boss fight.
* Linux desktop support (but we’ll keep the architecture open to it).

This aligns with the architecture-first contract: fix the ownership model instead of patching symptoms. 

---

## Behavior table

| Situation                 | Input                                                                 | Expected visible behavior                                                                                                         |
| ------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Add repo on mac           | User picks `/Users/jl/code/myrepo`                                    | Repo appears immediately, file changes stream, no conversion step, no “WSL agent” drama.                                          |
| Add repo on Windows drive | User picks `C:\UnrealProjects\InfiniteAbyss`                          | Repo is handled as **host-native**, not converted to `/mnt/c/...`. The system keeps running.                                      |
| Add repo in WSL           | User picks `\\wsl.localhost\Ubuntu\home\jl\code\repo` (or equivalent) | Repo is stored/handled as **WSL-native**. Host agent routes it to WSL subagent (or whatever your design is), UI still just works. |
| Drag file / bundle on mac | Drag a row                                                            | Drag uses a **mac absolute path** to a staged file. No `windowsPath` fiction.                                                     |
| “Open folder”             | Click folder icon                                                     | Finder opens on mac; Explorer opens on Windows. No conversion gymnastics.                                                         |

---

## Invariants you need (non-negotiable, or you’ll relapse)

1. **Repo path authority is explicit**

   * Every repo config must declare whether its root is **host-native** or **WSL-native**.
   * No implicit conversion and no “store only WSL path and hope”. That’s the bug factory you’re escaping.

2. **UI speaks in “host paths” only**

   * Anything draggable must resolve to a host-OS path (Windows or mac) because the desktop drag system lives there.

3. **WSL paths are internal metadata**

   * Useful for bridging, staging, and debugging on Windows, but not a UI-level contract.

4. **Staging is always on the host filesystem**

   * On Windows, WSL writes into staging via `/mnt/<drive>/...`.
   * On mac, staging is just… a folder. Like nature intended.

5. **Protocol fields do not encode OS lies**

   * Stop calling it `windowsPath` if it’s going to be used on mac. That naming rot spreads everywhere.

These are the contract-level fixes ADR-007 is talking about. 

---

## The one correct design for mac support (and also fixing Windows properly)

### Design: Host-first agent contract + platform-neutral paths

**Core move:** The UI’s “agent” contract becomes **host-native**. WSL becomes an implementation detail behind that contract (Windows only). This is consistent with the “host app owns drag-out + staging” architecture described in the system overview, but removes the v0 mistake where Windows repos were forced through WSL.

### What changes

#### 1) Config schema: replace `wslPath` with a `repoRoot` union

Today your RepoConfig is basically “everything is WSL”. That’s why mac is currently impossible without lying.

**New shape (conceptual):**

* `repoRoot.kind: "host" | "wsl"`
* If `host`: `hostPath` (Windows path on Windows; POSIX path on mac)
* If `wsl`: `wslPath` (Linux path, like `/home/jl/code/repo`)

And while you’re here:

* Rename `outputWindowsRoot` → `outputHostRoot` (or similar). On mac it’s not Windows. Shocking, I know.

You’ll also need a migration because your persisted config currently stores repos as `{ wslPath }`. That migration is straightforward:

* If a stored path starts with `/mnt/<drive>/...`, migrate it to `host` + convert to `C:\...`.
* If it starts with `/home/...` or `/...` (but not `/mnt/<drive>`), keep it as `wsl`.
  That’s an architecture fix, not a patch.

This is TypeScript contract work, so follow ADR-005 (no type weakening). 

#### 2) Protocol: rename “windowsPath” to “hostPath”

Your protocol types currently hardcode Windows naming in the payloads (`windowsPath`, `stagingWinRoot`, `dragIconWindowsPath`). That’s not “Windows-first”, that’s “Windows-only”.

For mac support:

* `StagedInfo.windowsPath` → `StagedInfo.hostPath`
* `BundleBuiltEvent.windowsPath` → `hostPath`
* `ClientHelloCommand.stagingWinRoot` → `stagingHostRoot`
* `AppPaths.dragIconWindowsPath` → `dragIconHostPath`
* Keep `stagingWslRoot` only as an **optional** field, and only when running on Windows with WSL support.

This change touches:

* TS protocol (`app/src/shared/protocol.ts`)
* Rust agent protocol structs (`crates/im_agent/...`) and whatever `im_host_agent` uses

Yes, this is one of those “rename everything” refactors. It’s still the right move, because it eliminates an entire class of confusion bugs permanently.

#### 3) Tauri backend paths: make `AppPaths` platform-neutral

Right now `AppPaths` is explicitly Windows and does Windows validation and always computes WSL conversions. That won’t survive a mac build cleanly.

You need:

* `stagingHostRoot` (always)
* `logDir` (always)
* `dragIconHostPath` (always)
* `stagingWslRoot` (only on Windows, only if you actually need WSL bridging)

Also the output override validator must become OS-aware:

* Windows: allow only drive paths (like today)
* mac: allow absolute POSIX paths

This is Rust runtime boundary work, so ADR-008/009 apply.

#### 4) UI: remove WSL assumptions from “Add repo”, “Open folder”, and drag

Obvious but non-trivial because it’s wired everywhere:

* **Add repo** cannot always call `convert_windows_to_wsl`. On mac it’s nonsense. On Windows it’s only valid for UNC `\\wsl...` selections.
* **Open folder** cannot go `wslPath -> convert_wsl_to_windows -> explorer`. On mac it’s Finder, and for host repos it’s direct.
* **Drag** must drag `hostPath`, not `windowsPath`.

#### 5) Supervisor: mac can’t “start WSL”

Your current agent supervisor is explicitly “WSL agent supervisor”, including running `wsl.exe`. mac needs a host-agent supervisor.

Assuming `im_host_agent` exists, the mac work is:

* Supervisor launches `im_host_agent` sidecar on mac (and Windows too)
* WSL agent launching becomes either:

  * internal to `im_host_agent` (cleaner), or
  * kept in Tauri but only used on Windows (messier, and duplicates ownership)

Either way, mac does not touch WSL at all.

#### 6) “Open in file manager” must be cross-platform

`open_in_file_manager` is Windows-only right now. mac needs Finder open (typically via `open <path>`), but do it in a platform module.

---

## How big is the mac refactor, really?

Assuming `im_host_agent` is implemented and behaves correctly on Windows, mac support is:

### Big refactors (the “ass crack bastard” part)

1. **Config schema migration** from “WSL-only paths” to “path authority union”
2. **Protocol rename and normalization** from Windows-specific names to host-neutral names
3. **Tauri AppPaths rewrite** to platform-neutral paths and platform validation
4. **Supervisor split** into host-agent supervisor (all platforms) + optional WSL support (Windows only)

These are structural changes that touch a lot of files, but they’re mechanical and testable.

### Smaller mac-specific work

* Finder open implementation
* UI hiding/adjusting WSL-only controls (distro override, WSL banner wording, etc.)
* Packaging: include the mac `im_host_agent` binary in the bundle resources

### Biggest mac risk (not code, but reality)

macOS privacy and sandbox behavior can block filesystem watching/access unless the app is signed correctly or the user granted permissions. If you distribute normally (not App Store sandbox), you can usually operate on user-selected folders. Still: expect some “why won’t it watch Desktop” tickets.

---

## Rejected (noncompliant) approaches

1. **“Just convert mac paths into some fake Windows/WSL path”**

   * Violates the path-authority invariant.
   * Creates the same category of bug you’re fixing right now, but with different punctuation.

2. **“Keep `windowsPath` fields and just shove mac paths into them”**

   * Works until it doesn’t.
   * Bakes semantic lies into your protocol forever, guaranteeing future regressions when you add Linux or do any debugging.

ADR-007 says no. 

---

## Tradeoffs

* **Pro:** Once the protocol/config are host-neutral, mac becomes “just another host”. This is the exact leverage you want.
* **Pro:** Fixes Windows-drive repos correctly too, because both problems are the same root contract violation.
* **Con:** You will touch a wide swath of TS and Rust. It’s not a “small diff” situation.
* **Con:** You must write and test a config migration carefully or you’ll strand existing users’ repo lists.
* **Con:** mac distribution will raise permissions/signing questions sooner than Windows did.

---

## Agent prompt ladder (designed for the end state)

These are written assuming your working tree includes `im_host_agent` already, but they don’t depend on it being perfect. They focus on the host-neutral contract that mac needs.

### Prompt 1: Protocol + TS config refactor (path authority + hostPath naming)

branch: `jl/host_path_authority_protocol_ts`

```text
Task: Refactor the UI-side config + protocol to be host-OS neutral (support macOS) by introducing explicit repo path authority and replacing windowsPath naming with hostPath.
Context: Intermediary currently treats repos as WSL-only (RepoConfig.wslPath) and protocol payloads are Windows-named. macOS support requires host-native repo roots and host-native staged paths. Assume im_host_agent exists and will accept the updated protocol.
Refs: {@docs/system_overview.md, @docs/prd.md, @docs/compliance/adr_005_typescript_native_contracts_and_rails.md, @docs/compliance/adr_007_architecture_first_execution.md, @app/src/shared/config/repo_config.ts, @app/src/shared/config/persisted_config.ts, @app/src/shared/config/persisted_config_migrations.ts, @app/src/shared/protocol.ts, @app/src/hooks/use_drag.ts, @app/src/hooks/use_client_hello.ts, @app/src/components/add_repo_button.tsx, @app/src/components/tab_bar.tsx, @app/src/components/options_overlay.tsx, @app/src/types/app_paths.ts}
Deliver:
- Update RepoConfig to use an explicit repo root authority union (host vs wsl) instead of only wslPath.
- Add a persisted config migration that converts existing repos: /mnt/<drive>/... => host repo root (Windows path), native Linux paths => wsl repo root.
- Rename protocol fields in TS from windowsPath/stagingWinRoot/dragIconWindowsPath to hostPath/stagingHostRoot/dragIconHostPath; keep stagingWslRoot optional.
- Update UI codepaths that stage/drag/open folders to use hostPath semantics.
- Keep types tight; no type weakening. Update any call sites and helper types accordingly.
Constraints: Respect ADR-000 modularity; no band-aids per ADR-007; follow ADR-005 TypeScript rails; update docs only if needed and follow docs workflow; Skills: architecture-first, typescript-native-rails, docs-discipline, workflow-closeout.
```

### Prompt 2: Tauri backend paths become platform-neutral (mac-ready AppPaths)

branch: `jl/platform_neutral_app_paths`

```text
Task: Make src-tauri AppPaths + related commands platform-neutral so the app can run on macOS (host paths always, optional WSL staging paths only on Windows).
Context: Current AppPaths logic is Windows-specific (outputWindowsRoot validation, stagingWindowsRoot, stagingWslRoot always computed). macOS needs host-only paths and cannot depend on wsl.exe. UI will rely on hostPath naming and stagingHostRoot.
Refs: {@docs/compliance/adr_008_rust_runtime_contracts_and_error_handling.md, @docs/compliance/adr_009_rust_concurrency_and_io_boundary_rules.md, @docs/compliance/adr_007_architecture_first_execution.md, @src-tauri/src/lib/paths/app_paths.rs, @src-tauri/src/lib/paths/wsl_convert.rs, @src-tauri/src/lib/commands/paths.rs}
Deliver:
- Refactor AppPaths struct + resolver to expose stagingHostRoot + dragIconHostPath on all platforms.
- Make stagingWslRoot optional and only resolved on Windows builds (or only when needed).
- Replace outputWindowsRoot with outputHostRoot (platform-appropriate validation).
- Update Tauri commands in @src-tauri/src/lib/commands/paths.rs to match the new API.
- Ensure no panics/unwraps across command boundaries; errors are actionable strings.
Constraints: Respect ADR-000 modularity; no band-aids per ADR-007; Rust rails ADR-008/009; Skills: architecture-first, rust-runtime-rails, workflow-closeout.
```

### Prompt 3: Cross-platform “open folder” and remove WSL-only assumptions

branch: `jl/cross_platform_open_folder`

```text
Task: Make open_in_file_manager cross-platform (Windows Explorer + macOS Finder) and update frontend usage to stop depending on WSL-to-Windows conversion for host repos.
Context: Current open_in_file_manager is Windows-only and tab bar assumes repos are WSL paths requiring conversion. With path authority, host repos open directly; WSL repos open via appropriate host-visible path if available.
Refs: {@docs/compliance/adr_005_typescript_native_contracts_and_rails.md, @src-tauri/src/lib/commands/file_manager.rs, @app/src/components/tab_bar.tsx, @app/src/components/options_overlay.tsx}
Deliver:
- Implement cross-platform open_in_file_manager behavior (Windows + macOS).
- Update the UI so “open folder” uses host repo root directly when repo is host-authority; avoid WSL conversion unless the repo is actually WSL-authority and needs it.
Constraints: Respect ADR-000 modularity; no band-aids per ADR-007; Skills: typescript-native-rails, rust-runtime-rails, workflow-closeout.
```

### Prompt 4: Supervisor refactor for mac (spawn host agent, WSL optional)

branch: `jl/host_agent_supervisor_macos`

```text
Task: Refactor the agent supervisor to support macOS by supervising the host agent as the primary daemon; keep WSL launching as Windows-only and/or delegated to the host agent.
Context: Current supervisor explicitly launches wsl.exe and installs a WSL agent bundle. macOS needs a host-agent supervisor and cannot launch WSL. Assume im_host_agent exists and should be the UI-facing WebSocket server.
Refs: {@docs/system_overview.md, @docs/compliance/adr_007_architecture_first_execution.md, @docs/compliance/adr_008_rust_runtime_contracts_and_error_handling.md, @docs/compliance/adr_009_rust_concurrency_and_io_boundary_rules.md, @src-tauri/src/lib/agent/supervisor.rs, @src-tauri/src/lib/agent/install.rs, @src-tauri/src/lib/commands/agent_control.rs, @app/src/hooks/agent/use_agent_supervisor.ts, @app/src/components/options/agent_section.tsx}
Deliver:
- Introduce a platform-neutral supervisor contract and macOS implementation that spawns the host agent.
- Gate WSL-specific fields (like distro override) behind Windows-only logic in both backend and UI.
- Update any resources/installation expectations so mac builds can bundle the host agent correctly.
Constraints: Respect ADR-000 modularity; no band-aids per ADR-007; Rust rails ADR-008/009; Skills: architecture-first, rust-runtime-rails, typescript-native-rails, docs-discipline, workflow-closeout.
```

---

## Bottom line

Mac support is not “extra work”. It’s the accountability audit for whether your new path authority model is real.

If you do the host-neutral protocol + config + AppPaths refactor properly, mac becomes:

* “spawn host agent”
* “watch posix paths”
* “drag staged host paths”
* “open Finder”

Which is exactly what software is supposed to feel like before humans get involved.
