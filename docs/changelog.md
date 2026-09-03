# Changelog

## 0.1.14 — 2026-09-03

- Added Source Control to the right rail (`[ ZIPS ] [ SOURCE n ]`, also a third handset section): branch, upstream, and ahead/behind status; STAGED CHANGES / CHANGES / MERGE CHANGES sections with per-row and per-section stage and unstage; commit box with Ctrl+Enter; per-file discard behind a confirm; read-only staged/worktree diffs in the shared workspace; pull (fast-forward only) and push. Git runs in the agent that owns the repo root, so WSL and Windows repos both work without leaving the app.
- The repo watcher now emits a coalesced `sourceControlChanged` event for Git metadata writes (including linked worktrees' real git dir) and working-tree changes, so external commits refresh the view without polling; topology refresh no longer fires for writes inside `.git`.
- Promoted the bounded Git runner, porcelain parser, and repo-prefix capture behind a public `im_bundle::git` facade shared by bundle evidence and source control; the runner now keeps stderr/stdout for actionable errors and stops mutations gracefully on timeout so `.git/index.lock` is never left behind.
- Host agent source-control commands are dispatched without holding the runtime write lock, and the two duplicated repo-id routing tables were replaced by one exhaustive `UiCommand::repo_id()`.
- Fixed `pnpm` 11 build-script approval (`pnpm-workspace.yaml`) so `pnpm exec` works again in WSL.

## 0.1.13 — 2026-09-01

- When a bundle Git patch that includes deleted-file bodies exceeds an 8 MiB reviewable budget (or the 32 MiB hard bound), capture now retries with header-only deletions (`--irreversible-delete`) instead of shipping a truncated or deletion-dominated patch, records `patchDeletions: headerOnly` in the manifest, and says so in `BUNDLE_GIT_STATUS.txt`. Ordinary patches keep full deletion bodies.
- Bundle format 3: added `BUNDLE_GIT_INDEX_DIFF.patch` (HEAD to index) and `BUNDLE_GIT_WORKTREE_DIFF.patch` (index to working tree) beside the combined patch, plus `candidateIndexTreeSha` in the manifest, computed read-only and equal to what `git write-tree` would return, so a reviewer can tell staged from unstaged and later match a commit to the reviewed index.
- Added `BUNDLE_GIT_OMITTED_PATHS.txt` naming every changed repository path the selection left out with its status and the exclusion rule; content of those paths stays out of the bundle.
- Raised the selected diff pathspec budget from 4,096 paths/256 KiB to 16,384 paths/1 MiB so a multi-thousand-file working tree gets a complete patch instead of a `pathLimit` partial.

## 0.1.12 — 2026-08-17

- Fixed Windows **Open Containing Folder** actions so project-relative file paths use native separators and Explorer selects the intended file instead of falling back to Desktop or Documents.
- Applied the shared reveal behavior across recent files, bundle include/exclude browsing, workspace titles, and bundle rows.
- Kept the configured Windows build mirror aligned with the WSL sync target, preventing successful installers from packaging stale source.

