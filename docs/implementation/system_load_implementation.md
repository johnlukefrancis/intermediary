# System Load Monitor Quest
Updated on: 2026-10-12
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

## Intent and witness

Put Task-Manager-grade CPU, memory, GPU, disk and network load into the status strip, with
hover panels naming the top processes, so JL can read machine pressure without opening Task
Manager. Witness the installed Windows app, merge into master, rebuild the installer, install
silently and relaunch for JL. Design: `docs/design/system_load_design.md`.

## Owner decisions

- New Tauri-free crate `crates/im_system_load` owns OS telemetry: one
  `NtQuerySystemInformation` process walk, one PDH query (English counter names) and DXGI
  adapter totals. The Tauri host holds one sampler and exposes `sample_system_load`.
- The frontend hook owns cadence: one request a second after the previous resolves, only
  while the document is visible. No background thread or event channel.
- Process groups are keyed by image name and labelled with the executable FileDescription.

## Live receipt

- Identity: worktree /home/johnf/dev/intermediary-sysload on branch feat/system-load from
  master 68951c2. Master holds codex@intermediary#72c2's uncommitted file-launch work, which
  must be preserved; overlap is src-tauri/Cargo.toml, docs/guide.md, docs/changelog.md and the
  ledger.
- Real-machine probe (disposable crate outside the repo, release): `Sampler::new` 220 ms,
  `sample` 15–19 ms; CPU, memory, two DXGI adapters, 8 disks, 7 interfaces and labelled
  groups agree with the Task Manager parity probe. Per-process `\GPU Process Memory`
  Dedicated Usage read 26 GB for `dwm` on a 16 GB card, so process VRAM uses Local Usage.
- Checks: tsc, eslint, `pnpm run test:ts` (147 pass), version contract, `cargo test` and
  clippy for `im_system_load` (Linux and msvc target), Windows `cargo check -p intermediary`
  via the separate mirror E:\Code\intermediary-sysload.
- Landed 8775f7b on master (fast-forward; #72c2's dirty overlap edits verified byte-identical
  after the union). JL then asked for less: no numbers on the strip, severity-tinted bars, and a
  lean hover (headline, verdict, slim trend, top programs). The simplification removes the
  per-core, commit, engine, per-disk and per-interface breakdowns.
- Frontier: commit the simplification, fast-forward master, build, install, witness.
- Evidence root: ignored target/system_load_witness in the master tree.
