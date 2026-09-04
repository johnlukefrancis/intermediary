# ZIPS Tree Write Surface — External Adversarial Review (2026-09-04)
Updated on: 2026-09-04
Owners: JL · Agents
Depends on: ADR-000, ADR-007

## Context

An external adversarial review of the bundled dirty tree
(`intermediary_context_20260904_002543_f16d468.zip`: the drag-in import and the tree actions on top of
`f16d468`) blocked the ZIPS write surface on three P0 ownership failures and one P1 recovery failure while
accepting the UX, routing, shared lock/drain, and the earlier WSL emergency-stop closure. Each finding was
verified against the current source and adjudicated against JL's decisions: worktree writes are allowed;
writing Git metadata, retargeting a confirmation across repos, and overwriting anything unreviewed never
were. All four are genuine. The review's *remedies* were partly rejected (below).

## Findings and closures

| # | Finding | Closure (owner) |
|---|---|---|
| P0-1 | `.git` was listed as an ordinary directory, navigable, and accepted as an import destination; a `.git` nested in a dropped folder was copied. | `ensure_no_git_component` on the import destination (the same law move/copy already applied); topology and listing never return `.git` and refuse listing under it; a nested `.git` source refuses the whole drop. |
| P0-2 | `RepoTab` state survived a repo switch, so a Replace reviewed in repo A could be confirmed and applied to repo B. | The tree's existing `[repoId]` reset law applied to the two request hooks that skipped it (sequence bump + pending cleared) and to the explorer's local state; the commit-message draft reset the same way. |
| P0-3 | Replace was a blanket re-request that overwrote every *current* collision; refuse-mode move and rename used a replacing rename after a check; case aliases collided on NTFS. | `onConflict: { replace: [reviewed paths] }`; authorization bound to each write (authorized → replacing primitive, all others → `create_new` / `rename_no_replace`); rename and refuse-mode move use `rename_no_replace` (case-only rename carved out); a missed alias is a filesystem conflict, never an overwrite. |
| P1-4 | The quarantine sweep (first status read) could remove a directory this process's delete created once its live guard dropped. | The registry remembers every quarantine op this process created for the process lifetime; the sweep never removes those. |

## Rejected remedies

- **Plan tokens / physical destination keys / confirmation nonces.** The reviewed conflict list *is* the
  authorization, sent back on confirm — the same discipline as `expectedSnapshotId`. Physical identity of
  the file at an authorized path is deliberately not verified (design: accepted boundaries).
- **Case-insensitive duplicate detection by heuristic or probe.** Wrong on ext4 (refuses a legitimate
  action), wrong on custom drvfs mounts, and unnecessary once every unauthorized write is non-replacing.
- **A remount key on the repo tab.** It would leave the tree's five existing per-hook resets as dead code
  and reset handset section and other cross-repo state as a side effect.
- **Moving the sweep into the mutation lock.** Wrong layer (serialization importing a feature) and it would
  stop a read-only session from ever releasing the previous session's bytes.
- **Skipping a nested `.git` silently.** Refusing the drop is explicit; a silent skip mangles a dropped
  project without saying so.

## References

- `docs/design/zips_tree_write_surface_design.md` — the surface's behaviour and accepted boundaries.
- `docs/design/source_control_design.md` — quarantine and discard contract.
- `docs/architecture/source_control_architecture.md` — lock, drain, quarantine ownership.
