#!/usr/bin/env bash
# Path: scripts/dev/wsl_agent_auth.sh
# Description: Resolve or bootstrap WSL backend websocket auth for dev launcher

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

generate_ws_token() {
  if command -v uuidgen >/dev/null 2>&1; then
    uuidgen | tr -d '-' | tr '[:upper:]' '[:lower:]'
    return 0
  fi

  if [[ -r /proc/sys/kernel/random/uuid ]]; then
    tr -d '-' < /proc/sys/kernel/random/uuid
    return 0
  fi

  od -An -N16 -tx1 /dev/urandom | tr -d ' \n'
}

create_ws_auth_file() {
  local auth_file="$1"
  local auth_dir=""
  auth_dir="$(dirname "${auth_file}")"
  mkdir -p "${auth_dir}"

  if [[ -f "${auth_file}" ]]; then
    return 0
  fi

  local host_token=""
  local wsl_token=""
  host_token="$(generate_ws_token)"
  wsl_token="$(generate_ws_token)"

  local temp_file=""
  temp_file="$(mktemp "${auth_dir}/ws_auth.json.tmp.XXXXXX")"
  printf '{"hostWsToken":"%s","wslWsToken":"%s"}\n' "${host_token}" "${wsl_token}" >"${temp_file}"
  if ln "${temp_file}" "${auth_file}" 2>/dev/null; then
    rm -f "${temp_file}"
    echo "Created websocket auth state at ${auth_file}" >&2
    return 0
  fi

  rm -f "${temp_file}"
  if [[ -f "${auth_file}" ]]; then
    return 0
  fi

  echo "Failed to create websocket auth state at ${auth_file}" >&2
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

  local explicit_app_id="${INTERMEDIARY_WS_AUTH_APP_ID:-}"
  if [[ -n "${explicit_app_id}" ]]; then
    local explicit_candidate="${local_app_data_wsl}/${explicit_app_id}/agent/ws_auth.json"
    if [[ ! -f "${explicit_candidate}" ]]; then
      create_ws_auth_file "${explicit_candidate}"
    fi
    printf '%s\n' "${explicit_candidate}"
    return 0
  fi

  local candidate
  for candidate in \
    "${local_app_data_wsl}/com.johnf.intermediary/agent/ws_auth.json" \
    "${local_app_data_wsl}/Intermediary/agent/ws_auth.json" \
    "${local_app_data_wsl}/com.johnf.intermediary.dev/agent/ws_auth.json"; do
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
    echo "ws_auth.json not found (checked INTERMEDIARY_WS_AUTH_FILE, INTERMEDIARY_WS_AUTH_APP_ID, INTERMEDIARY_WINDOWS_LOCALAPPDATA, cmd.exe %LOCALAPPDATA%, and /mnt/c/Users/<user>/AppData/Local fallback); falling back to dev token" >&2
  fi

  printf '%s\n' "im_dev_wsl_token"
}
