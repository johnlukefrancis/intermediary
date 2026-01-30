#!/usr/bin/env bash
set -euo pipefail

SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
if [[ -n "${INTERMEDIARY_WIN_PATH:-}" ]]; then
  DEST_DIR="${INTERMEDIARY_WIN_PATH}"
elif [[ -d "/mnt/d" ]]; then
  DEST_DIR="/mnt/d/code/intermediary"
else
  DEST_DIR="/mnt/c/code/intermediary"
fi

if ! command -v rsync >/dev/null 2>&1; then
  echo "rsync not found. Install it with: sudo apt install -y rsync" >&2
  exit 1
fi

if ! command -v inotifywait >/dev/null 2>&1; then
  echo "inotifywait not found. Install it with: sudo apt install -y inotify-tools" >&2
  exit 1
fi

mkdir -p "$DEST_DIR"

sync_once() {
  rsync -a --delete \
    --exclude ".git" \
    --exclude "node_modules" \
    --exclude "target" \
    --exclude "dist" \
    --exclude "dist-ssr" \
    --exclude "logs" \
    --exclude "src-tauri/target" \
    --exclude "agent/.venv" \
    --exclude "agent/__pycache__" \
    --exclude "scripts/zip/output" \
    "$SOURCE_DIR/" "$DEST_DIR/"
}

sync_once

echo "Watching $SOURCE_DIR -> $DEST_DIR"
echo "Excluding: .git node_modules target dist dist-ssr logs src-tauri/target agent/.venv agent/__pycache__ scripts/zip/output"
while inotifywait -r -e modify,create,delete,move \
  --exclude "(\\.git|node_modules|dist|dist-ssr|target|logs|src-tauri/target|agent/\\.venv|__pycache__|scripts/zip/output)" \
  "$SOURCE_DIR" >/dev/null 2>&1; do
  sync_once
  echo "Synced at $(date +%H:%M:%S)"
  sleep 0.1
done
