# ZIPS Tree Write Surface — Design
Updated on: 2026-09-04
Owners: JL · Agents
Depends on: ADR-000, ADR-007, ADR-008, ADR-009, ADR-010

## Problem

Intermediary replaced Explorer and VS Code for everything except moving files into and around a repository.
JL authorized one deliberate write surface for that: the Zip bundles explorer tree accepts OS drops
(`importFiles`) and organizes entries in place (`worktreeAction`: delete, move, copy, rename). Content editing
stays out of scope. This document is the owner of that surface's behaviour and boundaries; the Source Control
design owns the discard quarantine the delete reuses.

## Goals

- Every worktree write runs in the agent that owns the repo root, under the Source Control per-git-dir
  mutation lock, and appears through the ordinary watcher events.
- Nothing is overwritten that the user did not review and confirm; nothing is written before every refusal
  has been decided.
- Git's own directory is never touched by a non-Git write.
- A wrong delete is recoverable.

## Non-goals

- Editing file content. Auto-renaming (`name copy.ext`). A folder-tree "Move to…" picker. Merging a moved
  folder into an existing folder. Any write to `.git`.

## Behaviour table

| Situation | Behaviour |
|---|---|
| OS files or folders dropped on the tree | Copied into the targeted directory (`importFiles`); folders recurse (symlinks skipped, 10,000-entry bound). A dropped folder that contains a `.git` directory at any depth is refused whole with `INVALID_PATH` naming it — never a silent skip. |
| Any action whose source or destination has a `.git` path component (case-insensitive) | `INVALID_PATH` before anything resolves. `.git` is never listed as a directory and cannot be expanded or listed. |
| A destination file already exists (import, copy, move) | Refused with `ENTRY_CONFLICT` and the full list of colliding paths; nothing written. The Replace modal shows up to 8 and the true count. |
| Replace confirmed | The request carries `onConflict: { replace: [paths] }` — exactly the reviewed list. The agent recomputes the live collisions; any collision not in that list is refused again with the fresh full list (a new modal). Only authorized destinations use a replacing write (temp + rename over, or plain rename); every other destination uses a non-replacing primitive (`create_new`, `rename_no_replace`), so a file that appears during the dialog is refused by the filesystem, never overwritten. |
| Move under refuse; rename | The rename itself refuses to replace (`rename_no_replace`). A racing destination → `ENTRY_CONFLICT`; a filesystem with no such rename (WSL's drvfs mount of a Windows drive) → `SOURCE_CONTROL_UNSUPPORTED_LAYOUT`, serve that drive through the host agent. The case-only rename (`Notes.md` → `notes.md`) keeps the plain rename because the destination is proved to be the source. |
| Two entries in one action land on one path | Refused (`ENTRY_CONFLICT`) byte-exactly before any write. A case-insensitive volume's alias (`A.txt`/`a.txt`) is caught at the write by the non-replacing primitive and reported as a conflict, never an overwrite; no case probe. |
| A folder moved onto an existing folder | `ENTRY_CONFLICT` under both policies: move never merges or destroys a folder the user did not name. Copy merges. |
| File over folder or folder over file | `ENTRY_KIND_MISMATCH`, both policies. |
| Delete | Confirm modal, then each entry (file or whole folder) is claimed by rename into the repo's discard quarantine (`<git dir>/intermediary-discard/<opId>-<i>/`), marked `verified` with plan `delete`, and kept `retained`. No stamp check: the user named it. The sweep never removes a directory the same agent process created, so the copy survives until the next agent start regardless of when the first status read happens. |
| Cross-volume linked worktree | Delete (the claim) and move are renames; `SOURCE_CONTROL_UNSUPPORTED_LAYOUT` before anything moves. |
| Switching repos with a modal or request in flight | Pending Replace, delete confirm, rename, menus, drag, and the request sequence reset on `repoId`; a late response for the previous repo is discarded. |
| Partial failure mid-batch | `effect: unknown` with `details.applied` naming what landed; the watcher and the tree's own re-list reconcile. |

## Accepted boundaries

Recorded from the 2026-09-04 external review (`docs/reports/zips_tree_write_surface_review_20260904.md`):

- **Path-level authorization, not physical identity.** Replace authorizes the reviewed destination paths.
  If the file at an authorized path changed between review and confirm, it is still the path the user chose
  to overwrite; content identity is not verified (the Source Control discard, which restores from a diff the
  user read, is the one place stamps apply).
- **Move is rename-only.** Folder-over-folder is refused, not merged and not "replaced" by quarantining the
  destination.
- **drvfs through the WSL agent refuses move and rename** (no non-replacing rename exists there), while
  delete still works (the quarantine claim lands on empty ground of its own). Windows drives belong to the
  host agent; no configured repo uses this layout.
- **Nested `.git` in a dropped folder refuses the drop.** Dropping a cloned project imports nothing; drop its
  contents or an archive instead.
- **Same-folder paste is refused, not auto-renamed.**

## Acceptance

1. Drop, move, copy, rename, delete from the installed app on a WSL repo and a host repo; the tree, Auto
   Files, and SOURCE reflect each without a manual refresh.
2. A Replace confirmed after a new collision appeared is refused with the new list; nothing is overwritten.
3. `.git` never appears in the tree and every `.git` destination is refused.
4. A deleted entry's bytes are in the quarantine after the next status read and gone after the next agent
   start.
5. Switching repos with a Replace modal open closes it and applies nothing.
