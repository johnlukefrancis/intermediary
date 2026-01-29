# TexturePortal Documentation Guide
Updated on: 2026-01-25
Owners: JL · Agents
Depends on: ADR-000, ADR-006

## Purpose
- Single entrypoint for TexturePortal docs.
- Use this guide to locate the correct document before planning or editing.
- Paths are repo-relative and must exist.

## How to use
1. Read **Compliance (ADRs)** for non-negotiable constraints.
2. Read **System overview** + **Architecture** for current behavior.
3. Use **Design** for proposals and historical context.
4. Use **Reports** for investigations and debugging notes.
5. Use **Inventory** for the file ledger.

## Compliance (ADRs)
- `docs/compliance/adr_000_modular_file_discipline.md`
- `docs/compliance/adr_005_typescript_native_contracts_and_rails.md`
- `docs/compliance/adr_006_dev_environment_agent_workflow_discipline.md`
- `docs/compliance/adr_007_architecture_first_execution.md`
- `docs/compliance/adr_008_rust_runtime_contracts_and_error_handling.md`
- `docs/compliance/adr_009_rust_concurrency_and_io_boundary_rules.md`
- `docs/compliance/adr_010_tauri_security_baseline.md`
- `docs/compliance/adr_011_managed_ml_runtime_packs_and_binary_only_deps.md`
- `docs/compliance/adr_012_copy_safe_command_delivery.md`
- `docs/compliance/adr_013_unified_run_log_and_debug_console.md`

## System overview
- `docs/system_overview.md`
- `docs/roadmap.md`
- `docs/known_issues.md`

## Architecture
- `docs/architecture/output_architecture.md`
- `docs/architecture/io_pipeline_architecture.md`
- `docs/architecture/gpu_backend_architecture.md`
- `docs/architecture/ml_system_architecture.md`
- `docs/architecture/ml_attempt_logging_architecture.md`
- `docs/architecture/ml_runtimes_architecture.md`
- `docs/architecture/ml_sidecar_protocol_architecture.md`
- `docs/architecture/seam_repair_architecture.md`
- `docs/architecture/tile_checker_architecture.md`
- `docs/architecture/settings_panel_architecture.md`
- `docs/architecture/preview_window_architecture.md`
- `docs/architecture/webgpu_preview_architecture.md`
- `docs/architecture/unified_run_log_architecture.md`
- `docs/architecture/watch_cli_architecture.md`
- `docs/architecture/diffusion_process_visualizer_architecture.md`

## Design
- `docs/design/output_design.md`
- `docs/design/ml_pipeline_design.md`
- `docs/design/ml_generation_design.md`
- `docs/design/ml_runtimes_design.md`
- `docs/design/ml_sidecar_protocol_design.md`
- `docs/design/ui_terminal_noir_design.md`
- `docs/design/ml_advanced_options_menu_design.md`
- `docs/design/ml_sampling_controls_design.md`
- `docs/design/diffusion_process_visualizer_design.md`

## Implementation
- `docs/implementation/ml_advanced_options_menu_implementation.md`
- `docs/implementation/ml_sampling_controls_implementation.md`
- `docs/implementation/ml_pipeline_implementation.md`

## Environment
- `docs/environment/docs_workflow.md`
- `docs/environment/codex_operational_guide.md`
- `docs/environment/codex_prompting_guide.md`
- `docs/environment/chatgpt_custom_instructions.md`
- `docs/environment/runtime_pack_workflow.md`

## Inventory
- `docs/inventory/file_ledger.md`
- `docs/inventory/file_ledger.json`
- `docs/inventory/scripts_inventory.md`
- `docs/inventory/skills_inventory.md`

## Usage
- `docs/usage/unreal_import.md`

## Reports
- `docs/reports/bundles/chatgpt_sdxl_bundle_2026-01-16/changes_and_notes.md`
- `docs/reports/bundles/chatgpt_sdxl_bundle_2026-01-16/chatgpt_prompt.md`
- `docs/reports/bundles/chatgpt_sdxl_bundle_2026-01-16/manifest.md`
- `docs/reports/bundles/chatgpt_sdxl_bundle_2026-01-16/run_results.md`
- `docs/reports/ml/sdxl_cpu_offload_report.md`
- `docs/reports/ml/sdxl_fp16_variant_report.md`
- `docs/reports/ml/local_ml_models_report.md`
- `docs/reports/ml/low_vram_policy_report.md`
- `docs/reports/ml/ml_generation_controls_report.md`
- `docs/reports/ml/progress_evaluation_2026-01-16_report.md`
- `docs/reports/ml/sdpa_performance_report.md`
- `docs/reports/ml/sdpa_pascal_slowdown_report.md`
- `docs/reports/ml/sdxl_stall_report.md`
- `docs/reports/ml/sdxl_improvement_notes_report.md`
- `docs/reports/ml/sm61_doctor_report.md`
- `docs/reports/ml/vram_troubleshooting_report.md`
- `docs/reports/runtime_packs/host_owned_diagnostics_report.md`
- `docs/reports/runtime_packs/runtime_pack_idle_timeout_baseline_report.md`
- `docs/reports/runtime_packs/runtime_pack_overview_report.md`
- `docs/reports/runtime_packs/runtime_pack_queued_stall_report.md`
- `docs/reports/runtime_packs/runtime_pack_troubleshooting_report.md`
- `docs/reports/runtime_packs/runtime_pack_troubleshooting_followup_report.md`
- `docs/reports/runtime_packs/runtime_io_contract_mismatch_report.md`
- `docs/reports/runtime_packs/runtime_pack_diagnostics_report.md`
- `docs/reports/runtime_packs/python_install_manager_conflict_report.md`
- `docs/reports/runtime_packs/upscaling_torch_priming_report.md`
- `docs/reports/runtime_packs/runtime_pack_review_report.md`
- `docs/reports/environment/python_build_issue_resolution_report.md`
- `docs/reports/environment/python_build_issue_alternatives_report.md`
- `docs/reports/seam_repair/seam_repair_duplicate_dispatch_report.md`
- `docs/reports/seam_repair/seam_repair_override_settings_report.md`
- `docs/reports/seam_repair/seam_repair_prepare_mask_timeout_report.md`
- `docs/reports/system/path_mapping_mismatch_report.md`
- `docs/reports/system/duplicate_outputs_report.md`
- `docs/reports/system/run_log_cross_filesystem_access_report.md`
- `docs/reports/system/wsl_cross_device_link_report.md`

## Commands
- `docs/commands/ml/sdxl_disk_audit_commands.md`
- `docs/commands/ml/sdxl_harness_commands.md`
- `docs/commands/ml/gguf_sidecar_checks.md`
- `docs/commands/ml/gguf_model_downloads.md`
- `docs/commands/ml/hf_cli_setup.md`
- `docs/commands/ml/gguf_flux_setup.md`
- `docs/commands/ml/diffusers_model_downloads.md`
- `docs/commands/ml/gguf_benchmark_commands.md`
- `docs/commands/workflow/required_checks_commands.md`
- `docs/commands/workflow/git_commit_commands.md`
- `docs/commands/runtime_packs/gguf_engine_setup.md`
- `docs/commands/runtime_packs/windows_local_host_commands.md`
- `docs/commands/runtime_packs/windows_packaging_commands.md`
- `docs/commands/runtime_packs/windows_vendor_pack_build_commands.md`
- `docs/commands/runtime_packs/windows_packaging_recover.md`
- `docs/commands/runtime_packs/windows_registry_update_commands.md`
- `docs/commands/runtime_packs/wsl_packaging_commands.md`
- `docs/commands/runtime_packs/wsl_registry_update_commands.md`
- `docs/commands/seam_repair/seam_repair_harness_command.md`

## Python project READMEs
- `tp-ml/README.md` — Real-ESRGAN sidecar setup and usage.
- `tp-ml-diffusers/README.md` — Diffusers sidecar setup and usage.
- `tp-ml-gguf/README.md` — GGUF/sd.cpp sidecar setup and usage.
- `docs/environment/runtime_pack_workflow.md` — Python version map for runtime packs.

## Archive
- `docs/archive/` (design, implementation, reports, and historical references)
