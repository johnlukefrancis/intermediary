# ADR-005: TypeScript Native Contracts and Rails

Status: Accepted
Date: 2026-01-07
Owners: JL · Coding agents
Scope: app/src TypeScript + frontend contracts

---

## Context
Blankstop ships a TypeScript frontend from day one. We want tight, readable types
and no “make the compiler happy” weakening.

---

## Decision
1) **TypeScript is the runtime authoring language**
- All frontend runtime code lives in `app/src/**/*.ts`.
- Do not introduce new JS runtime files unless explicitly required.

2) **Types are contracts, not noise**
When TS flags an error, prefer:
- Fix runtime logic to match intended behavior.
- Tighten types to reflect reality.
- Use a local escape hatch only at true dynamic seams.

3) **Escape hatches are local and explicit**
- `any`/`unknown` only at genuine dynamic boundaries (IPC payloads, external APIs).
- Must include `// TODO(ts-precision): <why dynamic>` at the cast site.

4) **No type weakening**
Disallowed fixes:
- Widening specific unions to `string`/`any`.
- Making required fields optional to silence errors.
- Replacing known shapes with `Record<string, any>`.

5) **Ambient types are deliberate**
- Use `.d.ts` only for ambient declarations or generated types.
- Keep ambient globals minimal and documented.

---

## Invariants
- I5.1: Runtime TS stays in `app/src/**`.
- I5.2: No weakening types to make TS happy.
- I5.3: Escape hatches require TODO + local scope.
