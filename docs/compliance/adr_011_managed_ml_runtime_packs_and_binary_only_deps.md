# ADR-011: Managed ML Runtime Packs and Binary-Only Dependencies
Updated on: 2026-01-21
Owners: JL · Agents
Depends on: ADR-000, ADR-006, ADR-007, ADR-010

## Context
TexturePortal's ML sidecars run in heterogeneous environments (Windows + WSL) where
Python/CUDA dependency drift and GPU support churn (notably xformers) cause
non-deterministic failures and user-facing setup pain. Ad-hoc pip installs have
produced mismatched CUDA runtimes, missing wheels, and costly manual debugging.
Sidecars also require different Python majors (e.g., Real-ESRGAN vs diffusers),
so unmanaged installs silently drift between incompatible versions.

We need a deterministic, app-managed way to provision ML runtimes that is
repeatable across machines and resilient to upstream packaging changes.

## Decision
TexturePortal will use **Managed ML Runtime Packs** as the authoritative way to
ship ML runtime dependencies, with **binary-only** installs enforced.

### D11.1 Managed runtime packs
- ML runtimes are distributed as versioned runtime packs (zip + manifest).
- Packs are installed into app data and selected by host os/arch/backend/gpu_class.
- Pack downloads are **hash-verified** prior to use.
- Runtime entrypoints are resolved from the pack manifest.

### D11.2 Binary-only dependencies
- Runtime packs are built from **prebuilt wheels only** (no source builds).
- A wheelhouse is generated per runtime pack; SDists are forbidden.
- Any missing wheel is a build failure (not a fallback to source compile).

### D11.3 Pinned runtime profiles
- Each pack is built from a pinned dependency spec (exact versions).
- Pinned profiles are the only approved way to introduce new runtime stacks.
- Each pack pins a **Python version**; mixed versions across packs are expected
  (e.g., upscaling vs diffusers) and must be explicit in the spec.

### D11.4 Python version map (current)
- `tp-ml` (Real-ESRGAN upscaling) packs use **Python 3.11**.
- `tp-ml-diffusers` packs use **Python 3.11**.
- Any new runtime pack must declare its Python major/minor explicitly in the spec.

## Consequences
### Positive
- Deterministic installs across machines and environments.
- Eliminates user-driven Python/CUDA mismatches.
- Supports GPU-class-specific stacks (e.g., Pascal vs modern).

### Tradeoffs
- Requires hosting pack zips + registry updates.
- Larger initial download size (but predictable).
- Slower initial install; fast subsequent runs.

## Enforcement
- **CI gate**: runtime pack build must be binary-only (fail on SDists).
- **Policy**: no ad-hoc `pip install` in production/dev runtimes; use pack build.
- **Specs**: new runtime stacks require a pinned pack spec + manifest update.
- **Registry**: pack downloads must include sha256 and be verified.
- **Python version**: runtime pack builds must declare and use the spec’s Python
  version; ambiguity is a failure. Build commands must select that interpreter
  explicitly (e.g., `TP_RUNTIME_PACK_PYTHON` on Windows).
- **Builder tooling**: when `py -<version>` is unavailable on Windows, uv may be
  used to provision the spec-matching interpreter (keeps exact version pinning
  without relying on launcher state).

## Notes
- Manual runtime overrides (`TEXTUREPORTAL_ML_RUNTIMES`) are legacy-only.
- New runtime features must be added via a managed pack spec and release.
