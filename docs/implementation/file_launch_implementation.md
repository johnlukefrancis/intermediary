# File Actions and Project Launch Quest
Updated on: 2026-10-13
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

## Accepted outcome

JL accepted the installed Windows file actions, authoring gallery launch bindings,
separate Blorb HTML demo action and compact SVG on 2026-10-13, and explicitly
authorized committing this work together with Claude's completed resource meter.
The meter is already committed on master through ee35373; the remaining launch
changes are approved for one focused commit on top. No push was requested.

## Owners and invariants

- Windows Shell dispatch owns file/directory reveal and default file/browser
  opening. SHOpenFolderAndSelectItems selects the item in its parent; native
  failures return through the existing command boundary. Explorer command-string
  builders and their obsolete tests are retired.
- launchTargets owns each repo's project choice: a discovered top-level file,
  explicit absolute host file or HTTP(S) URL. Discovery keeps build-entry HTML
  excluded. Target validation and launch share one native route off the UI thread.
- demoTargets owns one working HTML demo per group/tab, separate from project
  ranking. Schema 30 defaults old settings to an empty demo map. Removing the
  last root for a tab removes its demo binding.
- Exact controls own their pending/success/error feedback. The demo uses a
  custom browser/play SVG in a 20px button, with its tooltip and accessible name.
- Repository path validation and release CSP are preserved. Browser pages open
  externally; existing gallery/demo servers remain externally owned.

## Installed bindings

| Tab/group | Target |
| --- | --- |
| blender-authoring | J:\BlenderAuthoring\blorb\mainland\index.html |
| blender-authoring-islands | http://localhost:8788/repairs/ |
| Blorb HTML demo | http://127.0.0.1:8777/Docs/Reference/momo.html |

The Blorb group opens the live blorb-lab MOMO copy confirmed by lab and master
owners, without harness query arguments. All three Blorb roots retain their own
Unreal launcher. Future moved demos require updating the saved binding; browser
history discovery and server startup are outside the shipped route.

## Final verification

- [x] TypeScript and ESLint pass. The 148-test frontend suite and eight launch
  plus two config-type Rust tests passed for the behavior change. Logic tests
  were not repeated for the final icon-only refinement.
- [x] Windows workspace cargo check passes. Existing terminal unused-mut and
  test-only dead-code warnings, plus the Vite chunk warning, remain unrelated.
- [x] Installed Windows/WSL file and directory context menus select exact items
  in Explorer, including file-viewer titles and paths with spaces, commas and
  Unicode. Missing files fail. The native oracle reads Explorer SelectedItems.
- [x] Both authoring galleries and normal MOMO open at their exact targets.
  Demo launch preserves project choices and the selected blorb-code worktree.
- [x] Native tab and all three dropdown rows have SVG-only 20px demo controls;
  tooltip/accessibility and success feedback remain. No runtime errors.
- [x] Combined NSIS/MSI candidate built and silently installed. Installed bytes
  match release SHA256
  A5C798BC02D6E4B3E72C3036E2A11263A503970A077CC2C7C7617AA39924B8DB.
- [x] Normal startup restored, inspection port 9225 closed, original navigation
  and project settings preserved, and bindings survive restart. Claude inspected
  its resource meter in the same candidate before the final normal relaunch.
- [x] Headers and generated file ledger cover the changed source owners.

Native Unreal editor launch and macOS/Linux desktop reruns were not run for this
Windows HTML task. The selected Unreal target was inspected through the existing
route without starting another editor.

Evidence is retained in ignored target/file_launch_witness,
target/demo_launch_witness and target/demo_icon_witness. An initial Unicode
selection mismatch was PowerShell output encoding; escaped JSON confirmed the
actual correct path. Screenshots demo_icon_tab.png and demo_icon_worktrees.png
show the final control. User settings snapshots remain outside Git.

## Changed files

- Windows Shell: src-tauri/Cargo.toml;
  src-tauri/src/lib/commands/{mod.rs,windows_shell.rs,file_manager.rs,
  file_open_policy.rs,file_opener.rs}.
- Launch model: src-tauri/src/lib/launch/{mod.rs,target.rs,candidates.rs,
  launch_action.rs}, src-tauri/src/lib/commands/launch.rs,
  src-tauri/src/lib/config/{types.rs,types/tests.rs}.
- Frontend contracts/config: app/src/shared/protocol_launch.ts,
  app/src/shared/config/{persisted_config.ts,version.ts};
  app/src/hooks/{use_config.tsx,use_config_actions_launch.ts,
  use_config_actions_remove.ts,use_launch_candidates.ts,use_project_launch.ts}.
- Controls: app/src/lib/launch/{launch_target.ts,launch_target_test.ts};
  app/src/components/tab_bar/{launch_control.tsx,launch_menu.tsx,
  project_launch_context.tsx,tab_bar_items.tsx,tab_bar_dropdowns.tsx};
  app/src/styles/tab_bar_launch.css.
- Documentation: docs/design/project_launch_design.md, docs/changelog.md,
  docs/guide.md, docs/commands/verify_file_launch.md, this Quest and generated
  docs/inventory/file_ledger.{json,md}.

Frontier: implementation, installation, verification and JL acceptance are
complete. Commit approval is recorded; Git owns the landing history.
