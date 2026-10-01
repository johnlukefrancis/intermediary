# Native macOS Development
Updated on: 2026-09-29
Owners: JL · Codex
Depends on: ADR-006, ADR-008, ADR-010, ADR-012

## Toolchain in this checkout

Native arm64 Node 22.23.3, pnpm 10.34.6 and Rust 1.98.1 are installed under
the ignored `target/devtools` directory. No shell startup file was modified.
Node and rustup downloads were checked against official SHA-256 checksums.
Sources: [Node](https://nodejs.org/dist/v22.23.3/SHASUMS256.txt),
[Rust](https://rust-lang.org/tools/install/).

The installed CLT defaults to SDK 27.0, whose stub architecture tags this
linker rejects. Select the installed 26.5 SDK for this build; do not change
the machine's global developer-directory selection.

```bash
cd /Users/johnlukefrancis/dev/intermediary
export CARGO_HOME="$PWD/target/devtools/cargo"
export RUSTUP_HOME="$PWD/target/devtools/rustup"
export PATH="$CARGO_HOME/bin:$PWD/target/devtools/node-v22.23.3-darwin-arm64/bin:$PWD/target/devtools/pnpm/bin:$PATH"
export TMPDIR="$PWD/target/devtools/tmp"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk
```

## Checks and app build

After activating the environment above:

```bash
pnpm exec tsc --noEmit
pnpm exec eslint app/src
cargo check
cargo test -p im_bundle fs_atomic
cargo test -p intermediary --lib terminal::
cargo test -p intermediary --lib macos_instance::
cargo test -p im_agent repos::import::
cargo test -p im_agent repos::worktree::
pnpm tauri build --bundles app
```

The pre-build route refreshes the native host agent and frontend. macOS bundles
only that host helper and version metadata, with local ad-hoc signing. The app
executes its packaged helper directly; it does not copy, chmod, remove quarantine
attributes, or replace an older helper in application data. Distribution to other
machines still requires a Developer ID/notarization release process. The app
bundle is `target/release/bundle/macos/Intermediary.app`. Local installation
and desktop witness receipts live in the macOS port Quest.

## Desktop terminal witness

Run these in the app's TERMINAL rail. They inspect the working directory,
native shell/profile tools, terminal identity and dimensions without starting
another agent. Paste exercises Command-V; use Control-C to interrupt the wait.

```zsh
pwd; printf 'shell=%s term=%s\n' "$SHELL" "$TERM"; command -v codex claude; stty size
sleep 30
```

## Repeated-launch witness

With the current release app already open, run this in its TERMINAL rail. The
second process must exit, retain the existing app/host, and leave the startup
log intact. The delayed form gives time to minimize the window before activation.

```zsh
'/Users/johnlukefrancis/dev/intermediary/target/release/bundle/macos/Intermediary.app/Contents/MacOS/intermediary'; printf 'duplicate launch exit=%s\n' "$?"
sleep 5; '/Users/johnlukefrancis/dev/intermediary/target/release/bundle/macos/Intermediary.app/Contents/MacOS/intermediary'; printf 'minimized launch exit=%s\n' "$?"
```

## Unsubmitted-input witness

Type each line into a fresh terminal without pressing Enter. Separately close
the tab, remove its fixture repository, and quit the app. None of the three
marker files may exist afterward. The fixture and markers remain in this project.

```zsh
printf submitted > /Users/johnlukefrancis/dev/intermediary/target/macos_witness/review_tab_marker
printf submitted > /Users/johnlukefrancis/dev/intermediary/target/macos_witness/review_repo_marker
printf submitted > /Users/johnlukefrancis/dev/intermediary/target/macos_witness/review_quit_marker
```

## Hands-on Finder import witness

The isolated fixture remains under
`/Users/johnlukefrancis/dev/intermediary/target/macos_witness/review_fixture`.
Add its `destination` folder with Intermediary's + repository button and open
the ZIPS rail. In Finder, drag `incoming/linked` into the blank FILES area;
repeat with its parent `incoming`. Each contains a real linked-worktree Git
pointer and must show IMPORT FAILED with a Git-control-metadata explanation,
without copying anything (the protocol error is INVALID_PATH). The
existing `destination/incoming/keep.txt` must remain unchanged. Remove the
fixture repository from Intermediary afterward; preserve its files.

Flat/nested imports and authorized ordinary-file replacement are covered by
the automated import checks above. The native Finder drop itself remains the
manual evidence gap. Include `scripts/build` when preparing a later external
review bundle of the packaging changes.
