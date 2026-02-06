# Known Issues — Intermediary

Updated on: 2026-02-06
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

*None*

---

## P3 — Minor issues

- 2026-02-02: Background GPU usage from substrate animations when minimized. Motion governor implemented; pending verification that GPU drops to near-idle when minimized.

---

## Resolved (recent)

- 2026-02-06: Narrow code classifier coverage could miss language families (for example `*.cpp`) in Docs/Code panes. Fixed by generated broad-language extension coverage, default-only codeGlobs migration, and a separate classification-excludes model in Options.
- 2026-02-06: Windows repos stored as `/mnt/<drive>/...` were watched from WSL and could hang or stall change tracking on large trees. Fixed by path-native repo roots plus host-agent routing (Windows roots watched locally; WSL backend only for WSL roots).
- 2026-02-03: Production CSP blocked WebSocket agent connections. CSP allowed `ws://localhost:3141` but frontend dialed `ws://127.0.0.1:3141`. Fixed by aligning CSP with actual loopback URL and removing WSL IP resolution path.
- 2026-02-03: Config persistence failed after frontend schema bumped to v12 while Rust still enforced v11. Fixed by aligning versions, adding v11 to v12 loopback host migration, and a cross-check guard.
