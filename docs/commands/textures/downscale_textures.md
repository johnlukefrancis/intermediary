# Downscale Theme Textures
Updated on: 2026-02-02
Owners: JL · Agents
Depends on: ADR-012

Downscale all theme textures in `app/assets/textures` to 256x256.

## Command

```bash
scripts/textures/downscale.sh
```

## Prerequisites

- `ffmpeg` + `ffprobe` available in PATH.

## Expected output

- Any texture not already 256x256 will be resized in-place.
- Console output lists each texture scanned and any conversions performed.
