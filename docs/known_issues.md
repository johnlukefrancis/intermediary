# Known Issues — TexturePortal
Updated on: 2026-01-21
Owners: JL · Agents
Depends on: ADR-000, ADR-006, ADR-007

## Ground rules (keep this file tiny)
- Log only what is observed; do not add theories or suspected causes.
- Each issue must be categorized P0–P3 by disruption (P0 = core workflow blocked).
- Group issues by system; keep entries short.

## P0 — Core workflow blocked

## P1 — Major functionality broken

## P2 — Degraded but usable
### Managed Runtime Packs
- Runtime pack install can take a long time (over 40 minutes to an hour) with no progress indicator (appears stalled during large zip unpack + venv install).

## P3 — Minor issues
### Advanced Options
- The “Disable attention slicing” label is confusing (renamed to “Attention slicing” in UI), but further wording polish may be needed.

## Resolved (recent)
- Seam repair mode toggle removed; seam repair is now an explicit action only. (2026-01-21)
- Top nav status line uses concise seam repair labels instead of verbose stall/no-step messaging. (2026-01-21)
- Seam repair and upscale nav timers now show real elapsed time (smooth updates). (2026-01-21)
- Seam repair retry attempts now emit denoising step progress during retries, so the top nav keeps step counts. (2026-01-21)
- Seam repair denoising progress now matches requested steps (strength-aware step mapping). (2026-01-21)
- SDXL seam repair on Pascal stalling at step 0 due to fp32 load headroom (fixed by torch_dtype passthrough + dtype logging). (2026-01-17)
- Advanced options overrides now propagate into WSL sidecars and auto-apply when idle. (2026-01-17)
