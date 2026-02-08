Below is a static review of the **current codebase state** from `intermediary_context_20260208_030535_a7637d7.zip`, focused on the requested areas. I’m ordering findings by severity and including **exact file + line references**.

---

## P0 Findings

### P0.1 — Forwarded WSL requests can hang indefinitely, potentially wedging the host agent

**Why this matters:** If the WSL backend accepts a WebSocket connection but then becomes unresponsive (or a bug causes it to stop replying), the host agent can wait forever on the pending response. Because host-agent dispatch is serialized via a runtime write lock, **one stuck forwarded request can effectively stall the host agent** (UI will time out client-side, but the backend remains wedged until restart).

**Evidence**

* Host runtime forwards WSL repo commands without any timeout:

  * `crates/im_host_agent/src/runtime/host_runtime.rs`:

    * `forward_wsl_command()` awaits `wsl_client.forward_command(..)` with no timeout. Lines **239–257**.
    * Called from `dispatch_repo_command()` for all WSL-routed repo commands (e.g., `stageFile`, `refresh`, `buildBundle`, etc.). Lines **149–200**.
* WSL backend client has no per-request timeout:

  * `crates/im_host_agent/src/wsl/wsl_backend_client.rs`:

    * `forward_command()` awaits `response_rx.await` indefinitely. Lines **55–84**.

**Risk / Failure mode**

* UI request timeouts (30s in the web client) can occur while the host agent is still waiting, leading to:

  * repeat timeouts,
  * inability to recover without restarting the agent/app,
  * potential pending-map growth if the backend never responds.

**Architectural contract impact**

* This is in tension with ADR-009’s emphasis on explicit cancellation/bounded lifecycles (unbounded await on an external component). See `docs/compliance/adr_009_rust_concurrency_and_io_boundary_rules.md` sections on explicit cancellation/timeouts and bounded lifetimes (esp. I9.3/I9.4).

**Recommended fix**

* Implement **server-side timeouts** for forwarded WSL commands. Options (in increasing rigor):

  1. Wrap `response_rx.await` in `tokio::time::timeout` inside `WslBackendClient::forward_command` and return a structured error on timeout.
  2. Additionally remove/cleanup pending entries on timeout (to prevent leaks).
  3. Add circuit-breaker behavior: after N timeouts, drop the WSL connection and fail pending requests immediately.

---

## P1 Findings

### P1.1 — WSL bootstrap can succeed “locally” while silently failing to initialize WSL backend config

**Why this matters:** `clientHello` is the bootstrap that seeds repo config and starts watchers (directly or indirectly). The host runtime currently returns the local host-backend `clientHelloResult` even if forwarding to WSL fails, and only emits an event. That makes startup appear “successful” from the request/response perspective while WSL repos may remain non-functional until a later `clientHello` happens again.

**Evidence**

* Host runtime treats WSL forwarding failure as non-fatal for `clientHello`:

  * `crates/im_host_agent/src/runtime/host_runtime.rs`:

    * `handle_client_hello()` returns `Ok(local_response)` even if `try_forward_client_hello()` fails; it emits an error event instead. Lines **115–136**.
* `try_forward_client_hello()` has a timeout (good), but there is no “retry on later readiness” mechanism:

  * `crates/im_host_agent/src/runtime/host_runtime.rs` lines **259–330** (timeout + emit).

**Concrete scenario**

* On Windows with WSL repos:

  * UI connects as soon as host agent is up.
  * `clientHello` is sent before WSL backend is ready (or backend is temporarily unreachable).
  * Forwarding fails → UI gets a *successful* `clientHelloResult` from the host backend, but WSL backend never receives config.
  * Later WSL backend comes up; WSL repo commands may fail with “unknown repo” until a new `clientHello` is sent.

**Recommended fix**

* Ensure WSL backend eventually receives config:

  * Cache the last WSL-filtered `clientHello` payload inside host runtime and **re-send it**:

    * when WSL backend connects/reconnects, or
    * before the first forwarded WSL repo command if WSL isn’t “initialized”.
  * Alternatively: extend `clientHelloResult` with a `wslReady` (or `wslConfigured`) boolean so the UI can retry `clientHello` on a backoff loop when required.

---

### P1.2 — macOS Gatekeeper/quarantine is only handled via diagnostics, not proactively remediated

**Why this matters:** You’ve added strong diagnostics and ensured executable permissions, but on macOS a copied-out helper binary can still fail to execute if it retains quarantine attributes (`com.apple.quarantine`) or if signing/notarization isn’t correct. Today, the code *explains* what might be wrong, but does not attempt remediation.

**Evidence**

* Exec bit hardening exists (good):

  * `src-tauri/src/lib/agent/install.rs` sets mode `0o755` on unix. Lines **414–451**.
* PermissionDenied diagnostics are high-signal (good):

  * `src-tauri/src/lib/agent/process_control.rs` returns a tailored message for `PermissionDenied` with macOS quarantine/signing hints. Lines **132–188**.
* But there is no explicit quarantine removal step after copy/install.

**Recommended hardening**

* After copying `im_host_agent` to the installed location on macOS, try to remove the quarantine attribute:

  * Preferred: use an xattr library (or `xattr` crate) programmatically.
  * Acceptable: invoke `/usr/bin/xattr -d com.apple.quarantine <path>` (careful with sandbox/entitlements).
  * Failures should degrade to a warning + keep current diagnostics.

---

## P2 Findings

### P2.1 — PRD and system overview still contain Windows-specific config/staging statements that no longer match macOS/Linux parity reality

**Why this matters:** Your implementation now clearly supports “host-native staging roots on all platforms,” but some doc “contracts” still describe Windows-only persistence/paths. That can mislead future changes and reviews.

**Evidence**

* PRD still says staging is on Windows filesystem and defaults to `%LOCALAPPDATA%`:

  * `docs/prd.md` lines **168–182** (“Windows filesystem”, `%LOCALAPPDATA%`).
* System overview says config persists to `%LOCALAPPDATA%\Intermediary\config.json` (Windows-only):

  * `docs/system_overview.md` lines **140–149**.

**Recommendation**

* Update both docs to specify platform-appropriate locations:

  * Windows: `%LOCALAPPDATA%\Intermediary\...`
  * macOS: `~/Library/Application Support/<bundle-id or app name>/...`
  * Linux: likely `~/.local/share/<app>/...`

This is “doc-only,” but because you explicitly requested contract checks vs PRD/ADRs, it’s worth correcting so design intent matches the implementation.

---

### P2.2 — Legacy TS path parsing helper can still misclassify POSIX host paths as WSL paths (migration risk)

**Why this matters:** The *current* add-repo flow correctly routes through Rust (`resolve_repo_root`) and is OS-aware. However, the older TS heuristic in `repo_root.ts` still treats any `/...` path as WSL unless it looks like `/mnt/<drive>...`, which is correct on Windows but wrong on macOS/Linux if it were used by a migration path or future UI code.

**Evidence**

* TS heuristic:

  * `app/src/shared/config/repo_root.ts`:

    * `if (trimmed.startsWith("/")) { ... return { kind: "wsl", path: normalizedWsl }; }` Lines **124–148**.
* Current UI add-repo uses Rust resolver (good):

  * `app/src/components/add_repo_button.tsx` uses `invoke("resolve_repo_root", ...)`. Lines **31–69**.

**Recommendation**

* Either:

  * Deprecate/remove the TS heuristic (and ensure migrations never rely on it for mac/Linux), or
  * Gate it by platform (`hostSupportsWsl()`), treating POSIX absolute paths as `{kind:"host"}` on non-Windows.

---

## P3 Findings

### P3.1 — Naming/comments still contain “windows” terminology in host-native code paths (low risk, but adds confusion)

**Examples**

* UI uses `outputWindowsRoot` even on macOS/Linux (functionally OK, but conceptually confusing):

  * `app/src/components/options_overlay.tsx` uses `setOutputWindowsRoot(...)` for all platforms. Lines **86–109**.
* Some naming and legacy fields persist for backward compat (expected), but consider tightening terminology in comments/docs.

**Recommendation**

* Not required for correctness, but improves long-term clarity:

  * Prefer “host” language in comments/docs when meaning “host across platforms.”

---

## Confirmed OK (requested focus areas)

### Rust protocol/config: host-native model + backward-compat serde aliases

* Repo root kind alias:

  * `crates/im_agent/src/runtime/config.rs` has `#[serde(alias = "windows")]` on both `RepoRootKind::Host` and `RepoRoot::Host`. Lines **132–145**.
  * Tauri config also aliases legacy `windows` roots:

    * `src-tauri/src/lib/config/types.rs` lines **214–236**.
* Protocol payload host-path aliasing:

  * Events:

    * `crates/im_agent/src/protocol/events.rs` uses `#[serde(alias = "windowsPath")]` and `#[serde(alias = "aliasWindowsPath")]`. Lines **34–52**.
  * Responses:

    * `crates/im_agent/src/protocol/responses.rs` includes `windowsPath` + `aliasWindowsPath` optional fields. Lines **33–43**.
* `clientHello` legacy staging field support:

  * `crates/im_agent/src/protocol/commands.rs` custom deserializer supports `stagingHostRoot` + legacy `stagingWinRoot`. Lines **34–83**.

### Staging layout

* Layout matches requested contract:

  * `crates/im_agent/src/staging/layout.rs`:

    * Files root: `staging/files/<repoId>/...` lines **74–76**.
    * Bundles dir: `staging/bundles/<repoId>/<presetId>/...` lines **82–85**.
* Bundle filenames produced as `<repoId>_<presetId>_<timestamp>.zip`:

  * `crates/im_agent/src/bundles/bundle_builder_blocking.rs` lines **35–62**.

### Host-agent backend routing correctness

* Clear split:

  * `RepoBackend { Host, Wsl }` in `crates/im_host_agent/src/runtime/repo_backend.rs`.
* Non-Windows fail-fast for WSL roots:

  * `crates/im_host_agent/src/runtime/host_runtime.rs` returns error if WSL repos present on non-Windows. Lines **93–104**.
* No hidden WSL client spawn on macOS/Linux:

  * WSL client only constructed under `cfg!(target_os = "windows")`:

    * `crates/im_host_agent/src/runtime/host_runtime.rs` lines **111–123**, **287–309**.

### macOS/Linux execution hardening

* Exec bit enforcement for installed host agent:

  * `src-tauri/src/lib/agent/install.rs` `ensure_host_agent_permissions` sets `0o755`. Lines **414–451**.
* High-signal `PermissionDenied` diagnostics:

  * `src-tauri/src/lib/agent/process_control.rs` lines **132–188**.
* Signing/notarization coverage exists (but could be expanded):

  * `docs/commands/agent_bundle.md` includes a note about signing/notarization. Lines **15–23**.

---

## Residual risks (production “still could fail” list)

Even if everything works locally, these could still bite in production:

1. **WSL backend partial hangs** (connected but not responding) wedging host agent due to missing timeouts/cancellation. (P0 above)
2. **WSL startup race** where host agent is ready before WSL backend and WSL never receives `clientHello` unless retried. (P1 above)
3. **macOS quarantine / Gatekeeper** blocking execution of the copied-out host agent binary even when chmod is correct (especially for non-notarized builds or certain distribution methods).
4. **Architecture mismatch on macOS** (Intel vs Apple Silicon) if `im_host_agent` isn’t built universal or matched to the shipped app slice → “Exec format error”.
5. **Missing sidecar signing**: app is signed/notarized but embedded helper binary isn’t (or signature is invalid after packaging) → `PermissionDenied` / OS refusal.
6. **File watcher edge cases on network/external volumes** on macOS/Linux (notify backend limitations); watchers might fail to start or miss events.
7. **Staging root on restricted locations** (user selects an output folder without write/exec rights) leading to confusing failures unless error messaging stays high-signal.

---

## Codex prompts for further hardening

Below are ready-to-run prompts (architecture-first, with explicit refs and deliverables).

### Prompt 1 — Add bounded timeouts + cleanup for WSL forwarded requests

```text
Task
Implement server-side timeouts and cleanup for host→WSL forwarded requests so the host agent cannot wedge indefinitely if the WSL backend stops responding.

Context
HostRuntime forwards repo commands to WslBackendClient without a timeout. WslBackendClient.forward_command awaits a oneshot forever; if the WSL backend is unresponsive, the host runtime can stall and UI timeouts do not recover the agent.

Refs
- crates/im_host_agent/src/runtime/host_runtime.rs (forward_wsl_command + dispatch_repo_command)
- crates/im_host_agent/src/wsl/wsl_backend_client.rs (forward_command, pending map lifecycle)
- docs/compliance/adr_009_rust_concurrency_and_io_boundary_rules.md (timeouts/cancellation expectations)

Deliver
1) Add per-request timeout (configurable constant) for forwarded WSL commands.
2) Ensure pending entries do not leak indefinitely on timeout (remove them or sweep).
3) Return a structured AgentError with a new raw_code (e.g., WSL_BACKEND_TIMEOUT) or reuse WSL_BACKEND_UNAVAILABLE with a distinct message.
4) Add targeted tests (unit-level where possible) covering timeout + cleanup behavior.

Constraints
- Follow ADR-008 error-handling pattern (structured codes/messages/details; no panics).
- Keep changes modular; avoid invasive refactors unless necessary.
- Do not widen security scope or change bind host behavior.
```

### Prompt 2 — Make WSL clientHello bootstrap self-healing

```text
Task
Make WSL bootstrap self-healing: if forwarding clientHello to WSL fails (backend not ready), ensure the host agent will eventually deliver the latest WSL-filtered clientHello once the backend becomes available.

Context
HostRuntime currently returns local clientHello success even if WSL forwarding fails, and only emits an error event. This can leave WSL backend unconfigured until a future clientHello, causing unknown-repo errors.

Refs
- crates/im_host_agent/src/runtime/host_runtime.rs (handle_client_hello, try_forward_client_hello)
- crates/im_host_agent/src/wsl/wsl_backend_client.rs (connection lifecycle)
- docs/system_overview.md (clientHello idempotency assumptions)

Deliver
1) Cache the most recent WSL-filtered clientHello payload in HostRuntime.
2) Add logic to retry sending it when:
   a) the WSL backend becomes connected, and/or
   b) before the first forwarded WSL repo command if not yet initialized.
3) Ensure idempotency: repeated clientHello forwards must not cause churn in the WSL runtime when config fingerprint is unchanged.
4) Add tests or a minimal harness to validate the sequence: host up → clientHello sent → WSL up later → WSL receives config automatically.

Constraints
- Windows-only behavior; no WSL spawning or connections on macOS/Linux.
- Maintain current “degrade gracefully” behavior (host repos should still work even if WSL is down).
```

### Prompt 3 — macOS quarantine hardening for installed host agent binary

```text
Task
Harden macOS execution by attempting to remove quarantine xattrs from the installed host agent binary after copying it out of the app bundle.

Context
We already chmod +x the installed binary and provide strong PermissionDenied diagnostics, but quarantine xattrs can still block execution on macOS for some distribution paths.

Refs
- src-tauri/src/lib/agent/install.rs (ensure_host_agent_permissions, install flow)
- src-tauri/src/lib/agent/process_control.rs (PermissionDenied diagnostics)
- docs/commands/agent_bundle.md (signing/notarization note)
- docs/compliance/adr_010_tauri_security_baseline.md (avoid risky scope widening)

Deliver
1) On macOS only, after copying and chmod, attempt to clear com.apple.quarantine on the installed host agent binary.
2) If clearing fails, log a warning and keep the existing high-signal diagnostics on spawn failure.
3) Add docs: short “Troubleshooting: Gatekeeper/quarantine” section with clear user steps (xattr command) and how it maps to the new behavior.

Constraints
- macOS-only code paths (cfg(target_os="macos")).
- No background daemons or privileged operations.
- Do not reduce current error message quality.
```

### Prompt 4 — Align docs (PRD/system overview) with host-native macOS/Linux parity

```text
Task
Update PRD + system overview docs to reflect host-native staging/config persistence across Windows/macOS/Linux.

Context
Implementation supports host-native paths across platforms, but docs still reference Windows-only locations in some sections.

Refs
- docs/prd.md (Staging System section 7.3)
- docs/system_overview.md (Config Persistence section)
- src-tauri/src/lib/paths/app_paths.rs (source of truth for app local data + staging roots)

Deliver
1) Update docs/prd.md staging section to describe host staging root for all platforms + WSL mirror root only on Windows.
2) Update docs/system_overview.md config persistence to list per-platform config locations (Windows/macOS/Linux).
3) Ensure wording matches current code + protocol fields (stagingHostRoot, stagingWslRoot?).

Constraints
- Keep docs concise; avoid implementation trivia.
- Don’t introduce requirements the code doesn’t meet.
```

---

If you want, I can also produce a **one-page “ship checklist”** specifically for macOS (agent bundle contents, codesign targets, notarization verification steps, and the expected error signatures if anything is wrong).
