# Intermediary — Roadmap

Updated on: 2026-09-06
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

- Two-column Auto Files and Zip Bundles deck per repo
- WSL agent with file watching and auto-staging
- Windows-native host agent endpoint with per-repo backend routing
- Native drag-out via tauri-plugin-drag
- Bundle building with manifest and retention
- Config persistence (tab, bundle selections)
- Status bar with staging path and error display
- VS Code tasks for Windows development workflow
- UI-based repo management (add via "+" button, remove via "×" with confirmation)
- Broad language-aware file classification with generated extension baseline + classifier excludes
- Source Control rail: Git status with staged/unstaged/conflict sections, stage/unstage, commit, discard, per-file diff, push/pull, event-driven refresh (replaces the last VS Code dependency). Three external reviews landed 2026-09-03, each answered at its owner: the adversarial review (snapshot-bound mutations, shutdown drain, corrected timeout ladder, physical-git-dir locking — `docs/reports/source_control_adversarial_review_20260903.md`); the fix-layer closure review (quarantined discard with nanosecond stamps, `expectedMissing`, drain-governed shutdown, Windows Job Object ownership of the Git process tree — `docs/reports/source_control_fix_layer_review_20260903.md`); and the hardening review (`docs/reports/source_control_hardening_review_20260903.md`), which produced one reviewed-snapshot identity for commits, hook reporting instead of a post-publication retraction, retained discard quarantine with a no-replace put-back, and a supervisor-owned process tree for the host agent. Its rejected findings are recorded as accepted boundaries in `docs/design/source_control_design.md`; the Windows job objects are cross-compiled but not yet exercised at runtime (`docs/known_issues.md`).
- Integrated terminal (2026-09-04, 0.1.18): a TERMINAL rail cell beside ZIPS and SOURCE hosting JL's own PowerShell 7 (profile loaded) over ConPTY in the Tauri process, one session group per repo, and native WSL roots preflighted in one pinned distro before `wsl.exe --cd` entry. Sessions park (never dispose) across rail/repo/mode switches; all retained tabs count toward twelve. One Rust transaction owner spans atomic admission through joined process, PTY, reader, and waiter receipt; app exit waits every phase before the WSL idle probe. Output uses a detachable sink and cumulative credit, and the Windows shell is born inside its Job Object through the same creation attribute list as ConPTY. The native scrollbar has a visible track and repo-accent draggable thumb. Tauri IPC only — no agent, plugin, CSP, or capability change (`docs/design/terminal_design.md`, `docs/architecture/terminal_architecture.md`). This removes the last reason to open VS Code.
- Stream panel (2026-09-06, 0.1.23): the left activity panel gained a STREAM mode, now the default, that prints what agents edit as they edit it — hunk cards in the workspace diff grammar, all-added new files, struck-through deletions, image thumbnails dropping in, burst cards for checkouts, a twenty-card ring, click-to-expand and double-click-to-open — fed by one additive `fileDelta` event from a bounded delta pipeline in the repo watcher (settle, budgeted read, `similar` diff against the previous sighting or the index blob). Persisted as `uiState.filesMode`. Contract `docs/design/stream_panel_design.md`; witness `docs/commands/verify_stream.md`.

---

## Future Enhancements (Backlog)

- System tray mode
- Global hotkey for "Build + focus app"
- Finalize dual-agent supervision hardening (host + conditional WSL launch/diagnostics)
- Custom bundle presets via UI
- "Save clipboard as report.md" feature
