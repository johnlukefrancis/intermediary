# Bundle Format Architecture
Updated on: 2026-07-15
Owners: JL · Agents
Depends on: ADR-000, ADR-007, ADR-008, ADR-009, ADR-012

## Purpose

An Intermediary bundle is a self-describing, immutable repository handoff. It carries the current files admitted by the bundle selection plus bounded evidence describing how that captured selection differs from the captured `HEAD`. A recipient does not need a `.git` directory, shell, earlier bundle, upstream branch, or live repository access.

Git evidence is a best-effort capability layered onto ordinary local bundle authority. Git failure never creates a second build path and does not prevent otherwise valid files from being bundled.

## Ownership

- `im_bundle` owns scanning, Git capture, generated metadata entries, ZIP accounting, and writing.
- One compiled bundle-selection predicate owns root inclusion, selected top-level directories, explicit file and subdirectory exclusions, and effective global exclusions. Filesystem scanning and Git-path admission both call this predicate.
- `im_agent` owns blocking-worker orchestration, atomic temp-to-final rename, cancellation cleanup, last-good retention, and pruning to one latest bundle per repo/preset.
- Host-native and WSL-backed requests reach the same `im_bundle` writer after routing, so they produce one archive contract.

## Archive contract

Bundle format version 3 reserves seven generated root entries:

| Entry | Contract |
| --- | --- |
| `BUNDLE_MANIFEST.json` | Machine-readable entry point for build, selection, accounting, and versioned Git-capture facts (Git capture contract version 2), including `candidateIndexTreeSha`. |
| `BUNDLE_GIT_STATUS.txt` | Human/model-readable HEAD orientation, candidate index tree, selected staging/worktree state, diff stat, name-status, omitted-change count, and capture issues. |
| `BUNDLE_GIT_DIFF.patch` | Selected tracked final-state delta from captured HEAD (HEAD to working tree). Empty for a clean selected tracked state or unavailable evidence. This is the coherence-verified patch. |
| `BUNDLE_GIT_INDEX_DIFF.patch` | Selected delta from captured HEAD to the index (what `git commit` would record). |
| `BUNDLE_GIT_WORKTREE_DIFF.patch` | Selected delta from the index to the working tree (what remains unstaged). Untracked files never appear here; they are ordinary archive entries. |
| `BUNDLE_GIT_OMITTED_PATHS.txt` | Every changed repository path the selection left out: porcelain XY status, path, and the exclusion rule that omitted it. Names only, never content. |
| `BUNDLE_HANDOFF.md` | Project-neutral read order and operator-path guidance for a fresh model. |

Repository files may not shadow these names. A selected collision fails before ZIP creation/finalization, leaving the previous good bundle untouched.

`candidateIndexTreeSha` is the tree id the whole-repository index would commit as, computed read-only from `git ls-files --stage` with Git's own tree hashing rather than `git write-tree`, so evidence capture never writes objects into the repository. It is absent, with a named issue, when the index holds unmerged entries or could not be listed. A later commit whose `^{tree}` equals it contains exactly the reviewed staged state.

`fileCount` includes ordinary selected files and all seven generated entries. `totalBytesBestEffort` is the converged sum of their uncompressed byte lengths, including the manifest itself.

## Manifest Git contract

The manifest `git` object has `contractVersion: 1` and records:

- `comparisonBase: "HEAD"`, `capturedAt`, captured `headSha`/`shortSha`, and branch when available;
- `status`: `complete`, `partial`, `unavailable`, or `unstable`;
- independent `repoDirty` and `selectionDirty` facts;
- selected changed, tracked, untracked, deleted, renamed, and conflicted counts;
- `omittedChangedPaths`, counted without reproducing omitted path names;
- generated artifact names, incomplete artifact names, and structured capture issues.

`complete` means the selected artifact set, captured HEAD/status, repeated patch, and selected current-file bytes agreed through capture. `partial` means a named command, parser, encoding, timeout, path, or output bound prevented a complete artifact. `unavailable` means there is no usable Git work tree/HEAD or Git execution failed before evidence could be established. `unstable` means HEAD, status, patch output, or selected file bytes moved during capture.

## Status and patch semantics

`BUNDLE_GIT_STATUS.txt` reports the two-character index/worktree status for selected paths. Selected untracked files are explicitly identified as having no HEAD ancestor. They remain ordinary current files in the archive and are not synthesized into the tracked patch.

Git porcelain status intentionally hides ignored files, but bundle selection does not inherit Git ignore rules. After scanning, the capture therefore submits only the actual selected ordinary-file paths to bounded `git check-ignore --stdin -z` classification. Selected ignored files are reported as `!!`, counted as selected untracked files, fingerprinted for coherence, and identified as ignored by Git with no HEAD ancestor. Ignored paths that were not admitted by bundle selection are never submitted to this command and cannot enter generated evidence.

The status artifact is summary-only: its stat command explicitly disables patch output, while its name-status command selects Git's mutually exclusive name/status format. Tracked patch headers and hunks therefore appear only in `BUNDLE_GIT_DIFF.patch`, never duplicated into the orientation artifact.

`BUNDLE_GIT_DIFF.patch` compares the captured HEAD commit to the final selected tracked working-tree state, not to the index alone. Staged and unstaged edits therefore collapse to the current file content. The patch retains deletions, fully selected renames, mode changes, conflicts Git can represent, and submodule pointer changes. Deleted files carry their full removed body by default because that body is ordinary review evidence. Only when a patch carrying those bodies exceeds the 8 MiB reviewable patch budget (or the 32 MiB hard bound) and the selection contains deletions does the capture retry with `--irreversible-delete`, keeping each deletion's header and index line without its preimage so a large contraction yields a complete, readable delta instead of a truncated one or one dominated by removed content. The manifest records this as `git.patchDeletions` (`full` or `headerOnly`), `BUNDLE_GIT_STATUS.txt` states it, and the stat and name-status sections still list every deletion in either mode.

External diff commands and text conversion are disabled. Literal pathspecs, fixed diff formatting, NUL-delimited porcelain parsing, and lossless host path transport avoid user aliases and UTF-8 filename assumptions. Selected paths are divided into deterministic host-safe argument batches before each diff process is spawned, so a selection within the product path budget does not inherit Windows' smaller process command-line ceiling. Binary files use Git's binary-difference marker; the patch does not request binary payload encoding. The selected current binary file remains an ordinary archive entry.

When only one endpoint of a rename is selected, capture uses no-rename evidence for that selected endpoint and never reproduces the excluded counterpart. Fully selected rename pairs remain atomic when batches are formed and use rename detection so both admitted names remain visible.

## Selection and privacy boundary

The bundle selection is authoritative for current files, deleted paths, rename endpoints, and Git artifact pathspecs. Explicit file/subdirectory exclusions and effective global excludes apply identically to the scanner and Git projection. Recommended directory-name excludes seed the selector state rather than becoming irreversible filters: an explicitly selected top-level directory or a path recorded in `includedSubdirs` overrides a matching directory-name exclude at that exact directory. Other excluded directory names beneath that subtree remain filtered. Path-pattern and file excludes are not weakened by a directory inclusion override.

Git may inspect whole-repository porcelain status and the whole-repository index listing locally to determine `repoDirty`, the omitted set, and `candidateIndexTreeSha`. Raw whole-repository output is never written to the archive. Excluded content is absent from every generated artifact and ordinary entry. Excluded names are absent from status, stat, name-status, and every patch; the one place they cross the boundary is `BUNDLE_GIT_OMITTED_PATHS.txt`, which names each changed-but-omitted path with its XY status and the exclusion rule that omitted it so a reviewer can adjudicate what the selection left out (decided 2026-09-02; before that only the count crossed). General selected deltas disable rename detection so Git cannot discover an excluded counterpart. Rename detection runs only over pairs whose endpoints both passed selection.

## Capture lifecycle and coherence

1. Start a blocking capture session, resolve the repo-relative Git prefix, capture NUL-delimited porcelain-v2 status, freeze the HEAD SHA used by all diff commands, fingerprint the initial selected tracked delta/current changed-file bytes, and compute the candidate index tree id.
2. Scan through the shared predicate, reject reserved-entry collisions, then reconcile the actual selected ordinary-file set against Git ignore rules through bounded NUL-delimited stdin. Add selected ignored files to untracked evidence and fingerprint them before writing.
3. Stream ordinary files into the ZIP. Selected changed/untracked regular files are hashed while their exact written bytes pass through the existing bounded copy buffer.
4. Regenerate the selected HEAD-to-worktree patch, the HEAD-to-index and index-to-worktree patches, stat, and name-status through deterministic host-safe selected-path batches with bounded Git subprocess output. All three patches share the deletion mode chosen in step 1.
5. Compare the initial/final patch and written-file fingerprints, re-run the final patch, re-hash watched current files, recompute the candidate index tree id, re-run selected-file ignore classification, and re-run status. Any mismatch prevents a `complete` verdict.
6. Write status, the three patches, the omitted-path listing, handoff, and the converged manifest; sync the temp archive.
7. The agent checks cancellation, atomically renames temp to final, then prunes older matching bundles.

Git capture subprocesses have bounded stdin/stdout/stderr buffers, five-second command timeouts, cancellation polling, and forced termination. Status and selected ignore-classification input/output are each capped at 8 MiB, ignore classification is capped at 65,536 selected files, a patch at 32 MiB (with an 8 MiB reviewable budget past which deletion bodies are dropped as described above), each status summary at 4 MiB, selected diff pathspecs at 16,384 paths/1 MiB, each spawned diff receives at most 24 KiB of path arguments, and streaming coherence verification is capped at 15 seconds. Rename pairs are indivisible at the 24 KiB transport boundary. Reaching a bound preserves the normal bundle and names the incomplete artifact; truncation is never silent.

## Behavior matrix

| Situation | Visible contract |
| --- | --- |
| Selected tracked paths exceed the host process command-line ceiling but remain within the bundle pathspec budget | Capture batches them deterministically and emits the complete selected patch, stat, and name-status evidence. |
| Selected patch with full deleted-file bodies exceeds the 8 MiB reviewable budget (or the 32 MiB hard bound) | Capture retries with header-only deletions, stays `complete`, records `patchDeletions: headerOnly`, and names the mode in the status artifact. Only if that patch also overruns the hard bound is the patch reported as truncated. |
| Selected ignored file is archived | Capture identifies it as ignored/untracked with no HEAD ancestor; selection is dirty and the untracked count includes it. |
| A selected file is staged and then edited again | `BUNDLE_GIT_DIFF.patch` shows the combined result, `BUNDLE_GIT_INDEX_DIFF.patch` shows only the staged hunk, and `BUNDLE_GIT_WORKTREE_DIFF.patch` shows the edit on top of it. |
| Changed paths fall outside the selection | The count is in the manifest, and each name, XY status, and exclusion rule is in `BUNDLE_GIT_OMITTED_PATHS.txt`; no content of those paths is bundled. |
| Index holds unmerged entries | `candidateIndexTreeSha` is absent, an `indexUnmerged` issue is recorded, and the capture is `partial`. |
| Nested source directory matches a recommended output name such as `target` | It starts excluded; selecting it records `includedSubdirs`, and both ordinary files and Git evidence include that exact subtree. |
| Another `target` exists outside the explicit inclusion | It remains excluded by the effective global directory-name policy. |
| Repository contains only selected ignored files beyond clean HEAD | Repository dirty remains false under Git semantics, selection dirty is true, and the tracked patch remains empty. |
| Ignored file is excluded by bundle selection | Its name and contents are absent from every generated artifact and ordinary ZIP entry. |
| Ignore reconciliation fails, times out, changes, or reaches a bound | The normal bundle survives with explicit `partial` or `unstable` status and `BUNDLE_GIT_STATUS.txt` named incomplete. |

## Failure behavior

| Situation | Result |
| --- | --- |
| Clean Git repo | `complete`, both dirty flags false, explicit clean status, empty patch. |
| Dirty repo with only excluded changes | Repo dirty, selection clean, omitted count nonzero, empty selected patch. |
| Non-Git root or no HEAD | Normal archive with `unavailable` evidence and a structured reason. |
| Missing Git, timeout, parse/command failure, unsupported path transport, or a safety bound | Normal archive with `unavailable` or `partial` evidence naming the cause/artifact. |
| HEAD/status/patch/selected bytes move | Normal archive with `unstable` evidence; exact coherence is not claimed. |
| Cancellation or ordinary scan/write/finalize failure | Temp output is removed and the previous successful bundle remains. |

## Handoff read order

A fresh model reads the manifest, status, patch, then project navigation when present: `docs/guide.md`, the recent part of `docs/changelog.md`, and relevant source/docs. The archive is captured evidence, not a mutable repository. Operator guidance always uses repo-local paths and never instructs a person to open or modify the bundle.

## Verification anchors

- `crates/im_bundle/tests/git_evidence_test.rs` builds dirty, clean, non-Git, unusual-path, and collision witnesses and inspects archive contents/accounting, including the staged/unstaged patch split, omitted-path naming, and `candidateIndexTreeSha` against `git write-tree`.
- `crates/im_bundle/src/git_capture/index_tree.rs` proves the read-only tree hashing against `git write-tree` including Git's directory ordering rule.
- `crates/im_bundle/tests/git_large_selection_test.rs` builds a selected-path set larger than the Windows process command-line ceiling but below the bundle pathspec budget and requires complete patch evidence.
- `crates/im_bundle/src/git_capture/tests.rs` covers missing Git, timeout, command failure, capture drift, and the header-only deletion fallback under a reduced patch budget.
- `crates/im_agent/src/bundles/bundle_builder_tests.rs` guards cancellation cleanup, last-good retention, atomic finalization, and one-latest pruning.
