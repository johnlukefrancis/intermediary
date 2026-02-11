#!/usr/bin/env bash
# Path: scripts/dev/run_wsl_agent_dev.sh
# Description: Launch the WSL backend agent for dev unless it is already listening.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
port="${INTERMEDIARY_AGENT_PORT:-3142}"

resolve_windows_local_app_data() {
  if [[ -n "${INTERMEDIARY_WINDOWS_LOCALAPPDATA:-}" ]]; then
    printf '%s\n' "${INTERMEDIARY_WINDOWS_LOCALAPPDATA}"
    return 0
  fi

  if ! command -v cmd.exe >/dev/null 2>&1; then
    return 1
  fi

  local raw_local_app_data=""
  raw_local_app_data="$(
    cmd.exe /C "echo %LOCALAPPDATA%" 2>/dev/null \
      | tr -d '\r' \
      | tail -n 1
  )"
  if [[ -z "${raw_local_app_data}" || "${raw_local_app_data}" == "%LOCALAPPDATA%" ]]; then
    return 1
  fi

  printf '%s\n' "${raw_local_app_data}"
}

windows_path_to_wsl() {
  local windows_path="$1"

  if command -v wslpath >/dev/null 2>&1; then
    local via_wslpath=""
    via_wslpath="$(wslpath -u "${windows_path}" 2>/dev/null || true)"
    if [[ -n "${via_wslpath}" ]]; then
      printf '%s\n' "${via_wslpath}"
      return 0
    fi
  fi

  local normalized="${windows_path//\\//}"
  if [[ "${normalized}" =~ ^([A-Za-z]):/(.*)$ ]]; then
    local drive="${BASH_REMATCH[1],,}"
    local suffix="${BASH_REMATCH[2]}"
    printf '/mnt/%s/%s\n' "${drive}" "${suffix}"
    return 0
  fi

  return 1
}

resolve_ws_auth_file() {
  local explicit_auth_file="${INTERMEDIARY_WS_AUTH_FILE:-}"
  if [[ -n "${explicit_auth_file}" && -f "${explicit_auth_file}" ]]; then
    printf '%s\n' "${explicit_auth_file}"
    return 0
  fi

  local local_app_data_win=""
  local_app_data_win="$(resolve_windows_local_app_data || true)"
  if [[ -z "${local_app_data_win}" ]]; then
    return 1
  fi

  local local_app_data_wsl=""
  local_app_data_wsl="$(windows_path_to_wsl "${local_app_data_win}" || true)"
  if [[ -z "${local_app_data_wsl}" ]]; then
    return 1
  fi

  local candidate
  for candidate in \
    "${local_app_data_wsl}/com.johnf.intermediary/agent/ws_auth.json" \
    "${local_app_data_wsl}/Intermediary/agent/ws_auth.json"; do
    if [[ -f "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  return 1
}

extract_wsl_ws_token() {
  local auth_file="$1"
  local token
  token="$(
    grep -Eo '"wslWsToken"[[:space:]]*:[[:space:]]*"[^"]+"' "${auth_file}" \
      | head -n 1 \
      | sed -E 's/.*:[[:space:]]*"([^"]+)"/\1/'
  )"
  if [[ -z "${token}" ]]; then
    return 1
  fi
  printf '%s\n' "${token}"
}

resolve_wsl_ws_token() {
  if [[ -n "${INTERMEDIARY_WSL_WS_TOKEN:-}" ]]; then
    printf '%s\n' "${INTERMEDIARY_WSL_WS_TOKEN}"
    return 0
  fi

  local auth_file=""
  auth_file="$(resolve_ws_auth_file || true)"
  if [[ -n "${auth_file}" ]]; then
    local resolved_token=""
    resolved_token="$(extract_wsl_ws_token "${auth_file}" || true)"
    if [[ -n "${resolved_token}" ]]; then
      echo "Using INTERMEDIARY_WSL_WS_TOKEN from ${auth_file}" >&2
      printf '%s\n' "${resolved_token}"
      return 0
    fi
    echo "Could not parse wslWsToken in ${auth_file}; falling back to dev token" >&2
  else
    echo "ws_auth.json not found; falling back to dev token" >&2
  fi

  printf '%s\n' "im_dev_wsl_token"
}

ws_token="$(resolve_wsl_ws_token)"

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
INTERMEDIARY_AGENT_PORT="${port}" INTERMEDIARY_WSL_WS_TOKEN="${ws_token}" cargo run -p im_agent --bin im_agent
