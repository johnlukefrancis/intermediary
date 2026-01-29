# TexturePortal — Roadmap
Updated on: 2026-01-17
Owners: JL · Agents
Depends on: ADR-000, ADR-006, ADR-007

---

## Snapshot
- **Lines of code:** ≈7.8k TS (app), ≈12.1k Rust (src-tauri + crates), ≈3.6k Python (tp-ml + tp-ml-diffusers) — wc 2026-01-16.
- **Latest milestone:** Seam repair stabilization + UI controls hardening (SDPA defaults, doctor fixes, advanced menu, fp16-only).

## Recently Shipped
- Seam repair quality gate + retry strategy (baseline vs candidate tileability).
- Seam repair output suffixing in `*_Output/` to avoid clobbering baseline maps.
- Mask semantics clarified: `band_px` now means pixels per edge.
- Inpaint tuning pass (strength, lower guidance).
- SDPA toggles + attention processor options in diffusers sidecar.
- Advanced options menu (SDPA cycle + diffusers toggles).
- Sampling controls for Generation + Seam Repair (steps/CFG/sampler).
- Per-attempt ML logs (diffusers stderr + job context).
- fp16-only SDXL weights (fp32 removed).
- Doctor diagnostics: arch list + SDPA backend flags + sm_61 compatibility note.
- HTML shell composition: index.html now composed from partials.

## Active Initiatives

┌────────────────────────────────────────────────────────────────────────────┐
│ Initiative: ML Runtime Stability (Seam Repair)                             │
│ Status: Tail end — hardening + environment correctness                     │
│ Workstreams:                                                               │
│  • Default SDPA=no_cudnn on Pascal when env unset.                         │
│  • Doctor diagnostics: arch list, torch path, SDPA backend flags.          │
│  • Harness repeat runs with summary report + timeout handling.             │
│  • Regression set for tileability (HF materials + directional patterns).   │
│  • Verify sm_61 toolchain situation; avoid cudnn kernel path on Pascal.    │
│  • Model load indicator on model button (shows loaded in VRAM).            │
│ Next Milestones:                                                           │
│  • Run 25x repeat harness across SDPA modes; log percentiles + errors.     │
│  • Build 5–10 texture regression set and lock in quality gate thresholds.  │
│  • Ship model loaded indicator UX (LOADED badge).                          │
│ Key Docs:                                                                  │
│  • docs/architecture/seam_repair_architecture.md                           │
│  • docs/reports/chatgpt_sdxl_bundle_2026-01-16/run_results.md              │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│ Initiative: Managed Runtime Packs (Pascal)                                 │
│ Status: Active — packaging + hosting path                                  │
│ Workstreams:                                                               │
│  • Build Pascal (sm_61) CUDA 11.8 diffusers pack from wheels only.         │
│  • Zip + sha256 + registry update tooling.                                 │
│  • Decide hosting target (GitHub Releases for early builds).               │
│ Next Milestones:                                                           │
│  • Publish pack zip and update registry with final URL + sha.              │
│  • Document hosting/registry lifecycle in architecture docs.               │
│ Key Docs:                                                                  │
│  • docs/architecture/ml_runtimes_architecture.md                           │
│  • docs/reports/managed_runtime_packs_report.md                            │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│ Initiative: ML Controls & Advanced Options UX                              │
│ Status: Shipped                                                            │
│ Workstreams:                                                               │
│  • Advanced Options menu (SDPA/attention toggles, global ML flags).        │
│  • Sampling controls (CFG, steps, sampler) for Generation + Seam Repair.   │
│  • Persist settings; request_id-safe updates; no auto-run.                 │
│ Next Milestones:                                                           │
│  • None (shipped).                                                         │
│ Key Docs:                                                                  │
│  • docs/design/ml_advanced_options_menu_design.md                          │
│  • docs/implementation/ml_advanced_options_menu_implementation.md          │
│  • docs/design/ml_sampling_controls_design.md                              │
│  • docs/implementation/ml_sampling_controls_implementation.md              │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│ Initiative: Generation v1 (BaseColor → PBR Derive)                         │
│ Status: Next                                                               │
│ Workstreams:                                                               │
│  • Sidecar generation recipe outputs BaseColor only.                       │
│  • Rust derives maps + SaveQueue writes outputs (single pipeline).         │
│  • Failure UX: OOM and model errors -> user-readable guidance.             │
│  • Tileability readout on generated results.                               │
│ Next Milestones:                                                           │
│  • Implement generation recipe + contract wiring.                          │
│  • Integrate sampling controls into generation run.                        │
│  • First usable generation pass + stability sweep.                         │
│ Key Docs:                                                                  │
│  • docs/architecture/ml_system_architecture.md                             │
│  • docs/architecture/tile_checker_architecture.md                          │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│ Initiative: Output Library / Browser                                       │
│ Status: Queued                                                             │
│ Workstreams:                                                               │
│  • Index output sets (derive/seam/upscale/gen) with metadata + thumbnails. │
│  • Filter/sort by job type, model, tileability, timestamp.                 │
│  • Lightweight local index only (no cloud).                                │
│ Next Milestones:                                                           │
│  • Define output-set schema and storage strategy.                          │
│  • Build minimal browser panel UI.                                         │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│ Initiative: Custom Model Viewer (Preview-Only)                             │
│ Status: Queued                                                             │
│ Workstreams:                                                               │
│  • Load glTF/GLB and apply current output set maps.                        │
│  • Orbit/zoom, material sanity checks, no painting.                        │
│  • Respect tp-out path scoping / safe asset loading.                       │
│ Next Milestones:                                                           │
│  • Minimal viewer wiring + model import validation.                        │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│ Initiative: ComfyUI Workflows (Optional Runtime)                           │
│ Status: Future                                                             │
│ Workstreams:                                                               │
│  • Treat Comfy as optional runtime with capability IDs.                    │
│  • Keep TexturePortal in control of outputs + derive pipeline.             │
│  • Avoid embedding full Comfy UI.                                          │
│ Next Milestones:                                                           │
│  • Draft runtime contract + capability list.                               │
│  • Spike a single workflow recipe for BaseColor output.                    │
└────────────────────────────────────────────────────────────────────────────┘

## Priority Order (Near-Term)
1) ML runtime stability + env correctness (sm_61, SDPA defaults, repeat harness)
2) Generation v1 (BaseColor → derive maps)
3) Output library / browser
4) Custom model viewer (preview only)
5) ComfyUI workflows (optional runtime)
