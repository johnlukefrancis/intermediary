# Intermediary Documentation Guide

This is the documentation index for Intermediary. Start here to find relevant docs.

## Product

| Document | Purpose |
|----------|---------|
| [docs/prd.md](prd.md) | Product requirements and implementation spec |
| [docs/system_overview.md](system_overview.md) | High-level architecture overview |
| [docs/roadmap.md](roadmap.md) | Current initiatives and priorities |
| [docs/known_issues.md](known_issues.md) | Known bugs and limitations |

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

## Design

| Document | Purpose |
|----------|---------|
| [docs/design/intermediary_ui_overhaul_design.md](design/intermediary_ui_overhaul_design.md) | UI design system, tokens, and visual guidelines |

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
| [docs/commands/dev_wsl_agent.md](commands/dev_wsl_agent.md) | Start the WSL agent for local development |
| [docs/commands/setup_wsl_node.md](commands/setup_wsl_node.md) | Install Node.js + pnpm in WSL for agent runs |
| [docs/commands/agent.md](commands/agent.md) | WSL agent development and testing commands |
| [docs/commands/bundle_cli.md](commands/bundle_cli.md) | Build and verify the Rust bundle CLI |
| [docs/commands/zip_bundles.md](commands/zip_bundles.md) | Context bundle creation for ChatGPT |
| [docs/commands/workflow/closeout_checks.md](commands/workflow/closeout_checks.md) | Required dependency sync, ledger updates, and closeout checks |

## Bundle Naming

When creating bundles for ChatGPT context:
- `Intermediary_Full_latest.zip` — Complete codebase
- `Intermediary_Docs_latest.zip` — Documentation only
- `Intermediary_App_latest.zip` — App code only (app/ + src-tauri/)

Use `scripts/zip/zip_bundles.mjs` to generate bundles.
