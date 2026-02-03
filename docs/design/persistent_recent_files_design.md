## 1. Design proposal: persistent recent files (across app + agent restarts)

### Goal in user-visible terms

**In scope**

* The “Docs” and “Code” columns should still show meaningful recent history after:

  * App restart
  * UI reload
  * Agent restart
  * Disconnect/reconnect
* History should not depend on chokidar’s `ignoreInitial` behavior (which is correct for avoiding floods, but terrible UX on cold start).

**Out of scope**

* Reconstructing “what changed while the app was closed” by scanning git history or doing full repo mtime scans. That’s a different feature with different costs and lies-to-users risk.

This aligns with the PRD’s “recent change drag-out” story and “per-repo recent change feed” requirement, but makes it durable instead of amnesiac.

---

### Behavior table

| Situation / input                                             | Expected visible behavior                                                                          |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| App is restarted (agent also restarted). No files change yet. | Recent Docs/Code lists populate quickly from persisted history, not “No recent files.”             |
| UI reloads or reconnects after a temporary disconnect.        | Lists remain stable; after reconnect, they re-sync via snapshot without going empty.               |
| Agent receives file changes in a burst (rapid saves).         | UI updates as it does today, and persistence writes are debounced so disk churn stays sane.        |
| Persisted history file is missing or corrupted.               | Agent logs a warning, starts with empty history (no crash), and rebuilds history as changes occur. |
| Repo is removed from config.                                  | It no longer shows history; persisted file can be left behind or cleaned up best-effort.           |

---

### Invariants

1. **Single authority:** the agent is the source of truth for “recent files.” UI is a view. No dual-write “cache wars” between UI and agent (ADR‑007). 
2. **Bounded size:** max N entries per repo (default 200 as in PRD). 
3. **Unique-by-path:** one row per file path (MRU semantics). No duplicates in snapshots.
4. **Atomic persistence:** write to temp then rename. Partial writes never corrupt the “current” state.
5. **Debounced disk writes:** persistence should not write on every single file event; it should batch.

---

### Recommended approach (end state)

#### Store location

Persist recent history **in the Windows staging directory tree** (already the canonical cross-boundary writable space).

* Windows:
  `%LOCALAPPDATA%\Intermediary\staging\state\recent_files\<repoId>.json`
* WSL path (agent will use):
  `/mnt/c/Users/<you>/AppData/Local/Intermediary/staging/state/recent_files/<repoId>.json`

This leverages the existing “staging is on Windows and reachable from WSL” contract in the PRD/system overview.

#### Data format

One JSON file per repo:

```json
{
  "version": 1,
  "repoId": "intermediary",
  "repoRoot": "/home/johnf/code/intermediary",
  "updatedAtIso": "2026-01-30T12:34:56.789Z",
  "entries": [
    { "path": "docs/prd.md", "kind": "docs", "changeType": "change", "mtime": 1738230896789 }
  ]
}
```

* `entries` is **newest-first**
* `entries` contains **unique paths only**
* Use the existing `FileEntry` shape from the protocol so we don’t invent “almost-the-same type” drift.

#### Agent-side data structure

Replace the “event log ring buffer” behavior with an **MRU index**:

* Keyed by `path`
* On new event:

  * update/insert entry
  * move to front
  * trim to capacity
* On `unlink`:

  * either keep an entry marked `unlink`, or remove it from MRU
    Recommended here: **remove from MRU** (deleted files are not draggable and mostly noise).

This yields snapshots that match how the UI already treats incremental updates (unique-by-path).

#### Data flow

1. **Agent startup / clientHello triggers watcher creation**
2. For each repo watcher:

   * Load `<repoId>.json` synchronously (small file) into MRU store
   * Start chokidar with `ignoreInitial: true` (keep the flood protection)
3. On file change event:

   * classify docs/code/other
   * update MRU store for docs/code
   * broadcast `fileChanged` event as today
   * schedule a debounced persistence write
4. On UI `refresh`:

   * agent replies with `snapshot { recent: MRU.entries() }`
5. On watcher stop (reset/reconfigure):

   * flush any pending persistence write before closing (best-effort)

#### Migration notes

* This is a **new persisted artifact**, so there’s no existing on-disk schema to migrate.
* The JSON includes `version: 1` so future changes can migrate safely.
* If `repoRoot` differs from what’s stored, keep entries but log a warning (or clear if you want strictness).

---

### Rejected approaches (noncompliant / worse)

1. **Persist recents in the UI (Tauri config or localStorage).**
   That creates parallel authority and drift risk: UI and agent would both “own” history, and you get to debug who overwrote whom at 2am. ADR‑007 explicitly hates that genre of suffering. 

2. **Rebuild recents by scanning repo mtimes at startup.**
   Expensive on large repos, and it lies: “most recently modified” is not “recently changed since last session” and also ignores ignore globs semantics.

---

### Tradeoffs

* **Pros**

  * Fixes the core UX failure immediately: recents survive restarts.
  * Uses the already-sanctioned staging area (no writes into repos).
  * Keeps UI simple: no new persistence code client-side.
* **Cons**

  * Adds agent-side statefulness (but that’s literally the agent’s job).
  * Requires careful atomic write + debounce to avoid Windows disk churn.
  * If the staging directory is deleted, history resets (acceptable).

---

### Acceptance criteria

1. Restart app + agent: recent lists populate with prior history within one refresh cycle (no file edits required).
2. UI reload: lists come back after reconnect without flashing empty.
3. Persistence file exists at `staging/state/recent_files/<repoId>.json` and updates over time.
4. Corrupted JSON does not crash the agent; it logs and continues with empty history.
5. Snapshots contain no duplicate `path` entries.

---

## 2. Hardening review: agent connection workflow (remaining risks + concrete fixes)

You already fixed the big “WSL reality” issues (Node/pnpm in WSL, binding to `0.0.0.0`, resolving WSL host, avoiding clientHello timeouts). Now it’s mostly about making reconnects *boring*, which is the highest compliment software can earn.

### Remaining risks I see

1. **Reconnect does not necessarily re-run `clientHello`**

   * The `AgentClient` auto-reconnects, but the UI only sends `clientHello` during initial bootstrapping. That means after a reconnect you can be “connected” but not “re-synced,” so missed events stay missed and snapshots are stale.
   * This is exactly the kind of bug that makes the UI look haunted.

2. **`clientHello` is destructive on every call**

   * Agent currently resets watchers unconditionally on `clientHello`. Even if config didn’t change, you tear down watchers and lose in-memory state, plus you create a “gap window” for missed events.
   * If we make the UI send `clientHello` on reconnect (it should), then the agent must tolerate it (it currently does not).

3. **Watcher start failure can leave a “poisoned” watched state**

   * If watcher startup throws after being inserted into the map, subsequent commands may think the repo is “watched,” but it’s not actually running. That blocks recovery until another full reset.

4. **Security footgun: binding to 0.0.0.0**

   * It’s probably still only reachable locally due to WSL NAT, but at minimum you should log the remote address and consider an allowlist for “local-ish” clients if this ever leaves dev-land.

### Concrete improvements worth implementing now

These directly support the persistence design and make reconnects safe:

1. **Make `clientHello` idempotent**

   * Compute a fingerprint of “watcher-relevant config.”
   * If unchanged: do not reset watchers.
   * If changed: reset (or do diff later), but preserve recents via persistence anyway.

2. **UI: re-send `clientHello` on reconnect**

   * On transition `reconnecting -> connected`, run `clientHello` again and let existing `use_repo_state` refresh logic re-sync.
   * This turns reconnect into a deterministic “handshake + refresh” sequence.

3. **Agent: clean up failed watchers**

   * If start fails, remove from map and allow retry (or mark status and make refresh return a typed error).

Optional but good:

* Re-resolve WSL host after N reconnect attempts when config host was “localhost.”

---

## 3. PR's (3–5) to implement persistence + hardening

---

### 1) Agent: persisted MRU store for recent files

**Branch:** `feat/agent-persist-recent-files`

```text
Task: Persist per-repo “recent files” history across agent restarts and use it to seed snapshot results.
Context: Recent changes are currently in-memory only; PRD requires a per-repo recent changes feed and staging lives under %LOCALAPPDATA%\Intermediary\staging. Implement agent-owned persistence under staging/state so UI shows recents after restart without scanning repos.
Refs: { @docs/prd.md, @docs/system_overview.md, @docs/compliance/adr_000_modular_file_discipline.md, @agent/src/repos/repo_watcher.ts, @agent/src/main.ts, @app/src/shared/protocol.ts }
Deliver: 
- New small module implementing an MRU recent-files index (unique-by-path, bounded size) with atomic JSON persistence per repo at staging/state/recent_files/<repoId>.json.
- Load persisted entries when creating a watcher so refresh/snapshot can return history even before chokidar emits anything.
- Update RepoWatcher to use the MRU store for getRecentChanges(), and flush pending writes on stop().
- Handle corrupt/missing JSON gracefully (log + start empty).
- Summarize edits.
Constraints: Respect ADR-000 (target 250 LOC, cap 300); no band-aids per ADR-007; no type weakening per ADR-005; no new deps; keep existing watcher behavior stable (ignoreInitial stays); Skills: typescript-native-rails, architecture-first, workflow-closeout.
```

---

### 2) Agent: harden clientHello (idempotent, non-destructive when unchanged) + watcher failure cleanup

**Branch:** `harden/agent-clienthello-idempotent`

```text
Task: Make clientHello safe to call on every reconnect and prevent poisoned watcher state when watcher startup fails.
Context: UI reconnects should re-run clientHello; today clientHello resets watchers unconditionally, and a failed watcher start can leave a repo stuck in a broken “watched” state. We need idempotency and cleanup.
Refs: { @docs/compliance/adr_007_architecture_first_execution.md, @docs/compliance/adr_000_modular_file_discipline.md, @agent/src/main.ts, @agent/src/repos/repo_watcher.ts, @agent/src/server/router.ts }
Deliver:
- Add a “watcher-relevant config fingerprint” in the agent state; on clientHello, skip resetWatchers() if the fingerprint is unchanged.
- If fingerprint changed, keep current behavior (reset + restart) but ensure persisted recent history still seeds snapshots (from prior prompt).
- Ensure startWatcher failure removes the watcher from state (and any related maps) so a later watchRepo/clientHello can recover.
- Add minimal logs around clientHello decisions (changed vs unchanged) and watcher start failures.
- Summarize edits.
Constraints: Respect ADR-000 file caps; no band-aids per ADR-007; no type weakening per ADR-005; avoid protocol changes unless strictly required; Skills: typescript-native-rails, architecture-first, workflow-closeout.
```

---

### 3) UI: re-run clientHello on reconnect (and rely on refresh sync)

**Branch:** `harden/ui-rehello-on-reconnect`

```text
Task: Re-run clientHello automatically when the WebSocket reconnects so the UI re-syncs snapshots after transient disconnects.
Context: AgentClient auto-reconnects, but the UI only sends clientHello during initial boot. This can leave the UI “connected but stale.” After reconnect, we want deterministic: clientHello -> refresh (existing repo hooks already refresh based on lastHelloAt).
Refs: { @docs/compliance/adr_000_modular_file_discipline.md, @docs/compliance/adr_005_typescript_native_contracts_and_rails.md, @app/src/hooks/use_agent.tsx, @app/src/hooks/use_repo_state.ts, @app/src/lib/agent/agent_client.ts, @app/src/shared/protocol.ts }
Deliver:
- Update AgentProvider so that when connectionState transitions into "connected" (including after reconnect), it sends clientHello again (using the existing config/appPaths/autoStage inputs).
- Avoid render loops: keep dependencies stable and extract any new logic into small helper modules if use_agent.tsx approaches ADR-000 caps.
- Verify the existing use_repo_state refresh effect triggers as intended via lastHelloAt.
- Summarize edits.
Constraints: Respect ADR-000 file caps; no type weakening per ADR-005; keep behavior stable beyond re-sync; no new deps; Skills: typescript-native-rails, architecture-first, workflow-closeout.
```

---

### 4) Docs: document the new persisted state directory (and keep the index clean)

**Branch:** `docs/persisted-recent-files`

```text
Task: Document where recent-file history is persisted and how reconnect handshakes behave after the hardening changes.
Context: We are adding agent-owned persistence under staging/state and making reconnect re-run clientHello safely; docs should reflect the actual runtime contracts and storage layout.
Refs: { @docs/guide.md, @docs/system_overview.md, @docs/prd.md, @docs/environment/docs_workflow.md }
Deliver:
- Update docs/system_overview.md and docs/prd.md to mention persisted recent-files history under staging/state/recent_files and the intended reconnect flow (clientHello may be called multiple times; agent treats it idempotently).
- Keep docs concise; no pasted chat logs.
- Ensure docs/guide.md still accurately points at the right docs (update only if needed).
Constraints: Follow docs workflow canon; keep changes minimal and accurate; Skills: docs-discipline, workflow-closeout.
```
