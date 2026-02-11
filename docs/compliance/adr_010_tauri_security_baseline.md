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

---

## Invariants
- I10.1: Release builds have a non-null CSP configured.
- I10.2: Dev-only CSP relaxations are not present in release builds.
- I10.3: Local file access is limited to asset protocol scope; `file://` is disallowed.
- I10.4: Asset protocol scope is minimal and documented.
- I10.5: Local WebSocket IPC handshakes are gated by app-scoped auth and do not accept unauthenticated drive-by connections.

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
