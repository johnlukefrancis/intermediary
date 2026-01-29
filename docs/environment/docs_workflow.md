# Docs Workflow Canon — TexturePortal

Updated on: 2026-01-16
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-006, ADR-007

## 1. Purpose & scope

- Canon for how documentation is written and organized in TexturePortal.
- Applies to all documentation in `docs/` (Design, Implementation, Architecture, Reports, Inventory, Environment, Usage, ADRs).
- Use `docs/guide.md` as the index, and follow the naming + content rules below.

## 2. Naming + placement rules

All docs live under `docs/` and use a type suffix at the end of the filename.

- Design → `docs/design/<thing>_design.md`
- Implementation → `docs/implementation/<thing>_implementation.md`
- Architecture → `docs/architecture/<thing>_architecture.md`
- Report → `docs/reports/<thing>_report.md`
- Inventory → `docs/inventory/<thing>_inventory.md` (or `file_ledger.{md,json}` for ledger)
- Environment → `docs/environment/<thing>.md`
- Usage → `docs/usage/<thing>.md`
- ADRs → `docs/compliance/adr_###_<short_title>.md`
- System overview → `docs/system_overview.md` (root-level, no suffix)

Archived docs live in `docs/archive/<type>/` with the same suffix rules.

## 3. Required header for authored docs

Every authored doc must include this header (top of file):

```
# <Title>
Updated on: YYYY-MM-DD
Owners: <names/roles>
Depends on: ADR-000, ADR-006 (and any other relevant ADRs)
```

## 4. Doc types: purpose + content

| Type | When | Purpose | Minimum content |
| --- | --- | --- | --- |
| Design | Before implementation | Behavior, goals, non-goals, user-visible outcomes | Problem, Goals, Non-goals, MVP, Behavior table, Acceptance |
| Implementation | Before coding | PR ladder + verification plan | PR ladder + verification checklist |
| Architecture | After ship | Shipped system as it works today | Ownership, lifecycle, invariants, failure modes |
| Report | Investigations | Findings + actions + references | Context, Findings, Actions, References |
| Inventory | Periodic | Index of modules/files | File/module list + upkeep rules |
| Environment | Process rails | Agent workflow + commands | Rules + required steps |
| Usage | User workflows | External usage steps | Step-by-step instructions |
| ADR | Decisions | Constraints and irreversible choices | Context, Decision, Consequences |

## 5. When to archive

- Move stale or superseded docs to `docs/archive/<type>/` instead of deleting.
- Update `docs/guide.md` when moving/archiving docs.

## 6. Always check the canon

- Before writing or editing docs, open **both** `docs/guide.md` and this file.
- Adhere to ADR-000 and ADR-006 while editing.
