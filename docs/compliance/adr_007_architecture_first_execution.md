# ADR-007: Architecture-First Execution (No Band-Aids)

**Status**: Accepted
**Date**: 2025-01-10

---

## Decision

When a system cannot reliably support required behavior, the correct response is to **change the system into what it needs to be** — not to coax it with hacks.

### D1) No temporary patches
- No "temporary relief" changes meant to be removed later
- Intermediate steps are only allowed if they're on the direct path to the final architecture

### D2) Fix at the contract level
- For any bug: identify the violated contract/invariant
- Fix by restoring correct ownership, not by adding special cases

### D3) Change structure when necessary
- If existing structure prevents a correct solution, rewrite the structure
- Don't work around it with delays, shadow state, or parallel lifecycles

### D4) Assume agentic throughput
- Don't downscope correctness because "it's a lot of work"
- Choose designs that keep the codebase coherent at scale

---

## What's Non-Compliant

A change is a band-aid if it:
- Adds a parallel authority ("also track state here just in case")
- Uses timing-based luck ("delay/retry until it works")
- Special-cases a symptom without fixing the underlying contract
- Adds glue code stated as "temporary" or "for now"

---

## Consequences

- More up-front refactors; fewer recurring bugs
- Less "layering fixes on top of fixes"
- Agents are authorized to do correct rewrites when architecture demands it
