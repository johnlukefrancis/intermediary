#!/usr/bin/env bash
# Path: scripts/dev/run_wsl_agent_dev.sh
# Description: Launch the WSL backend agent for dev unless it is already listening.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
port="${INTERMEDIARY_AGENT_PORT:-3142}"
ready_timeout_seconds="${INTERMEDIARY_AGENT_READY_TIMEOUT_SECONDS:-120}"

source "${repo_root}/scripts/dev/wsl_agent_auth.sh"

emit_ready_marker() {
  echo "INTERMEDIARY_WSL_AGENT_READY port=${port}"
}

emit_begin_marker() {
  echo "INTERMEDIARY_WSL_AGENT_BEGIN port=${port}"
}

ws_token="$(resolve_wsl_ws_token)"
emit_begin_marker

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

probe_websocket_auth() {
  local status_line=""
  exec 3<>"/dev/tcp/127.0.0.1/${port}" || return 1
  printf 'GET /?token=%s HTTP/1.1\r\nHost: 127.0.0.1:%s\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n' "${ws_token}" "${port}" >&3 || {
    exec 3>&- 3<&-
    return 1
  }
  IFS= read -r status_line <&3 || {
    exec 3>&- 3<&-
    return 1
  }
  exec 3>&- 3<&-
  [[ "${status_line}" == *" 101 "* ]]
}

list_listener_pid() {
  command -v ss >/dev/null 2>&1 || return 1
  ss -ltnp "( sport = :${port} )" 2>/dev/null \
    | awk 'NR>1 {print $NF}' \
    | sed -E 's/.*pid=([0-9]+).*/\1/' \
    | head -n 1
}

pid_is_intermediary_agent_listener() {
  local pid="$1"
  local comm=""
  comm="$(cat "/proc/${pid}/comm" 2>/dev/null || true)"
  if [[ "${comm}" != "im_agent" ]]; then
    return 1
  fi

  local env_lines=""
  env_lines="$(tr '\0' '\n' <"/proc/${pid}/environ" 2>/dev/null || true)"
  [[ $'\n'"${env_lines}"$'\n' == *$'\n'"INTERMEDIARY_AGENT_PORT=${port}"$'\n'* ]] || return 1
  [[ $'\n'"${env_lines}"$'\n' == *$'\n'"INTERMEDIARY_WSL_WS_TOKEN="* ]] || return 1
}

retire_stale_listener() {
  local listener_pid=""
  listener_pid="$(list_listener_pid || true)"
  if [[ -z "${listener_pid}" ]]; then
    return 1
  fi
  if ! pid_is_intermediary_agent_listener "${listener_pid}"; then
    echo "Listener on ${port} is not an Intermediary WSL agent; refusing to terminate it" >&2
    return 1
  fi

  echo "Retiring stale WSL agent listener on ${port} (pid=${listener_pid})" >&2
  kill "${listener_pid}" >/dev/null 2>&1 || true
  local deadline=$((SECONDS + 5))
  while (( SECONDS < deadline )); do
    if ! is_port_listening; then
      return 0
    fi
    sleep 0.1
  done

  kill -9 "${listener_pid}" >/dev/null 2>&1 || true
  sleep 0.2
  ! is_port_listening
}

if is_port_listening; then
  if probe_websocket_auth; then
    echo "WebSocket server started (already running on ${port})"
    emit_ready_marker
    exit 0
  fi

  echo "WSL agent listener on ${port} rejected current websocket token; replacing stale listener" >&2
  if ! retire_stale_listener; then
    echo "Could not retire stale WSL agent listener on ${port}" >&2
    exit 1
  fi
fi

cd "${repo_root}"
echo "INTERMEDIARY_WSL_AGENT_STARTING port=${port}"
INTERMEDIARY_AGENT_PORT="${port}" INTERMEDIARY_WSL_WS_TOKEN="${ws_token}" cargo run -p im_agent --bin im_agent &
agent_pid=$!
ready_emitted=0

cleanup() {
  if [[ "${ready_emitted}" -eq 0 ]] && kill -0 "${agent_pid}" >/dev/null 2>&1; then
    kill "${agent_pid}" >/dev/null 2>&1 || true
  fi
}

trap cleanup EXIT INT TERM

deadline=$((SECONDS + ready_timeout_seconds))
while (( SECONDS < deadline )); do
  if is_port_listening && probe_websocket_auth; then
    emit_ready_marker
    ready_emitted=1
    break
  fi

  if ! kill -0 "${agent_pid}" >/dev/null 2>&1; then
    wait "${agent_pid}"
    exit $?
  fi

  sleep 0.1
done

if [[ "${ready_emitted}" -eq 0 ]]; then
  echo "Timed out waiting for WSL agent to listen on ${port}" >&2
  wait "${agent_pid}"
  exit 1
fi

wait "${agent_pid}"
