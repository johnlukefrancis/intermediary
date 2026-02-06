# Intermediary — Roadmap

Updated on: 2026-02-06
Owners: JL · Agents
Depends on: ADR-000, ADR-007

---

## Snapshot

| Metric | Value |
|--------|-------|
| Lines of code | ~4000 |
| Latest milestone | Daily-driver MVP complete |
| Status | Ready for production use |

---

## Active Initiatives

**Status:** Core functionality complete. Host-routed dual-backend rollout is in progress.

Repos are user-configured via the UI (add/remove buttons in tab bar). Ships with no default repos.

**Next:** Maintenance and enhancements based on real-world usage.

---

## Priority Order

1. ~~Foundation setup (docs, scripts, config)~~ ✓
2. ~~Technical spikes (drag-out, WSL watcher)~~ ✓
3. ~~MVP implementation~~ ✓
4. ~~Daily-driver polish (persistence, observability, docs)~~ ✓
5. **Maintenance / enhancements** ← current

---

## Completed Features

- Three-column UI (Docs, Code, Bundles) per repo
- WSL agent with file watching and auto-staging
- Windows-native host agent endpoint with per-repo backend routing
- Native drag-out via tauri-plugin-drag
- Bundle building with manifest and retention
- Config persistence (tab, bundle selections)
- Status bar with staging path and error display
- VS Code tasks for Windows development workflow
- UI-based repo management (add via "+" button, remove via "×" with confirmation)

---

## Future Enhancements (Backlog)

- System tray mode
- Global hotkey for "Build + focus app"
- Finalize dual-agent supervision hardening (host + conditional WSL launch/diagnostics)
- Custom bundle presets via UI
- "Save clipboard as report.md" feature
