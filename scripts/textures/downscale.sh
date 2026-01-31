#!/usr/bin/env bash
# Path: scripts/textures/downscale.sh
# Description: Downscale app/assets/noise.png to 256x256 for substrate texture

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEXTURE="$REPO_ROOT/app/assets/noise.png"
TMP_FILE="$REPO_ROOT/app/assets/noise_downscaled.png"

if [[ ! -f "$TEXTURE" ]]; then
  echo "Error: Texture not found: $TEXTURE"
  exit 1
fi

echo "Downscaling noise.png to 256x256..."

# Use explicit png extension for temp file
ffmpeg -y -i "$TEXTURE" -vf "scale=256:256:flags=lanczos" "$TMP_FILE" 2>/dev/null || true

if [[ -f "$TMP_FILE" ]]; then
  mv "$TMP_FILE" "$TEXTURE"
  echo "Done: app/assets/noise.png (256x256)"
else
  echo "Error: ffmpeg failed to create output"
  exit 1
fi
