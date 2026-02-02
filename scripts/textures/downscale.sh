#!/usr/bin/env bash
# Path: scripts/textures/downscale.sh
# Description: Downscale app/assets/textures/*.png to 256x256 for substrate textures

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEXTURE_DIR="$REPO_ROOT/app/assets/textures"
TMP_FILE="$REPO_ROOT/app/assets/.texture_downscaled.tmp.png"

if [[ ! -d "$TEXTURE_DIR" ]]; then
  echo "Error: Texture folder not found: $TEXTURE_DIR"
  exit 1
fi

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "Error: ffmpeg not found in PATH"
  exit 1
fi

if ! command -v ffprobe >/dev/null 2>&1; then
  echo "Error: ffprobe not found in PATH"
  exit 1
fi

shopt -s nullglob
TEXTURES=("$TEXTURE_DIR"/*.png)
shopt -u nullglob

if [[ ${#TEXTURES[@]} -eq 0 ]]; then
  echo "No textures found in $TEXTURE_DIR"
  exit 0
fi

echo "Scanning textures in $TEXTURE_DIR..."

for TEXTURE in "${TEXTURES[@]}"; do
  DIMENSIONS="$(ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=p=0:s=x "$TEXTURE" || true)"
  if [[ -z "$DIMENSIONS" ]]; then
    echo "Warning: Unable to read dimensions for $TEXTURE"
    continue
  fi

  if [[ "$DIMENSIONS" == "256x256" ]]; then
    echo "OK: $(basename "$TEXTURE") already 256x256"
    continue
  fi

  echo "Downscaling $(basename "$TEXTURE") ($DIMENSIONS -> 256x256)..."

  # Use explicit png extension for temp file
  ffmpeg -y -i "$TEXTURE" -vf "scale=256:256:flags=lanczos" "$TMP_FILE" 2>/dev/null || true
  if [[ -f "$TMP_FILE" ]]; then
    mv "$TMP_FILE" "$TEXTURE"
    echo "Done: $(basename "$TEXTURE")"
  else
    echo "Error: ffmpeg failed to create output for $TEXTURE"
    exit 1
  fi
done
