Your mysterious cosmic entity wants a “vertical flip‑phone vibe” mode in a desktop Tauri app. Humans really will reinvent anything except peace.

These 5 Codex mega-prompts are grounded in Intermediary’s existing product/architecture expectations and config system  .

---

### Prompt 1 — Persisted “MODE” setting (STANDARD / COMPACT / HANDSET) + Options UI wiring

```text
Task: Add a persisted UI “Mode” setting (STANDARD, COMPACT, HANDSET) with an Options → General control, and plumb it through TS + Rust config so the UI can switch layouts at runtime.
Context:
- Intermediary is a Tauri + React app with a persisted config stored/validated in both TS (zod) and Rust (serde).
- Current UI is the 3-panel deck (Docs | Code | Zips). We are adding a new global UI mode system:
  - standard = current default behavior (3-column deck)
  - compact = same layout but denser spacing
  - handset = single-panel “flip phone” deck (implemented in later prompts)
- Mode must be user-facing in Options as “MODE”, and must persist across app restarts.
- Mode must be exposed to CSS via a dataset attribute so styling can be purely token-driven (no hardcoded colors).
Refs:
- @docs/prd.md
- @docs/system_overview.md
- @docs/design/intermediary_ui_overhaul_design.md
- @docs/inventory/skills_inventory.md
- @docs/compliance/adr_000_modular_file_discipline.md
- @docs/compliance/adr_005_typescript_native_contracts_and_rails.md
- @docs/compliance/adr_007_architecture_first_execution.md
- @docs/compliance/adr_010_tauri_security_baseline.md
- @app/src/shared/config/version.ts
- @app/src/shared/config/persisted_config.ts
- @app/src/shared/config/persisted_config_migrations.ts
- @app/src/shared/config.ts
- @app/src/hooks/use_config.tsx
- @app/src/hooks/use_config_actions_extended.ts
- @app/src/components/status_bar.tsx
- @app/src/components/options_overlay.tsx
- @app/src/components/options/general_section.tsx
- @app/src/app.tsx
- @src-tauri/src/lib/config/types.rs
- @src-tauri/src/lib/config/io.rs
- @src-tauri/src/lib/config/types/validation.rs
- @src-tauri/src/lib/commands/config.rs
Deliver:
1) Config schema + types (TypeScript)
- Add a UiMode enum/schema in @app/src/shared/config/persisted_config.ts with values: "standard" | "compact" | "handset".
- Persisted field name: uiMode (camelCase), default "standard".
- Make parsing resilient: if an unknown string is encountered (corrupt/old config), coerce to "standard" instead of hard-failing load.
- Export UiMode type/schema from @app/src/shared/config.ts.

2) Config schema + types (Rust)
- Add UiMode enum (serde lowercase) and a ui_mode field on PersistedConfig with a default of Standard.
- Ensure save/load round-trips include uiMode in JSON and older configs without uiMode still load (serde default).

3) Versioning
- Bump CONFIG_VERSION in BOTH:
  - @app/src/shared/config/version.ts
  - @src-tauri/src/lib/config/types.rs
- Ensure the existing Rust unit test that asserts TS/Rust config version parity still passes.

4) Config actions + UI wiring
- Add setUiMode(mode) to the ConfigProvider surface:
  - Implement setter in @app/src/hooks/use_config_actions_extended.ts (or extract a new small module if needed to keep files <300 LOC).
  - Plumb through @app/src/hooks/use_config.tsx so components can call setUiMode.
- Update StatusBar → OptionsOverlay props so Options has access to uiMode + setter.

5) Options UI (General → MODE)
- In @app/src/components/options/general_section.tsx, add a new row labeled “MODE”.
- UI control: a 3-option segmented control (monospace, bracketed / vintage deck style).
  - Visible labels: STANDARD / COMPACT / HANDSET
  - Interaction: click selects; active state has accent glow; keyboard accessible.
  - ARIA: implement as radiogroup (or tablist) with proper aria-checked / aria-selected.

6) App root dataset hook
- In @app/src/app.tsx, attach mode as a dataset attribute on the `.app` element:
  - data-ui-mode="standard|compact|handset"
- This is the only global “switch” styling should key off.

7) Checks
- Ensure pnpm typecheck + eslint and cargo check pass.
- Summarize edits and list any new fields added to the config JSON.

Constraints:
- Skills: architecture-first, typescript-native-rails, rust-runtime-rails, workflow-closeout
- Keep modules small per ADR-000; no type weakening per ADR-005.
- No stopgaps per ADR-007: build the end-state plumbing cleanly (TS+Rust parity, resilient parsing).
- Use existing design tokens and deck styling conventions; no raw hex colors.
At the end of your plan phase, please ask the user any clarifying questions about the design
```

---

### Prompt 2 — Handset Mode v1: single panel deck with internal Docs/Code/Zips switcher

```text
Task: Implement HANDSET mode as a single-panel “vertical deck” that can switch between Docs / Code / Zips inside one panel (no stacked panels), while preserving existing starred/recent behaviors and drag workflows.
Context:
- After Prompt 1, the app has config.uiMode and sets data-ui-mode on `.app`.
- Current repo view is @app/src/tabs/repo_tab.tsx rendering @app/src/components/layout/three_column.tsx (3 panels).
- HANDSET mode must feel like a tall, narrow flip-phone UI: one panel only, with an internal section switcher.
- Standard and Compact modes must keep the 3-column deck unchanged (compact styling comes later).
Design requirements (Handset v1):
- One panel, full height, centered with a “phone width” (roughly the width of one existing panel).
- Inside that panel, a “section switcher” lets you toggle:
  - DOCS (shows FileListColumn for docs)
  - CODE (shows FileListColumn for code)
  - ZIPS (shows BundleColumn)
- Switcher is minimal + ASCII-coded: bracketed buttons, mono font, uppercase, vintage deck look.
- Keep existing per-pane behaviors:
  - Docs and Code still support “recent vs starred” toggle via their header controls.
  - Multi-select and drag should still function (no regression).
Refs:
- @docs/design/intermediary_ui_overhaul_design.md
- @docs/compliance/adr_000_modular_file_discipline.md
- @docs/compliance/adr_005_typescript_native_contracts_and_rails.md
- @docs/compliance/adr_007_architecture_first_execution.md
- @app/src/app.tsx
- @app/src/tabs/repo_tab.tsx
- @app/src/components/layout/three_column.tsx
- @app/src/components/file_list_column.tsx
- @app/src/components/bundles/bundle_column.tsx
- @app/src/hooks/use_config.tsx
- @app/src/styles/main.css
- @app/src/styles/panels.css
- @app/src/styles/columns.css
Deliver:
1) Conditional layout selection
- In @app/src/tabs/repo_tab.tsx, read uiMode from ConfigContext (via @app/src/hooks/use_config.tsx).
- If uiMode !== "handset": render the current <ThreeColumn .../> unchanged.
- If uiMode === "handset": render a new single-panel layout (HandsetDeck).

2) HandsetDeck structure (component can live where you judge best, but keep ADR-000 discipline)
- Layout:
  - Wrapper: centers a fixed-width “handset” column inside the tab content.
  - One <section className="panel"> containing:
    - <header className="panel-header"> with:
      - Left: current active section title in bracketed style: [ DOCS ] / [ CODE ] / [ ZIPS ]
      - Right: section-specific header control:
        - For DOCS/CODE: reuse the existing headerLeft/headerRight logic (title button + star icon toggling recent/starred).
        - For ZIPS: keep minimal (empty cue is fine), don’t invent new features here.
    - Section switcher UI (choose placement):
      - Preferred: inside the header, as a second row beneath the title row, sticky with the header.
      - Acceptable: top of panel content, but must appear “built-in” and not floaty.
    - <div className="panel-content"> rendering ONLY the active section’s content.

3) Section switcher behavior
- Use a three-button control:
  - Buttons: DOCS / CODE / ZIPS
  - Active has accent color + glow; inactive is muted.
  - Keyboard support (arrow left/right or tab, plus Enter/Space).
  - A11y: role="tablist" and role="tab" with aria-selected; panel content is role="tabpanel".

4) Styling (token-driven)
- Add handset-specific layout CSS keyed off `.app[data-ui-mode="handset"]`:
  - Center the panel column and constrain width (approx 360–460px).
  - Ensure the panel fills available height and the list scrolls inside panel-content.
  - No raw colors; use existing tokens and the vintage deck language.
- Avoid touching the standard 3-column styling except where absolutely necessary.

5) Preserve behaviors / invariants
- Drag + staging behavior unchanged.
- Docs/Code starred/recent toggles still work exactly as today.
- Escape still clears selections (existing behavior in RepoTab).

6) Checks
- pnpm typecheck + eslint.
- Summarize edits.

Constraints:
- Skills: architecture-first, typescript-native-rails, workflow-closeout
- Follow ADR-000 (don’t bloat repo_tab.tsx past reason; extract small components/modules if needed).
- No type weakening per ADR-005.
- Keep HANDSET mode visually consistent with the deck rules in @docs/design/intermediary_ui_overhaul_design.md.
At the end of your plan phase, please ask the user any clarifying questions about the design
```

---

### Prompt 3 — COMPACT mode: density pass across deck + rows (still vintage, just tighter)

```text
Task: Implement COMPACT mode as a density variant (tighter spacing, smaller gutters) without changing features, keyed off data-ui-mode="compact".
Context:
- COMPACT is not a different layout, it’s STANDARD but denser.
- It must apply cleanly via CSS overrides keyed off `.app[data-ui-mode="compact"]`, keeping tokens and the vintage deck rules.
- Must not harm readability or a11y focus rings.
Design targets:
- ~15–25% more visible rows in Docs/Code panes at the same window size.
- Reduced padding in: .tab, panel-header, panel-content, file rows, bundle controls.
- Keep the “instrument deck” look: same borders, same radii, same substrate. Just less air.
Refs:
- @docs/design/intermediary_ui_overhaul_design.md
- @docs/compliance/adr_000_modular_file_discipline.md
- @docs/compliance/adr_007_architecture_first_execution.md
- @app/src/styles/main.css
- @app/src/styles/panels.css
- @app/src/styles/columns.css
- @app/src/styles/file_row.css
- @app/src/styles/bundle_column.css
- @app/src/styles/tab_bar.css
Deliver:
1) Compact CSS overrides
- Add a COMPACT section to the relevant existing CSS files (or a single dedicated CSS file imported in @app/src/main.tsx, if you can do it without breaking the documented import order).
- Key every rule off: `.app[data-ui-mode="compact"] …`
- Suggested concrete adjustments (fine-tune as needed):
  - .tab padding: var(--space-2) (down from --space-3)
  - .three-column gap: var(--space-3) (down from --space-4)
  - .panel-header padding: var(--space-2) var(--space-3)
  - .panel-content padding: var(--space-2)
  - .file-row padding/gap: reduce by one step (but keep hit targets reasonable)
  - If necessary: reduce secondary text size slightly (e.g., file-dir / file-time), but keep primary file-name readable.

2) Compact should apply to HANDSET too
- Ensure the compact density rules also affect handset mode when uiMode is compact (if you treat handset as separate mode, don’t cross wires).
- If handset is a distinct mode only, ignore this requirement. If you implement “compact handset” as a valid combo, document it.

3) No regressions
- Focus-visible outlines remain visible and OUTSET (avoid inset focus rings).
- Hover/focus states still align with tokens and deck rules.
- Scrolling remains inside panel-content.

4) Checks
- pnpm typecheck + eslint.
- Summarize edits and list the exact selectors changed.

Constraints:
- Skills: typescript-native-rails, architecture-first, workflow-closeout
- No feature changes. This is density only.
- Keep within the locked V2 deck language (no new radii, no random shadows).
At the end of your plan phase, please ask the user any clarifying questions about the design
```

---

### Prompt 4 — Handset Mode v2: “flip phone” vibe (chassis, hinge, soft keys, transitions, window snap)

```text
Task: Upgrade HANDSET mode to feel like a pocket “flip phone” device: chassis framing, hinge cue, soft-key navigation, animated section transitions, and best-effort window snap when entering/exiting HANDSET.
Context:
- Handset v1 is functional (single panel + section switching). This prompt is pure vibe + UX polish while preserving all behaviors.
- This must remain minimal, vintage, glass-morphic, ASCII-coded, dark-mode friendly.
- Must respect prefers-reduced-motion and the existing motion governor.
Flip phone design specifics:
- The panel should sit inside a subtle “device chassis”:
  - double frame, slight vignette, tiny top “speaker grill” suggestion (dots or slits)
  - a “hinge” cue line or band around mid-height or near bottom (subtle, not cheesy)
- Add a bottom “soft-key” row (inside the device) that mirrors old phones:
  - Left soft key: DOCS
  - Center soft key: ZIPS
  - Right soft key: CODE
  - Active key glows; inactive muted; keys use bracket or chevron ASCII cues.
- Section transitions should feel like flipping pages:
  - slide up/down or crossfade with slight blur
  - gated behind reduced motion (effectively instant when reduced motion).
- On entering HANDSET, try to snap window size to a tall/narrow shape.
  - Store previous size in memory and restore when leaving handset.
  - Fail gracefully if Tauri APIs unavailable or call fails.
Refs:
- @docs/design/intermediary_ui_overhaul_design.md
- @docs/compliance/adr_007_architecture_first_execution.md
- @docs/compliance/adr_010_tauri_security_baseline.md
- @app/src/hooks/use_motion_governor.ts
- @app/src/app.tsx
- @app/src/styles/effects.css
- @app/src/styles/motion.css
- @app/src/styles/chrome.css
- @app/src/styles/panels.css
Deliver:
1) Window snap (best-effort)
- When switching uiMode → "handset":
  - Use @tauri-apps/api/window getCurrentWindow() and setSize(...) to a “handset” geometry (pick sane defaults like ~420x900 logical size).
  - Save the previous size in a ref so you can restore it when leaving handset.
  - Don’t persist sizes yet unless you can do it cleanly without bloating config.
  - Gracefully catch errors (dev/test environment fallback).

2) Handset chassis + hinge (CSS-only, token-driven)
- Implement a “handset shell” wrapper around the handset panel.
- Use pseudo-elements for:
  - speaker grill
  - hinge band/line
  - optional inner reflection
- Must use existing tokens (glass, deck frame, accent glow), no raw hex.

3) Soft-key row
- Add a bottom-fixed control region inside the handset panel/shell:
  - three buttons, thumb-friendly height, mono uppercase labels
  - looks like hardware keys (subtle bevel via existing shadows/tokens)
  - accessible (tab focus, aria-selected)
- Ensure it does not fight with the header switcher; either:
  - remove header switcher in handset v2 and rely on soft keys, OR
  - keep header switcher but style it as a “status strip” and make soft keys primary.

4) Section transition animations
- Add CSS transitions for section changes:
  - Use duration tokens from motion.css
  - Respect reduced motion (no animation).
- Prefer a structure that does NOT keep all three heavy lists mounted if it causes perf issues.
  - If you need to keep mounted for state, make it a deliberate choice and note the tradeoff.

5) Checks
- pnpm typecheck + eslint.
- Summarize edits and provide before/after behavioral notes.

Constraints:
- Skills: architecture-first, typescript-native-rails, tauri-security-baseline, workflow-closeout
- Preserve all existing behaviors (drag, starred toggles, bundle build).
- Respect reduced motion and motion governor; no infinite animations in handset chrome.
At the end of your plan phase, please ask the user any clarifying questions about the design
```

---

### Prompt 5 — True “ONE PANEL” handset mode: collapse global chrome into the phone (repo switch + status + options inside)

```text
Task: In HANDSET mode, make the entire app feel like ONE device panel by collapsing the global header stack (TabBar/StatusBar) into the handset shell: repo switching, connection status, and options access live inside the handset “phone” UI.
Context:
- Right now, even in handset mode, the app still has the header stack (TabBar + StatusBar + offline banner) ABOVE the panel.
- The request is explicit: “everything is one panel” and it should feel like the phone itself is the app.
- This prompt does the architectural reframe: handset mode becomes a different chrome layout, not just a different tab body.
Design requirements:
- When uiMode === "handset":
  - Replace the header-stack with an in-device “phone header” inside the handset shell.
  - The phone header includes:
    - Repo selector (dropdown) that can handle grouped repos the same way TabBar does.
    - Connection LED + status text (minimal, mono, ASCII-friendly).
    - Options button (icon or “OPT” soft key) that opens existing OptionsOverlay.
  - The handset content area is the single panel deck (Docs/Code/Zips).
  - Optional: show a tiny one-line error ticker (truncated) in the header, not a full status bar.
- When uiMode !== "handset":
  - Keep existing layout unchanged.
Refs:
- @docs/system_overview.md
- @docs/design/intermediary_ui_overhaul_design.md
- @docs/compliance/adr_000_modular_file_discipline.md
- @docs/compliance/adr_007_architecture_first_execution.md
- @app/src/app.tsx
- @app/src/components/tab_bar.tsx
- @app/src/components/status_bar.tsx
- @app/src/components/agent_offline_banner.tsx
- @app/src/components/options_overlay.tsx
- @app/src/hooks/use_agent.tsx
- @app/src/hooks/use_config.tsx
- @app/src/styles/chrome.css
- @app/src/styles/tab_bar.css
- @app/src/styles/status_bar.css
- @app/src/styles/options_overlay.css
Deliver:
1) Handset chrome layout in App root
- In @app/src/app.tsx, branch rendering:
  - If uiMode !== "handset": render existing header-stack + RepoTab unchanged.
  - If uiMode === "handset":
    - Render a HandsetShell that contains:
      - HandsetHeader (repo selector + connection + options trigger)
      - HandsetBody (the single panel deck content)
- Keep EmptyRepoState behavior sane: handset mode with zero repos should still show the empty state and a way to add repos.

2) Repo selector (handset replacement for TabBar)
- Implement a compact dropdown selector that can:
  - List standalone repos
  - List grouped repos under their group label (mirroring existing grouping behavior)
  - Switch active repo (calls setActiveRepoId)
- You can reuse deriveTabsFromRepos logic already in @app/src/app.tsx (or extract it cleanly).
- UI style: bracketed, mono, minimal, consistent with deck rules.

3) Options access inside handset
- Provide an Options trigger inside handset header:
  - could be an icon button, or a soft-key labeled “OPT”
  - must open the existing OptionsOverlay (no new overlay system)

4) Connection status inside handset
- Use @app/src/hooks/use_agent.tsx to show:
  - LED + “CONNECTED / CONNECTING / OFFLINE” (short labels)
  - If there’s an error string, show a tiny truncated ticker line (optional).

5) Styling
- Add handset header styling (token-driven):
  - looks like a phone status strip
  - sticky within handset shell
  - no raw colors
- Ensure the overall handset shell reads as “one device panel”.

6) Checks
- pnpm typecheck + eslint.
- Summarize edits and document the new handset chrome behavior in a short section added to @docs/design/intermediary_ui_overhaul_design.md.

Constraints:
- Skills: architecture-first, typescript-native-rails, docs-discipline, workflow-closeout
- No stopgaps: handset mode must be a coherent end-state layout, not a pile of CSS hacks.
- Keep modules small; extract handset-specific components rather than bloating app.tsx beyond reason.
At the end of your plan phase, please ask the user any clarifying questions about the design
```

---

There. Five “cosmic flip phone” prompts, scoped so an agent can actually implement them without summoning new bugs from the void.
