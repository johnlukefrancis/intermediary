# Intermediary — Roadmap

Updated on: 2026-01-29
Owners: JL · Agents
Depends on: ADR-000, ADR-007

---

## Snapshot

| Metric | Value |
|--------|-------|
| Lines of code | 0 (new project) |
| Latest milestone | Foundation setup complete |
| Status | Assumptions locked, ready for spikes |

---

## Active Initiatives

**Next:** Technical spikes to de-risk the hard parts before MVP implementation.

1. **Drag-out spike** — Validate Tauri drag-out works reliably into ChatGPT browser upload zones. Pass/fail decides Tauri vs Electron.
2. **WSL watcher spike** — WSL agent emits file events via inotify for the initial repo set; UI renders "recent changes" reliably.

Initial repo targets (WSL Linux FS):
- `/home/johnf/code/textureportal`
- `/home/johnf/code/worktrees/tr-engine`
- `/home/johnf/code/intermediary`

---

## Priority Order

1. ~~Foundation setup (docs, scripts, config)~~ ✓
2. **Technical spikes** (drag-out, WSL watcher) ← current
3. MVP implementation
4. Polish and multi-repo/worktree support
