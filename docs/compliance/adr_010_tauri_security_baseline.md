# ADR-010: Tauri Security Baseline

Status: Accepted
Date: 2026-01-14
Owners: JL · Coding agents
Scope: Tauri shell configuration, asset protocol, and webview security

---

## Context
Intermediary ships as a local desktop app. The webview must be locked down for production while still allowing dev ergonomics. We also expose local files via an asset protocol (for staging directory access) and must keep that scoped.

---

## Decision
1) **CSP required for production builds**
- Production builds must set an explicit CSP.
- Dev builds may override CSP to ease iteration.

2) **Dev override is explicit**
- CSP relaxations are only allowed in dev config or dev runtime overrides.

3) **Local file exposure is scoped**
- Only the asset protocol (e.g., `asset://` or `tp-out://`) is allowed for local file access.
- No direct `file://` access from the webview.
- Asset protocol scope must be the minimal required paths.

4) **No silent widening**
- Broadening CSP or asset scope requires an ADR update and explicit justification.

5) **Local WebSocket IPC is authenticated**
- Localhost WebSocket IPC endpoints must require app-scoped authentication data.
- For browser-facing sockets, validate a shared secret token (required) and enforce an origin allowlist when an `Origin` header is present.
- Internal backend sockets (for example host→WSL) must use a separate secret not exposed to the frontend.
- Tokens must not be written to logs or persisted in app config.

6) **OS drag-in paths are data, not access** (2026-09-03)
- The main window enables Tauri's native drag-drop (`dragDropEnabled`), so the webview receives the absolute OS paths of dropped items as strings.
- The webview never reads, writes, or navigates to those paths; it forwards them to the agent that owns the target repository root, which validates, translates, and copies them.
- Tauri widens a runtime filesystem scope for each dropped path; no `fs` plugin or asset protocol is registered, so that scope is inert. Registering an `fs` plugin or asset protocol later must revisit this decision.

7) **Integrated terminal is a Tauri IPC surface** (2026-09-04)
- Pseudoconsole sessions (Job Object, ConPTY, the pwsh child, its reader and waiter threads) are owned by the Tauri process (`src-tauri/src/lib/terminal/`) and reachable only through app commands over Tauri IPC — `terminal_open`, `terminal_write`, `terminal_resize`, `terminal_ack`, `terminal_close`, `terminal_clipboard_text` — with output on a per-session raw-byte `Channel`. Neither agent socket carries terminal bytes.
- No shell plugin (`tauri-plugin-shell` has no PTY) and no clipboard plugin (`tauri-plugin-clipboard-manager` would widen capabilities). No CSP change and no capability change: the raw channel's large-frame path fetches from `http://ipc.localhost`, which `connect-src` already allows.
- The clipboard is read in Rust (`CF_UNICODETEXT`) for paste, because WebView2 blocks `navigator.clipboard.readText` without a permission prompt the app cannot grant from its window config; copy keeps using `navigator.clipboard.writeText`.
- The terminal is user-driven access by design: it is JL's own shell with JL's profile and environment, and whatever the user runs there is the user's action, exactly as in Windows Terminal. The app injects nothing but the WSL-root entry (`wsl.exe [-d <distro>] --cd '<repo>'`) and the `TERM_PROGRAM` / `COLORTERM` identity; it never runs commands of its own through a session.

---

## Invariants
- I10.1: Release builds have a non-null CSP configured.
- I10.2: Dev-only CSP relaxations are not present in release builds.
- I10.3: Local file access is limited to asset protocol scope; `file://` is disallowed.
- I10.4: Asset protocol scope is minimal and documented.
- I10.5: Local WebSocket IPC handshakes are gated by app-scoped auth and do not accept unauthenticated drive-by connections.
- I10.6: Dropped OS paths cross into the webview as strings only; no webview-side file access follows from a drop, and no filesystem plugin consumes the drop scope.
- I10.7: Terminal sessions are owned by the Tauri process and reached only through the six app commands and their raw-byte channel; no shell or clipboard plugin, CSP entry, or capability exists for them, and the app injects nothing into a session but the WSL-root entry.

---

## Noncompliant examples
- `csp: null` in a release build.
- Allowing `file://` navigation from the webview.
- Asset protocol scoped to `**` without explicit documentation.
- Localhost WebSocket server accepting unauthenticated upgrades from arbitrary origins/pages.

---

## Consequences
- Reduced attack surface in production.
- Clear separation between dev convenience and production security.

---

## Enforcement
- Release checklists must verify CSP is set and asset scope is minimal.
- PRs that broaden CSP or asset scope must include a security review note.
