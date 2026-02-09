# Known Issues — Intermediary

Updated on: 2026-02-08
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

- 2026-01-31: Bundle build can stall during zipping; UI progress may appear stuck around mid-build.

---

## P2 — Degraded but usable

- 2026-02-08: macOS release packaging can fail to launch `im_host_agent` if helper-binary signing/notarization is incomplete. App now enforces executable permissions at install time and reports high-signal spawn errors, but final notarization coverage still depends on release pipeline configuration.

---

## P3 — Minor issues

- 2026-02-02: Background GPU usage from substrate animations when minimized. Motion governor implemented; pending verification that GPU drops to near-idle when minimized.

---

## Resolved (recent)

- 2026-02-09: Windows installer startup could briefly show empty Docs/Code panes and transient `WSL backend is not available` before WSL repos hydrated. Fixed by adding a supervisor startup gate before first agent connect for WSL-required sessions, plus bounded backoff retries for repo and bundle hydration on transient WSL transport failures.
- 2026-02-08: macOS parity hardening (prompt 1/2 path) introduced repo hydration regressions (tab switch could drop snapshots) and bundle completion stalls (build UI could remain stuck at final stage). Fixed by restoring eager watcher startup during `clientHello`, removing bundle refresh gating on `watchedRepoIds`, and tightening host-runtime routing/error handling for unsupported roots.
- 2026-02-06: Narrow code classifier coverage could miss language families (for example `*.cpp`) in Docs/Code panes. Fixed by generated broad-language extension coverage, default-only codeGlobs migration, and a separate classification-excludes model in Options.
- 2026-02-06: Windows repos stored as `/mnt/<drive>/...` were watched from WSL and could hang or stall change tracking on large trees. Fixed by path-native repo roots plus host-agent routing (Windows roots watched locally; WSL backend only for WSL roots).
- 2026-02-03: Production CSP blocked WebSocket agent connections. CSP allowed `ws://localhost:3141` but frontend dialed `ws://127.0.0.1:3141`. Fixed by aligning CSP with actual loopback URL and removing WSL IP resolution path.
- 2026-02-03: Config persistence failed after frontend schema bumped to v12 while Rust still enforced v11. Fixed by aligning versions, adding v11 to v12 loopback host migration, and a cross-check guard.
