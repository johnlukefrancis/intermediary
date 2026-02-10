# Compact Mode Report
Updated on: 2026-02-10
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007

## Context

This report previously planned a three-mode system:
- `standard`
- `compact`
- `handset`

Product direction is now intentionally simpler:
- Keep `standard` as the canonical desktop layout.
- Keep `handset` as the single narrow vertical layout.
- Remove `compact` entirely.

## Decision

`compact` is retired as a first-class mode.

Canonical mode set moving forward:
- `standard`
- `handset`

Backward compatibility contract:
- Legacy `uiMode: "compact"` must coerce to `standard`.
- Legacy `uiState.windowBoundsByMode.compact` must be migrated into
  `uiState.windowBoundsByMode.standard` when `standard` is absent.
- Persisted config should no longer emit a `compact` mode key.

## Defunct Work Removed

The following scope is removed from the ladder:
- Compact-density specific CSS pass keyed on `.app[data-ui-mode="compact"]`.
- Any “compact handset” combination behavior.
- Any UI control copy or iconography dedicated to `compact`.

## Updated Ladder

1. Mode contract collapse (TS + Rust)
- Reduce `UiMode` to `standard | handset`.
- Keep legacy compact parsing/migration compatibility.
- Bump config schema version and maintain TS/Rust parity.

2. Runtime + persistence migration
- Resolve per-mode window defaults for two modes only.
- Carry legacy compact window bounds into standard where needed.
- Keep startup/load behavior stable (no flicker regressions).

3. Options/control surface cleanup
- Replace three-option MODE selector with two-option selector.
- Keep keyboard and ARIA behavior intact.
- Remove compact-specific labels/icons.

4. Polish-only follow-ups
- Continue handset/styling/overlay polish work under two-mode assumptions.
- No new architecture branches for mode variants.

## Acceptance

- No user-visible `compact` mode in UI.
- Existing users with compact configs land in standard without losing size preference.
- `handset` behavior remains unchanged.
- TypeScript and Rust config contracts stay in lockstep.
