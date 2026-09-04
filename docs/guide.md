# Intermediary Documentation Guide
Updated on: 2026-09-04
Owners: JL · Agents
Depends on: ADR-000, ADR-006

---

This is the documentation index for Intermediary. Start here to find relevant docs.

## Product

| Document | Purpose |
|----------|---------|
| [docs/prd.md](prd.md) | Product requirements and implementation spec |
| [docs/system_overview.md](system_overview.md) | High-level architecture overview |
| [docs/roadmap.md](roadmap.md) | Current initiatives and priorities |
| [docs/known_issues.md](known_issues.md) | Known bugs and limitations |
| [docs/changelog.md](changelog.md) | Shipped release notes |

## Compliance (ADRs)

Architectural Decision Records — the primary contracts for this codebase.

| ADR | Title |
|-----|-------|
| [ADR-000](compliance/adr_000_modular_file_discipline.md) | Modular File Discipline |
| [ADR-005](compliance/adr_005_typescript_native_contracts_and_rails.md) | TypeScript Native Contracts and Rails |
| [ADR-006](compliance/adr_006_dev_environment_agent_workflow_discipline.md) | Dev Environment and Agent Workflow Discipline |
| [ADR-007](compliance/adr_007_architecture_first_execution.md) | Architecture-First Execution |
| [ADR-008](compliance/adr_008_rust_runtime_contracts_and_error_handling.md) | Rust Runtime Contracts and Error Handling |
| [ADR-009](compliance/adr_009_rust_concurrency_and_io_boundary_rules.md) | Rust Concurrency and IO Boundary Rules |
| [ADR-010](compliance/adr_010_tauri_security_baseline.md) | Tauri Security Baseline |
| [ADR-012](compliance/adr_012_copy_safe_command_delivery.md) | Copy-safe Command Delivery |
| [ADR-013](compliance/adr_013_wsl_agent_lifecycle.md) | WSL Agent Lifecycle, Ownership, and Shutdown |

## Design

| Document | Purpose |
|----------|---------|
| [docs/design/intermediary_ui_overhaul_design.md](design/intermediary_ui_overhaul_design.md) | UI design system, tokens, and visual guidelines |
| [docs/design/source_control_design.md](design/source_control_design.md) | Source Control view: goals, behaviour table, cancellation/timeouts, acceptance |
| [docs/design/zips_tree_write_surface_design.md](design/zips_tree_write_surface_design.md) | ZIPS tree write surface: drag-in import and delete / move / copy / rename behaviour, replace authorization, `.git` law, accepted boundaries |
| [docs/design/terminal_design.md](design/terminal_design.md) | Integrated terminal (TERMINAL rail): goals, behaviour table, key and clipboard policy, accepted boundaries, acceptance |

## Architecture

| Document | Purpose |
|----------|---------|
| [docs/architecture/bundle_format_architecture.md](architecture/bundle_format_architecture.md) | Bundle v2 generated handoff entries, selection-bounded Git evidence, coherence, and failure semantics |
| [docs/architecture/source_control_architecture.md](architecture/source_control_architecture.md) | Source Control ownership, protocol, watcher signal, cancellation and timeout ladder |
| [docs/architecture/terminal_architecture.md](architecture/terminal_architecture.md) | Integrated terminal ownership, open/close/app-exit ordering, invariants I1–I10, failure modes |

## Implementation

| Document | Purpose |
|----------|---------|
| [docs/implementation/terminal_hardening_implementation.md](implementation/terminal_hardening_implementation.md) | Active execution owner for the terminal lifecycle, WSL-entry, bounded-output, Job-at-creation, cap, and flow-credit hardening |

## Reports

| Document | Purpose |
|----------|---------|
| [docs/reports/bundle_global_excludes_report.md](reports/bundle_global_excludes_report.md) | Model verdict for bundle global-exclude ownership and manifest evidence |
| [docs/reports/source_control_adversarial_review_20260903.md](reports/source_control_adversarial_review_20260903.md) | External adversarial review of the Source Control mutation transaction (P0/P1 findings and their required end state) |
| [docs/reports/source_control_fix_layer_review_20260903.md](reports/source_control_fix_layer_review_20260903.md) | External closure review of the hardening layer: two remaining P0 owners (effect-boundary binding, drain-governed shutdown) |
| [docs/reports/source_control_hardening_review_20260903.md](reports/source_control_hardening_review_20260903.md) | External hardening review of the split tree: reviewed-snapshot commit identity, post-publication hook reporting, discard quarantine safety, and process-tree ownership |
| [docs/reports/source_control_shutdown_owner_review_20260903.md](reports/source_control_shutdown_owner_review_20260903.md) | Fourth external review: one remaining P0 — the WSL emergency-stop route killed the agent after 750 ms, orphaning its Git process groups |
| [docs/reports/zips_tree_write_surface_review_20260904.md](reports/zips_tree_write_surface_review_20260904.md) | Fifth external review, on the ZIPS write surface: `.git` reachability, cross-repo confirmation, replace authorization and replacing renames, quarantine sweep timing — closures and rejected remedies |

## Environment

Workflow and tooling documentation.

| Document | Purpose |
|----------|---------|
| [docs/environment/docs_workflow.md](environment/docs_workflow.md) | Documentation workflow canon |
| [docs/environment/codex_prompting_guide.md](environment/codex_prompting_guide.md) | Guide for prompting Codex agents |
| [docs/environment/codex_operational_guide.md](environment/codex_operational_guide.md) | Operational guide for Codex |
| [docs/environment/chatgpt_custom_instructions.md](environment/chatgpt_custom_instructions.md) | ChatGPT collaboration instructions |

## Usage

| Document | Purpose |
|----------|---------|
| [docs/usage/staging_probe_usage.md](usage/staging_probe_usage.md) | Test doc for staging detection |
| [docs/usage/agent_wsl_bruised_states.md](usage/agent_wsl_bruised_states.md) | Troubleshooting runbook for recoverable agent/WSL degraded states |

## Inventory

| Document | Purpose |
|----------|---------|
| [docs/inventory/skills_inventory.md](inventory/skills_inventory.md) | Available agent skills for this project |
| [docs/inventory/file_ledger.md](inventory/file_ledger.md) | Auto-generated file inventory |

## Commands

Runnable commands organized by area (ADR-012 compliant).

| Document | Purpose |
|----------|---------|
| [docs/commands/dev_windows.md](commands/dev_windows.md) | Windows development workflow with WSL sync |
| [docs/commands/dev_wsl_agent.md](commands/dev_wsl_agent.md) | Start the Rust WSL agent for local development |
| [docs/commands/agent.md](commands/agent.md) | WSL agent development and testing commands |
| [docs/commands/kill_agent_ports_windows.md](commands/kill_agent_ports_windows.md) | Clear stale Windows or WSL listeners from the Intermediary agent port |
| [docs/commands/verify_wsl_port_detection.md](commands/verify_wsl_port_detection.md) | Manually verify WSL port-listener detection end-to-end through wsl.exe |
| [docs/commands/verify_wsl_agent_tree_kill.md](commands/verify_wsl_agent_tree_kill.md) | Manually verify the WSL emergency stop's process-tree sweep and the agent's stdin-EOF shutdown owner |
| [docs/commands/verify_terminal.md](commands/verify_terminal.md) | Manually witness the integrated terminal in the installed app: tabs, WSL entry, TUIs, clipboard, switches, close semantics, flood, minimize, app-exit idle probe |
| [docs/commands/agent_bundle.md](commands/agent_bundle.md) | Build the bundled agent runtime for installers |
| [docs/commands/release_windows.md](commands/release_windows.md) | Windows-first release flow, version bumping, and GitHub release automation |
| [docs/commands/build_installer_from_wsl.md](commands/build_installer_from_wsl.md) | Build, silently install, and relaunch the Windows installer from a WSL shell |
| [docs/commands/bundle_cli.md](commands/bundle_cli.md) | Build and verify the Rust bundle CLI |
| [docs/commands/fix_inotify_limits.md](commands/fix_inotify_limits.md) | Raise inotify limits in WSL for large repos |
| [docs/commands/zip_bundles.md](commands/zip_bundles.md) | Context bundle creation for ChatGPT |
| [docs/commands/workflow/closeout_checks.md](commands/workflow/closeout_checks.md) | Required dependency sync, ledger updates, and closeout checks |
| [docs/commands/textures/downscale_textures.md](commands/textures/downscale_textures.md) | Downscale theme texture assets to 256x256 |

## Bundle Naming

Intermediary produces timestamped bundles with this pattern:

```
{repoId}_{presetId}_{YYYYMMDD_HHMMSS}_{shortSha}.zip
```

- Timestamp is UTC (matches manifest's `generatedAt`)
- One bundle per repo+preset (building a new one replaces the old)
- Each bundle contains the bundle v2 generated handoff set: manifest, Git status, selected tracked patch, and orientation note

To find the latest bundle: sort by filename timestamp descending, take first match.

For legacy scripts (`scripts/zip/zip_bundles.mjs`), see [docs/commands/zip_bundles.md](commands/zip_bundles.md).
