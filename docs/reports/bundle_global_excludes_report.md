# Bundle Global Excludes Report
Updated on: 2026-05-21
Owners: JL · Agents
Depends on: ADR-000, ADR-007, ADR-008, ADR-009, ADR-012

## Context

A TriangleRain context bundle omitted `Scripts/Build/Build-TriangleRainEditor.ps1`
while including other PowerShell scripts under `Scripts/Git` and `Scripts/Test`.
The manifest reported no user-selected `excludedSubdirs`, so the omission was not
explainable from bundle metadata.

## Model Verdict

The current behavior is a contract bug, not a file-extension or TriangleRain-specific
selection bug. Recommended bundle excludes are intended to be user-configurable defaults,
but the agent-side bundle builder re-adds the recommended baseline whenever the UI sends
an explicit `globalExcludes` payload. Because the scanner applies global directory names
case-insensitively to every directory segment, re-added `build` filters source/control
directories named `Build` even after the user removes that recommendation.

## Behavior Table

| Situation | Expected visible behavior |
| --- | --- |
| `globalExcludes` is omitted from a bundle request | Recommended global excludes seed the effective bundle scan. |
| `globalExcludes` is present and empty | The scan uses no user-configured global excludes. |
| `globalExcludes.dirNames` does not contain `build` | Directories named `Build` or `build` are bundled unless filtered by another explicit rule. |
| A global exclude filters a directory | `BUNDLE_MANIFEST.json` shows the effective global exclude state used for that scan. |

## Invariants

- The UI/persisted config owns recommended defaults for first-run and absent config.
- The agent mapper must preserve explicit user `globalExcludes` payloads exactly; scanner
  normalization is the only case/format normalization step.
- Recommended excludes are not hidden mandatory filters.
- No hard safety exclude is added by this fix; any future hard exclude needs a named policy
  and manifest/debug evidence.

## Actions

- Change the agent bundle mapper so absent config uses defaults and present config is
  passed through without merging recommended values.
- Add effective global exclude metadata to bundle manifests.
- Update regression coverage for omitted config, explicit empty config, and a source
  directory named `Scripts/Build`.
- Update docs and UI copy so “recommended” no longer implies “always applied.”

## References

- `crates/im_agent/src/bundles/bundle_builder_blocking.rs`
- `crates/im_bundle/src/global_excludes.rs`
- `crates/im_bundle/src/scanner.rs`
- `crates/im_bundle/src/manifest.rs`
- `app/src/shared/config/persisted_config.ts`
- `app/src/components/options_overlay.tsx`
