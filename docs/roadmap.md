# Intermediary — Roadmap

Updated on: 2026-01-30
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

**Status:** Core functionality complete. Ready for daily use.

Configured repos (WSL Linux FS):
- `/home/johnf/code/textureportal`
- `/home/johnf/code/worktrees/tr-engine`
- `/home/johnf/code/intermediary`

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
- Native drag-out via tauri-plugin-drag
- Bundle building with manifest and retention
- Config persistence (tab, worktree, bundle selections)
- Status bar with staging path and error display
- VS Code tasks for Windows development workflow

---

## Future Enhancements (Backlog)

- System tray mode
- Global hotkey for "Build + focus app"
- Windows-native repo support (no agent needed)
- Custom bundle presets via UI
- "Save clipboard as report.md" feature
