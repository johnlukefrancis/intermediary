# Windows Release Commands
Updated on: 2026-03-27
Owners: JL · Agents
Depends on: ADR-000, ADR-006, ADR-012

Windows is the only release automation target in scope today. The maintainer-validated release path is Windows 10/11 packaging with a bundled Windows `im_host_agent.exe` and a bundled Linux `im_agent` for WSL.

## Release Architecture

The release pipeline is intentionally split by binary authority:

1. Linux job builds `im_agent` because the bundled WSL backend must be a real Linux binary.
2. Windows job downloads that Linux artifact, builds `im_host_agent.exe`, and validates the bundle contract expected by `src-tauri/src/lib/agent/install.rs`.
3. Windows job packages the Tauri app and stages Windows release assets plus `.sha256` sidecars.
4. Tag-triggered release workflow publishes those staged Windows artifacts to GitHub Releases.

The version contract is also explicit: `package.json`, `src-tauri/tauri.conf.json`, the Rust crate versions, and `src-tauri/resources/agent_bundle/version.json` must all match before CI or release packaging can proceed.

## Required Checks

Run version validation first:

```bash
pnpm run version:check
```

Run the standard repo checks:

```bash
pnpm exec tsc --noEmit
pnpm exec eslint
cargo check
```

See [docs/commands/checks_local.md](checks_local.md) for the local checks companion.

## Bump The Release Version

Update every release-facing version file in one step:

```bash
pnpm run version:bump -- 0.1.1
```

Re-run the version check after the bump:

```bash
pnpm run version:check
```

## Build The Windows Release Locally

1. From WSL/Linux, build the bundled Linux `im_agent`:

```bash
bash ./scripts/build/build_agent_bundle.sh
```

2. Still from WSL/Linux, sync the updated repo tree into the Windows mirror so the Windows packaging step can see the refreshed `src-tauri/resources/agent_bundle/im_agent`:

```bash
./scripts/windows/sync_to_windows.sh
```

3. Then from the Windows mirror checkout, build the packaged app:

```powershell
pnpm install --frozen-lockfile
node scripts/build/ensure_agent_bundle.mjs
pnpm tauri build
node scripts/release/stage_windows_release_assets.mjs
```

Staged installer assets and checksum sidecars are written to:

`artifacts/windows-release/`

## Trigger The GitHub Release Workflow

After the version contract is bumped and committed, create and push a `v<version>` tag:

```bash
git tag v0.1.1
git push origin v0.1.1
```

That tag runs `.github/workflows/windows_release.yml`, which rebuilds the Linux helper, rebuilds the Windows helper, packages the Tauri app, generates `.sha256` sidecars, and publishes the Windows release artifacts to GitHub Releases.
