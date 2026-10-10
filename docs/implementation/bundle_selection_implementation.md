# Bundle Descendant Selection
Updated on: 2026-10-10
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

## Intent and witness

Allow files and subfolders inside an excluded folder (including Saved) to be
included independently, without including siblings. Build and reinstall the
Windows installer, inspect the installed tree and archive, and restore the
user's existing selection after the witness.

## Owner decision

Extend the existing BundleSelection contract: includedSubdirs overrides less
specific directory exclusions; includedFiles records individual positive
overrides. Closest directory rule wins, with exact file exclusions last.
Global file/path rules remain hard exclusions. Explicit descendant selection
can cross default directory-name exclusions without including sibling paths.
The Rust selector owns archive and Git admission; traversal also visits the
ancestors needed to reach explicit selections. TypeScript projects these rules
for toggles and mixed parent states. Persistence and agent protocol carry the
same selection without requiring topology enumeration.

## Live receipt

- Identity: /home/johnf/dev/intermediary; clean worktree at start.
- Baseline: screenshot and source confirm disabled descendant controls and
  backend pruning at unselected/excluded parents.
- Frontier: selection contract, predicates, traversal, toggles and persistence
  implemented. Existing Rust bundle suite and new ZIP/Git-boundary regression
  pass; 116 frontend tests, TypeScript, ESLint and agent checks pass.
- Windows workspace check and current NSIS/MSI build pass. NSIS installed;
  executable SHA-256 matches the current Windows release build.
- Native UI/ZIP witness passes: with root files and Saved excluded, selecting
  Saved/Keep/Nested and Saved/Logs/one.txt produces exactly deep.txt and one.txt.
  Parents display mixed states and sibling controls remain usable and excluded.
- Restart witness passes: both positive selections survive native relaunch;
  nested exclusion/re-inclusion and None clearing overrides work through the UI.
- Blorb witness: Saved remains unchecked and all its visible child directory
  controls are enabled. Original repository definitions, selections and UI
  settings compare equal to the pre-work snapshot (allowing the new empty field).
- Complete: test repository removed through the UI; app reopened normally on
  blorb-code, without the process-local DevTools argument. JL retains UX acceptance.
- Evidence location: ignored target/bundle_selection_witness.

## Verification

- [x] Frontend selection/persistence regression, TypeScript and ESLint.
- [x] Rust scanner/Git-selection regression and agent compilation.
- [x] Windows workspace check and current installer build.
- [x] Installed UI descendant toggles, archive contents, persistence, normal relaunch.

## Verification receipt

- Frontend: TypeScript no-emit check and ESLint pass; all 116 TypeScript tests
  pass, including independent descendant selection and persistence regressions.
- Rust: existing im_bundle suite and new descendant ZIP/Git integration test
  pass; 22 agent bundle tests pass; im_agent/im_host_agent checks pass in WSL;
  full workspace check passes on Windows, the supported desktop build surface.
- Packaging: current 0.1.24 NSIS/MSI built through the established script. NSIS
  reinstalled; installed application hash matches the current release executable.
- Source: file headers, regenerated full ledger and whitespace checks pass.
- Evidence: selected_descendants.png, archive_result.json, restart_result.json,
  blorb_saved_fixed.png, blorb_result.json and final_launch.log beneath the
  ignored evidence directory above.
- Limits: Windows reports the existing unused-mut warning in terminal
  session_close.rs:70. Startup briefly reported the existing WSL hello timeout;
  it recovered before the successful native checks. macOS witness not run.
