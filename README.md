# Intermediary

Intermediary is a desktop handoff console for agent-assisted local development. It watches the repos you care about, keeps recent docs and code close at hand, and builds timestamped context bundles you can drag into browser-based LLM tools without hunting through Explorer, Finder, or `\\wsl$`.

![Intermediary window](assets/readme/intermediary_window.png)

## Problem

Agent workflows break down when the "last mile" is manual: finding the right files, rebuilding a clean bundle, and proving that what you uploaded is the latest state from the repo you meant to share.

## Solution

Intermediary keeps that loop in one place:

- Watches configured repos and surfaces recently changed docs and code.
- Stages drag-and-drop-safe file copies on the host side.
- Builds zip bundles with a `BUNDLE_MANIFEST.json` provenance record.
- Preserves a bundle-first workflow for sharing local context with browser-based LLM tools.

## Screenshot

The screenshot above is tracked at [assets/readme/intermediary_window.png](assets/readme/intermediary_window.png).

## Workflow Companion

Intermediary does not integrate with ChatGPT directly and should not be described as an official integration. The intended workflow is still browser-first: build a bundle or drag staged files, then upload them into the tool you already use.

If you use a bundle-first prompting workflow, the companion doc is [docs/environment/chatgpt_custom_instructions.md](docs/environment/chatgpt_custom_instructions.md). Treat it as an optional workflow companion, not a hidden product requirement.

## Support Matrix

| Environment | Status | Notes |
| --- | --- | --- |
| Windows 10/11 + WSL2 | Maintainer-validated | Recommended path for the full workflow, including the bundled Linux WSL backend. |
| Windows 10/11 without WSL2 | Maintainer-validated | Validated for host-native repo workflows; the WSL companion/backend path does not apply. |
| macOS | Experimental | Code paths exist in places, but the maintainer has not validated macOS to the same standard as Windows. |
| Linux | Experimental | Code paths exist in places, but the maintainer has not validated Linux to the same standard as Windows. |

If you are evaluating the repo as a portfolio project, read "supported today" as "Windows 10/11," with WSL2 as the recommended path for the full handoff workflow.

## Install And Build

Intermediary is currently documented as a source-first project.

- Windows + WSL setup and daily development flow: [docs/commands/dev_windows.md](docs/commands/dev_windows.md)
- WSL backend agent workflow: [docs/commands/dev_wsl_agent.md](docs/commands/dev_wsl_agent.md)
- Closeout and verification commands: [docs/commands/workflow/closeout_checks.md](docs/commands/workflow/closeout_checks.md)

Prerequisites for the recommended Windows + WSL2 path:

- Windows 10 or 11
- WSL2
- Rust stable
- Node.js 20+
- pnpm

## Privacy

- Intermediary is designed for local-first use.
- The repo docs specify no telemetry by default.
- Bundle manifests include provenance such as repo ID, preset, timestamps, and best-effort git metadata so uploads are auditable.
- Repo access is scoped to the roots you configure inside the app; it is not a cloud sync tool.

## Known Limits

- Windows 10/11 is the maintainer-tested runtime today; WSL2 is recommended when you want the full WSL-backed workflow.
- Bundle sharing is drag-and-drop based; there is no direct ChatGPT API or official ChatGPT integration in the product.
- Very large or contended WSL bundle builds can still time out; see [docs/known_issues.md](docs/known_issues.md).
- Mounted Windows paths inside WSL can have degraded watcher reliability on large trees; the validated path is native WSL repos with Windows-hosted UI/runtime.

## Documentation

Start with [docs/guide.md](docs/guide.md) for the docs index, then read:

- [docs/system_overview.md](docs/system_overview.md) for architecture
- [docs/prd.md](docs/prd.md) for product intent and implementation scope
- [docs/known_issues.md](docs/known_issues.md) for current limitations

## License

This repository is licensed under the [MIT License](LICENSE).
