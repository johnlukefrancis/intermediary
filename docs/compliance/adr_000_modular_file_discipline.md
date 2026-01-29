# ADR-000: Modular File Discipline

**Status**: Accepted
**Date**: 2025-01-10

---

## Decision

We adopt hard constraints on file size and module scope:

1. **No monoliths** — Each file should have a single, clear purpose.
2. **Target 250 LOC, hard cap 300** — If a file exceeds this, split it.
3. **Language-appropriate naming** — Follow conventions for the stack (snake_case, PascalCase, etc.)
4. **PascalCase types/classes** — `TextureProcessor`, `PortalManager`

---

## Invariants

### File Length
- **I1.1**: Target ≤250 LOC; hard cap 300 LOC (including blanks and comments)
- **I1.2**: If a file exceeds 300 LOC and you're adding code, split it as part of the change

### Module Scope
- **I2.1**: Each module has a single, clear responsibility (one-sentence summary)
- **I2.2**: Folders with 10+ sibling modules should sprout subfolders by concern

---

## Practical Rules

### When adding new behavior
1. If the target file is under 250 LOC, add there
2. If near/over 300 LOC, create a new module and import it
3. Keep orchestrators thin — they wire things, not implement everything

### When a file is already too large
1. Identify natural seams (processing, display, IO, helpers)
2. Extract into focused files: `big_thing` → `big_thing_core`, `big_thing_io`
3. Don't make it worse — any addition should move toward smaller modules

---

## Context

This prevents the codebase from accumulating God files that do too many jobs, which:
- Slows down human comprehension
- Hurts AI coding agents
- Makes testing harder
- Creates merge conflicts

Small, focused modules are easier to understand, test, and modify.
