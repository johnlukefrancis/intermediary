# Known Issues — Intermediary

Updated on: 2026-02-11
Owners: JL · Agents
Depends on: ADR-000, ADR-007

---

## Ground rules (keep this file tiny)

- Log only what is observed; do not add theories or speculation.
- Categorize by disruption level: P0 (blocked), P1 (major), P2 (degraded), P3 (minor).
- Keep entries short and actionable.
- Move resolved issues to the Resolved section with date.

---

## P0 — Core workflow blocked

*None*

---

## P1 — Major functionality broken

*None*

---

## P2 — Degraded but usable

- 2026-02-08: macOS release packaging can fail to launch `im_host_agent` if helper-binary signing/notarization is incomplete. App now enforces executable permissions at install time and reports high-signal spawn errors, but final notarization coverage still depends on release pipeline configuration.
- 2026-02-11: WSL bundle builds are bounded by timeout windows (5 minutes for build requests). Very large or contended builds can return timeout while preserving the previously successful bundle; retry is usually sufficient after backend recovers.
- 2026-02-11: Linux/WSL runtime watching on mounted Windows paths (`/mnt/<drive>/...`) can be degraded on large or busy trees. Intermediary now emits a watcher warning with runbook guidance, but this mode remains warn-only (not blocked).

---

## P3 — Minor issues

- 2026-02-02: Background GPU usage from substrate animations when minimized. Motion governor implemented; pending verification that GPU drops to near-idle when minimized.
- 2026-02-11: After sleep/wake, status can briefly show `Reconnecting (...)` while the client reconnects and rehydrates repo state. This is expected during recovery, but can feel noisy on frequent wake cycles.

---

## Resolved (recent)

- 2026-02-11: Screenshot/image files with common extensions (`.png`, `.jpg`, `.jpeg`, `.webp`, etc.) are now classified as Docs so they appear in the Docs pane instead of being filtered as `other`.
- 2026-02-11: Bundle build no longer requires delete-before-write semantics. Finalization now uses temp-write + atomic rename, then post-finalize pruning, so failed builds keep the last good bundle intact.
- 2026-02-11: WSL sleep/wake and backend restarts could leave stale `WSL backend is not available` errors in the status bar and skip WSL re-bootstrap when `clientHello` payloads were unchanged. Fixed by generation-aware WSL `clientHello` replay, transition-only WSL transport error emission, and explicit `wslBackendStatus` online/offline events that clear stale WSL transport errors on recovery.
- 2026-02-09: Installer builds could intermittently show `NOT CONFIGURED: Staging not configured` while agent status appeared connected. Fixed by gating staging-dependent actions on successful `clientHello`, adding one-shot `clientHello` re-sync + retry on staging-not-configured errors, and isolating dev channel identity/default agent port from installer defaults.
- 2026-02-09: Windows installer startup could briefly show empty Docs/Code panes and transient `WSL backend is not available` before WSL repos hydrated. Fixed by adding a supervisor startup gate before first agent connect for WSL-required sessions, plus bounded backoff retries for repo and bundle hydration on transient WSL transport failures.
- 2026-02-08: macOS parity hardening (prompt 1/2 path) introduced repo hydration regressions (tab switch could drop snapshots) and bundle completion stalls (build UI could remain stuck at final stage). Fixed by restoring eager watcher startup during `clientHello`, removing bundle refresh gating on `watchedRepoIds`, and tightening host-runtime routing/error handling for unsupported roots.
- 2026-02-06: Narrow code classifier coverage could miss language families (for example `*.cpp`) in Docs/Code panes. Fixed by generated broad-language extension coverage, default-only codeGlobs migration, and a separate classification-excludes model in Options.
- 2026-02-06: Windows repos stored as `/mnt/<drive>/...` were watched from WSL and could hang or stall change tracking on large trees. Fixed by path-native repo roots plus host-agent routing (Windows roots watched locally; WSL backend only for WSL roots).
- 2026-02-03: Production CSP blocked WebSocket agent connections. CSP allowed `ws://localhost:3141` but frontend dialed `ws://127.0.0.1:3141`. Fixed by aligning CSP with actual loopback URL and removing WSL IP resolution path.
- 2026-02-03: Config persistence failed after frontend schema bumped to v12 while Rust still enforced v11. Fixed by aligning versions, adding v11 to v12 loopback host migration, and a cross-check guard.
