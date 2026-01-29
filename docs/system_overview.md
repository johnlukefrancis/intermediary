# TexturePortal System Overview
Updated on: 2026-01-16
Owners: JL · Agents
Depends on: ADR-000, ADR-006, ADR-007

## Goal
Turn a single texture into a seamless, PBR-ready map set with optional ML steps
(seam repair, generation, upscale) and real-time preview + tileability scoring.

## High-level structure
```
app/           # Vite frontend (Viewer, Tile Checker, Generation, Upscale)
src-tauri/     # Tauri shell + command bridge + ML orchestration
crates/
  textureportal_core/   # pure Rust image pipeline
  textureportal_ml_*    # ML protocol + client types
  textureportal_watch/  # Midjourney organizer CLI

(tp-ml/)          # Real-ESRGAN sidecar (upscale)
(tp-ml-diffusers/)# Diffusion sidecar (generation + seam repair)
(tp-ml-gguf/)     # GGUF sidecar (generation via sd.cpp)
```

## Core pipeline (deterministic)
1. Load input and convert to linear RGB.
2. Optional delight (auto/on/off).
3. Auto-parameter analysis for defaults.
4. Height → Normal → AO → Roughness → Metallic.
5. Pack ORM.
6. Write outputs to `<input_stem>_Output/` (async SaveQueue).

GPU acceleration is best-effort for blur + gradients and falls back to CPU.

## ML pipeline (sidecar-driven)
1. UI issues `ml_run_job` (request_id-safe).
2. Sidecar produces a BaseColor in a request-scoped scratch folder.
3. Rust derives maps from that BaseColor using the core pipeline.
4. SaveQueue writes outputs and emits `done`.

ML job types:
- Seam Repair (Tile Checker action)
- Generation (Generation tab)
- Upscale (Upscale tab)

## Tile Checker
- Repeats the latest BaseColor in a grid with pan/zoom.
- Computes Tileable % via Rust core analysis.
- Owns the Seam Repair action and shows before/after deltas.

## Preview
- Preview consumes the same output events and renders with WebGPU (preferred) or
  a Three.js fallback.
- Images are loaded via the `tp-out` protocol using IO-safe paths.

## Invariants
- All UI updates are request_id-safe.
- Deterministic core outputs for a fixed input + options.
- ML outputs never overwrite baseline maps in `*_Output/` folders.

## Related docs
- `docs/architecture/io_pipeline_architecture.md`
- `docs/architecture/ml_system_architecture.md`
- `docs/architecture/seam_repair_architecture.md`
- `docs/architecture/tile_checker_architecture.md`
- `docs/architecture/preview_window_architecture.md`
- `docs/architecture/gpu_backend_architecture.md`
