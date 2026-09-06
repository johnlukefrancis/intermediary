# Intermediary UI Design System

Updated on: 2026-09-06 (Stream panel card grammar and arrival choreography; motion governor carve-out for the stream scroller; deliberate `--ease-spring` arrivals)
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-006

---

## Design Principles

**V2: Vintage Instrument Deck** — The UI reads as a physical device, not a floating web app.

1. **Chassis Frame** — The app window has a visible border frame with subtle accent glow, like a hardware faceplate.
2. **Layered Substrate** — Dot grid + grain texture sits BEHIND content (z-index: 0), adding depth without fuzzing text.
3. **Aggressive Dark** — Base palette is ~35% darker than V1, creating maximum contrast and drama.
4. **Hardware Radii** — Corners are crisper (2-4px) to evoke instruments, not soft rounded app UI.
5. **Per-Tab Substrate** — Each tab modulates the substrate's grid dot color and vignette tint.
6. **Token-Driven** — All visual values flow from design tokens. No hardcoded colors in components.

---

## Token Architecture

### Layer Order (Critical)

CSS imports must follow this order in `app/src/main.tsx`:

```
tokens.css        → Abstract primitives (spacing, radii, blur, shadows, typography, motion)
theme_dark.css    → Fills semantic slots with dark theme values
theme_accents.css → Default accent fallback (runtime values applied via inline styles)
effects.css       → Background gradient, grain, glass utilities
motion.css        → Transition presets, reduced-motion support
boot.css          → Boot phase opacity gate, splash-to-main fade-in
a11y.css          → Focus rings, disabled states, screen reader utilities
badges.css        → Unified badge primitives
main.css          → Layout reset and base structure
[components]      → Component-specific styles
```

### File Responsibilities

| File | Purpose | LOC |
|------|---------|-----|
| `tokens.css` | Spacing, radii, deck radii, blur, shadows, typography, motion, semantic color slots | ~130 |
| `theme_dark.css` | Dark theme values (V2: aggressive), semantic states, glass surface, deck frame/substrate | ~85 |
| `theme_accents.css` | Default accent fallback only (runtime values applied via inline styles in app.tsx) | ~22 |
| `effects.css` | Deck chassis frame, substrate (grid + grain at z:0), vignette, glass/glow utilities | ~80 |
| `boot.css` | Boot phase opacity gate for splash-to-main fade-in transition | ~10 |
| `main.css` | Reset, document sizing, app shell layout | ~90 |

### Global Window Opacity

Intermediary now exposes a global window opacity control in Options -> General:

- **Range:** `0-100`
- **Default:** `100`
- **Storage:** `config.windowOpacityPercent`
- **Runtime vars (set in `app.tsx`):**
  - `--window-opacity-percent`
  - `--window-opacity-alpha` (`percent / 100`)

Theme background and glass tokens consume `--window-opacity-alpha` so opacity applies consistently to deck surfaces while preserving the existing token system.

### Global Texture Intensity

Intermediary also exposes a global substrate texture-intensity control in Options -> General:

- **Range:** `0-100`
- **Default:** `100`
- **Storage:** `config.textureIntensityPercent`
- **Runtime vars (set in `app.tsx`/tokens):**
  - `--texture-intensity-percent`
  - `--texture-intensity-alpha` (`percent / 100`)

Texture intensity is independent from window opacity. The substrate breathe keyframes now compute opacity directly from `--texture-intensity-alpha` so the slider is always authoritative.

---

## Color Palette

### Backgrounds (V2: Aggressive Dark)

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-bg-base` | `#050508` | App background, deepest layer |
| `--color-bg-surface` | `#0a0a10` | Panels, cards, columns |
| `--color-bg-elevated` | `#0f0f18` | Headers, dropdowns, elevated surfaces |
| `--color-bg-hover` | `#151520` | Hover states |
| `--color-bg-active` | `#1a1a28` | Active/pressed states |

### Text

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-text-primary` | `#e8e8ec` | Body text, primary content |
| `--color-text-secondary` | `#9898a8` | Labels, metadata, secondary info |
| `--color-text-muted` | `#606070` | Disabled text, hints |

### Borders (V2: Adjusted for darker palette)

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-border` | `#1a1a28` | Standard borders |
| `--color-border-subtle` | `#12121a` | Subtle dividers |
| `--color-border-highlight` | `rgba(255,255,255,0.06)` | Glass edge highlights |

### Accents (Config-Driven)

Per-tab accent colors are now **config-driven** rather than hardcoded in CSS:

- **Default**: `#c4688a` (dusty rose) when no custom theme is set
- **User-configurable**: Users can set custom accent colors per tab via Options → Theme Colors
- **Storage**: `config.tabThemes[tabKey].accentHex` where tabKey is groupId (for grouped repos) or repoId

**Runtime application** (in `app.tsx`):
1. Compute `activeThemeKey` from the active repo (groupId if grouped, else repoId)
2. Look up `config.tabThemes[activeThemeKey]?.accentHex` or use default
3. Convert hex to CSS variables via `hexToAccentCssVars()` utility
4. Apply as inline style on the `.app` element

**Derived variants** (from `app/src/lib/theme/accent_utils.ts`):
- `--accent-soft`: 15% opacity
- `--accent-glow`: 40% opacity
- `--deck-grid-dot`: 2.5% opacity
- `--deck-vignette-tint`: 3% opacity

**Note**: Both `--accent-*` and `--color-accent-*` variables are set to the same values. This is required because CSS custom properties resolve at definition time — `--color-accent: var(--accent-primary)` in `:root` would capture the root value and not update when inline styles change `--accent-primary`.

### Semantic States

| State | Primary | Soft | Muted |
|-------|---------|------|-------|
| Success | `#4ade80` | `#1a3d2a` | `#2f7a4b` |
| Error | `#f87171` | `#2a1414` | `#6b1d1d` |
| Info | `#93c5fd` | `#1a2a3d` | `#23324d` |
| Warning | `#fbbf24` | `#2a2414` | `#7a5c1d` |

### Activity Telemetry

Activity meters use dedicated semantic tokens instead of component-local colors:

- `--color-activity-idle`, `--color-activity-low`, `--color-activity-mid`, `--color-activity-high`, and `--color-activity-hot`
- `--color-activity-glow`, `--color-activity-hot-glow`, and `--color-activity-row-soft`

Normal rows should stay muted. Hot/rising rows are the only rows that should make the warm/high activity colors visually prominent.

---

## Spacing Scale

Based on 4px unit:

| Token | Value |
|-------|-------|
| `--space-1` | 4px |
| `--space-2` | 8px |
| `--space-3` | 12px |
| `--space-4` | 16px |
| `--space-5` | 20px |
| `--space-6` | 24px |
| `--space-8` | 32px |

---

## Typography

| Token | Value | Usage |
|-------|-------|-------|
| `--font-sans` | System font stack | Body text |
| `--font-mono` | SF Mono, Fira Code, Consolas | Paths, code, metadata |
| `--text-xs` | 10px | Tiny badges |
| `--text-sm` | 11px | Small labels, status |
| `--text-base` | 13px | Secondary text |
| `--text-md` | 14px | Body text |
| `--text-lg` | 16px | Headers |

---

## Shared Workspace

The shared workspace is the clean, full-pane surface used for per-repo notes, opened file scratch buffers, and image previews.

- Standard layout: one workspace panel replaces Auto Files while the Zips panel remains visible. Both states share one `ThreeColumn` shell whose grid class flips, so the rail is never remounted and the ZIPS tree keeps its expansion, selection, and scroll across opening and closing a file.
- Handset layout: the workspace replaces the active deck content until closed.
- Editor text uses `--font-mono`, theme-owned grey editor tokens, and active accent variables for caret, selection, focus rail, and title brackets.
- Notes and Markdown-like text files render a live semantic Markdown layer over the textarea; the textarea remains the editing authority and no rendered HTML is injected.
- Workspace controls use existing deck panel headers and `panel-header-icon` buttons; text file buffers have no save action because they never write back to repository files.
- Opened text-file titles support the same staged drag-out and single-file context-menu actions as Auto Files rows; repo notes do not expose file actions.
- Line and character counts sit inside text editors at bottom-right using muted mono metadata styling.
- Image previews use the same panel footprint, center the image with `object-fit: contain`, and keep drag-out on the preview surface without exposing filesystem paths in the DOM.
- A changed image opened from SOURCE renders as an image diff instead of a single preview: two equal panes split by a divider, each header pairing a plain word with the Git term (`PREVIOUS · HEAD`, `CURRENT · INDEX`, etc.), each image centered on a checkerboard so transparency reads, each footer showing dimensions and bytes; a one-sided change (new or deleted) collapses to one full-width pane headed `NEW · WORKTREE` / `DELETED · INDEX` and so on, an over-bound side shows a `TOO LARGE TO PREVIEW` slot; the panes stack vertically on handset instead of sitting side by side.

## Bundle File Explorer

The Zip Bundles selection surface is a compact file explorer, not a directory-only checklist.

- Root-level files are visible beside top-level directories; expanding a directory lazily fetches only that directory's direct files and subdirectories.
- Directory toggles remain the authority for including top-level directories and excluding nested subdirectories.
- File rows use the same `FileIcon` family/color system as Auto Files rows; the icon is the include/exclude toggle for that file.
- Included file icons carry a strong `currentColor` glow derived from the icon color; excluded files keep the same icon color with lower opacity and a softer glow.
- File-name right-click menus reuse the existing file actions (`Open Containing Folder`, `Open File`, `Copy Relative Path`), and double-click opens through the shared workspace.
- File and directory rows carry Git-status decorations when the active repo has a changed working tree: a
  changed file gets a tinted name plus a trailing `[letter]` badge (the same `CHANGE_BADGES` palette as the
  SOURCE rows — A/U green, M/T info-blue, D red, R/C amber, `!` conflict red); a directory gets a tinted
  name plus a trailing count of distinct changed paths beneath it, colored by the worst change beneath
  (conflict > deleted > modified/renamed/copied/type-changed > new/untracked). Deleted files have no row on
  disk but still count toward and color their directory. Decorations are a pure client-side projection of
  the same source-control status that feeds the SOURCE count; the tree never runs Git, and double-click
  still opens the file — diffs stay in SOURCE.
- Expanded directories re-list in place (keeping expansion state) on `sourceControlChanged` for the repo, so
  a file created inside an already-expanded directory appears with its decoration. A topology refresh
  (`repoTopologyChanged`: any directory create/remove or rename at ≤ depth 4, root-level file
  create/remove; `repo_topology_change.rs`) hands over fresh top-level arrays, and expansion survives it:
  `use_directory_listings.ts` compares their content rather than identity, drops only the expanded
  directories whose top-level ancestor disappeared, and re-lists every surviving expanded directory so a
  subdirectory created inside one appears. Only switching repos collapses the tree.
- The tree is also the inbound drop surface. Tauri's native drag-drop (`dragDropEnabled`) reports OS
  paths and a physical position; `use_tree_drop_import.ts` converts it with `devicePixelRatio` and
  hit-tests `closest('[data-drop-dir]')`. The attribute sits on each directory *wrapper* (row, gaps, and
  children all resolve to the enclosing directory) and on the list (`""` = root), so a file row means its
  containing folder and blank space means the root; file rows carry no attribute of their own.
- While a drag is over the tree the list shows a soft inset accent ring, and the target directory row (or
  the list, for the root) an accent-soft fill with the accent left rail. Hovering a collapsed directory
  for 700 ms expands it — never collapses — and the list auto-scrolls within 28 px of its edges.
- A drop sends `importFiles` with `refuse`; `IMPORT_CONFLICT` opens the shared `ConfirmModal` ("Replace
  N existing files?", up to 8 paths listed, destructive tone) and confirm resends with `replace`. Any
  other failure is an inline `> IMPORT FAILED` notice in the `.build-error` idiom. There is no success
  toast: the file appears with its untracked badge through the ordinary re-list.
- The app's own drag-out re-entering the window (paths under the staging root) is latched at `enter` and
  ignored for the whole gesture, because on Windows those events arrive as a stale burst after the native
  drag ends.
- Rows are selectable: `[data-selected]` draws an accent-soft fill with a 1 px accent inset ring (the
  "selection box"), `[data-cut]` dims a row to 50 % until it is pasted. Click selects, Ctrl toggles, Shift
  ranges in visible order (root: directories then files; inside a directory: files then directories —
  `flatten_visible_tree.ts` encodes that asymmetry). A plain click on a directory also toggles its
  expansion, so the directory name is a span and the checkbox is the only inclusion control. Buttons,
  inputs, and labels inside a row never start a selection. The list is focusable and owns the keyboard map
  (Up/Down, Shift-extend, Left/Right, Enter, Delete, F2, Ctrl+X/C/V, Escape); it stays silent while a modal
  (`[data-intermediary-modal-root]`) is open.
- Row context menus (files and directories) append Cut · Copy · Paste · Rename · Delete after the OS file
  actions, separated by a rule; Delete is error-toned (`destructive`). Blank list space offers Paste into
  the root. Delete opens the shared `ConfirmModal` (destructive) naming the count and the quarantine.
- Moving within the tree is a pointer drag (6 px threshold, pointer capture) with a floating glass ghost
  (`pointer-events: none`) showing the name or "N items"; it shares the OS drop's hit-test, 700 ms
  hover-to-expand, and edge auto-scroll (`tree_drop_targeting.ts`, CSS px in — only the OS hook divides
  physical px). A dragged directory, its descendants, and the entries' current parent are never highlighted.
- Rename swaps the name for an input (focused, text selected); Enter or blur commits, Escape cancels, and
  the input is disabled while the agent answers. Conflicts (`ENTRY_CONFLICT`) are notices for rename and a
  Replace modal for move/copy; folder-over-folder moves, cross-kind collisions, and same-folder copies are
  notices with the agent's reason. After any action the tree forgets a moved directory's stale listings and
  re-lists the affected folders itself.

## Rail, Source Control, and Terminal

The right column is a rail with a slim (~36px) header holding one shared segmented icon rocker
(`DeckSectionSwitcher`): a bordered, elevated cluster of cells — ZIPS an archive-box glyph, SOURCE a
git-branch glyph, TERMINAL a bare prompt glyph (chevron plus cursor bar, `>_`, judged at the rendered 15 px where
a framed variant blurred into a block) — with the active cell lit (accent glyph on a
soft accent fill with glow) and inactive cells muted; the same component drives the handset deck's
rocker, which prepends a stacked-documents FILES cell. The section word survives as the accessible name (screen-reader-only text, so the SOURCE cell reads
"SOURCE 3") and as the `title` tooltip; the bracket `[ ]` idiom stays on panel titles and badges, only the
switcher dropped it. The active rail persists globally (`uiState.activeRail`); the handset FILES choice is
local and the ZIPS/SOURCE choice writes through, so a resize across the 980/860 band never loses SOURCE.
The SOURCE cell shows the change count in accent tabular numerals beside the branch glyph and hides it at
zero.

- Rows fit the 300px workspace-mode rail: `28px minmax(0,1fr) auto` grid, `FileIcon`, name over
  directory (`.auto-files-copy` idiom), a bracket badge (`--add/--modify/--delete/--warning` for rename,
  `--error` for conflict, `--untracked`, `--typechange`), and a hover stage/unstage icon. Deleted rows
  strike the name and disable open actions. `.badge--staged` keeps its drag-handoff meaning and is never
  used for Git state.
- COMMIT uses the build-button language; while in flight the label reads `COMMITTING…` with `aria-busy`
  (the sweep alone is invisible under reduced motion). There is no cancel affordance: mutations are
  deliberately non-cancellable.
- Section headers carry one always-visible `+` / `−` icon action (stage all / unstage all), the same
  glyphs as the row hover actions; sections cap at 500 rows with a `+N MORE` footer in the empty-state
  language.
- Diffs open in the shared workspace as a read-only line grid in the editor shell styling: old/new
  line-number gutters, hunk headers muted, additions success-tinted, deletions error-tinted; `BINARY FILE`
  and `DIFF TRUNCATED AT 2 MiB` notices.
- Merge conflicts are the one alert state: the SOURCE rocker cell turns error-toned with a pulsing halo
  (pseudo-element, so the active inset ring survives) and `!N` (conflict count); the column leads with a
  conflict banner row and the MERGE CONFLICTS section in the error tone; conflict diffs carry the `MERGE
  CONFLICT` subtitle, a pinned notice, and warning-toned marker lines.
- Empty states follow the console prompt style: `> READING WORKING TREE`, `NO CHANGES`,
  `NOT A GIT REPOSITORY`, `GIT NOT FOUND`, `AGENT UPDATE REQUIRED`, `COMMIT RESULT UNKNOWN — REFRESHING`.

### Terminal column

- The TERMINAL cell is the third rail section (fourth on handset, where the terminal fills the chassis).
  The column is a tab strip over the xterm host: `PWSH 1`, `PWSH 2` … buttons in the deck tab idiom (the
  shell's OSC title as tooltip), a `×` per tab, and a trailing `+` (disabled at the twelve-session cap).
  The host fills the remaining height; the session element sits `position: absolute; inset: 0` inside it
  and the `.xterm` padding is the fit inset. Notices use the console prompt style: `> STARTING PWSH`,
  `> PROCESS EXITED · CODE n` with RESTART / CLOSE, `> PWSH FAILED TO START` + the message with RETRY /
  CLOSE, `> NO TERMINAL` with `+ NEW`.
- Rail width: the deck grids are `minmax(0,1fr) <divider> clamp(360px, var(--rail-width), 70%)` (files)
  and `clamp(300px, …)` (workspace), with the column gap replaced by a 16 px `DeckSplitter` column
  (`deck_splitter.tsx` / `deck_splitter.css`): a 2 px grip that lights in accent on hover, focus, and
  while dragging; pointer drag with capture previews `--rail-width` on the grid root and commits on
  release; Left/Right keys step 2 %, Home and double-click reset to the 35 % default. The value persists
  as `uiState.railWidthPercent` (20-70) and is shared by ZIPS, SOURCE, and TERMINAL. Under 768 px the
  grids stack and the divider hides. `repo_rail.css` / `handset_deck.css` drop the body padding and
  overflow for the TERMINAL section so the terminal reaches the panel edge.
- Token slots, not a terminal scheme: each theme file (`theme_dark.css`, `theme_warm.css`,
  `theme_light.css`) defines `--terminal-bg`, `--terminal-fg`, `--terminal-cursor`,
  `--terminal-cursor-accent`, `--terminal-selection`, and the sixteen `--terminal-ansi-*` slots
  (`black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, and their `bright-` forms).
  Every value is derived from existing tokens — reds/greens/blues/yellows from the success / error / info
  / warning states, black/white from the surface and text tokens, magenta, cursor, and selection from the
  accent variables — so the terminal follows the tab accent like every other panel. `--terminal-bg` carries
  `--window-opacity-alpha` and xterm runs with `allowTransparency`, so the substrate shows through.
  `terminal_theme.ts` reads the slots with `getComputedStyle` on `.app` at adopt time and on accent or
  theme-mode changes; no colour is hardcoded in the terminal module.
- Font `var(--font-mono)` at 14 px, cursor bar with blink. Blink follows the motion governor
  (`isForegroundWindow()`), so an unfocused window shows a still cursor.
- Parked sessions sit in an off-screen fixed host (`left: -10000px`, 960×600, `inert`) rather than
  `visibility: hidden` in place: xterm pauses rendering when its element does not intersect the viewport,
  and measurement still works for the next adopt.

## Auto Files

The left deck surface is a single Auto Files table matching `docs/screenshots/unified_files_ref.png`.

- The table spans the former Latest and Active lane width while Zips remains a separate right panel.
- The header exposes three sort modes: Auto, Latest, and Active.
- The same header exposes icon filters for all files, documents, code, and images.
- Image files are a first-class feed kind and use the image icon/color family.
- Favourites are not part of the current UI; legacy starred config remains loadable but ignored by the feed surface.
- Rows show rank, file kind, path, last active time, update count, and one consolidated activity telemetry column.
- The activity column keeps the rough left-to-right waveform as the primary read, with the 24-hour pulse strip tucked into the top-left of the same section as a secondary indicator.
- Bursty or rising files get subtle warm activity treatment without changing row height, and selected rows keep the active accent state.

## Stream panel

STREAM is the default left-panel mode and shares the Auto Files chassis: same `.panel` glass, same header
rocker chrome, same filters. The table is replaced by a flat, scrolling deck of cards that print what an
agent just wrote. The `.panel` keeps the glass; the scroller inside it is flat editor background with no
`backdrop-filter`, because nothing that moves may composite a blur.

- One chassis, `.stream-card[data-kind][data-content][data-static]`: deck stroke, `--radius-deck-sm`,
  `--color-editor-bg`, `contain: layout paint style`, and a 3 px accent spine whose colour carries the
  change class — added → success, modified → info, deleted → error, binary → warning, burst → accent.
  Colour is never the only channel: the `[A]`/`[M]`/`[D]`/`[R]` badge letter, the `+`/`−` glyphs, and the
  strike rule carry the same fact.
- The 24 px head is a fixed grid: badge, `FileIcon`, filename over a head-truncated directory line,
  tabular mono `+N −M`, a baseline chip (`SINCE LAST` / `VS INDEX` / `NEW` / `GONE`), `×N` when edits
  merged, and an absolute clock stamped once at admit. Chips reuse the deck's `--radius-deck-xs`,
  uppercase, `0.06em` letter-spacing idiom.
- Bodies reuse existing grammars rather than inventing surfaces: text uses the shared
  `.diff-line[data-kind]` rows at stream density, images use the image-diff checkerboard pane (one-up on
  add, two-up BEFORE/AFTER on modify), opaque payloads and deletions use one 48 px uppercase mono ghost,
  and notices use the console-prompt idiom (`> RECONNECTED — RESUMING`) already used by empty states.
- Focus follows the deck convention exactly: `outline: 2px solid var(--color-accent)` with
  `outline-offset: 2px`, roving tabindex down the ring, and Escape releases focus and resumes follow.
- Narrow chassis drops detail rather than reflowing: ≤ 980 px collapses the LIVE label to its dot,
  ≤ 380 px drops the clock, ≤ 320 px hides the directory line.

### Stream choreography

Arrivals are the point of the surface, so the Stream is the one place in the deck that spends motion
deliberately. Everything lives in `app/src/styles/stream/stream_motion.css`, loaded last.

- **Transform, opacity and clip only.** No keyframe touches a layout property, so a flood costs the
  compositor and never the layout engine.
- **Every keyframe's base state is its resting state.** "No animation" always renders the final look —
  that is what makes an instant landing (hidden window, reduced motion, an old card) legal rather than a
  second visual design.
- **Four inherited custom properties carry the whole rhythm** — `--stream-enter-duration`,
  `--stream-drop-duration`, `--stream-line-step`, `--stream-chain-step` — rebound on
  `.stream-scroller[data-pressure="calm|busy|flood"]`. Under pressure the choreography compresses instead
  of queueing; at flood the stagger steps are zero and only the duration tokens remain.
- **The grammar:** cards enter on `--ease-out` (translate + fade); a new file unfolds from its top edge;
  an image drops with a slight rotate and settles, cascading `--stream-chain-step` apart for up to three
  arrivals; diff lines print left-to-right one `--stream-line-step` apart; a deletion's strike rule draws
  across each removed line after it prints; the spine sweeps down; an evicted card exits upward; the burst
  count pops.
- **`--ease-spring` (documented "use sparingly") is used here on purpose**, and only here: the new-file
  unfold, the image drop, the thumbnail settle, and the burst count. The overshoot is what makes an
  arrival read as an arrival; every other stream motion stays on `--ease-out`.
- **Cards older than one second carry `data-static`** and drop out of every arrival binding, so
  scroll-back and panel remounts never replay a flood.

---

## V2: Deck Tokens

### Deck Radii (Hardware Feel)

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-deck-none` | 0 | Square corners |
| `--radius-deck-xs` | 2px | Tabs, small controls |
| `--radius-deck-sm` | 3px | Buttons, chips |
| `--radius-deck-md` | 4px | Panels |

### Deck Strokes

| Token | Value | Usage |
|-------|-------|-------|
| `--deck-stroke-hairline` | 1px | Fine details |
| `--deck-stroke-thin` | 2px | Chassis frame |

### Deck Frame

| Token | Value | Usage |
|-------|-------|-------|
| `--deck-frame-inset` | 8px | Substrate margin from window edge |
| `--deck-frame-outer` | `#1a1a24` | Outer frame border color |
| `--deck-frame-inner` | `rgba(255,255,255,0.04)` | Inner highlight stroke |

### Deck Substrate

| Token | Value | Usage |
|-------|-------|-------|
| `--deck-grid-dot` | Per-tab (2-3% opacity) | Dot grid color |
| `--deck-grid-size` | 16px | Dot grid spacing |
| `--deck-grain-opacity` | 0.4 | Grain layer opacity |
| `--deck-vignette-strength` | 0.35 | Edge darkening intensity |
| `--deck-vignette-tint` | Per-tab | Vignette color tint |

---

## Do / Don't

### DO

- Use semantic tokens (`--color-success`) instead of raw hex values
- Apply `.glass-surface` class for frosted panel effects
- Use `var(--color-accent)` for interactive highlights (changes per tab)
- Use `--radius-deck-*` tokens for panel/button corners (V2)
- Test substrate visibility on actual content

### DON'T

- Hardcode hex colors in component CSS
- Use `!important` to override tokens
- Apply backdrop-filter blur to large scrolling areas (performance)
- Mix multiple accent colors within the same component
- Add external image assets for texture effects
- Apply grain as a topmost overlay (causes text fuzzing)

---

## V2: Deck Language Rules (Locked)

These rules define what the V2 deck language is and prevent visual drift.

### What's Forbidden

- `--radius-sm`/`--radius-md`/`--radius-lg` on deck components (use `--radius-deck-*` only)
- Pixel-based letter-spacing (use `em` units: 0.03em tight, 0.05em normal, 0.08em wide)
- Inset focus rings (`outline-offset: -2px`) — always use outset (`2px`) for visibility
- Texture overlays above content (grain/grid must stay at z-index: 0)
- Backdrop-filter on content areas (only on glass surfaces like panels, toasts)

### What's Allowed

- Hardcoded 1-2px micro-spacing for sub-component details (too small for `--space-1`)
- Multi-layer box-shadows for premium button styling (build button pattern)
- Component-specific pixel dimensions (drag handles, toggles, LED dots)
- Inline em-based letter-spacing values (already consistent: 0.03em, 0.05em, 0.08em)

### Must Remain Consistent

- All transitions: `--duration-normal` (150ms) + `--ease-out`
- All focus rings: `outline: 2px solid var(--color-accent); outline-offset: 2px`
- All panels: `--radius-deck-md` corners, multi-layer box-shadow
- All rows: left rail accent with hover glow

---

## Glass Surface Pattern

For frosted glass panels, use the utility class or manual application:

```css
/* Utility class */
.my-panel {
  @extend .glass-surface;
}

/* Manual */
.my-panel {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  box-shadow: var(--shadow-inset-subtle);
}
```

---

## Motion

### Duration Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--duration-instant` | 0ms | Immediate state changes |
| `--duration-fast` | 100ms | Micro-interactions, quick feedback |
| `--duration-normal` | 150ms | Standard transitions (default) |
| `--duration-slow` | 250ms | Deliberate animations |
| `--duration-slower` | 400ms | Page transitions, complex animations |

### Easing Tokens

| Token | Value | Usage |
|-------|-------|-------|
| `--ease-out` | `cubic-bezier(0.33, 1, 0.68, 1)` | Enter animations, appearing elements |
| `--ease-in-out` | `cubic-bezier(0.65, 0, 0.35, 1)` | Symmetric animations, pulsing |
| `--ease-spring` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Bouncy effects (use sparingly). Deliberate exception: Stream card arrivals — new-file unfold, image drop, thumbnail settle, burst count — where the overshoot is the point. |

### Reduced Motion

All animations and transitions respect `prefers-reduced-motion: reduce`. When enabled:
- Animation duration collapses to near-instant (0.01ms)
- Animation iteration count becomes 1
- Transition duration collapses to near-instant
- Scroll behavior becomes `auto`

This is handled globally in `motion.css` — no per-component opt-out needed.

### Motion Governor

**Contract:** ALL animation MUST pause whenever the window is not truly foreground — hidden, minimized, **or visible-but-unfocused** (the user switched to another app). This is not limited to the substrate: decorative animations (`substrate-breathe`, `substrate-drift`) and functional/status animations (connection LED pulse, error marquee, build-progress sweep, waiting-state pulse) all halt so background GPU compositing drops to near-idle when nobody is looking.

- **Foreground test:** `isForegroundWindow()` (`app/src/lib/window/foreground.ts`) = `!document.hidden && visibilityState === "visible" && document.hasFocus()`. Shared with the resume detector so "foreground" has one definition.
- **Detection:** `document.visibilitychange` + window `focus`/`blur` (primary, cross-platform) plus Tauri window focus events (secondary, for Windows edge cases). Any focus loss pauses — not only minimize.
- **CSS gate:** `[data-motion="paused"]` on the `.app` element drives a universal `animation-play-state: paused` rule in `motion.css` (mirrors the reduced-motion block), so every current and future animation is governed without per-component opt-in. The substrate additionally releases `will-change` in `effects.css`.
- **Behavior:** Animations pause in place and resume seamlessly on refocus.
- **Implementation:** `app/src/hooks/use_motion_governor.ts`

**Amendment — the stream scroller (2026-09-06).** The universal rule above splits "hidden" from
"unfocused" for exactly one surface. A second monitor showing an agent at work must keep printing, so
`app/src/styles/stream/stream_motion.css` (loaded after `motion.css`) carves the scroller out of the
pause and tightens the hidden case in its place. This is the only scoped exception to the universal
contract; everything outside `.stream-scroller`, the LIVE dot included, stays governed exactly as before.

```css
/* Visible but unfocused: the stream keeps running; everything outside the scroller still pauses. */
.app[data-motion="paused"] .stream-scroller *,
.app[data-motion="paused"] .stream-scroller *::before,
.app[data-motion="paused"] .stream-scroller *::after { animation-play-state: running !important; }
/* Hidden or minimized: nothing animates and cards render in their resting state. */
.app[data-visibility="hidden"] .stream-scroller *,
.app[data-visibility="hidden"] .stream-scroller *::before,
.app[data-visibility="hidden"] .stream-scroller *::after { animation: none !important; }
```

- **Second gate:** `data-visibility="hidden" | "visible"` is written on `.app` from `document.hidden`
  (the same governor hook reports it), so hidden/minimized and merely-unfocused stop being one state for
  this surface. Cards admitted while hidden land instantly in their resting state.
- **Why `animation: none` is safe:** every stream keyframe rests at the final look, so the carve-out
  never leaves a card mid-transform. Cards older than `STATIC_AFTER_MS` carry `data-static` and are
  outside the arrival bindings entirely.
- **Reduced motion still wins:** `motion.css` collapses the durations globally and the stream sheet zeroes
  its stagger steps on top, so a capped body lands at once instead of rippling in.
- **Design authority:** `docs/design/stream_panel_design.md` § Motion governor amendment.

---

## Responsive Handset Override

UI mode now has two runtime layers:

- **Preferred mode**: persisted `config.uiMode` (`standard` or `handset`) selected by the user in Options.
- **Effective mode**: runtime render mode used by layout/CSS datasets.

Runtime layout responds to window geometry from either preferred mode:

- **Always standard while maximized**
- **Width hysteresis**:
  - Enter standard at `>= 980px`
  - Return to handset at `<= 860px`

Preferred mode remains the baseline intent used when entering the hysteresis deadband, while effective mode follows geometry at threshold crossings. This avoids flapping near breakpoints and keeps the layout responsive during live resize.

Implementation anchors:

- Policy: `app/src/lib/window/effective_ui_mode_policy.ts`
- Runtime hook: `app/src/hooks/use_effective_ui_mode.ts`
- Render switch: `app/src/app.tsx` + `app/src/tabs/repo_tab.tsx`

---

## Accessibility

### Focus Ring Convention

All interactive elements use a consistent `:focus-visible` outline:

```css
.element:focus-visible {
  outline: 2px solid var(--color-accent);
  outline-offset: 2px;
}
```

The base focus state (`:focus`) removes the default outline, and `:focus-visible` adds the styled ring only when keyboard navigation is detected.

### Disabled State Convention

Disabled elements use consistent styling:

```css
.element:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
```

Some components may also add `filter: saturate(0.5)` for accent-colored buttons.

---

## ASCII Cue Pattern

Panel headers use a monospace `:: ` prefix as a subtle terminal-style decoration:

```css
.panel-header::before {
  content: ':: ';
  color: var(--color-text-muted);
}
```

This is the only ASCII decoration currently in use. Keep ASCII cues minimal and consistent.

---

## Implementation Checklist

Phase 1: Foundation (complete)
- [x] Create `tokens.css`
- [x] Create `theme_dark.css`
- [x] Create `theme_accents.css`
- [x] Create `effects.css`

Phase 2: Wiring (complete)
- [x] Update `main.tsx` import order
- [x] Add `data-active-tab` to `app.tsx`

Phase 3: Migration (complete)
- [x] Refactor `main.css` to layout-only
- [x] Migrate `status_bar.css` (fix --bg-hover bug)
- [x] Replace `file_row.css` with `auto_files.css`
- [x] Migrate `bundle_column.css`
- [x] Migrate `offline_banner.css`
- [x] Migrate `drag_error_notice.css`
- [x] Migrate `tab_bar.css`
- [x] Migrate `columns.css`

Phase 4: Documentation (complete)
- [x] Create this design doc
- [x] Update `docs/guide.md`

Phase 5: Polish (complete)
- [x] Create `motion.css` with reduced-motion support
- [x] Create `a11y.css` with focus ring utilities
- [x] Add `--color-warning-muted` token
- [x] Replace hardcoded colors with tokens
- [x] Add `:focus-visible` to all interactive elements
- [x] Delete deprecated `offline_banner.css`
- [x] Document accent variable inheritance behavior

---

## V2 Implementation (complete)

Phase 6: Vintage Instrument Deck
- [x] Add deck tokens (`--radius-deck-*`, `--deck-stroke-*`, `--deck-frame-*`, `--deck-grid-*`)
- [x] Aggressive palette darkening (~35% darker)
- [x] Per-tab substrate hooks (`--deck-grid-dot`, `--deck-vignette-tint`)
- [x] Chassis frame on `.app` (box-shadow: outer border + inner highlight + accent glow)
- [x] Substrate layer at z-index: 0 (dot grid + grain, behind content)
- [x] Vignette layer at z-index: 1 (radial gradient edge darkening)
- [x] Update all component radii to use `--radius-deck-*` tokens
- [x] Update this design doc

Phase 7: QA & Polish (complete)
- [x] Audit all UI controls for vintage deck consistency
- [x] Fix pixel-based letter-spacing (`0.5px` → `0.05em`)
- [x] Fix non-deck radii (`--radius-md/sm` → `--radius-deck-sm`)
- [x] Fix inset focus ring (`outline-offset: -2px` → `2px`)
- [x] Add V2 Deck Language Rules section (locked rules)

---

## Future Enhancements

- Light theme variant (`theme_light.css`)
- Additional accent color presets
- Component-specific glass variants (lighter/heavier blur)
- Alternative substrate patterns (line grid, crosshatch)
