#!/usr/bin/env bash
# Path: scripts/dev/run_wsl_agent_dev.sh
# Description: Launch the WSL backend agent for dev unless it is already listening.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
port="${INTERMEDIARY_AGENT_PORT:-3142}"
ready_timeout_seconds="${INTERMEDIARY_AGENT_READY_TIMEOUT_SECONDS:-30}"

emit_ready_marker() {
  echo "INTERMEDIARY_WSL_AGENT_READY port=${port}"
}

resolve_windows_cmd_exe() {
  if command -v cmd.exe >/dev/null 2>&1; then
    local cmd_from_path=""
    cmd_from_path="$(command -v cmd.exe 2>/dev/null || true)"
    if [[ -n "${cmd_from_path}" && -x "${cmd_from_path}" ]]; then
      printf '%s\n' "${cmd_from_path}"
      return 0
    fi
  fi

  local candidate
  for candidate in /mnt/c/Windows/System32/cmd.exe /mnt/c/windows/system32/cmd.exe; do
    if [[ -x "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  return 1
}

resolve_windows_local_app_data() {
  if [[ -n "${INTERMEDIARY_WINDOWS_LOCALAPPDATA:-}" ]]; then
    printf '%s\n' "${INTERMEDIARY_WINDOWS_LOCALAPPDATA}"
    return 0
  fi

  local cmd_exe=""
  cmd_exe="$(resolve_windows_cmd_exe || true)"
  if [[ -z "${cmd_exe}" ]]; then
    return 1
  fi

  local raw_local_app_data=""
  raw_local_app_data="$(
    "${cmd_exe}" /C "echo %LOCALAPPDATA%" 2>/dev/null \
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

  if [[ "${windows_path}" == /* ]]; then
    printf '%s\n' "${windows_path}"
    return 0
  fi

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

resolve_local_app_data_from_user_fallback() {
  local windows_username="${INTERMEDIARY_WINDOWS_USERNAME:-}"
  local wsl_username="${USER:-}"
  local windows_username_title=""
  local wsl_username_title=""
  if [[ -n "${windows_username}" ]]; then
    windows_username_title="${windows_username^}"
  fi
  if [[ -n "${wsl_username}" ]]; then
    wsl_username_title="${wsl_username^}"
  fi

  local user_candidate
  for user_candidate in \
    "${windows_username}" \
    "${windows_username_title}" \
    "${wsl_username}" \
    "${wsl_username_title}"; do
    if [[ -z "${user_candidate}" ]]; then
      continue
    fi

    local candidate="/mnt/c/Users/${user_candidate}/AppData/Local"
    if [[ -d "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

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
  local local_app_data_wsl=""
  if [[ -n "${local_app_data_win}" ]]; then
    local_app_data_wsl="$(windows_path_to_wsl "${local_app_data_win}" || true)"
  fi
  if [[ -z "${local_app_data_wsl}" ]]; then
    local_app_data_wsl="$(resolve_local_app_data_from_user_fallback || true)"
  fi
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
    echo "ws_auth.json not found (checked INTERMEDIARY_WS_AUTH_FILE, INTERMEDIARY_WINDOWS_LOCALAPPDATA, cmd.exe %LOCALAPPDATA%, and /mnt/c/Users/<user>/AppData/Local fallback); falling back to dev token" >&2
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
  emit_ready_marker
  exit 0
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
  if is_port_listening; then
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
