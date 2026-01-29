#!/usr/bin/env bash
set -euo pipefail

SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if [[ -n "${TEXTUREPORTAL_WIN_PATH:-}" ]]; then
  DEST_DIR="${TEXTUREPORTAL_WIN_PATH}"
elif [[ -d "/mnt/d" ]]; then
  DEST_DIR="/mnt/d/code/textureportal"
else
  DEST_DIR="/mnt/c/code/textureportal"
fi

if ! command -v rsync >/dev/null 2>&1; then
  echo "rsync not found. Install it with: sudo apt install -y rsync" >&2
  exit 1
fi

mkdir -p "$DEST_DIR"

rsync -a --delete \
  --exclude ".git" \
  --exclude "node_modules" \
  --exclude "target" \
  --exclude "dist" \
  --exclude "dist-ssr" \
  --exclude "logs" \
  --exclude "src-tauri/target" \
  --exclude "tp-ml/.venv" \
  --exclude "tp-ml/models" \
  --exclude "tp-ml/**/__pycache__" \
  --exclude "tp-ml-diffusers/.venv" \
  --exclude "tp-ml-diffusers/models" \
  --exclude "tp-ml-diffusers/.cache" \
  --exclude "tp-ml-diffusers/**/__pycache__" \
  "$SOURCE_DIR/" "$DEST_DIR/"

echo "Synced to $DEST_DIR"
