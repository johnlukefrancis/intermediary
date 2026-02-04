#!/usr/bin/env bash
# Path: scripts/build/build_agent_bundle.sh
# Description: Build and stage the Linux im_agent binary into Tauri agent_bundle resources.

set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "build_agent_bundle.sh must run in WSL/Linux so the Linux agent binary can be produced." >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
output_dir="${repo_root}/src-tauri/resources/agent_bundle"
temp_dir="${output_dir}.tmp"
binary_name="im_agent"
binary_path="${repo_root}/target/release/${binary_name}"

if [[ -s "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1090
  . "${HOME}/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not available in PATH" >&2
  exit 1
fi

version="$(sed -n 's/^[[:space:]]*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "${repo_root}/package.json" | head -n1)"
if [[ -z "${version}" ]]; then
  echo "package.json version is missing" >&2
  exit 1
fi

cargo build -p im_agent --bin im_agent --release

if [[ ! -f "${binary_path}" ]]; then
  echo "im_agent binary not found after build" >&2
  exit 1
fi

rm -rf "${temp_dir}"
mkdir -p "${temp_dir}"
cp "${binary_path}" "${temp_dir}/${binary_name}"
chmod 755 "${temp_dir}/${binary_name}"

cat > "${temp_dir}/version.json" <<EOF
{
  "version": "${version}"
}
EOF

rm -rf "${output_dir}"
mv "${temp_dir}" "${output_dir}"
