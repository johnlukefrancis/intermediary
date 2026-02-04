#!/usr/bin/env bash
# Path: scripts/dev/run_wsl_agent_dev.sh
# Description: Launch the WSL agent for dev unless it is already listening.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
port="${INTERMEDIARY_AGENT_PORT:-3141}"

is_port_listening() {
  if (echo >"/dev/tcp/127.0.0.1/${port}") >/dev/null 2>&1; then
    return 0
  fi
  if command -v ss >/dev/null 2>&1; then
    ss -lnt 2>/dev/null | grep -q "[[:space:]]:${port}[[:space:]]"
    return $?
  fi
  return 1
}

if is_port_listening; then
  echo "WebSocket server started (already running on ${port})"
  exit 0
fi

cd "${repo_root}"
cargo run -p im_agent --bin im_agent
